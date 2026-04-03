import { expect, test } from "@playwright/test";

const sample = "= Sample Document\n\n== First Section\n\nHello from the browser.\n";

test.beforeEach(async ({ page }) => {
  await page.goto("/");
  await page.waitForFunction(() => {
    return window.__asciidoctorState === "ready" || window.__asciidoctorState === "error";
  });
  const state = await page.evaluate(() => window.__asciidoctorState);
  expect(state).toBe("ready");
  await page.evaluate(() => window.__asciidoctorReady);
});

test("exports prepared document as JSON", async ({ page }) => {
  const json = await page.evaluate((input) => window.__prepareDocumentJson(input), sample);
  const document = JSON.parse(json);

  expect(document.type).toBe("document");
  expect(document.hasHeader).toBe(true);
  expect(document.title).toBe("Sample Document");
  expect(document.sections).toEqual([
    {
      id: "_first_section",
      title: "First Section",
      level: 1,
      num: "",
      sections: []
    }
  ]);
});

test("exports author attribute in document metadata", async ({ page }) => {
  const source = "= Sample Document\n:author: Jane Doe\n\nHello from the browser.\n";
  const json = await page.evaluate((input) => window.__prepareDocumentJson(input), source);
  const document = JSON.parse(json);

  expect(document.attributes).toMatchObject({
    author: "Jane Doe",
    firstname: "Jane",
    lastname: "Doe",
    authorinitials: "JD"
  });
  expect(document.authors).toEqual([
    {
      name: "Jane Doe"
    }
  ]);
});

test("preview writes author metadata into head", async ({ page }) => {
  const source = "= Sample Document\n:author: Jane Doe\n\nHello from the browser.\n";

  await page.fill("#source", source);
  await page.click("#render");

  const authorMeta = await page.locator("#preview-frame").evaluate((iframe) =>
    iframe.contentDocument?.querySelector('meta[name="author"]')?.getAttribute("content")
  );

  expect(authorMeta).toBe("Jane Doe");
});

test("exports email attribute in document metadata", async ({ page }) => {
  const source =
    "= Sample Document\n:author: Jane Doe\n:email: jane@example.com\n\nHello from the browser.\n";
  const json = await page.evaluate((input) => window.__prepareDocumentJson(input), source);
  const document = JSON.parse(json);

  expect(document.attributes).toMatchObject({
    author: "Jane Doe",
    email: "jane@example.com"
  });
  expect(document.authors).toEqual([
    {
      name: "Jane Doe",
      email: "jane@example.com"
    }
  ]);
});

test("preview writes email metadata into head", async ({ page }) => {
  const source =
    "= Sample Document\n:author: Jane Doe\n:email: jane@example.com\n\nHello from the browser.\n";

  await page.fill("#source", source);
  await page.click("#render");

  const metadata = await page.locator("#preview-frame").evaluate((iframe) => ({
    author: iframe.contentDocument?.querySelector('meta[name="author"]')?.getAttribute("content"),
    email: iframe.contentDocument?.querySelector('meta[name="email"]')?.getAttribute("content")
  }));

  expect(metadata).toEqual({
    author: "Jane Doe",
    email: "jane@example.com"
  });
});

test("exports revision attributes in document metadata", async ({ page }) => {
  const source =
    "= Sample Document\n:revnumber: 1.2\n:revdate: 2026-03-31\n:revremark: Draft\n\nHello from the browser.\n";
  const json = await page.evaluate((input) => window.__prepareDocumentJson(input), source);
  const document = JSON.parse(json);

  expect(document.attributes).toMatchObject({
    revnumber: "1.2",
    revdate: "2026-03-31",
    revremark: "Draft"
  });
  expect(document.revision).toEqual({
    number: "1.2",
    date: "2026-03-31",
    remark: "Draft"
  });
});

test("preview writes revision metadata into head", async ({ page }) => {
  const source =
    "= Sample Document\n:revnumber: 1.2\n:revdate: 2026-03-31\n:revremark: Draft\n\nHello from the browser.\n";

  await page.fill("#source", source);
  await page.click("#render");

  const metadata = await page.locator("#preview-frame").evaluate((iframe) => ({
    revnumber:
      iframe.contentDocument?.querySelector('meta[name="revnumber"]')?.getAttribute("content"),
    revdate: iframe.contentDocument?.querySelector('meta[name="revdate"]')?.getAttribute("content"),
    revremark:
      iframe.contentDocument?.querySelector('meta[name="revremark"]')?.getAttribute("content")
  }));

  expect(metadata).toEqual({
    revnumber: "1.2",
    revdate: "2026-03-31",
    revremark: "Draft"
  });
});

test("exports implicit header metadata in document metadata", async ({ page }) => {
  const source =
    "= Sample Document\nStuart Rackham <founder@asciidoc.org>\nv8.6.8, 2012-07-12: See changelog.\n\nHello from the browser.\n";
  const json = await page.evaluate((input) => window.__prepareDocumentJson(input), source);
  const document = JSON.parse(json);

  expect(document.attributes).toMatchObject({
    author: "Stuart Rackham",
    email: "founder@asciidoc.org",
    revnumber: "8.6.8",
    revdate: "2012-07-12",
    revremark: "See changelog."
  });
  expect(document.authors).toEqual([
    {
      name: "Stuart Rackham",
      email: "founder@asciidoc.org"
    }
  ]);
  expect(document.revision).toEqual({
    number: "8.6.8",
    date: "2012-07-12",
    remark: "See changelog."
  });
});

test("preview writes implicit header metadata into head", async ({ page }) => {
  const source =
    "= Sample Document\nStuart Rackham <founder@asciidoc.org>\nv8.6.8, 2012-07-12: See changelog.\n\nHello from the browser.\n";

  await page.fill("#source", source);
  await page.click("#render");

  const metadata = await page.locator("#preview-frame").evaluate((iframe) => ({
    author: iframe.contentDocument?.querySelector('meta[name="author"]')?.getAttribute("content"),
    email: iframe.contentDocument?.querySelector('meta[name="email"]')?.getAttribute("content"),
    revnumber:
      iframe.contentDocument?.querySelector('meta[name="revnumber"]')?.getAttribute("content"),
    revdate: iframe.contentDocument?.querySelector('meta[name="revdate"]')?.getAttribute("content"),
    revremark:
      iframe.contentDocument?.querySelector('meta[name="revremark"]')?.getAttribute("content")
  }));

  expect(metadata).toEqual({
    author: "Stuart Rackham",
    email: "founder@asciidoc.org",
    revnumber: "8.6.8",
    revdate: "2012-07-12",
    revremark: "See changelog."
  });
});

test("exports multiple implicit authors without trailing semicolon", async ({ page }) => {
  const source =
    "= Sample Document\nDoc Writer <thedoctor@asciidoc.org>; Junior Writer <junior@asciidoctor.org>\n\nHello from the browser.\n";
  const json = await page.evaluate((input) => window.__prepareDocumentJson(input), source);
  const document = JSON.parse(json);

  expect(document.attributes).toMatchObject({
    author: "Doc Writer",
    author_1: "Doc Writer",
    author_2: "Junior Writer",
    email: "thedoctor@asciidoc.org",
    email_1: "thedoctor@asciidoc.org",
    email_2: "junior@asciidoctor.org",
    authors: "Doc Writer, Junior Writer",
    authorcount: "2"
  });
  expect(document.authors).toEqual([
    {
      name: "Doc Writer",
      email: "thedoctor@asciidoc.org"
    },
    {
      name: "Junior Writer",
      email: "junior@asciidoctor.org"
    }
  ]);
});

test("exports explicit authors attribute name parts in document metadata", async ({ page }) => {
  const source = "= Sample Document\n:authors: Doc Writer; Other Author\n\nHello from the browser.\n";
  const json = await page.evaluate((input) => window.__prepareDocumentJson(input), source);
  const document = JSON.parse(json);

  expect(document.attributes).toMatchObject({
    author: "Doc Writer",
    author_1: "Doc Writer",
    author_2: "Other Author",
    firstname: "Doc",
    firstname_2: "Other",
    lastname: "Writer",
    lastname_2: "Author",
    authorinitials: "DW",
    authorinitials_2: "OA"
  });
  expect(document.authors).toEqual([
    {
      name: "Doc Writer"
    },
    {
      name: "Other Author"
    }
  ]);
});

test("preview writes multiple implicit authors into head", async ({ page }) => {
  const source =
    "= Sample Document\nDoc Writer <thedoctor@asciidoc.org>; Junior Writer <junior@asciidoctor.org>\n\nHello from the browser.\n";

  await page.fill("#source", source);
  await page.click("#render");

  const metadata = await page.locator("#preview-frame").evaluate((iframe) => ({
    authors: [...iframe.contentDocument.querySelectorAll('meta[name="author"]')].map((node) =>
      node.getAttribute("content")
    ),
    emails: [...iframe.contentDocument.querySelectorAll('meta[name="email"]')].map((node) =>
      node.getAttribute("content")
    )
  }));

  expect(metadata).toEqual({
    authors: ["Doc Writer", "Junior Writer"],
    emails: ["thedoctor@asciidoc.org", "junior@asciidoctor.org"]
  });
});

test("exports prepared document as a JS value", async ({ page }) => {
  const document = await page.evaluate((input) => window.__prepareDocumentValue(input), sample);

  expect(document.type).toBe("document");
  expect(document.blocks[0].type).toBe("section");
  expect(document.blocks[0].title).toBe("First Section");
  expect(document.blocks[0].blocks[0].content).toBe("Hello from the browser.");
});

test("preview renders strong and emphasis inline markup", async ({ page }) => {
  const source = "= Sample Document\n\nA *bold* and _emphasis_ example.\n";

  await page.fill("#source", source);
  await page.click("#render");

  const frame = page.frameLocator("#preview-frame");
  await expect(frame.locator("strong")).toHaveText("bold");
  await expect(frame.locator("em")).toHaveText("emphasis");
});

test("preview rerenders as the source changes", async ({ page }) => {
  await page.fill("#source", "= Sample Document\n\nbefore\n");

  const frame = page.frameLocator("#preview-frame");
  await expect(frame.locator("p")).toHaveCount(1);
  await expect(frame.locator("p").first()).toHaveText("before");

  await page.fill("#source", "= Sample Document\n\nA *bold* change\n");
  await expect(frame.locator("p")).toHaveCount(1);
  await expect(frame.locator("strong")).toHaveText("bold");
  await expect(frame.locator("p").first()).toHaveText("A bold change");
});

test("preview renders links", async ({ page }) => {
  const source =
    "= Sample Document\n\nSee https://example.org[example] and http://foo.com, please.\n";

  await page.fill("#source", source);

  const frame = page.frameLocator("#preview-frame");
  const links = frame.locator("a");
  await expect(links).toHaveCount(2);
  await expect(links.nth(0)).toHaveText("example");
  await expect(links.nth(0)).toHaveAttribute("href", "https://example.org");
  await expect(links.nth(1)).toHaveText("http://foo.com");
  await expect(links.nth(1)).toHaveAttribute("href", "http://foo.com");
});

test("preview renders special links with window targets", async ({ page }) => {
  const source =
    "= Sample Document\n\nSee https://example.org[example^] and link:/home.html[Home,window=_blank].\n";

  await page.fill("#source", source);

  const frame = page.frameLocator("#preview-frame");
  const links = frame.locator("a");
  await expect(links).toHaveCount(2);
  await expect(links.nth(0)).toHaveAttribute("target", "_blank");
  await expect(links.nth(0)).toHaveAttribute("rel", "noopener");
  await expect(links.nth(1)).toHaveAttribute("target", "_blank");
  await expect(links.nth(1)).toHaveAttribute("href", "/home.html");
});

test("preview renders xrefs", async ({ page }) => {
  const source = "= Sample Document\n\nSee <<First Section>>.\n\n== First Section\n\nHello.\n";

  await page.fill("#source", source);

  const frame = page.frameLocator("#preview-frame");
  const link = frame.locator("a").first();
  await expect(link).toHaveText("First Section");
  await expect(link).toHaveAttribute("href", "#_first_section");
  await expect(frame.locator("#_first_section")).toHaveCount(1);
});

test("preview renders generated section ids for direct xrefs", async ({ page }) => {
  const source = "= Sample Document\n\nSee <<_first_section>>.\n\n== First Section\n\nHello.\n";

  await page.fill("#source", source);

  const frame = page.frameLocator("#preview-frame");
  const link = frame.locator("a").first();
  await expect(link).toHaveText("First Section");
  await expect(link).toHaveAttribute("href", "#_first_section");
  await expect(frame.locator("#_first_section")).toHaveCount(1);
});

test("preview resolves explicit section anchors", async ({ page }) => {
  const source = "= Sample Document\n\nSee <<install>>.\n\n[[install,Installation]]\n== First Section\n\nHello.\n";

  await page.fill("#source", source);

  const frame = page.frameLocator("#preview-frame");
  const link = frame.locator("a").first();
  await expect(link).toHaveText("Installation");
  await expect(link).toHaveAttribute("href", "#install");
  await expect(frame.locator("#install")).toHaveCount(1);
});

test("preview resolves inline anchors", async ({ page }) => {
  const source =
    "= Sample Document\n\nSee <<bookmark-a>> and [[bookmark-a,Marked Spot]]look here.\n";

  await page.fill("#source", source);

  const frame = page.frameLocator("#preview-frame");
  const link = frame.locator("a[href=\"#bookmark-a\"]").first();
  await expect(link).toHaveText("Marked Spot");
  await expect(frame.locator("#bookmark-a")).toHaveCount(1);
});

test("preview renders phrase-applied inline anchors", async ({ page }) => {
  const source =
    "= Sample Document\n\nSee <<bookmark-b>> and [#bookmark-b]#visible text#.\n";

  await page.fill("#source", source);

  const frame = page.frameLocator("#preview-frame");
  const link = frame.locator("a[href=\"#bookmark-b\"]").first();
  await expect(link).toHaveText("visible text");
  await expect(frame.locator("#bookmark-b")).toHaveCount(1);
  await expect(frame.locator("p")).toContainText("See visible text and visible text.");
});

test("preview renders monospace inline markup", async ({ page }) => {
  const source = "= Sample Document\n\nRun `cargo test` to verify and re``link`` packages.\n";

  await page.fill("#source", source);

  const frame = page.frameLocator("#preview-frame");
  const codes = frame.locator("code");
  await expect(codes).toHaveCount(2);
  await expect(codes.nth(0)).toHaveText("cargo test");
  await expect(codes.nth(1)).toHaveText("link");
});

test("preview renders unordered lists", async ({ page }) => {
  const source = "= Sample Document\n\n* first item\n- second item\n";

  await page.fill("#source", source);

  const frame = page.frameLocator("#preview-frame");
  const items = frame.locator("ul > li");
  await expect(items).toHaveCount(2);
  await expect(items.nth(0)).toContainText("first item");
  await expect(items.nth(1)).toContainText("second item");
});

test("exports and renders delimited listing sidebar and example blocks", async ({ page }) => {
  const source = `= Sample Document

----
fn main() {
    println!("Hello from the browser!");
}
----

****
* phone
* keys
****

====
inside example
====`;

  const json = await page.evaluate((input) => window.__prepareDocumentJson(input), source);
  const document = JSON.parse(json);

  expect(document.blocks[0].type).toBe("preamble");
  expect(document.blocks[0].blocks.map((block) => block.type)).toEqual([
    "listing",
    "sidebar",
    "example"
  ]);
  expect(document.blocks[0].blocks[0].content).toContain('println!("Hello from the browser!");');

  await page.fill("#source", source);
  await page.click("#render");

  const frame = page.frameLocator("#preview-frame");
  await expect(frame.locator(".listingblock pre")).toContainText('println!("Hello from the browser!");');
  await expect(frame.locator(".sidebarblock ul > li")).toHaveCount(2);
  await expect(frame.locator(".exampleblock p")).toHaveText("inside example");
});

test("exports and renders delimited block titles and attributes", async ({ page }) => {
  const source = `= Sample Document

.Exhibit A
[source,rust]
----
fn main() {}
----

.Callout Box
[.featured,%open]
****
inside sidebar
****`;

  const json = await page.evaluate((input) => window.__prepareDocumentJson(input), source);
  const document = JSON.parse(json);
  const preambleBlocks = document.blocks[0].blocks;

  expect(preambleBlocks[0]).toMatchObject({
    type: "listing",
    title: "Exhibit A",
    style: "source"
  });
  expect(preambleBlocks[0].attributes).toMatchObject({
    title: "Exhibit A",
    style: "source",
    language: "rust"
  });
  expect(preambleBlocks[1]).toMatchObject({
    type: "sidebar",
    title: "Callout Box",
    role: "featured"
  });
  expect(preambleBlocks[1].attributes).toMatchObject({
    title: "Callout Box",
    role: "featured",
    "open-option": ""
  });

  await page.fill("#source", source);
  await page.click("#render");

  const frame = page.frameLocator("#preview-frame");
  await expect(frame.locator(".listingblock > .title")).toHaveText("Exhibit A");
  await expect(frame.locator(".sidebarblock > .content > .title")).toHaveText("Callout Box");
});

test("exports and renders admonition paragraphs", async ({ page }) => {
  const source = `= Sample Document

NOTE: This is just a note.`;

  const json = await page.evaluate((input) => window.__prepareDocumentJson(input), source);
  const document = JSON.parse(json);
  const preambleBlocks = document.blocks[0].blocks;

  expect(preambleBlocks[0]).toMatchObject({
    type: "admonition",
    variant: "note"
  });
  expect(preambleBlocks[0].blocks[0]).toMatchObject({
    type: "paragraph",
    content: "This is just a note."
  });

  await page.fill("#source", source);
  await page.click("#render");

  const frame = page.frameLocator("#preview-frame");
  await expect(frame.locator(".admonitionblock.note .icon .title")).toHaveText("Note");
  await expect(frame.locator(".admonitionblock.note .content p")).toHaveText("This is just a note.");
});

test("exports and renders block admonitions", async ({ page }) => {
  const source = `= Sample Document

[NOTE]
Remember the milk.

[TIP]
====
Ship it carefully.
====`;

  const json = await page.evaluate((input) => window.__prepareDocumentJson(input), source);
  const document = JSON.parse(json);
  const preambleBlocks = document.blocks[0].blocks;

  expect(preambleBlocks[0]).toMatchObject({
    type: "admonition",
    variant: "note",
    style: "NOTE"
  });
  expect(preambleBlocks[1]).toMatchObject({
    type: "admonition",
    variant: "tip",
    style: "TIP"
  });

  await page.fill("#source", source);
  await page.click("#render");

  const frame = page.frameLocator("#preview-frame");
  await expect(frame.locator(".admonitionblock.note .content p")).toHaveText("Remember the milk.");
  await expect(frame.locator(".admonitionblock.tip .icon .title")).toHaveText("Tip");
  await expect(frame.locator(".admonitionblock.tip .content p")).toHaveText("Ship it carefully.");
});

test("preview ignores header comments and preserves header attributes", async ({ page }) => {
  const source = `// leading comment
= Sample Document
// comment between title and attrs
:toc: left

Hello from the browser.`;

  const json = await page.evaluate((input) => window.__prepareDocumentJson(input), source);
  const document = JSON.parse(json);

  expect(document.title).toBe("Sample Document");
  expect(document.attributes).toEqual({ toc: "left" });
  expect(document.blocks[0].type).toBe("preamble");
  expect(document.blocks[0].blocks[0].content).toBe("Hello from the browser.");

  await page.fill("#source", source);
  await page.click("#render");

  const frame = page.frameLocator("#preview-frame");
  await expect(frame.locator("#header h1")).toHaveText("Sample Document");
  await expect(frame.locator("#content p").first()).toHaveText("Hello from the browser.");
  await expect(frame.locator("text=leading comment")).toHaveCount(0);
});
