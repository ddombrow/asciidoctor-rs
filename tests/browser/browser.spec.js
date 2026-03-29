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
