import { cpSync, existsSync, mkdirSync, rmSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..");
const outDir = resolve(root, "tests", "browser", "pkg");
const wasmBindgenVersion = "0.2.115";
const wasmBindgenExe = resolve(
  root,
  ".tools",
  "wasm-bindgen",
  `wasm-bindgen-${wasmBindgenVersion}-x86_64-pc-windows-msvc`,
  "wasm-bindgen.exe"
);
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
    `Missing wasm-bindgen executable at ${wasmBindgenExe}. Run scripts/install-wasm-bindgen.ps1 first.`
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
  const result = spawnSync(command, args, {
    cwd: root,
    stdio: "inherit",
    shell: process.platform === "win32"
  });

  if (result.status !== 0) {
    throw new Error(errorMessage);
  }
}
