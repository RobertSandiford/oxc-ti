// oxlint-disable no-console
// Builds oxlint or oxfmt in release mode, assembles it as oxlint-ti or oxfmt-ti, and publishes it to npm.
// Usage: node tasks/tightscript/npm-publish.mjs <oxlint|oxfmt> [version] [--no-build] [--dry-run]
// Without a version, the patch number of the latest published version is bumped.
// --dry-run runs `npm publish --dry-run` and allows a dirty git tree.
import { execSync } from "node:child_process";
import { cpSync, existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "../..");
const repo = "https://github.com/RobertSandiford/oxc-ti";
const tools = {
  oxlint: { name: "oxlint-ti", description: "oxlint with TightScript syntax support" },
  oxfmt: { name: "oxfmt-ti", description: "oxfmt with TightScript syntax support" },
};

const args = process.argv.slice(2);
const flags = new Set(args.filter((arg) => arg.startsWith("--")));
const [app, requestedVersion] = args.filter((arg) => !arg.startsWith("--"));
const tool = tools[app?.replace(/-ti$/, "")];
if (!tool) {
  fail(
    "Usage: node tasks/tightscript/npm-publish.mjs <oxlint|oxfmt> [version] [--no-build] [--dry-run]",
  );
}
const appName = app.replace(/-ti$/, "");
const dryRun = flags.has("--dry-run");

function fail(message) {
  console.error(message);
  process.exit(1);
}

// npm and pnpm are .cmd files on Windows, which only start through a shell; no argument here needs quoting.
function exec(command, commandArgs, options = {}) {
  return execSync([command, ...commandArgs].join(" "), { cwd: root, encoding: "utf8", ...options });
}

const dirty = exec("git", ["status", "--porcelain"]).trim();
if (dirty) {
  if (!dryRun) fail(`Git tree isn't clean; commit or stash first:\n${dirty}`);
  console.warn("Git tree isn't clean; continuing because of --dry-run.");
}

function nextVersion() {
  let latest;
  try {
    latest = exec("npm", ["view", tool.name, "version"], {
      stdio: ["ignore", "pipe", "pipe"],
    }).trim();
  } catch (e) {
    if (String(e.stderr).includes("E404")) {
      fail(`${tool.name} isn't on npm yet; give the first version.`);
    }
    throw e;
  }
  const match = /^(\d+)\.(\d+)\.(\d+)$/.exec(latest);
  if (!match) {
    fail(
      `Latest ${tool.name} is ${latest}, which has no plain patch number to bump; give a version.`,
    );
  }
  return `${match[1]}.${match[2]}.${Number(match[3]) + 1}`;
}

const version = requestedVersion ?? nextVersion();
if (!/^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$/.test(version)) fail(`${version} isn't a valid version.`);

const appDir = join(root, "apps", appName);
if (!flags.has("--no-build")) exec("pnpm", ["run", "build"], { cwd: appDir, stdio: "inherit" });
const binary = join(appDir, "dist", `${appName}.win32-x64-msvc.node`);
if (!existsSync(binary)) fail(`Missing ${binary}; build without --no-build.`);

const upstream = JSON.parse(readFileSync(join(root, "npm", appName, "package.json"), "utf8"));
const {
  author: _author,
  funding: _funding,
  napi: _napi,
  preferUnplugged: _unplugged,
  ...kept
} = upstream;
const pkg = {
  ...kept,
  name: tool.name,
  version,
  description: tool.description,
  keywords: [...upstream.keywords, "tightscript"],
  homepage: repo,
  bugs: `${repo}/issues`,
  repository: { type: "git", url: `git+${repo}.git` },
  files: upstream.files.filter((file) => file !== "README.md"),
  os: ["win32"],
  cpu: ["x64"],
};

const out = join(root, "target/npm", tool.name);
rmSync(out, { recursive: true, force: true });
mkdirSync(out, { recursive: true });
for (const file of ["configuration_schema.json", "bin"]) {
  cpSync(join(root, "npm", appName, file), join(out, file), { recursive: true });
}
cpSync(join(appDir, "dist"), join(out, "dist"), { recursive: true });
cpSync(join(root, "LICENSE"), join(out, "LICENSE"));
writeFileSync(join(out, "package.json"), `${JSON.stringify(pkg, null, 2)}\n`);
console.log(`Assembled ${tool.name}@${version} at ${out}`);

exec("npm", ["publish", ...(dryRun ? ["--dry-run"] : [])], { cwd: out, stdio: "inherit" });
console.log(`${dryRun ? "Dry run of" : "Published"} ${tool.name}@${version}`);
