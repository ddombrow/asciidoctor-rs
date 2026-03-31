import init, {
  prepare_document_json,
  prepare_document_value
} from "../pkg/asciidoctor_rs.js";

const asciidoctorStylesheetHref =
  "/site/vendor/asciidoctor-default.css";
const asciidoctorStylesheetFallbackHref =
  "https://cdn.jsdelivr.net/gh/asciidoctor/asciidoctor@2.0/data/stylesheets/asciidoctor-default.css";
const asciidoctorFontsHref =
  "/site/vendor/google-fonts.css";
const asciidoctorFontsFallbackHref =
  "https://fonts.googleapis.com/css?family=Open+Sans:300,300italic,400,400italic,600,600italic%7CNoto+Serif:400,400italic,700,700italic%7CDroid+Sans+Mono:400,700";

const sample = `= Sample Document

An introductory paragraph for the sample document.

== First Section

This section gives the parser a level-1 heading to recognize.

=== Nested Section

This subsection is here so we can grow section handling next.
`;

const statusEl = document.querySelector("[data-status]");
const sourceEl = document.querySelector("#source");
const jsonEl = document.querySelector("#json-output");
const previewFrameEl = document.querySelector("#preview-frame");
const renderButton = document.querySelector("#render");
const sampleButton = document.querySelector("#load-sample");
let renderTimer = null;

window.__asciidoctorState = "loading";
updateStatus("loading", "Initializing WASM module...");

window.__asciidoctorReady = init()
  .then(() => {
    window.__prepareDocumentJson = prepare_document_json;
    window.__prepareDocumentValue = prepare_document_value;
    window.__asciidoctorState = "ready";
    updateStatus("ready", "WASM module ready");
    sourceEl.value = sample;
    renderSource(sample);
  })
  .catch((error) => {
    window.__asciidoctorState = "error";
    window.__asciidoctorError = error instanceof Error ? error.message : String(error);
    updateStatus("error", `Initialization failed: ${window.__asciidoctorError}`);
    throw error;
  });

renderButton.addEventListener("click", () => {
  renderSource(sourceEl.value);
});

sampleButton.addEventListener("click", () => {
  sourceEl.value = sample;
  renderSource(sample);
});

sourceEl.addEventListener("keydown", (event) => {
  if ((event.ctrlKey || event.metaKey) && event.key === "Enter") {
    renderSource(sourceEl.value);
  }
});

sourceEl.addEventListener("input", () => {
  if (window.__asciidoctorState !== "ready") {
    return;
  }

  if (renderTimer !== null) {
    clearTimeout(renderTimer);
  }

  renderTimer = window.setTimeout(() => {
    renderTimer = null;
    renderSource(sourceEl.value);
  }, 120);
});

function renderSource(source) {
  if (window.__asciidoctorState !== "ready") {
    return;
  }

  try {
    const json = prepare_document_json(source);
    const document = prepare_document_value(source);

    renderJson(json);
    renderPreview(document);
    updateStatus("ready", "Rendered successfully");
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    jsonEl.textContent = message;
    renderPreviewError(message);
    updateStatus("error", message);
  }
}

function renderJson(json) {
  const lines = json
    .split("\n")
    .map((line) => `<span class="json-line">${highlightJsonLine(line)}</span>`)
    .join("");
  jsonEl.innerHTML = lines;
}

function highlightJsonLine(line) {
  const escaped = escapeHtml(line);
  return escaped
    .replace(
      /(&quot;(?:\\.|[^&]|&(?!quot;))*?&quot;)(\s*:)?/g,
      (_, stringToken, colon) =>
        colon
          ? `<span class="json-key">${stringToken}</span><span class="json-punc">${colon}</span>`
          : `<span class="json-string">${stringToken}</span>`
    )
    .replace(/\b(true|false)\b/g, '<span class="json-boolean">$1</span>')
    .replace(/\bnull\b/g, '<span class="json-null">$&</span>')
    .replace(/(-?\b\d+(?:\.\d+)?(?:[eE][+-]?\d+)?\b)/g, '<span class="json-number">$1</span>')
    .replace(/([{}[\],])/g, '<span class="json-punc">$1</span>');
}

function renderDocument(document) {
  const title = document.title ? `<h1>${escapeHtml(document.title)}</h1>` : "";
  const blocks = renderBlocks(document.blocks ?? []);

  return `
    <div id="header">
      ${title}
    </div>
    <div id="content">
      ${blocks}
    </div>
  `;
}

function renderHeadMetadata(document) {
  return (document.authors ?? [])
    .flatMap((author) => {
      const tags = [];
      if (author?.name) {
        tags.push(`<meta name="author" content="${escapeHtml(author.name)}" />`);
      }
      if (author?.email) {
        tags.push(`<meta name="email" content="${escapeHtml(author.email)}" />`);
      }
      return tags;
    })
    .join("\n");
}

function renderBlocks(blocks, sectionLevel = 0) {
  return blocks.map((block) => renderBlock(block, sectionLevel)).join("");
}

function renderBlock(block, parentSectionLevel = 0) {
  if (block.type === "preamble") {
    return `
      <div id="preamble">
        <div class="sectionbody">
          ${renderBlocks(block.blocks ?? [], parentSectionLevel)}
        </div>
      </div>
    `;
  }

  if (block.type === "paragraph") {
    const id = block.id ? ` id="${escapeHtml(block.id)}"` : "";
    return `
      <div class="paragraph"${id}>
        <p>${renderInlines(block.inlines ?? [])}</p>
        </div>
    `;
  }

  if (block.type === "section") {
    const level = Math.min((block.level ?? 1) + 1, 6);
    const sectionClass = `sect${block.level ?? Math.max(parentSectionLevel + 1, 1)}`;
    const blocks = renderBlocks(block.blocks ?? [], block.level ?? parentSectionLevel + 1);
    const number =
      block.numbered && block.num
        ? `<span class="section-num">${escapeHtml(block.num)}</span>`
        : "";
    const id = block.id ? ` id="${escapeHtml(block.id)}"` : "";
    return `
      <div class="${sectionClass}"${id}>
        <h${level}>${number}${escapeHtml(block.title ?? "")}</h${level}>
        <div class="sectionbody">
          ${blocks}
        </div>
      </div>
    `;
  }

  if (block.type === "unordered_list") {
    const items = (block.items ?? [])
      .map(
        (item) => `
          <li>
            ${renderBlocks(item.blocks ?? [], parentSectionLevel)}
          </li>
        `
      )
      .join("");
    return `
      <div class="ulist">
        <ul>
          ${items}
        </ul>
      </div>
    `;
  }

  if (block.type === "ordered_list") {
    const items = (block.items ?? [])
      .map(
        (item) => `
          <li>
            ${renderBlocks(item.blocks ?? [], parentSectionLevel)}
          </li>
        `
      )
      .join("");
    return `
      <div class="olist arabic">
        <ol class="arabic">
          ${items}
        </ol>
      </div>
    `;
  }

  return `<pre class="unknown-block">${escapeHtml(JSON.stringify(block, null, 2))}</pre>`;
}

function renderInlines(inlines) {
  return inlines
    .map((inline) => {
      if (inline.type === "text") {
        return escapeHtml(inline.value ?? "");
      }

      if (inline.type === "span") {
        const tag =
          inline.variant === "strong"
            ? "strong"
            : inline.variant === "emphasis"
              ? "em"
              : inline.variant === "monospace"
                ? "code"
              : "span";
        return `<${tag}>${renderInlines(inline.inlines ?? [])}</${tag}>`;
      }

      if (inline.type === "link") {
        const bare = inline.bare ? ' class="bare"' : "";
        const target = inline.window ? ` target="${escapeHtml(inline.window)}"` : "";
        const rel = inline.window === "_blank" ? ' rel="noopener"' : "";
        return `<a href="${escapeHtml(inline.target ?? "")}"${bare}${target}${rel}>${renderInlines(inline.inlines ?? [])}</a>`;
      }

      if (inline.type === "xref") {
        return `<a href="${escapeHtml(inline.href ?? inline.target ?? "")}">${renderInlines(inline.inlines ?? [])}</a>`;
      }

      if (inline.type === "anchor") {
        return `<a id="${escapeHtml(inline.id ?? "")}"></a>${renderInlines(inline.inlines ?? [])}`;
      }

      return escapeHtml(JSON.stringify(inline));
    })
    .join("");
}

function renderPreview(document) {
  const doc = previewFrameEl.contentDocument;
  if (!doc) {
    throw new Error("Preview frame is not available");
  }

  doc.open();
  doc.write(`<!doctype html>
  <html lang="en">
    <head>
      <meta charset="utf-8" />
      ${renderHeadMetadata(document)}
      <link rel="stylesheet" href="${asciidoctorFontsHref}" onerror="this.onerror=null;this.href='${asciidoctorFontsFallbackHref}'" />
      <link rel="stylesheet" href="${asciidoctorStylesheetHref}" onerror="this.onerror=null;this.href='${asciidoctorStylesheetFallbackHref}'" />
      <style>
        body {
          margin: 0;
          padding: 0;
          background: white;
        }

        .page-shell {
          background: white;
          min-height: 100vh;
        }

        .page-shell > #header,
        .page-shell > #content,
        .page-shell > #footnotes,
        .page-shell > #footer {
          width: auto;
          max-width: none;
        }

        .page-shell > #content {
          padding-bottom: 2rem;
        }
      </style>
    </head>
    <body class="article">
      <div class="page-shell">
        ${renderDocument(document)}
      </div>
    </body>
  </html>`);
  doc.close();
}

function renderPreviewError(message) {
  const doc = previewFrameEl.contentDocument;
  if (!doc) {
    return;
  }

  doc.open();
  doc.write(`<!doctype html>
  <html lang="en">
    <head>
      <meta charset="utf-8" />
      <style>
        body {
          margin: 0;
          padding: 20px;
          font-family: "Segoe UI", sans-serif;
          background: #fff0f0;
          color: #8b1e1e;
        }
      </style>
    </head>
    <body>${escapeHtml(message)}</body>
  </html>`);
  doc.close();
}

function updateStatus(kind, message) {
  statusEl.dataset.kind = kind;
  statusEl.textContent = message;
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;");
}
