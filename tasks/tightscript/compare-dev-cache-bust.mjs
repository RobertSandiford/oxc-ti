// oxlint-disable no-console
// Compares BabelAddDevCacheBust with the crate's `devCacheBust` pass on the corpus, where real files
// are stat'ed in place, plus a temp dir of every candidate case with fractional mtimes.
// Usage: node tasks/tightscript/compare-dev-cache-bust.mjs
import { mkdirSync, mkdtempSync, readFileSync, rmSync, utimesSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { comparePass, corpus, loadPlugin } from "./tungsten-compare.mjs";

const { MakeBabelAddDevCacheBust } = await loadPlugin("BabelAddDevCacheBust");

const dir = mkdtempSync(join(tmpdir(), "tungsten-cache-bust-"));
try {
  let seconds = 1_700_000_000.123_456_7;
  for (const name of ["a.js", "b.jsx", "c.ts", "d.tsx", "e.jsx", "e.js", "sub/f.tsx", "g.ts"]) {
    mkdirSync(join(dir, name, ".."), { recursive: true });
    writeFileSync(join(dir, name), "");
    utimesSync(join(dir, name), seconds, seconds);
    seconds += 1000.000_37;
  }
  const synthetic = {
    file: join(dir, "m.jsx").replaceAll("\\", "/"),
    source: [
      "import a from './a.js'",
      "import b from './b.js'",
      "import c from './c.js'",
      "import d from './d.js'",
      "import e from './e.jsx'",
      "import f from './sub/f.tsx'",
      "import g from './g.ts'",
      "import g2 from './g.tsx'",
      "import m from './missing.js'",
      "import p from 'pkg/a.js'",
      "import q from './a.js?x=1'",
      "import n from './a'",
      "export { x } from './a.js'",
      "export * from './b.js'",
      "export * as ns from './c.js'",
      "export const h = <div>{a}{b}{c}{d}{e}{f}{g}{g2}{m}{p}{q}{n}</div>",
    ].join("\n"),
  };
  const files = [
    ...corpus().map(({ file }) => ({ file, source: readFileSync(file, "utf8") })),
    synthetic,
  ];
  const tally = comparePass({
    name: "dev-cache-bust",
    files,
    options: () => ({ devCacheBust: true }),
    makePlugins: () => [MakeBabelAddDevCacheBust()],
  });
  process.exitCode = tally.print(`devCacheBust, ${files.length} files`) ? 0 : 1;
} finally {
  rmSync(dir, { recursive: true, force: true });
}
