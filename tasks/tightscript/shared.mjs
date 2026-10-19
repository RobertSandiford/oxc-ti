import { execFileSync } from "node:child_process";
import { join } from "node:path";

export const typescriptRepo =
  process.env.TIGHTSCRIPT_REPO ?? "C:/ImportantFiles/tightscript/TypeScript";
export const forkBranch = process.env.TIGHTSCRIPT_BRANCH ?? "tightscript";

export function gitShow(path) {
  return execFileSync("git", ["-C", typescriptRepo, "show", `${forkBranch}:${path}`], {
    encoding: "utf8",
    stdio: ["ignore", "pipe", "ignore"],
    maxBuffer: 1 << 28,
  });
}

export function gitAddedFiles(dir) {
  return execFileSync(
    "git",
    [
      "-C",
      typescriptRepo,
      "diff",
      "--name-only",
      "--diff-filter=A",
      `main..${forkBranch}`,
      "--",
      dir,
    ],
    { encoding: "utf8", maxBuffer: 1 << 28 },
  )
    .split("\n")
    .filter(Boolean);
}

export function example(examplesDir, name) {
  return join(examplesDir, process.platform === "win32" ? `${name}.exe` : name);
}

// Runs a command and returns its combined output, whether or not it exits 0.
export function run(exe, args) {
  try {
    return {
      ok: true,
      out: execFileSync(exe, args, {
        encoding: "utf8",
        stdio: ["ignore", "pipe", "pipe"],
        maxBuffer: 1 << 26,
      }),
    };
  } catch (e) {
    return { ok: false, out: (e.stdout ?? "") + (e.stderr ?? "") };
  }
}

export function firstError(out) {
  return (/^\s*x (.*)$/m.exec(out) ?? [])[1] ?? out.split("\n")[0];
}
