// oxlint-disable no-console
// Compares BabelTransformComponentStyles with the crate's two-phase `componentStyles` pass. The real
// plugin runs, wrapped to record each template it replaces: its tag, first quasi, whether it is in a
// class, and the object it became. The crate's phase 1 listing must match the record, and its phase 2,
// given the recorded objects as the JS results, must produce Babel's output.
// Usage: node tasks/tightscript/compare-component-styles.mjs
import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import {
  babelParses,
  babelTransform,
  comparePass,
  corpus,
  loadPlugin,
} from "./tungsten-compare.mjs";

const { MakeBabelTransformComponentStyles } = await loadPlugin("BabelTransformComponentStyles");

const names = (object) =>
  Object.fromEntries(
    object.properties.map((property) => [property.key.value, property.value.value]),
  );

// The JS result for a template, read back from the object Babel replaced it with.
function resultFrom(object) {
  const [moduleName, tags, classes, keyframes] = object.properties.map(
    (property) => property.value,
  );
  return {
    moduleName: moduleName.value,
    tags: names(tags),
    classes: names(classes),
    keyframes: names(keyframes),
    stylesheet: `i/${moduleName.value.replaceAll("#", "")}.css`,
  };
}

// The plugin, recording into `entry.templates` each template it replaced.
function recording(entry, makePlugin) {
  entry.templates = [];
  return (api) => {
    const plugin = makePlugin(api);
    const visit = plugin.visitor.TaggedTemplateExpression;
    return {
      ...plugin,
      visitor: {
        ...plugin.visitor,
        TaggedTemplateExpression(path, state) {
          const { node } = path;
          const seen = {
            index: entry.templates.length + 1,
            tag: node.tag.name,
            raw: node.quasi.quasis[0]?.value.raw,
            inClass: path.findParent((parent) => parent.isClassBody()) !== null,
          };
          visit.call(this, path, state);
          if (path.node !== node) entry.templates.push({ seen, result: resultFrom(path.node) });
        },
      },
    };
  };
}

const dir = mkdtempSync(join(tmpdir(), "tungsten-component-styles-"));
try {
  const plugins = new Map();
  const pluginFor = (root) => {
    if (!plugins.has(root)) {
      plugins.set(
        root,
        MakeBabelTransformComponentStyles({
          projectAbsCodeDir: root,
          projectAbsSourceDir: root,
          projectAbsStyleImportDir: root,
          classNameFormat: "[folder]-[name]_[local]",
          cssOutputFile: join(dir, String(plugins.size), "ccss.css"),
          watch: false,
        }),
      );
    }
    return plugins.get(root);
  };

  const synthetic = {
    root: "C:/synthetic",
    file: "C:/synthetic/pages/Card.jsx",
    source: [
      "export class Card {",
      "  style = Css`.box { color: red } div { margin: 0 }`",
      "  render() { return Css`.a { } ${1} b` }",
      "}",
      "class Outer extends base(Css`.e { }`) { x = class { y = Css`.g { }` } }",
      "const outer = Css`.o { } ${Css`.inner { }`}`",
      "const ignored = [css`x`, Other`y`, obj.Css`z`]",
      "export { Outer, outer, ignored }",
    ].join("\n"),
  };
  const all = [
    ...corpus().map(({ root, file }) => ({ root, file, source: readFileSync(file, "utf8") })),
    synthetic,
  ];

  // Babel runs first so the crate's phase 2 can be given what Babel computed
  const failed = [];
  const files = all.filter((entry) => {
    if (!babelParses(entry.file, entry.source)) return true;
    try {
      babelTransform(entry.file, entry.source, [recording(entry, pluginFor(entry.root))]);
      return true;
    } catch (error) {
      failed.push(`${entry.file}: ${error.message.split("\n")[0]}`);
      return false;
    }
  });

  const tally = comparePass({
    name: "component-styles",
    files,
    options: (entry) => ({
      componentStyles: { results: (entry.templates ?? []).map(({ result }) => result) },
    }),
    makePlugins: (entry) => [recording(entry, pluginFor(entry.root))],
    checkOutput: (entry, output) => {
      const expected = JSON.stringify(entry.templates.map(({ seen }) => seen));
      const actual = JSON.stringify(
        output.componentStyles.map(({ index, tag, raw, inClass }) => ({
          index,
          tag,
          raw,
          inClass,
        })),
      );
      return expected === actual
        ? undefined
        : `Babel replaced ${expected}, the crate listed ${actual}`;
    },
  });
  const ok = tally.print(`componentStyles, ${all.length} files`);
  const templates = files.reduce((n, entry) => n + (entry.templates?.length ?? 0), 0);
  console.log(`templates compared: ${templates}`);
  console.log(`skipped, the plugin threw (its Sass compile): ${failed.length}`);
  for (const line of failed) console.log(`  ${line}`);
  process.exitCode = ok ? 0 : 1;
} finally {
  rmSync(dir, { recursive: true, force: true });
}
