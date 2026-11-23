// The public landing page (DEC-083): plain HTML, CSS and a little TypeScript,
// built into dist/ at the site base. The game is built into dist/play/ by
// ../vite.config.ts, after this build (this one empties dist/).
//   Build:   npx vite build -c landing/vite.config.ts   (part of `npm run build`)
//   Dev:     npx vite -c landing/vite.config.ts
//   Preview: npx vite preview -c landing/vite.config.ts  (serves landing, play/, spike-hub/)
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { defineConfig, type HtmlTagDescriptor, type Plugin } from "vite";
import { cspMeta, landingPolicy } from "../csp.ts";

const root = fileURLToPath(new URL(".", import.meta.url));
// The single deploy variable (DEC-064): /mandate/ on GitHub Pages, / on a custom domain.
const base = process.env.MANDATE_BASE ?? "/mandate/";
// Absolute URLs (canonical, Open Graph) need an origin. GitHub Pages is the
// default for the default base; a custom domain sets MANDATE_ORIGIN, and
// without it the page simply omits the absolute-only tags.
const origin = process.env.MANDATE_ORIGIN ?? (base === "/mandate/" ? "https://jackhale98.github.io" : "");
const siteUrl = origin ? `${origin}${base}` : "";

const months = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
const today = new Date();
const buildDate = `${today.getUTCDate()} ${months[today.getUTCMonth()]} ${today.getUTCFullYear()}`;

/** Head tags that depend on where the site is deployed. */
function siteMeta(): Plugin {
  return {
    name: "mandate-site-meta",
    transformIndexHtml(html) {
      const out = html.replaceAll("%BUILD_DATE%", buildDate);
      const image = `${siteUrl || base}og.png`;
      const tags: HtmlTagDescriptor[] = [
        { tag: "meta", attrs: { property: "og:image", content: image }, injectTo: "head" },
        { tag: "meta", attrs: { name: "twitter:image", content: image }, injectTo: "head" },
      ];
      if (siteUrl) {
        tags.push(
          { tag: "link", attrs: { rel: "canonical", href: siteUrl }, injectTo: "head" },
          { tag: "meta", attrs: { property: "og:url", content: siteUrl }, injectTo: "head" },
        );
      }
      return { html: out, tags };
    },
  };
}

export default defineConfig({
  root,
  base,
  appType: "mpa",
  publicDir: "public",
  plugins: [siteMeta(), cspMeta(landingPolicy, { hashInlineScripts: true })],
  build: {
    target: "es2020",
    outDir: "../dist",
    emptyOutDir: true,
    // Two pages: the front page and the plain-text privacy notice.
    rollupOptions: { input: { index: resolve(root, "index.html"), privacy: resolve(root, "privacy.html") } },
    sourcemap: false,
    // Small icons and marks are inlined; the screenshots stay files.
    assetsInlineLimit: 2048,
  },
  server: { port: 5198 },
});
