import type { Page } from "@playwright/test";

/** A deliberately wide face (a CI runner's fallback fonts are wider than a dev machine's), so layout checks do not depend on
 *  which fonts the machine has. Applies to every element of every page the test opens. */
export const WIDE_FONT_CSS = `* { font-family: "DejaVu Sans", Verdana, sans-serif !important; letter-spacing: 0.04em; } html { font-size: 115%; }`;

export async function useWideFont(page: Page): Promise<void> {
  await page.addInitScript((css) => {
    // A constructed stylesheet: an injected <style> element would trip the page's Content Security Policy.
    const sheet = new CSSStyleSheet();
    sheet.replaceSync(css);
    document.adoptedStyleSheets = [...document.adoptedStyleSheets, sheet];
  }, WIDE_FONT_CSS);
}
