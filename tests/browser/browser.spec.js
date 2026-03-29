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
      num: "1",
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
