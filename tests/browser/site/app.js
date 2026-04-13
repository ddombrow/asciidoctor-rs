import init, {
  prepare_document_json,
  prepare_document_value
} from "../pkg/asciidoctor_rs.js";

const asciidoctorStylesheetHref =
  "/site/vendor/asciidoctor-default.css";
const asciidoctorStylesheetFallbackHref =
  "https://cdn.jsdelivr.net/gh/asciidoctor/asciidoctor@2.0/data/stylesheets/asciidoctor-default.css";
const fontAwesomeStylesheetHref =
  "/site/vendor/font-awesome.css";
const fontAwesomeStylesheetFallbackHref =
  "https://cdnjs.cloudflare.com/ajax/libs/font-awesome/4.7.0/css/font-awesome.min.css";
const asciidoctorFontsHref =
  "/site/vendor/google-fonts.css";
const asciidoctorFontsFallbackHref =
  "https://fonts.googleapis.com/css?family=Open+Sans:300,300italic,400,400italic,600,600italic%7CNoto+Serif:400,400italic,700,700italic%7CDroid+Sans+Mono:400,700";
const highlightJsStylesheetHref =
  "https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.11.1/styles/github.min.css";
const highlightJsScriptHref =
  "https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.11.1/highlight.min.js";

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
const filePathEl = document.querySelector("#file-path");
const loadPathButton = document.querySelector("#load-path");

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

async function loadFromPath(path) {
  if (!path.trim()) {
    updateStatus("error", "Path is empty");
    return;
  }
  updateStatus("loading", `Loading ${path}...`);
  try {
    const url = `/api/expand?path=${encodeURIComponent(path.trim())}`;
    const resp = await fetch(url);
    if (!resp.ok) {
      updateStatus("error", `Could not load: ${path} (${resp.status})`);
      return;
    }
    const text = await resp.text();
    sourceEl.value = text;
    renderSource(text);
  } catch (err) {
    updateStatus("error", String(err));
  }
}

loadPathButton?.addEventListener("click", () => loadFromPath(filePathEl.value));

filePathEl?.addEventListener("keydown", (event) => {
  if (event.key === "Enter") loadFromPath(filePathEl.value);
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
  renderSource(sourceEl.value);
});

let renderRequestId = 0;

async function preprocessSource(source) {
  const path = filePathEl?.value?.trim() ?? "";
  if (!source.includes("include::") || !path) {
    return source;
  }

  const response = await fetch("/api/preprocess", {
    method: "POST",
    headers: {
      "Content-Type": "application/json"
    },
    body: JSON.stringify({ source, path })
  });
  if (response.status === 404) {
    const fallback = await fetch(`/api/expand?path=${encodeURIComponent(path)}`);
    if (!fallback.ok) {
      throw new Error(`Preprocess failed (${response.status})`);
    }
    return await fallback.text();
  }
  if (!response.ok) {
    throw new Error(`Preprocess failed (${response.status})`);
  }
  return await response.text();
}

async function renderSource(source) {
  if (window.__asciidoctorState !== "ready") {
    return;
  }

  try {
    const requestId = ++renderRequestId;
    const expanded = await preprocessSource(source);
    if (requestId !== renderRequestId) {
      return;
    }
    const json = prepare_document_json(expanded);
    const document = prepare_document_value(expanded);

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

function getAttribute(attributes, key) {
  if (!attributes) {
    return undefined;
  }

  if (typeof attributes.get === "function") {
    return attributes.get(key);
  }

  return attributes[key];
}

function renderDocument(document) {
  const title = document.title ? `<h1>${escapeHtml(document.title)}</h1>` : "";
  const attributes = document.attributes ?? {};
  const sections = document.sections ?? [];
  const tocPlacement = attributes.toc;
  const autoToc = tocPlacement !== undefined && tocPlacement !== "macro"
    ? renderToc(sections, attributes)
    : "";
  const blocks = renderBlocks(document.blocks ?? [], 0, attributes, sections);
  const footnotes = renderFootnotes(document.footnotes ?? []);

  return `
    <div id="header">
      ${title}
      ${autoToc}
    </div>
    <div id="content">
      ${blocks}
    </div>
    ${footnotes}
  `;
}

function renderToc(sections, documentAttributes = {}) {
  if (!sections.length) return "";
  const title = documentAttributes.toctitle || "Table of Contents";
  const maxLevel = parseInt(documentAttributes.toclevels ?? "2", 10);
  return `
    <div id="toc" class="toc">
      <div id="toctitle">${escapeHtml(title)}</div>
      ${renderTocSections(sections, 1, maxLevel)}
    </div>
  `;
}

function renderTocSections(sections, level, maxLevel) {
  if (level > maxLevel || !sections.length) return "";
  const items = sections.map((section) => {
    const nested = section.sections?.length && level < maxLevel
      ? renderTocSections(section.sections, level + 1, maxLevel)
      : "";
    return `<li><a href="#${escapeHtml(section.id)}">${escapeHtml(section.title)}</a>${nested ? "\n" + nested : ""}</li>`;
  }).join("\n");
  return `<ul class="sectlevel${level}">\n${items}\n</ul>`;
}

function renderHeadMetadata(document) {
  const authorTags = (document.authors ?? []).flatMap((author) => {
      const tags = [];
      if (author?.name) {
        tags.push(`<meta name="author" content="${escapeHtml(author.name)}" />`);
      }
      if (author?.email) {
        tags.push(`<meta name="email" content="${escapeHtml(author.email)}" />`);
      }
      return tags;
    });
  const revisionTags = [];
  if (document.revision?.number) {
    revisionTags.push(`<meta name="revnumber" content="${escapeHtml(document.revision.number)}" />`);
  }
  if (document.revision?.date) {
    revisionTags.push(`<meta name="revdate" content="${escapeHtml(document.revision.date)}" />`);
  }
  if (document.revision?.remark) {
    revisionTags.push(`<meta name="revremark" content="${escapeHtml(document.revision.remark)}" />`);
  }

  return [...authorTags, ...revisionTags].join("\n");
}

function renderBlocks(blocks, sectionLevel = 0, documentAttributes = {}, sections = []) {
  return blocks.map((block) => renderBlock(block, sectionLevel, documentAttributes, sections)).join("");
}

function trimDelimitedBlockLines(content) {
  const lines = content === "" ? [] : String(content).split("\n");
  let start = 0;
  let end = lines.length;

  while (start < end && lines[start].trim() === "") {
    start += 1;
  }
  while (end > start && lines[end - 1].trim() === "") {
    end -= 1;
  }

  return { lineOffset: start, lines: lines.slice(start, end) };
}

function renderBlock(block, parentSectionLevel = 0, documentAttributes = {}, sections = []) {
  if (block.type === "preamble") {
    return `
      <div id="preamble">
        <div class="sectionbody">
          ${renderBlocks(block.blocks ?? [], parentSectionLevel, documentAttributes, sections)}
        </div>
      </div>
    `;
  }

  if (block.type === "paragraph") {
    const id = block.id ? ` id="${escapeHtml(block.id)}"` : "";
    const title = block.title ? `<div class="title">${escapeHtml(block.title)}</div>` : "";
    return `
      <div class="paragraph"${id}>
        ${title}
        <p>${renderInlines(block.inlines ?? [])}</p>
        </div>
    `;
  }

  if (block.type === "admonition") {
    const id = block.id ? ` id="${escapeHtml(block.id)}"` : "";
    const title = block.title ? `<div class="title">${escapeHtml(block.title)}</div>` : "";
    const label = renderAdmonitionLabel(block.variant ?? "", block.attributes ?? {}, documentAttributes);
    const icon = renderAdmonitionIcon(block.variant ?? "", block.attributes ?? {}, documentAttributes, label);
    return `
      <div class="admonitionblock ${escapeHtml(block.variant ?? "")}"${id}>
        <table>
          <tr>
            <td class="icon">
              ${icon}
            </td>
            <td class="content">
              ${title}
              ${renderBlocks(block.blocks ?? [], parentSectionLevel, documentAttributes, sections)}
            </td>
          </tr>
        </table>
      </div>
    `;
  }

  if (block.type === "section") {
    const level = Math.min((block.level ?? 1) + 1, 6);
    const sectionClass = `sect${block.level ?? Math.max(parentSectionLevel + 1, 1)}`;
    const number =
      block.numbered && block.num
        ? `<span class="section-num">${escapeHtml(block.num)}</span>`
        : "";
    const id = block.id ? ` id="${escapeHtml(block.id)}"` : "";
    return `
      <div class="${sectionClass}"${id}>
        <h${level}>${number}${escapeHtml(block.title ?? "")}</h${level}>
        <div class="sectionbody">
          ${renderBlocks(block.blocks ?? [], block.level ?? parentSectionLevel + 1, documentAttributes, sections)}
        </div>
      </div>
    `;
  }

  if (block.type === "unordered_list") {
    const id = block.id ? ` id="${escapeHtml(block.id)}"` : "";
    const title = block.title ? `<div class="title">${escapeHtml(block.title)}</div>` : "";
    const items = (block.items ?? [])
      .map(
        (item) => `
          <li>
            ${renderBlocks(item.blocks ?? [], parentSectionLevel, documentAttributes, sections)}
          </li>
        `
      )
      .join("");
    return `
      <div class="ulist"${id}>
        ${title}
        <ul>
          ${items}
        </ul>
      </div>
    `;
  }

  if (block.type === "ordered_list") {
    const id = block.id ? ` id="${escapeHtml(block.id)}"` : "";
    const title = block.title ? `<div class="title">${escapeHtml(block.title)}</div>` : "";
    const items = (block.items ?? [])
      .map(
        (item) => `
          <li>
            ${renderBlocks(item.blocks ?? [], parentSectionLevel, documentAttributes, sections)}
          </li>
        `
      )
      .join("");
    return `
      <div class="olist arabic"${id}>
        ${title}
        <ol class="arabic">
          ${items}
        </ol>
      </div>
    `;
  }

  if (block.type === "description_list") {
    const id = block.id ? ` id="${escapeHtml(block.id)}"` : "";
    const title = block.title ? `<div class="title">${escapeHtml(block.title)}</div>` : "";
    const items = (block.items ?? [])
      .map((item) => {
        const terms = (item.terms ?? [])
          .map((term) => `<dt class="hdlist1">${renderInlines(term.inlines ?? [])}</dt>`)
          .join("");
        const description = item.description ? `\n<dd>\n${renderBlocks(item.description.blocks ?? [], parentSectionLevel, documentAttributes, sections)}\n</dd>` : "";
        return `${terms}${description}`;
      })
      .join("");
    return `
      <div class="dlist"${id}>
        ${title}
        <dl>
          ${items}
        </dl>
      </div>
    `;
  }

  if (block.type === "table") {
    const id = block.id ? ` id="${escapeHtml(block.id)}"` : "";
    const title = block.title
      ? `<caption class="title">${escapeHtml(block.title)}</caption>`
      : "";
    const header = block.header
      ? `
        <thead>
          <tr>
            ${(block.header.cells ?? [])
              .map((cell) => renderTableCell(cell, true, documentAttributes))
              .join("")}
          </tr>
        </thead>
      `
      : "";
    const rows = (block.rows ?? [])
      .map(
        (row) => `
          <tr>
            ${(row.cells ?? [])
              .map((cell) => renderTableCell(cell, false, documentAttributes))
              .join("")}
          </tr>
        `
      )
      .join("");
    return `
      <table class="tableblock frame-all grid-all stretch"${id}>
        ${title}
        ${header}
        <tbody>
          ${rows}
        </tbody>
      </table>
    `;
  }

  if (block.type === "literal") {
    const id = block.id ? ` id="${escapeHtml(block.id)}"` : "";
    const title = block.title
      ? `<div class="title">${escapeHtml(block.title)}</div>`
      : "";
    const content = trimDelimitedBlockLines(block.content ?? "").lines.join("\n");
    return `
      <div class="literalblock"${id}>
        ${title}
        <div class="content">
          <pre>${escapeHtml(content)}</pre>
        </div>
      </div>
    `;
  }

  if (block.type === "listing") {
    const id = block.id ? ` id="${escapeHtml(block.id)}"` : "";
    const title = block.title
      ? `<div class="title">${escapeHtml(block.title)}</div>`
      : "";
    const calloutMap = Object.fromEntries(
      (block.calloutLines ?? []).map(([i, n]) => [i, n])
    );
    const { lineOffset, lines } = trimDelimitedBlockLines(block.content ?? "");
    const renderedContent = lines
      .map((line, i) =>
        calloutMap[i + lineOffset] !== undefined
          ? `${escapeHtml(line)}<i class="conum" data-value="${calloutMap[i + lineOffset]}"></i><b>${calloutMap[i + lineOffset]}</b>`
          : escapeHtml(line)
      )
      .join("\n");
    const lang = block.attributes?.language;
    const isSource = block.style === "source" && lang;
    const innerHtml = isSource
      ? `<pre class="highlight"><code class="language-${escapeHtml(lang)}" data-lang="${escapeHtml(lang)}">${renderedContent}</code></pre>`
      : `<pre>${renderedContent}</pre>`;
    const wrappedInnerHtml = innerHtml;
    return `
      <div class="listingblock"${id}>
        ${title}
        <div class="content">
          ${wrappedInnerHtml}
        </div>
      </div>
    `;
  }

  if (block.type === "callout_list") {
    const rows = (block.items ?? [])
      .map(item => {
        const n = item.number;
        return `<tr>\n<td><i class="conum" data-value="${n}"></i><b>${n}</b></td>\n<td>${renderInlines(item.inlines ?? [])}</td>\n</tr>`;
      })
      .join("\n");
    return `<div class="colist arabic">\n<table>\n<tbody>\n${rows}\n</tbody>\n</table>\n</div>`;
  }

  if (block.type === "example") {
    const id = block.id ? ` id="${escapeHtml(block.id)}"` : "";
    const title = block.title
      ? `<div class="title">${escapeHtml(block.title)}</div>`
      : "";
    return `
      <div class="exampleblock"${id}>
        ${title}
        <div class="content">
          ${renderBlocks(block.blocks ?? [], parentSectionLevel, documentAttributes, sections)}
        </div>
      </div>
    `;
  }

  if (block.type === "sidebar") {
    const id = block.id ? ` id="${escapeHtml(block.id)}"` : "";
    const title = block.title
      ? `<div class="title">${escapeHtml(block.title)}</div>`
      : "";
    return `
      <div class="sidebarblock"${id}>
        <div class="content">
          ${title}
          ${renderBlocks(block.blocks ?? [], parentSectionLevel, documentAttributes, sections)}
        </div>
      </div>
    `;
  }

  if (block.type === "quote") {
    const id = block.id ? ` id="${escapeHtml(block.id)}"` : "";
    const title = block.title ? `<div class="title">${escapeHtml(block.title)}</div>` : "";
    const attribution = block.attribution || block.citetitle
      ? `<div class="attribution">\n&#8212; ${escapeHtml(block.attribution ?? "")}${block.citetitle ? `<br>\n<cite>${escapeHtml(block.citetitle)}</cite>` : ""}\n</div>`
      : "";
    if (block.isVerse) {
      const content = trimDelimitedBlockLines(block.content ?? "").lines.join("\n");
      return `
        <div class="verseblock"${id}>
          ${title}
          <pre class="content">${escapeHtml(content)}</pre>
          ${attribution}
        </div>
      `;
    }
    return `
      <div class="quoteblock"${id}>
        ${title}
        <blockquote>
          ${renderBlocks(block.blocks ?? [], parentSectionLevel, documentAttributes, sections)}
        </blockquote>
        ${attribution}
      </div>
    `;
  }

  if (block.type === "open") {
    const id = block.id ? ` id="${escapeHtml(block.id)}"` : "";
    const title = block.title ? `<div class="title">${escapeHtml(block.title)}</div>` : "";
    return `
      <div class="openblock"${id}>
        ${title}
        <div class="content">
          ${renderBlocks(block.blocks ?? [], parentSectionLevel, documentAttributes, sections)}
        </div>
      </div>
    `;
  }

  if (block.type === "passthrough") {
    return trimDelimitedBlockLines(block.content ?? "").lines.join("\n");
  }

  if (block.type === "image") {
    const classes = ["imageblock"];
    if (block.float) classes.push(block.float);
    if (block.align) classes.push(`text-${block.align}`);
    if (block.role) classes.push(block.role);
    const id = block.id ? ` id="${escapeHtml(block.id)}"` : "";
    const src = resolveImageSrc(block.target ?? "", documentAttributes);
    const widthAttr = block.width ? ` width="${escapeHtml(block.width)}"` : "";
    const heightAttr = block.height ? ` height="${escapeHtml(block.height)}"` : "";
    const imgTag = `<img src="${escapeHtml(src)}" alt="${escapeHtml(block.alt ?? "")}"${widthAttr}${heightAttr}>`;
    let content = imgTag;
    if (block.link) {
      const href = block.link === "self" ? src : block.link;
      content = `<a class="image" href="${escapeHtml(href)}">${imgTag}</a>`;
    }
    const title = block.title ? `<div class="title">${escapeHtml(block.title)}</div>` : "";
    return `
      <div class="${classes.join(" ")}"${id}>
        <div class="content">
          ${content}
        </div>
        ${title}
      </div>
    `;
  }

  if (block.type === "toc") {
    return renderToc(sections, documentAttributes);
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
                : inline.variant === "subscript"
                  ? "sub"
                  : inline.variant === "superscript"
                    ? "sup"
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

      if (inline.type === "passthrough") {
        return inline.value ?? "";
      }

      if (inline.type === "image") {
        const widthAttr = inline.width ? ` width="${escapeHtml(inline.width)}"` : "";
        const heightAttr = inline.height ? ` height="${escapeHtml(inline.height)}"` : "";
        return `<span class="image"><img src="${escapeHtml(inline.target ?? "")}" alt="${escapeHtml(inline.alt ?? "")}"${widthAttr}${heightAttr}></span>`;
      }

      if (inline.type === "icon") {
        const name = escapeHtml(inline.name ?? "");
        let classes = `fa fa-${name}`;
        if (inline.size) classes += ` fa-${escapeHtml(inline.size)}`;
        if (inline.role) classes += ` ${escapeHtml(inline.role)}`;
        const titleAttr = inline.title ? ` title="${escapeHtml(inline.title)}"` : "";
        return `<span class="icon"><i class="${classes}"${titleAttr}></i></span>`;
      }

      if (inline.type === "footnote") {
        const index = inline.index ?? 0;
        return `<sup class="footnote" id="_footnoteref_${index}"><a href="#_footnotedef_${index}">${index}</a></sup>`;
      }

      return escapeHtml(JSON.stringify(inline));
    })
    .join("");
}

function renderFootnotes(footnotes) {
  if (!footnotes.length) {
    return "";
  }

  const items = footnotes
    .map((footnote) => {
      const index = footnote.index ?? 0;
      return `
        <div class="footnote" id="_footnotedef_${index}">
          <a href="#_footnoteref_${index}">${index}</a>. ${renderInlines(footnote.inlines ?? [])}
        </div>
      `;
    })
    .join("");

  return `
    <div id="footnotes">
      <hr />
      ${items}
    </div>
  `;
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
      <link rel="stylesheet" href="${fontAwesomeStylesheetHref}" onerror="this.onerror=null;this.href='${fontAwesomeStylesheetFallbackHref}'" />
      <link rel="stylesheet" href="${asciidoctorFontsHref}" onerror="this.onerror=null;this.href='${asciidoctorFontsFallbackHref}'" />
      <link rel="stylesheet" href="${asciidoctorStylesheetHref}" onerror="this.onerror=null;this.href='${asciidoctorStylesheetFallbackHref}'" />
      <link rel="stylesheet" href="${highlightJsStylesheetHref}" />
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
      <script src="${highlightJsScriptHref}" onload="hljs.highlightAll()"></script>
    </body>
  </html>`);
  doc.close();
}

function renderAdmonitionLabel(variant, blockAttributes = {}, documentAttributes = {}) {
  const caption = getAttribute(blockAttributes, "caption");
  if (typeof caption === "string" && caption.length > 0) {
    return caption;
  }

  const documentCaption = getAttribute(documentAttributes, `${variant}-caption`);
  if (typeof documentCaption === "string" && documentCaption.length > 0) {
    return documentCaption;
  }

  if (variant === "note") return "Note";
  if (variant === "tip") return "Tip";
  if (variant === "important") return "Important";
  if (variant === "caution") return "Caution";
  if (variant === "warning") return "Warning";
  return variant;
}

function renderAdmonitionIcon(variant, blockAttributes = {}, documentAttributes = {}, label = variant) {
  const fontIconClass = resolveAdmonitionFontIconClass(variant, blockAttributes, documentAttributes);
  if (fontIconClass) {
    return `<i class="fa ${escapeHtml(fontIconClass)}" title="${escapeHtml(label)}"></i>`;
  }

  const iconTarget = resolveAdmonitionIconTarget(variant, blockAttributes, documentAttributes);
  if (!iconTarget) {
    return `<div class="title">${escapeHtml(label)}</div>`;
  }

  return `<img src="${escapeHtml(iconTarget)}" alt="${escapeHtml(label)}" />`;
}

function renderTableCell(cell, header = false, documentAttributes = {}) {
  const tag = header || cell.style === "header" ? "th" : "td";
  const colspan = cell.colspan > 1 ? ` colspan="${escapeHtml(String(cell.colspan))}"` : "";
  const rowspan = cell.rowspan > 1 ? ` rowspan="${escapeHtml(String(cell.rowspan))}"` : "";
  return `<${tag} class="tableblock halign-left valign-top"${colspan}${rowspan}>${renderTableCellContent(cell, tag === "th", documentAttributes)}</${tag}>`;
}

function renderTableCellContent(cell, header = false, documentAttributes = {}) {
  const blocks = cell.blocks ?? [];
  if (blocks.length === 1 && blocks[0].type === "paragraph") {
    const inlines = blocks[0].inlines ?? cell.inlines ?? [];
    if (header) {
      return renderInlines(inlines);
    }
    return `<p class="tableblock">${renderInlines(inlines)}</p>`;
  }

  return renderBlocks(blocks, 0, documentAttributes);
}

function resolveAdmonitionFontIconClass(variant, blockAttributes = {}, documentAttributes = {}) {
  const icons = getNamedAttribute(blockAttributes, documentAttributes, "icons");
  if (icons !== "font") {
    return undefined;
  }

  if (variant === "note") return "icon-note";
  if (variant === "tip") return "icon-tip";
  if (variant === "important") return "icon-important";
  if (variant === "caution") return "icon-caution";
  if (variant === "warning") return "icon-warning";
  return "icon-note";
}

function resolveAdmonitionIconTarget(variant, blockAttributes = {}, documentAttributes = {}) {
  const icons = getNamedAttribute(blockAttributes, documentAttributes, "icons");
  if (typeof icons !== "string" || icons === "font") {
    return undefined;
  }

  const iconName = getNamedAttribute(blockAttributes, documentAttributes, "icon") || variant;
  const iconsdir = getNamedAttribute(blockAttributes, documentAttributes, "iconsdir") || "./images/icons";
  const separator = iconsdir.endsWith("/") || iconsdir.endsWith("\\") ? "" : "/";

  if (iconNameHasExtension(iconName)) {
    return `${iconsdir}${separator}${iconName}`;
  }

  const iconType =
    getNamedAttribute(blockAttributes, documentAttributes, "icontype") ||
    (icons !== "" && icons !== "image" ? icons : "png");

  return `${iconsdir}${separator}${iconName}.${iconType}`;
}

function getNamedAttribute(blockAttributes, documentAttributes, key) {
  const blockValue = getAttribute(blockAttributes, key);
  if (typeof blockValue === "string") {
    return blockValue;
  }

  const documentValue = getAttribute(documentAttributes, key);
  return typeof documentValue === "string" ? documentValue : undefined;
}

function iconNameHasExtension(iconName) {
  const fileName = String(iconName).split(/[\\/]/).pop() ?? String(iconName);
  return fileName.includes(".");
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

function resolveImageSrc(target, documentAttributes) {
  if (target.startsWith("http://") || target.startsWith("https://") || target.startsWith("data:") || target.startsWith("/")) {
    return target;
  }
  const imagesdir = getAttribute(documentAttributes, "imagesdir");
  if (imagesdir) {
    const dir = imagesdir.replace(/\/+$/, "");
    return `${dir}/${target}`;
  }
  return target;
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;");
}
