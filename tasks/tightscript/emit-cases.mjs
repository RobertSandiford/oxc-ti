// oxlint-disable no-console
// Compares oxc's JS output with the fork's emit baselines for the units written by parse-cases.mjs.
// Both sides are re-printed minified by oxc's codegen, so only differences in the code itself remain.
// Usage: node emit-cases.mjs <examples dir> <units dir>
import { readdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { example, gitShow, run } from "./shared.mjs";

const [examplesDir, unitsDir] = process.argv.slice(2);
const codegen = example(examplesDir, "codegen");
const transformer = example(examplesDir, "transformer");

const afterMarker = (text, marker) => {
  const i = text.indexOf(marker);
  return i < 0 ? null : text.slice(i + marker.length).trim();
};
const minified = (path) => afterMarker(run(codegen, [path, "--minify"]).out, "First time:");

// The `//// [name.js]` sections of a baseline, keyed by file name.
function jsSections(baseline) {
  const sections = new Map();
  const parts = baseline.split(/^\/\/\/\/ \[(.+?)\] \/\/\/\/?$|^\/\/\/\/ \[(.+?)\]\s*$/m);
  for (let i = 1; i < parts.length; i += 3) {
    const name = parts[i] ?? parts[i + 1];
    if (name?.endsWith(".js")) sections.set(name.split("/").pop(), parts[i + 2]);
  }
  return sections;
}

let same = 0;
const differ = [];
let noBaseline = 0;
let commonJs = 0;
for (const caseName of readdirSync(unitsDir)) {
  let baseline;
  try {
    baseline = gitShow(`tsc/testdata/baselines/reference/compiler/${caseName}.js`);
  } catch {
    noBaseline++;
    continue;
  }
  const sections = jsSections(baseline);
  const dir = join(unitsDir, caseName);
  const units = readdirSync(dir).filter((f) => /\.(ts|tsx)$/.test(f) && !f.endsWith(".d.ts"));
  for (const unit of units) {
    const expectedJs = sections.get(unit.replace(/\.tsx?$/, ".js"));
    if (expectedJs === undefined) continue;
    if (/\brequire\(|Object\.defineProperty\(exports/.test(expectedJs)) {
      commonJs++;
      continue;
    }
    const expectedPath = join(dir, `${unit}.expected.js`);
    // tsc adds this under the harness's `@strict: true`.
    writeFileSync(expectedPath, expectedJs.replace(/^\s*"use strict";\s*/, ""));
    const transformed = afterMarker(
      run(transformer, [join(dir, unit), "--target", "esnext"]).out,
      "Transformed:",
    );
    if (transformed === null) {
      differ.push(`${caseName}/${unit}: oxc failed to transform`);
      continue;
    }
    const actualPath = join(dir, `${unit}.actual.js`);
    writeFileSync(actualPath, transformed);
    const expected = minified(expectedPath);
    const actual = minified(actualPath);
    if (expected === actual) {
      same++;
    } else {
      differ.push(`${caseName}/${unit}`);
      writeFileSync(join(dir, `${unit}.expected.norm.js`), expected ?? "");
      writeFileSync(join(dir, `${unit}.actual.norm.js`), actual ?? "");
    }
  }
}
console.log(
  `same: ${same}  differ: ${differ.length}  skipped CommonJS: ${commonJs}  cases without a JS baseline: ${noBaseline}`,
);
for (const d of differ) console.log(d);
