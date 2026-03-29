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

run(
  "cargo",
  ["build", "--offline", "--target", "wasm32-unknown-unknown", "--features", "wasm"],
  "Rust WASM build failed"
);

run(
  wasmBindgenExe,
  ["--target", "web", "--out-dir", outDir, targetWasm],
  "wasm-bindgen generation failed"
);

const siteDir = resolve(root, "tests", "browser", "site");
const upstreamCss = resolve(root, "..", "asciidoctor", "src", "stylesheets", "asciidoctor.css");
mkdirSync(siteDir, { recursive: true });
cpSync(resolve(root, "examples", "sample.adoc"), resolve(siteDir, "sample.adoc"));
cpSync(upstreamCss, resolve(siteDir, "asciidoctor.css"));

function run(command, args, errorMessage) {
  const resolvedCommand =
    process.platform === "win32" && command === "cargo" ? "cargo.exe" : command;
  const result = spawnSync(resolvedCommand, args, {
    cwd: root,
    stdio: "inherit"
  });

  if (result.status !== 0) {
    throw new Error(errorMessage);
  }
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
