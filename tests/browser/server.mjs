import { createServer } from "node:http";
import { readFileSync, existsSync, statSync } from "node:fs";
import { join, normalize, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(fileURLToPath(new URL(".", import.meta.url)));
const serverRoot = normalize(root);

const contentTypes = {
  ".adoc": "text/plain; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".json": "application/json; charset=utf-8",
  ".wasm": "application/wasm"
};

createServer((request, response) => {
  const url = new URL(request.url ?? "/", "http://127.0.0.1");
  const pathname = url.pathname === "/" ? "/site/index.html" : url.pathname;
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
    "Content-Type": contentType
  });
  response.end(readFileSync(filePath));
}).listen(4173, "127.0.0.1", () => {
  console.log("Browser test server listening on http://127.0.0.1:4173");
});
