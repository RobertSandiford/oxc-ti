// oxlint-disable no-console
// Times type removal over every .ts file under a folder: oxc-ti's blanker, Node's own stripper
// (swc compiled to WebAssembly) and oxc's full transform. Files any of them rejects are left out.
// Usage: node bench.mjs <folder> [package dir]
import { readdirSync, readFileSync, statSync } from "node:fs";
import { stripTypeScriptTypes } from "node:module";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const [folder, packageArg] = process.argv.slice(2);
const packageDir = resolve(packageArg ?? join(here, "../../../target/tightscript-node"));
const { blankTypesSync, transformSync } = await import(
  pathToFileURL(join(packageDir, "oxc-transform/index.js")).href
);

function* walk(dir) {
  for (const name of readdirSync(dir)) {
    const path = join(dir, name);
    if (name === "node_modules") continue;
    if (statSync(path).isDirectory()) yield* walk(path);
    else if (/\.(?:ts|mts|cts)$/.test(name) && !/\.d\.[cm]?ts$/.test(name)) yield path;
  }
}

const tools = {
  "oxc-ti blank": (path, source) => {
    if (blankTypesSync(path, source).errors.length > 0) throw new Error("rejected");
  },
  "node strip (swc wasm)": (_, source) => stripTypeScriptTypes(source, { mode: "strip" }),
  "oxc transform": (path, source) => {
    if (transformSync(path, source).errors.length > 0) throw new Error("rejected");
  },
};

const files = [];
for (const path of walk(folder)) {
  const source = readFileSync(path, "utf8");
  const acceptedByAll = Object.values(tools).every((tool) => {
    try {
      tool(path, source);
      return true;
    } catch {
      return false;
    }
  });
  if (acceptedByAll) files.push([path, source]);
}
const bytes = files.reduce((sum, [, source]) => sum + source.length, 0);
console.log(`${files.length} files, ${(bytes / 1024).toFixed(0)} KiB accepted by all three`);

for (const [name, tool] of Object.entries(tools)) {
  for (let i = 0; i < 3; i++) for (const [path, source] of files) tool(path, source);
  const runs = 10;
  const start = process.hrtime.bigint();
  for (let i = 0; i < runs; i++) for (const [path, source] of files) tool(path, source);
  const ms = Number(process.hrtime.bigint() - start) / 1e6 / runs;
  console.log(
    `${name.padEnd(22)} ${ms.toFixed(1).padStart(7)} ms per pass  ${(bytes / 1024 / 1024 / (ms / 1000)).toFixed(0).padStart(4)} MiB/s`,
  );
}
