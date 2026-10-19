// oxlint-disable no-console
// Builds oxlint in release mode and assembles it as an npm-style package that can be run in place or linked.
// Usage: node tasks/tightscript/package.mjs [--no-build] [--out <dir>]
// The default output is target/oxlint-tightscript.
import { execFileSync } from "node:child_process";
import { cpSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "../..");
const args = process.argv.slice(2);
const outIndex = args.indexOf("--out");
const out = outIndex >= 0 ? resolve(args[outIndex + 1]) : join(root, "target/oxlint-tightscript");

if (!args.includes("--no-build")) {
  execFileSync("pnpm", ["run", "build"], {
    cwd: join(root, "apps/oxlint"),
    stdio: "inherit",
    shell: true,
  });
}

rmSync(out, { recursive: true, force: true });
mkdirSync(out, { recursive: true });
for (const file of ["package.json", "configuration_schema.json", "bin"]) {
  cpSync(join(root, "npm/oxlint", file), join(out, file), { recursive: true });
}
cpSync(join(root, "apps/oxlint/dist"), join(out, "dist"), { recursive: true });
// Windows tools such as the VS Code extension run the path through cmd, which can't start a bare Node script.
writeFileSync(join(out, "bin/oxlint.cmd"), '@node "%~dp0oxlint" %*\r\n');
console.log(`Packaged oxlint at ${out}`);
