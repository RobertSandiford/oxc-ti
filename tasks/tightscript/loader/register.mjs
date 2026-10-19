// Lets Node run TypeScript and TightScript: imported .ts, .mts, .cts and .tsx files have their
// types overwritten with whitespace, so lines and columns match the source without a source map.
// Usage: node --import <package>/register.mjs app.ts
import { existsSync, readFileSync } from "node:fs";
import { registerHooks } from "node:module";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { blankTypesSync } from "./oxc-transform/index.js";

const typescriptFile = /\.(?:ts|mts|cts|tsx)$/;
// TypeScript imports name the emitted file (`./a.js`), so these fall back to the source file.
const emittedToSource = {
  ".js": [".ts", ".tsx"],
  ".mjs": [".mts"],
  ".cjs": [".cts"],
  ".jsx": [".tsx"],
};

registerHooks({ resolve, load });

function resolve(specifier, context, nextResolve) {
  try {
    return nextResolve(specifier, context);
  } catch (error) {
    const extension = /\.[cm]?jsx?$/.exec(specifier)?.[0];
    const relative =
      specifier.startsWith(".") || specifier.startsWith("/") || specifier.startsWith("file:");
    if (error?.code !== "ERR_MODULE_NOT_FOUND" || !extension || !relative) throw error;
    for (const sourceExtension of emittedToSource[extension]) {
      const candidate = specifier.slice(0, -extension.length) + sourceExtension;
      try {
        return nextResolve(candidate, context);
      } catch {}
    }
    throw error;
  }
}

function load(url, context, nextLoad) {
  if (!url.startsWith("file:")) return nextLoad(url, context);
  const filename = fileURLToPath(url);
  if (!typescriptFile.test(filename)) return nextLoad(url, context);
  const source = readFileSync(filename, "utf8");
  const { code, errors } = blankTypesSync(filename, source);
  if (errors.length > 0) {
    const details = errors.map((e) => e.codeframe ?? e.message).join("\n");
    const error = new SyntaxError(
      `${filename}: TightScript types could not be blanked\n${details}`,
    );
    error.code = "ERR_TIGHTSCRIPT_BLANK";
    throw error;
  }
  return { format: moduleFormat(filename, context.format), source: code, shortCircuit: true };
}

// Node's own rules: .mts is ESM, .cts is CommonJS, and otherwise the nearest package.json decides.
function moduleFormat(filename, nodeFormat) {
  if (nodeFormat === "module-typescript" || nodeFormat === "module") return "module";
  if (nodeFormat === "commonjs-typescript" || nodeFormat === "commonjs") return "commonjs";
  if (filename.endsWith(".mts")) return "module";
  if (filename.endsWith(".cts")) return "commonjs";
  return packageType(dirname(filename)) === "module" ? "module" : "commonjs";
}

const packageTypes = new Map();

function packageType(dir) {
  if (packageTypes.has(dir)) return packageTypes.get(dir);
  const manifest = join(dir, "package.json");
  let type;
  if (existsSync(manifest)) {
    type = JSON.parse(readFileSync(manifest, "utf8")).type ?? "commonjs";
  } else {
    const parent = dirname(dir);
    type = parent === dir ? "commonjs" : packageType(parent);
  }
  packageTypes.set(dir, type);
  return type;
}
