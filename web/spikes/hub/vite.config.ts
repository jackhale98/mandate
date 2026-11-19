// THROWAWAY SPIKE (DEC-050): a separate Vite config so the hub spike is never
// part of the app build (the app's vite.config.ts only builds web/index.html).
// Two pages:
//   index.html    the walkable town prototype on the real engine (M6 groundwork)
//   compare.html  the original Canvas 2D vs PixiJS comparison (DEC-080 evidence)
//   Dev:     npx vite --config spikes/hub/vite.config.ts
//   Build:   npx vite build --config spikes/hub/vite.config.ts
//   Preview: npx vite preview --config spikes/hub/vite.config.ts
import { fileURLToPath } from "node:url";
import { defineConfig } from "vite";

const root = fileURLToPath(new URL(".", import.meta.url));

export default defineConfig({
  root,
  base: "./",
  // The town page runs the engine in its Web Worker, as the app does.
  worker: { format: "es" },
  build: {
    target: "es2022",
    outDir: "dist",
    emptyOutDir: true,
    sourcemap: false,
    rollupOptions: { input: { index: fileURLToPath(new URL("index.html", import.meta.url)), compare: fileURLToPath(new URL("compare.html", import.meta.url)) } },
  },
  server: { port: 5199 },
  preview: { port: 5199 },
});
