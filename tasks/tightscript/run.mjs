// oxlint-disable no-console
// Builds the oxc examples and runs the parse, emit and format comparisons against the fork.
// Usage: node tasks/tightscript/run.mjs [format root]...
// With no roots, formats every branch's `trial` folder, tungsten's src-ti and the fork's test units.
import { execFileSync } from "node:child_process";
import { existsSync, readdirSync, rmSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "../..");
const examplesDir = join(root, "target/debug/examples");
const workDir = join(root, "target/tightscript");
const unitsDir = join(workDir, "units");

const branchesDir = process.env.TIGHTSCRIPT_BRANCHES ?? "C:/ImportantFiles/tightscript/branches";
const tungstenSrcTi =
  "C:/ImportantFiles/NodePackages/framework/tungsten-work-folder/tungsten/src-ti";

function defaultFormatRoots() {
  const trials = existsSync(branchesDir)
    ? readdirSync(branchesDir)
        .map((branch) => join(branchesDir, branch, "trial"))
        .filter(existsSync)
    : [];
  return [...trials, tungstenSrcTi, unitsDir].filter(existsSync);
}

const node = (script, args) =>
  execFileSync(process.execPath, [join(here, script), ...args], {
    stdio: "inherit",
    maxBuffer: 1 << 28,
  });

const examples = [
  ["oxc_parser", "parser"],
  ["oxc_codegen", "codegen"],
  ["oxc_transformer", "transformer"],
  ["oxc_formatter", "formatter"],
];
execFileSync(
  "cargo",
  ["build", ...examples.flatMap(([crate, name]) => ["-p", crate, "--example", name])],
  { cwd: root, stdio: "inherit" },
);

rmSync(workDir, { recursive: true, force: true });
console.log("\n== parse: fork test cases");
node("parse-cases.mjs", [examplesDir, unitsDir]);
console.log("\n== emit: fork JS baselines");
node("emit-cases.mjs", [examplesDir, unitsDir]);
console.log("\n== format: stable and reparseable");
const roots = process.argv.slice(2);
node("fmt-corpus.mjs", [
  examplesDir,
  join(workDir, "fmt-out"),
  ...(roots.length ? roots : defaultFormatRoots()),
]);
