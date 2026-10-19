// oxlint-disable no-console
// Checks the crate's `packageImports` pass against the AST half of BabelTransformPackageImports: the
// bare specifiers its ImportDeclaration visitor reaches, and the source it rewrites for a non-JS
// target. The resolution and package walk are JS's, so a stand-in visitor takes the plugin's place
// in tungsten's dev plugin order, between the real JsxExtensionsToJs and CssImports plugins.
// Usage: node tasks/tightscript/compare-package-imports.mjs
import { readFileSync } from "node:fs";

import { comparePass, corpus, loadPlugin } from "./tungsten-compare.mjs";

const { MakeBabelTransformJsxExtensionsToJs } = await loadPlugin("BabelTransformJsxExtensionsToJs");
const { MakeBabelTransformCssImports } = await loadPlugin("BabelTransformCssImports");
const { isBareImport } = await loadPlugin("isBareImport");

// Every bare specifier with a non-JS extension in the file, to its stand-in web location.
function replacementsFor(source) {
  const replacements = {};
  for (const [, specifier] of source.matchAll(
    /['"]([^'"\s]+\.(?:css|scss|sass|json|svg|png))['"]/g,
  )) {
    if (isBareImport(specifier)) replacements[specifier] = `/.packages/stand-in/${specifier}`;
  }
  return replacements;
}

// The visitor's effect on the AST, recording what it reaches into `entry.seen`.
function standIn(entry) {
  entry.seen = [];
  return () => ({
    name: "BabelTransformPackageImportsStandIn",
    visitor: {
      ImportDeclaration(path) {
        const location = path.node.source.value;
        if (!isBareImport(location)) return;
        entry.seen.push(location);
        const replacement = entry.replacements[location];
        if (replacement !== undefined) path.node.source.value = replacement;
      },
    },
  });
}

const synthetic = {
  root: "C:/synthetic",
  file: "C:/synthetic/imports.tsx",
  source: [
    "import type { T } from 'types-only'",
    "import { U } from 'used-as-type'",
    "import { v } from 'value'",
    "import theme from 'pkg/theme.scss'",
    "import 'pkg/global.css'",
    "import { w } from 'pkg/w.tsx'",
    "import { v as again } from 'value'",
    "import local from './local.js'",
    "export const x: T | U = v",
    "export const e = <div>{theme}{w}{again}{local}</div>",
  ].join("\n"),
};
const files = [
  ...corpus().map(({ root, file }) => ({ root, file, source: readFileSync(file, "utf8") })),
  synthetic,
];
for (const entry of files) entry.replacements = replacementsFor(entry.source);

const sortedSet = (list) => [...new Set(list)].sort((a, b) => a.localeCompare(b));
const tally = comparePass({
  name: "package-imports",
  files,
  options: ({ root, replacements }) => ({
    jsxExtensionsToJs: true,
    packageImports: { replacements },
    cssImports: { absCodeDir: root, absSourceDir: root },
  }),
  makePlugins: (entry) => [
    MakeBabelTransformJsxExtensionsToJs(),
    standIn(entry),
    MakeBabelTransformCssImports({
      projectDir: entry.root,
      absCodeDir: entry.root,
      absSourceDir: entry.root,
    }),
  ],
  checkOutput: (entry, output) => {
    const expected = JSON.stringify(sortedSet(entry.seen));
    const actual = JSON.stringify(sortedSet(output.bareImports));
    return expected === actual
      ? undefined
      : `Babel's visitor saw ${expected}, the crate listed ${actual}`;
  },
});
process.exitCode = tally.print(`packageImports, ${files.length} files`) ? 0 : 1;
const listed = files.filter((entry) => entry.seen?.length > 0);
console.log(
  `bare import lists compared: ${listed.length} non-empty, ${listed.reduce((n, e) => n + new Set(e.seen).size, 0)} specifiers;`
    + ` replacements given in ${files.filter((e) => Object.keys(e.replacements).length > 0).length} files`,
);
