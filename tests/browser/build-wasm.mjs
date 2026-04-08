import { cpSync, existsSync, mkdirSync, rmSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..");
const outDir = resolve(root, "tests", "browser", "pkg");
const wasmBindgenVersion = "0.2.115";
const wasmBindgenExe = resolveWasmBindgenExe(root, wasmBindgenVersion);
const targetWasm = resolve(
  root,
  "target",
  "wasm32-unknown-unknown",
  "debug",
  "asciidoctor_rs.wasm"
);

rmSync(outDir, { recursive: true, force: true });
mkdirSync(outDir, { recursive: true });

if (!existsSync(wasmBindgenExe)) {
  throw new Error(
    `Missing wasm-bindgen executable at ${wasmBindgenExe}. Run node scripts/install-wasm-bindgen.mjs first.`
  );
}

buildRustWasm();

runOrThrow(
  wasmBindgenExe,
  ["--target", "web", "--out-dir", outDir, targetWasm],
  "wasm-bindgen generation failed"
);

const siteDir = resolve(root, "tests", "browser", "site");
mkdirSync(siteDir, { recursive: true });
cpSync(resolve(root, "examples", "sample.adoc"), resolve(siteDir, "sample.adoc"));

function buildRustWasm() {
  const buildArgs = ["--target", "wasm32-unknown-unknown", "--features", "wasm"];
  const offlineOnly = process.env.ASCIIDOCTOR_RS_WASM_BUILD_OFFLINE === "1";
  const offlineResult = run("cargo", ["build", "--offline", ...buildArgs], true);

  if (offlineResult.status === 0) {
    if (offlineResult.stdout) process.stdout.write(offlineResult.stdout);
    if (offlineResult.stderr) process.stderr.write(offlineResult.stderr);
    return;
  }

  if (offlineOnly || !isMissingOfflineDependency(offlineResult.stderr)) {
    if (offlineResult.stdout) process.stdout.write(offlineResult.stdout);
    if (offlineResult.stderr) process.stderr.write(offlineResult.stderr);
    throw new Error("Rust WASM build failed");
  }

  const onlineResult = run("cargo", ["build", ...buildArgs]);
  if (onlineResult.status !== 0) {
    throw new Error("Rust WASM build failed");
  }
}

function run(command, args, captureOnly = false) {
  const resolvedCommand =
    process.platform === "win32" && command === "cargo" ? "cargo.exe" : command;
  const result = spawnSync(resolvedCommand, args, {
    cwd: root,
    encoding: "utf8"
  });

  if (!captureOnly) {
    if (result.stdout) {
      process.stdout.write(result.stdout);
    }
    if (result.stderr) {
      process.stderr.write(result.stderr);
    }
  }

  return result;
}

function runOrThrow(command, args, errorMessage) {
  const result = run(command, args);
  if (result.status !== 0) {
    throw new Error(errorMessage);
  }
}

function isMissingOfflineDependency(stderr) {
  return (
    stderr.includes("attempting to make an HTTP request, but --offline was specified") ||
    stderr.includes("failed to download") ||
    (stderr.includes("no matching package named") && stderr.includes("you're using offline mode"))
  );
}

function resolveWasmBindgenExe(rootDir, version) {
  const executableName = process.platform === "win32" ? "wasm-bindgen.exe" : "wasm-bindgen";
  const candidates = [];

  if (process.platform === "win32") {
    candidates.push(`wasm-bindgen-${version}-x86_64-pc-windows-msvc`);
  } else if (process.platform === "darwin") {
    candidates.push(`wasm-bindgen-${version}-aarch64-apple-darwin`);
    candidates.push(`wasm-bindgen-${version}-x86_64-apple-darwin`);
  } else if (process.platform === "linux") {
    candidates.push(`wasm-bindgen-${version}-aarch64-unknown-linux-musl`);
    candidates.push(`wasm-bindgen-${version}-x86_64-unknown-linux-musl`);
  }

  for (const candidate of candidates) {
    const path = resolve(rootDir, ".tools", "wasm-bindgen", candidate, executableName);
    if (existsSync(path)) {
      return path;
    }
  }

  return resolve(rootDir, ".tools", "wasm-bindgen", candidates[0] ?? "", executableName);
}
