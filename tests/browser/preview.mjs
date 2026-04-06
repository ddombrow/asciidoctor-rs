import { watch } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawn } from "node:child_process";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..");
const rustWatchRoot = resolve(root, "src");
const watchedRootFiles = new Set(["Cargo.toml", "Cargo.lock", "build.rs"]);

let rebuildInFlight = false;
let rebuildQueued = false;
let hadInitialBuildFailure = false;

await rebuildWasm("initial");
await import("./server.mjs");

const rootWatcher = watch(root, (eventType, filename) => {
  if (!filename || !watchedRootFiles.has(filename)) {
    return;
  }
  queueRebuild(`${eventType}:${filename}`);
});

const rustWatcher = watch(
  rustWatchRoot,
  { recursive: true },
  (_eventType, filename) => {
    if (!filename || !filename.endsWith(".rs")) {
      return;
    }
    queueRebuild(filename);
  }
);

process.on("SIGINT", () => {
  rootWatcher.close();
  rustWatcher.close();
  process.exit(0);
});

process.on("SIGTERM", () => {
  rootWatcher.close();
  rustWatcher.close();
  process.exit(0);
});

function queueRebuild(reason) {
  if (rebuildInFlight) {
    rebuildQueued = true;
    return;
  }

  void rebuildWasm(reason);
}

async function rebuildWasm(reason) {
  rebuildInFlight = true;
  console.log(`[preview] rebuilding browser WASM (${reason})...`);

  const exitCode = await new Promise((resolveExit) => {
    const child = spawn(process.execPath, ["tests/browser/build-wasm.mjs"], {
      cwd: root,
      stdio: "inherit"
    });
    child.on("exit", (code) => resolveExit(code ?? 1));
  });

  if (exitCode !== 0) {
    hadInitialBuildFailure = hadInitialBuildFailure || reason === "initial";
    console.error("[preview] browser WASM rebuild failed");
  } else {
    hadInitialBuildFailure = false;
    console.log("[preview] browser WASM ready");
  }

  rebuildInFlight = false;

  if (rebuildQueued) {
    rebuildQueued = false;
    await rebuildWasm("queued change");
    return;
  }

  if (hadInitialBuildFailure) {
    process.exit(1);
  }
}
