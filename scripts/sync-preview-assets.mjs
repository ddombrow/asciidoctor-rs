import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const vendorDir = resolve(root, "tests", "browser", "site", "vendor");
const fontsDir = resolve(vendorDir, "fonts");

const asciidoctorStylesheetUrl =
  "https://cdn.jsdelivr.net/gh/asciidoctor/asciidoctor@2.0/data/stylesheets/asciidoctor-default.css";
const googleFontsCssUrl =
  "https://fonts.googleapis.com/css?family=Open+Sans:300,300italic,400,400italic,600,600italic%7CNoto+Serif:400,400italic,700,700italic%7CDroid+Sans+Mono:400,700";

mkdirSync(vendorDir, { recursive: true });
mkdirSync(fontsDir, { recursive: true });

await syncAsciidoctorStylesheet();
await syncGoogleFonts();

console.log("Preview assets synchronized to tests/browser/site/vendor");

async function syncAsciidoctorStylesheet() {
  const response = await fetch(asciidoctorStylesheetUrl);
  if (!response.ok) {
    throw new Error(`Failed to download Asciidoctor stylesheet: ${response.status} ${response.statusText}`);
  }

  writeFileSync(
    resolve(vendorDir, "asciidoctor-default.css"),
    await response.text(),
    "utf8"
  );
}

async function syncGoogleFonts() {
  const response = await fetch(googleFontsCssUrl, {
    headers: {
      "user-agent":
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/137.0.0.0 Safari/537.36"
    }
  });

  if (!response.ok) {
    throw new Error(`Failed to download Google Fonts stylesheet: ${response.status} ${response.statusText}`);
  }

  const css = await response.text();
  const fontUrls = [...css.matchAll(/url\((https:\/\/fonts\.gstatic\.com\/[^)]+)\)/g)].map(
    (match) => match[1]
  );

  let rewrittenCss = css;
  for (const fontUrl of new Set(fontUrls)) {
    const fontResponse = await fetch(fontUrl);
    if (!fontResponse.ok) {
      throw new Error(`Failed to download font asset: ${fontResponse.status} ${fontResponse.statusText}`);
    }

    const fileName = fontUrl.split("/").pop()?.split("?")[0];
    if (!fileName) {
      throw new Error(`Could not derive filename from font URL: ${fontUrl}`);
    }

    const fontPath = resolve(fontsDir, fileName);
    writeFileSync(fontPath, Buffer.from(await fontResponse.arrayBuffer()));
    rewrittenCss = rewrittenCss.replaceAll(fontUrl, `/site/vendor/fonts/${fileName}`);
  }

  writeFileSync(resolve(vendorDir, "google-fonts.css"), rewrittenCss, "utf8");
}
