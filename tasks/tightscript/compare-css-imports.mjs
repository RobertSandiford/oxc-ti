// oxlint-disable no-console
// Compares BabelTransformCssImports with the crate's `cssImports` pass on the corpus, each root its
// own code and source dir, plus a temp app with a separate source dir covering the plugin's branches.
// Usage: node tasks/tightscript/compare-css-imports.mjs
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { comparePass, corpus, loadPlugin } from "./tungsten-compare.mjs";

const { MakeBabelTransformCssImports } = await loadPlugin("BabelTransformCssImports");

const dir = mkdtempSync(join(tmpdir(), "tungsten-css-imports-")).replaceAll("\\", "/");
try {
  const codeDir = `${dir}/code`;
  const sourceDir = `${dir}/src`;
  for (const stylesheet of ["pages/g.scss", "x.css", "a.sass", "s.css"]) {
    mkdirSync(join(sourceDir, stylesheet, ".."), { recursive: true });
    writeFileSync(join(sourceDir, stylesheet), "");
  }
  const app = (file, lines) => ({
    root: codeDir,
    sourceRoot: sourceDir,
    file: `${codeDir}/${file}`,
    source: lines.join("\n"),
  });
  const synthetic = [
    app("pages/Home.jsx", [
      "import g from './g.scss'",
      "import x from '../x.css'",
      "import t from 'pkg/theme.scss'",
      "import u from '/abs/u.css'",
      "import { sheet } from '../s.css'",
      "import * as ns from '../s.css'",
      "export const h = <div>{g}{x}{t}{u}{sheet}{ns}</div>",
    ]),
    app("Root.jsx", ["import a from './a.sass'", "export const r = <i>{a}</i>"]),
    // Babel only logs these and keeps the import
    app("Logged.jsx", ["import './s.css'", "import a, { b } from './s.css'", "export { a, b }"]),
    app("Missing.jsx", ["import n from './nope.css'", "export { n }"]),
    app("Typed.tsx", [
      "import unused from './missing.css'",
      "import type T from './types.css'",
      "import s from './s.css'",
      "export const sheet: string = s",
    ]),
  ];
  const files = [
    ...corpus().map(({ root, file }) => ({
      root,
      sourceRoot: root,
      file,
      source: readFileSync(file, "utf8"),
    })),
    ...synthetic,
  ];
  const tally = comparePass({
    name: "css-imports",
    files,
    options: ({ root, sourceRoot }) => ({
      cssImports: { absCodeDir: root, absSourceDir: sourceRoot },
    }),
    makePlugins: ({ root, sourceRoot }) => [
      MakeBabelTransformCssImports({ projectDir: dir, absCodeDir: root, absSourceDir: sourceRoot }),
    ],
  });
  process.exitCode = tally.print(`cssImports, ${files.length} files`) ? 0 : 1;
} finally {
  rmSync(dir, { recursive: true, force: true });
}
