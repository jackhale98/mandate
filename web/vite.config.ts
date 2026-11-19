import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";
import { VitePWA } from "vite-plugin-pwa";

// One variable controls the deploy base path (DEC-064):
//   GitHub Pages project site: /mandate/   (default)
//   Custom domain / Cloudflare: /
// The landing page is served at the base and the game at <base>play/ (DEC-083).
// The landing page is a separate build (landing/vite.config.ts) into dist/.
const site = process.env.MANDATE_BASE ?? "/mandate/";
const base = `${site}play/`;

export default defineConfig({
  base,
  define: { __APP_VERSION__: JSON.stringify(process.env.npm_package_version ?? "dev") },
  plugins: [
    react(),
    VitePWA({
      registerType: "autoUpdate",
      injectRegister: false,
      manifest: {
        // Before DEC-083 the app's id was its start_url, the site base. Keeping
        // it lets browsers that update installed apps move an existing install
        // to play/ instead of treating this as a different app.
        id: site,
        name: "Mandate",
        short_name: "Mandate",
        description: "A political career roguelike.",
        start_url: base,
        scope: base,
        display: "standalone",
        orientation: "any",
        background_color: "#11131a",
        theme_color: "#11131a",
        icons: [
          { src: "icons/icon-192.png", sizes: "192x192", type: "image/png" },
          { src: "icons/icon-512.png", sizes: "512x512", type: "image/png" },
          { src: "icons/icon-maskable-512.png", sizes: "512x512", type: "image/png", purpose: "maskable" },
        ],
      },
      workbox: {
        globPatterns: ["**/*.{js,css,html,wasm,png,svg,woff2}"],
        maximumFileSizeToCacheInBytes: 16 * 1024 * 1024, // the engine (WASM) is ~8.5 MB raw, ~1.4 MB gzipped; it must be precached for offline play
        navigateFallback: `${base}index.html`,
        // Prototype pages published beside the app (DEC-050) are never served the
        // app shell. They are outside this worker's scope since DEC-083; kept as a guard.
        navigateFallbackDenylist: [/\/spike-/],
      },
    }),
  ],
  worker: { format: "es" },
  build: { target: "es2022", sourcemap: true, outDir: "dist/play" },
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./src/test-setup.ts"],
    include: ["src/**/*.test.{ts,tsx}", "landing/src/**/*.test.ts"],
  },
});
