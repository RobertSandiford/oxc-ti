// oxlint-disable no-console
// Runs the fixtures through a packaged loader. Usage: node test.mjs [package dir]
// The default package dir is target/tightscript-node, written by package-loader.mjs.
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const packageDir = resolve(process.argv[2] ?? join(here, "../../../target/tightscript-node"));
const register = pathToFileURL(join(packageDir, "register.mjs")).href;
const fixtures = join(here, "fixtures");

function run(file) {
  return spawnSync(process.execPath, ["--import", register, join(fixtures, file)], {
    encoding: "utf8",
  });
}

const esm = run("esm/main.ts");
assert.equal(esm.status, 0, esm.stderr);
const [result, frame] = esm.stdout.trim().split("\n");
assert.deepEqual(JSON.parse(result), { count: 2, value: 4 });
assert.match(frame, /main\.ts:12:9\)?$/, "the stack trace keeps the source position");

const unsupported = run("enum.ts");
assert.notEqual(unsupported.status, 0);
assert.match(unsupported.stderr, /ERR_TIGHTSCRIPT_BLANK|enum is not supported/);

console.log("loader tests passed");
