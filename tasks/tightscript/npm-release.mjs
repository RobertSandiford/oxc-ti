// oxlint-disable no-console
// Runs npm-publish.mjs for several tools without its arguments.
// Usage: node tasks/tightscript/npm-release.mjs <build|publish> [oxlint|oxfmt]...
// `build` builds each tool, one after the other. `publish` publishes each built tool at the
// version of its upstream package.json, the one its binary reports, and stops at the first failure.
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "../..");
const allTools = ["oxlint", "oxfmt"];

const [action, ...requested] = process.argv.slice(2);
if (!["build", "publish"].includes(action) || requested.some((tool) => !allTools.includes(tool))) {
  console.error("Usage: node tasks/tightscript/npm-release.mjs <build|publish> [oxlint|oxfmt]...");
  process.exit(1);
}

for (const tool of requested.length ? requested : allTools) {
  const publishArgs = () => {
    const { version } = JSON.parse(readFileSync(join(root, "npm", tool, "package.json"), "utf8"));
    return [version, "--no-build"];
  };
  const toolArgs = action === "build" ? ["--build-only"] : publishArgs();
  execFileSync(process.execPath, [join(here, "npm-publish.mjs"), tool, ...toolArgs], {
    stdio: "inherit",
  });
}
