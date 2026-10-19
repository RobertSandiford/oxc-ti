// oxlint-disable no-console
// Parses every compiler test case added on the fork's branch, one file per `// @filename:` unit.
// Usage: node parse-cases.mjs <examples dir> <units dir>
import { mkdirSync, writeFileSync } from "node:fs";
import { basename, join } from "node:path";

import { example, firstError, gitAddedFiles, gitShow, run } from "./shared.mjs";

const [examplesDir, unitsDir] = process.argv.slice(2);
const parser = example(examplesDir, "parser");

// The text before the first `// @filename:` is one unit named after the case.
function splitUnits(name, text) {
  const units = [];
  let current = { name, lines: [] };
  for (const line of text.split(/\r?\n/)) {
    const filename = /^\s*\/\/\s*@filename:\s*(.+?)\s*$/i.exec(line);
    if (filename) {
      if (current.lines.some((l) => l.trim())) units.push(current);
      current = { name: filename[1], lines: [] };
    } else if (/^\s*\/\/\s*@\w+\s*:/.test(line)) {
      // Blank out harness directives such as `// @strict: true`, keeping line numbers.
      current.lines.push("");
    } else {
      current.lines.push(line);
    }
  }
  units.push(current);
  return units.filter((u) => /\.(ts|tsx|mts|cts)$/.test(u.name));
}

const cases = gitAddedFiles("tsc/testdata/tests/cases/compiler").filter((f) =>
  /\.(ts|tsx)$/.test(f),
);
let parsed = 0;
const failures = [];
for (const file of cases) {
  const caseName = basename(file).replace(/\.(ts|tsx)$/, "");
  const dir = join(unitsDir, caseName);
  mkdirSync(dir, { recursive: true });
  for (const unit of splitUnits(basename(file), gitShow(file))) {
    const path = join(dir, basename(unit.name));
    writeFileSync(path, unit.lines.join("\n"));
    const { out } = run(parser, [path]);
    if (out.includes("Parsed Successfully")) parsed++;
    else failures.push(`${caseName}/${basename(unit.name)}: ${firstError(out)}`);
  }
}
console.log(`cases: ${cases.length}  units parsed: ${parsed}  failed: ${failures.length}`);
for (const f of failures) console.log(f);
