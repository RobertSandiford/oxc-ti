// oxlint-disable no-console
// Compares BabelTransformJsxExtensionsToJs with the crate's `jsxExtensionsToJs` pass on the corpus,
// plus import forms the corpus lacks.
// Usage: node tasks/tightscript/compare-jsx-extensions.mjs
import { readFileSync } from "node:fs";

import { comparePass, corpus, loadPlugin } from "./tungsten-compare.mjs";

const { MakeBabelTransformJsxExtensionsToJs } = await loadPlugin("BabelTransformJsxExtensionsToJs");

const synthetic = {
  file: "C:/synthetic/imports.jsx",
  source: [
    "import a from './a.jsx'",
    "import b from '../b.tsx'",
    "import c from 'pkg/c.ts'",
    "import d from './d.js'",
    "import e from './e.mts'",
    "import f from './f.css'",
    "export { x } from './x.jsx'",
    "export * from './y.tsx'",
    "export const g = () => import('./g.jsx')",
    "export const h = <div>{a}{b}{c}{d}{e}{f}</div>",
  ].join("\n"),
};
const files = [
  ...corpus().map(({ file }) => ({ file, source: readFileSync(file, "utf8") })),
  synthetic,
];

const tally = comparePass({
  name: "jsx-extensions",
  files,
  options: () => ({ jsxExtensionsToJs: true }),
  makePlugins: () => [MakeBabelTransformJsxExtensionsToJs()],
});
process.exitCode = tally.print(`jsxExtensionsToJs, ${files.length} files`) ? 0 : 1;
