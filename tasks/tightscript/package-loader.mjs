// oxlint-disable no-console
// Builds the oxc-transform binding and assembles the Node loader package around it.
// Usage: node tasks/tightscript/package-loader.mjs [--no-build] [--out <dir>]
// The default output is target/tightscript-node; run code with `node --import <out>/register.mjs app.ts`.
import { execFileSync } from "node:child_process";
import { cpSync, mkdirSync, readdirSync, rmSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "../..");
const binding = join(root, "napi/transform");
const args = process.argv.slice(2);
const outIndex = args.indexOf("--out");
const out = outIndex >= 0 ? resolve(args[outIndex + 1]) : join(root, "target/tightscript-node");

if (!args.includes("--no-build")) {
  execFileSync("pnpm", ["run", "build"], { cwd: binding, stdio: "inherit", shell: true });
}

rmSync(out, { recursive: true, force: true });
mkdirSync(join(out, "oxc-transform"), { recursive: true });
for (const file of ["package.json", "register.mjs"]) {
  cpSync(join(here, "loader", file), join(out, file));
}
const nativeFiles = readdirSync(binding).filter((file) => file.endsWith(".node"));
if (nativeFiles.length === 0) throw new Error(`no .node binary in ${binding}; build it first`);
for (const file of ["index.js", "index.d.ts", ...nativeFiles]) {
  cpSync(join(binding, file), join(out, "oxc-transform", file));
}
console.log(`Packaged the TightScript Node loader at ${out}`);
