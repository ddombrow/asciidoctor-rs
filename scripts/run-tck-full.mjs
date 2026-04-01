import process from "node:process";
import { dirname, resolve } from "node:path";
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const tckRoot = process.env.ASCIIDOC_TCK_ROOT ?? resolve(repoRoot, "..", "asciidoc-tck");
const testsDir = resolve(tckRoot, "tests");
const harness = resolve(tckRoot, "harness", "bin", "asciidoc-tck.js");
const adapterCommand = `node ${JSON.stringify(resolve(repoRoot, "scripts", "tck-adapter.mjs"))}`;
const extraArgs = process.argv.slice(2);

const child = spawn(
  process.execPath,
  [harness, "cli", `--tests=${testsDir}`, "--adapter-command", adapterCommand, ...extraArgs],
  {
    cwd: repoRoot,
    stdio: "inherit"
  }
);

child.on("exit", (code, signal) => {
  if (signal) {
    process.kill(process.pid, signal);
    return;
  }

  process.exit(code ?? 1);
});

child.on("error", (error) => {
  console.error(error);
  process.exit(1);
});
