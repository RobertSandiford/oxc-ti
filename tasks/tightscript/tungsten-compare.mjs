// oxlint-disable no-console
// Shared by the compare-*.mjs scripts: runs tungsten's Babel plugins and the tungsten_passes crate on
// the same files and compares the two after normalising formatting.
// tungsten is read only: its Babel and plugins load from its own node_modules and dist.
import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, readdirSync, statSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { basename, dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

import { example } from "./shared.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "../..");

export const tungstenDir =
  process.env.TUNGSTEN_DIR
  ?? "C:/ImportantFiles/NodePackages/framework/tungsten-work-folder/tungsten";
const workFolder = join(tungstenDir, "..");

const require = createRequire(join(tungstenDir, "package.json"));
export const babel = require("@babel/core");
const generate = require("@babel/generator").default;
const presetReact = require("@babel/preset-react").default;
const presetTypescript = require("@babel/preset-typescript").default;

export const loadPlugin = async (name) =>
  import(pathToFileURL(join(tungstenDir, "dist/babelPlugins", `${name}.js`)).href);

// tungsten's dev presets with `typescript: true`; the typescript preset only acts on .ts/.tsx files
export const presets = () => [
  [presetReact, { runtime: "automatic", importSource: "tungsten" }],
  [presetTypescript, { allowDeclareFields: true }],
];

export function babelTransform(file, source, plugins) {
  return babel.transformSync(source, {
    filename: file,
    babelrc: false,
    configFile: false,
    presets: presets(),
    plugins,
  }).code;
}

// Whether Babel can parse the file at all: it cannot parse TightScript.
export function babelParses(file, source) {
  try {
    babel.parseSync(source, {
      filename: file,
      babelrc: false,
      configFile: false,
      presets: presets(),
    });
    return true;
  } catch {
    return false;
  }
}

// The source trees compared on: tungsten's own, as written and as tsc-ti leaves it for Babel, and the
// apps beside it that use its features. Each is compiled as if it were the code and source dir.
// Babel cannot parse TightScript, so most of src-ti only counts as skipped.
export const corpusRoots = [
  join(tungstenDir, "src-ti"),
  join(tungstenDir, "intermed-ti"),
  join(workFolder, "sample-apps/sample-qtc-2/src/client"),
  join(workFolder, "sample-apps/sample-dev-sync/packages/ui/src"),
  join(workFolder, "playground/src"),
  join(workFolder, "trpc-crud-next/src/client"),
].filter(existsSync);

function* walk(dir) {
  for (const name of readdirSync(dir)) {
    if (name === "node_modules" || name === "dist") continue;
    const path = join(dir, name);
    if (statSync(path).isDirectory()) yield* walk(path);
    else if (/\.(js|jsx|ts|tsx)$/.test(name) && !name.endsWith(".d.ts")) yield path;
  }
}

// Every source file, as { root, file } with forward slashes, as tungsten's glob gives them.
export function corpus() {
  return corpusRoots.flatMap((corpusRoot) =>
    [...walk(corpusRoot)].map((file) => ({
      root: corpusRoot.replaceAll("\\", "/"),
      file: file.replaceAll("\\", "/"),
    })),
  );
}

// Compiles each job ({ path, source?, options }) with the crate's `compile` example, in one process.
export function compileWithPasses(jobs) {
  execFileSync("cargo", ["build", "-p", "tungsten_passes", "--example", "compile"], {
    cwd: root,
    stdio: ["ignore", "ignore", "inherit"],
  });
  const exe = example(join(root, "target/debug/examples"), "compile");
  const input = jobs.map((job) => ({ ...job, options: JSON.stringify(job.options) }));
  const out = execFileSync(exe, [], { input: JSON.stringify(input), maxBuffer: 1 << 30 });
  return JSON.parse(out.toString("utf8"));
}

function strip(node) {
  if (Array.isArray(node)) {
    for (const child of node) strip(child);
  } else if (node !== null && typeof node === "object") {
    delete node.extra;
    delete node.leadingComments;
    delete node.trailingComments;
    delete node.innerComments;
    // oxc writes `{ to }` where Babel keeps `{ to: to }`
    if (node.shorthand === true) node.shorthand = false;
    for (const value of Object.values(node)) strip(value);
  }
}

// Reprints code from its AST with Babel's generator, dropping comments, quotes and parentheses.
// Imports move to the top in their order, as they are hoisted, and the JSX runtime import goes last:
// Babel adds it after the last import, oxc before the first other statement, which differ once a
// pass has turned an import into a `const`.
export function normalize(code) {
  const ast = babel.parseSync(code, { babelrc: false, configFile: false, sourceType: "module" });
  strip(ast);
  const isImport = (statement) => statement.type === "ImportDeclaration";
  const isJsxRuntime = (statement) =>
    isImport(statement) && statement.source.value.endsWith("/jsx-runtime");
  ast.program.body = [
    ...ast.program.body.filter((statement) => isImport(statement) && !isJsxRuntime(statement)),
    ...ast.program.body.filter(isJsxRuntime),
    ...ast.program.body.filter((statement) => !isImport(statement)),
  ];
  return generate(ast, { comments: false }).code;
}

function multisetMinus(lines, minus) {
  const left = new Map();
  for (const line of minus) left.set(line, (left.get(line) ?? 0) + 1);
  return lines.filter((line) => {
    const n = left.get(line) ?? 0;
    if (n === 0) return true;
    left.set(line, n - 1);
    return false;
  });
}

// The lines a pass removed and added, from normalised code without and with it.
export function lineChange(without, withPass) {
  const before = without.split("\n");
  const after = withPass.split("\n");
  return JSON.stringify({
    removed: multisetMinus(before, after).sort(),
    added: multisetMinus(after, before).sort(),
  });
}

// Compares Babel's and the crate's output for one file, given both normalised with and without the
// pass. Where the two compile the rest of the file differently (the baseline), the pass's own change
// to the lines is compared instead.
export function classify(babelWith, babelWithout, rustWith, rustWithout) {
  const applies = babelWith !== babelWithout || rustWith !== rustWithout;
  if (babelWith === rustWith) return applies ? "same" : "same (pass did nothing)";
  if (babelWithout === rustWithout) return "DIFFERENT";
  return lineChange(babelWithout, babelWith) === lineChange(rustWithout, rustWith)
    ? applies
      ? "same change, baseline differs"
      : "same (pass did nothing), baseline differs"
    : "DIFFERENT";
}

// Compiles each entry { file, source } with the plugins `makePlugins(entry)` returns and with the
// crate's passes `options(entry)` selects, each with and without, and tallies how the outputs compare.
// `checkOutput(entry, output)`, when given, runs after Babel and returns a problem with the crate's
// output JSON, if any. Differences are written under target/tightscript/compare/<name>.
export function comparePass({ name, files, options, makePlugins, checkOutput }) {
  const tally = new Tally();
  const parsed = files.filter(({ file, source }) => {
    if (babelParses(file, source)) return true;
    tally.count("skipped, Babel cannot parse");
    return false;
  });
  const jobs = parsed.flatMap((entry) => [
    { path: entry.file, source: entry.source, options: options(entry) },
    { path: entry.file, source: entry.source, options: {} },
  ]);
  const results = compileWithPasses(jobs);
  const retries = [];
  const differs = (file, i, outputs) =>
    tally.problem("DIFFERENT", `${file} (${writeOutputs(name, i, file, outputs)})`);
  parsed.forEach((entry, i) => {
    const { file, source } = entry;
    const [withPass, without] = results.slice(2 * i, 2 * i + 2);
    const crateError = [...withPass.errors, ...without.errors].find((e) => e.severity === "error");
    let babelWith;
    try {
      babelWith = babelTransform(file, source, makePlugins(entry));
    } catch (error) {
      if (crateError === undefined) {
        tally.problem("DIFFERENT, only Babel failed", `${file}: ${error.message.split("\n")[0]}`);
      } else {
        tally.count("both failed");
      }
      return;
    }
    const coreError = without.errors.find((e) => e.severity === "error");
    if (coreError !== undefined) {
      tally.note("skipped, the core fails without passes", `${file}: ${coreError.message}`);
      return;
    }
    if (crateError !== undefined) {
      tally.problem("crate error", `${file}: ${crateError.message}`);
      return;
    }
    const outputProblem = checkOutput?.(entry, withPass.output);
    if (outputProblem !== undefined) tally.problem("OUTPUT DIFFERENT", `${file}: ${outputProblem}`);
    const outputs = {
      babel: normalize(babelWith),
      "babel-without": normalize(babelTransform(file, source, [])),
      crate: normalize(withPass.code),
      "crate-without": normalize(without.code),
    };
    const kind = classify(...Object.values(outputs));
    if (kind !== "DIFFERENT") tally.count(kind);
    else if (outputs["babel-without"] === outputs["crate-without"]) differs(file, i, outputs);
    else retries.push({ entry, i, outputs });
  });

  // Where the baselines differ, the plugin runs alone, keeping types and JSX, and the crate compiles
  // its output: both sides then go through the same TypeScript and JSX transforms
  const pluginOnly = retries.map(({ entry }) => ({
    path: entry.file,
    source: babel.transformSync(entry.source, {
      filename: entry.file,
      babelrc: false,
      configFile: false,
      plugins: makePlugins(entry),
      parserOpts: { plugins: /\.tsx?$/.test(entry.file) ? ["typescript", "jsx"] : ["jsx"] },
    }).code,
    options: {},
  }));
  const recompiled = pluginOnly.length === 0 ? [] : compileWithPasses(pluginOnly);
  retries.forEach(({ entry: { file }, i, outputs }, j) => {
    const viaPluginOnly = normalize(recompiled[j].code);
    if (viaPluginOnly === outputs.crate) tally.count("same via the plugin alone, baseline differs");
    else differs(file, i, { ...outputs, "babel-plugin-alone-then-crate": viaPluginOnly });
  });
  return tally;
}

// Writes a differing file's normalised outputs where they can be diffed, returning the stem.
export function writeOutputs(name, i, file, outputs) {
  const dir = join(root, "target/tightscript/compare", name);
  mkdirSync(dir, { recursive: true });
  const stem = join(dir, `${i}-${basename(file)}`);
  for (const [kind, code] of Object.entries(outputs)) writeFileSync(`${stem}.${kind}.js`, code);
  return stem;
}

// Collects results by kind and prints counts, then the listed problems.
export class Tally {
  counts = new Map();
  problems = [];
  notes = [];

  count(kind) {
    this.counts.set(kind, (this.counts.get(kind) ?? 0) + 1);
  }

  problem(kind, text) {
    this.count(kind);
    this.problems.push(`${kind}: ${text}`);
  }

  // Listed like a problem, but not one for the pass under test.
  note(kind, text) {
    this.count(kind);
    this.notes.push(`${kind}: ${text}`);
  }

  print(title) {
    console.log(`${title}: ${[...this.counts].map(([kind, n]) => `${kind} ${n}`).join("  ")}`);
    for (const line of [...this.notes, ...this.problems]) console.log(`  ${line}`);
    return this.problems.length === 0;
  }
}
