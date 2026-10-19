// oxlint-disable no-console
// Formats every parseable .ts/.tsx file under the roots, then checks the output parses and formats to itself.
// Usage: node fmt-corpus.mjs <examples dir> <output dir> <root>...
import { mkdirSync, readdirSync, statSync, writeFileSync } from "node:fs";
import { basename, extname, join } from "node:path";

import { example, firstError, run } from "./shared.mjs";

const [examplesDir, outDir, ...roots] = process.argv.slice(2);
const parser = example(examplesDir, "parser");
const formatter = example(examplesDir, "formatter");
const parses = (file) => run(parser, [file]).out.includes("Parsed Successfully");

function* walk(dir) {
  for (const name of readdirSync(dir)) {
    if (name === "node_modules" || name === "dist") continue;
    const path = join(dir, name);
    if (statSync(path).isDirectory()) yield* walk(path);
    else if (/\.(ts|tsx)$/.test(name) && !name.endsWith(".d.ts")) yield path;
  }
}

mkdirSync(outDir, { recursive: true });
let stable = 0;
let n = 0;
const problems = [];
for (const root of roots) {
  for (const file of walk(root)) {
    if (!parses(file)) continue;
    const once = run(formatter, [file]);
    if (!once.ok) {
      problems.push(`${file}: formatter failed: ${once.out.split("\n")[0]}`);
      continue;
    }
    const ext = extname(file);
    const outPath = join(outDir, `${n++}-${basename(file, ext)}${ext}`);
    writeFileSync(outPath, once.out);
    const reparsed = run(parser, [outPath]).out;
    if (!reparsed.includes("Parsed Successfully")) {
      problems.push(`${file}: output does not parse: ${firstError(reparsed)} (${outPath})`);
    } else if (run(formatter, [outPath]).out === once.out) {
      stable++;
    } else {
      problems.push(`${file}: not idempotent (${outPath})`);
    }
  }
}
console.log(`formatted and stable: ${stable}  problems: ${problems.length}`);
for (const p of problems) console.log(p);
