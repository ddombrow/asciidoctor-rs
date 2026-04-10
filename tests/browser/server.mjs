import { createServer } from "node:http";
import { readFileSync, existsSync, statSync, realpathSync } from "node:fs";
import { join, normalize, resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(fileURLToPath(new URL(".", import.meta.url)));
const serverRoot = normalize(root);
const projectRoot = normalize(resolve(root, "..", ".."));

const contentTypes = {
  ".adoc": "text/plain; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".json": "application/json; charset=utf-8",
  ".svg": "image/svg+xml; charset=utf-8",
  ".wasm": "application/wasm"
};

function openingBlockDelimiter(line) {
  const trimmed = line.trim();
  if (trimmed === "--") return null;
  if (trimmed.length < 4) return null;
  const marker = trimmed[0];
  if (!"-=*+_./".includes(marker)) return null;
  return [...trimmed].every((ch) => ch === marker) ? trimmed : null;
}

// Mirrors the Rust preprocessor: expand include:: directives recursively.
function expandIncludes(filePath, seen = new Set(), depth = 0) {
  return expandIncludesInText(readFileSync(filePath, "utf8"), dirname(filePath), seen, depth);
}

function expandIncludesInText(content, baseDir, seen = new Set(), depth = 0) {
  if (depth > 64) return content;
  const out = [];
  let openDelim = null;

  for (const rawLine of content.split("\n")) {
    const line = rawLine.replace(/\r$/, "");
    if (openDelim) {
      out.push(line);
      if (line.trim() === openDelim) openDelim = null;
      continue;
    }
    const delimiter = openingBlockDelimiter(line);
    if (delimiter) {
      openDelim = delimiter;
      out.push(line);
      continue;
    }
    const m = line.match(/^include::([^\[]+)\[([^\]]*)\]$/);
    if (m) {
      const includePath = join(baseDir, m[1]);
      if (existsSync(includePath)) {
        const canonical = realpathSync(includePath);
        if (seen.has(canonical)) {
          continue;
        }
        seen.add(canonical);
        let expanded = expandIncludesInText(
          readFileSync(includePath, "utf8"),
          dirname(includePath),
          seen,
          depth + 1
        );
        const leveloffset = parseLevelOffset(m[2]);
        if (leveloffset !== 0) expanded = applyLevelOffset(expanded, leveloffset);
        out.push(expanded);
        if (!expanded.endsWith("\n")) out.push("");
        seen.delete(canonical);
      }
      // missing file: skip silently
      continue;
    }
    out.push(line);
  }

  return out.join("\n");
}

function parseLevelOffset(attrStr) {
  for (const part of attrStr.split(",")) {
    const m = part.trim().match(/^leveloffset=([+-]?\d+)$/);
    if (m) return parseInt(m[1], 10);
  }
  return 0;
}

function applyLevelOffset(content, offset) {
  return content.split("\n").map(line => {
    const level = line.match(/^(=+)( |$)/);
    if (!level) return line;
    const newLevel = Math.max(1, level[1].length + offset);
    return "=".repeat(newLevel) + line.slice(level[1].length);
  }).join("\n");
}

const server = createServer((request, response) => {
  const url = new URL(request.url ?? "/", "http://127.0.0.1");
  const pathname = url.pathname === "/" ? "/site/index.html" : url.pathname;

  // /api/expand?path=examples/include-demo/index.adoc
  if (pathname === "/api/expand") {
    const relPath = url.searchParams.get("path") ?? "";
    const filePath = normalize(join(projectRoot, relPath));
    if (!filePath.startsWith(projectRoot) || !existsSync(filePath)) {
      response.writeHead(404);
      response.end("Not found");
      return;
    }
    try {
      const expanded = expandIncludes(filePath);
      response.writeHead(200, {
        "Content-Type": "text/plain; charset=utf-8",
        "Cache-Control": "no-store",
      });
      response.end(expanded);
    } catch (err) {
      response.writeHead(500);
      response.end(String(err));
    }
    return;
  }

  if (pathname === "/api/preprocess" && request.method === "POST") {
    let body = "";
    request.on("data", (chunk) => {
      body += chunk;
    });
    request.on("end", () => {
      try {
        const { source = "", path = "" } = JSON.parse(body || "{}");
        const filePath = normalize(join(projectRoot, path));
        if (path && !filePath.startsWith(projectRoot)) {
          response.writeHead(400);
          response.end("Invalid path");
          return;
        }
        const baseDir = path ? dirname(filePath) : projectRoot;
        const expanded = expandIncludesInText(String(source), baseDir);
        response.writeHead(200, {
          "Content-Type": "text/plain; charset=utf-8",
          "Cache-Control": "no-store",
        });
        response.end(expanded);
      } catch (err) {
        response.writeHead(500);
        response.end(String(err));
      }
    });
    return;
  }

  const filePath = normalize(join(serverRoot, pathname));

  if (!filePath.startsWith(serverRoot) || !existsSync(filePath) || statSync(filePath).isDirectory()) {
    response.writeHead(404);
    response.end("Not found");
    return;
  }

  const lowerPath = pathname.toLowerCase();
  const contentType =
    Object.entries(contentTypes).find(([extension]) => lowerPath.endsWith(extension))?.[1] ??
    "application/octet-stream";

  response.writeHead(200, {
    "Content-Type": contentType,
    "Cache-Control": "no-store, no-cache, must-revalidate, max-age=0",
    Pragma: "no-cache",
    Expires: "0"
  });
  response.end(readFileSync(filePath));
});

server.on("error", (error) => {
  if (error.code === "EADDRINUSE") {
    console.error(
      "Browser preview server could not start because http://127.0.0.1:4173 is already in use. Stop the existing preview server and run the command again."
    );
    process.exit(1);
  }

  throw error;
});

server.listen(4173, "127.0.0.1", () => {
  console.log("Browser test server listening on http://127.0.0.1:4173");
});
