import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";
import { VitePWA } from "vite-plugin-pwa";

// One variable controls the deploy base path (DEC-064):
//   GitHub Pages project site: /mandate/   (default)
//   Custom domain / Cloudflare: /
const base = process.env.MANDATE_BASE ?? "/mandate/";

export default defineConfig({
  base,
  define: { __APP_VERSION__: JSON.stringify(process.env.npm_package_version ?? "dev") },
  plugins: [
    react(),
    VitePWA({
      registerType: "autoUpdate",
      injectRegister: false,
      manifest: {
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
        maximumFileSizeToCacheInBytes: 8 * 1024 * 1024,
        navigateFallback: `${base}index.html`,
        // Prototype pages published beside the app (DEC-050) are never served the app shell.
        navigateFallbackDenylist: [/\/spike-/],
      },
    }),
  ],
  worker: { format: "es" },
  build: { target: "es2022", sourcemap: true },
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./src/test-setup.ts"],
    include: ["src/**/*.test.{ts,tsx}"],
  },
});
