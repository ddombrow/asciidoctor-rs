import process from "node:process";
import { spawn } from "node:child_process";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const cargo = process.platform === "win32" ? "cargo.exe" : "cargo";
const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const child = spawn(
  cargo,
  ["run", "--quiet", "--", "--format", "tck-json", "--stdin"],
  {
    cwd: repoRoot,
    stdio: ["pipe", "pipe", "inherit"]
  }
);

process.stdin.pipe(child.stdin);
child.stdout.pipe(process.stdout);

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
