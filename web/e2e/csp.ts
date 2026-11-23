// A Playwright `test` that fails when the page reports a Content Security
// Policy violation (the policy is a <meta> tag injected into the built pages;
// see csp.ts at the web root). Import `test` from here instead of
// "@playwright/test" in any spec that should guard against a policy that
// blocks something the game needs.
import { expect, test as base } from "@playwright/test";

export { expect };

export const test = base.extend<{ cspGuard: void }>({
  cspGuard: [
    async ({ page }, use) => {
      const violations: string[] = [];
      await page.addInitScript(() => {
        document.addEventListener("securitypolicyviolation", (e) => {
          console.error(`CSP-VIOLATION ${e.violatedDirective} blocked ${e.blockedURI || "inline"} at ${e.sourceFile ?? ""}:${e.lineNumber}`);
        });
      });
      page.on("console", (m) => {
        const t = m.text();
        if (/CSP-VIOLATION|Content Security Policy|Refused to (load|execute|apply|connect|create|frame|evaluate)/i.test(t)) violations.push(t);
      });
      page.on("pageerror", (e) => {
        if (/Content Security Policy|unsafe-eval|wasm-eval/i.test(e.message)) violations.push(e.message);
      });
      await use();
      expect(violations, "Content Security Policy violations").toEqual([]);
    },
    { auto: true },
  ],
});
