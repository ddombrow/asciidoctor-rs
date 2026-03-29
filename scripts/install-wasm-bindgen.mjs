import { createWriteStream, existsSync, mkdirSync } from "node:fs";
import { rm, stat } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { pipeline } from "node:stream/promises";
import { spawn } from "node:child_process";

const version = "0.2.115";
const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const toolsDir = resolve(root, ".tools", "wasm-bindgen");
const targetTriple = resolveTargetTriple();
const archiveName = `wasm-bindgen-${version}-${targetTriple}.tar.gz`;
const archivePath = resolve(toolsDir, archiveName);
const installDir = resolve(toolsDir, `wasm-bindgen-${version}-${targetTriple}`);
const executableName = process.platform === "win32" ? "wasm-bindgen.exe" : "wasm-bindgen";
const executablePath = resolve(installDir, executableName);
const url = `https://github.com/rustwasm/wasm-bindgen/releases/download/${version}/${archiveName}`;

await mkdirIfNeeded(toolsDir);

if (existsSync(executablePath)) {
  console.log(`wasm-bindgen already installed at ${executablePath}`);
  process.exit(0);
}

console.log(`Downloading ${url}`);
const response = await fetch(url);
if (!response.ok || response.body == null) {
  throw new Error(`Failed to download wasm-bindgen archive: ${response.status} ${response.statusText}`);
}

await pipeline(response.body, createWriteStream(archivePath));

console.log(`Extracting ${archiveName}`);
await extractArchive(archivePath, toolsDir);

await ensureInstalled(executablePath);
console.log(`Installed wasm-bindgen to ${installDir}`);

async function mkdirIfNeeded(path) {
  mkdirSync(path, { recursive: true });
}

function resolveTargetTriple() {
  const archMap = {
    x64: "x86_64",
    arm64: "aarch64"
  };

  const platformMap = {
    win32: "pc-windows-msvc",
    linux: "unknown-linux-musl",
    darwin: "apple-darwin"
  };

  const arch = archMap[process.arch];
  const platform = platformMap[process.platform];

  if (arch == null || platform == null) {
    throw new Error(
      `Unsupported host platform for wasm-bindgen installer: ${process.platform}/${process.arch}`
    );
  }

  // The official release artifacts we use on Windows are the x64 MSVC builds.
  // They work on this repo's current Windows ARM setup under emulation.
  if (process.platform === "win32") {
    return `x86_64-${platform}`;
  }

  return `${arch}-${platform}`;
}

async function extractArchive(archive, destination) {
  const tar = process.platform === "win32" ? "tar.exe" : "tar";

  await new Promise((resolvePromise, rejectPromise) => {
    const child = spawn(tar, ["-xzf", archive, "-C", destination], {
      cwd: root,
      stdio: "inherit"
    });

    child.on("error", rejectPromise);
    child.on("exit", (code) => {
      if (code === 0) {
        resolvePromise();
      } else {
        rejectPromise(new Error(`tar extraction failed with exit code ${code ?? "unknown"}`));
      }
    });
  });
}

async function ensureInstalled(path) {
  try {
    const info = await stat(path);
    if (!info.isFile()) {
      throw new Error(`Expected file at ${path}`);
    }
  } catch (error) {
    await rm(archivePath, { force: true }).catch(() => undefined);
    throw error;
  }
}
