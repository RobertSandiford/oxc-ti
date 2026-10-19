// oxlint-disable no-console
// Compares BabelTransformDocInit with the crate's `docInit` pass: the compiled module, and the head
// script, which Babel writes and JS would build from the crate's entries as below. Corpus files that
// mention DocInit, plus the cases of the plugin's own tests. Left out, as known divergences: a call
// inside JSX, which Babel rejects (its saved call path is stale once the preset rewrites the JSX) and
// the crate compiles; and JSX inside the function, which Babel accepts (writing `_jsx` into the head,
// where it does not exist) and the crate rejects.
// Usage: node tasks/tightscript/compare-doc-init.mjs
import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, relative } from "node:path";

import { comparePass, corpus, loadPlugin, normalize } from "./tungsten-compare.mjs";

const { MakeBabelTransformDocInit } = await loadPlugin("BabelTransformDocInit");

// The head script tungsten writes, from the crate's entries.
function headScript(entries, root, file) {
  if (entries.length === 0) return "";
  const where = relative(root, file).replaceAll("\\", "/");
  return [
    "window.__t_docInit = {};",
    "window.__t_docInitErrors = [];",
    ...entries.map(
      ({ id, function: fn, line, column }) =>
        `try { window.__t_docInit[${id}] = (${fn})(); } catch (e) {`
        + ` window.__t_docInitErrors.push({ id: ${id}, source: ${JSON.stringify(`${where}:${line}:${column}`)}, error: e }); throw e; }`,
    ),
  ].join("\n");
}

const cases = [
  "export const theme = DocInit(() => {\n  document.documentElement.dataset.theme = localStorage.getItem('theme') ?? 'light'\n  return document.documentElement.dataset.theme\n}, 'light')\n",
  "export const a = DocInit(() => 'ok', '')\n\nexport const b = DocInit(() => { throw new Error('boom') }, '')\nexport const c = DocInit(() => 'not reached', '')",
  "DocInit(() => { document.body.classList.add('foo') })",
  "export const x = init(() => 1)",
  "const DocInit = fn => fn();\nexport const x = DocInit(() => 1);",
  "const _docInit = 1;\nexport const x = DocInit(() => _docInit2, _docInit);",
  "export const x = DocInit(function named() { return named.length + arguments.length })",
  "let out\nexport const x = DocInit(() => { out = 1; [out] = [2]; ({ out } = { out: 3 }) })",
  "export const x = DocInit(() => { const k = 'theme'; const read = () => localStorage.getItem(k); return read() ?? 'light' }, '')",
  "export const x = DocInit(function () { return this }, 0)",
  "export const x = DocInit(() => [1].map(function () { return this }), 0)",
  "/* é😀 */ export const c = DocInit(() => 'wide')",
  // errors: Babel throws, the crate reports
  "const fn = () => 1\nexport const x = DocInit(fn, 0)",
  "const key = 'theme'\nexport const x = DocInit(() => localStorage.getItem(key), '')",
  "let n = 0\nexport const x = DocInit(() => { n++ })",
  "class Root { data = DocInit(() => this.x, 0) }",
  "export const x = DocInit(async () => 1, 0)",
  "export const x = DocInit(function* () {}, 0)",
  "export const x = DocInit(a => a, 0)",
  "export const x = DocInit(() => 1, 0, 2)",
];
const typescriptCases = [
  "type Theme = 'light' | 'dark'\nexport const theme = DocInit((): Theme => 'dark', 'light')\n",
  "export const n = DocInit((): number => { const x: number = 1; return x as number }, 0)",
];

const dir = mkdtempSync(join(tmpdir(), "tungsten-doc-init-"));
try {
  const synthetic = [
    ...cases.map((source, i) => ({
      root: "C:/synthetic",
      file: `C:/synthetic/case${i}.jsx`,
      source,
    })),
    ...typescriptCases.map((source, i) => ({
      root: "C:/synthetic",
      file: `C:/synthetic/ts${i}.tsx`,
      source,
    })),
  ];
  const files = [
    ...corpus()
      .map(({ root, file }) => ({ root, file, source: readFileSync(file, "utf8") }))
      .filter(({ source }) => source.includes("DocInit")),
    ...synthetic,
  ];
  let runs = 0;
  let scripts = 0;
  const tally = comparePass({
    name: "doc-init",
    files,
    options: () => ({ docInit: { baseId: 0 } }),
    // a plugin per run: its ids count from 0 and its head script holds only this file's entries
    makePlugins: (entry) => {
      entry.outputFile = join(dir, `${runs++}`, "docInit.js");
      return [MakeBabelTransformDocInit({ outputFile: entry.outputFile, projectDir: entry.root })];
    },
    checkOutput: (entry, output) => {
      const babelScript = readFileSync(entry.outputFile, "utf8");
      const crateScript = headScript(output.docInit, entry.root, entry.file);
      if (babelScript !== "") scripts++;
      if (babelScript === "" && crateScript === "") return undefined;
      if (normalize(babelScript) === normalize(crateScript)) return undefined;
      return `head scripts differ:\n${babelScript}\n---\n${crateScript}`;
    },
  });
  const ok = tally.print(`docInit, ${files.length} files`);
  console.log(`head scripts compared: ${scripts}`);
  process.exitCode = ok ? 0 : 1;
} finally {
  rmSync(dir, { recursive: true, force: true });
}
