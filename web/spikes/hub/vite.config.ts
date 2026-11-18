// THROWAWAY SPIKE (DEC-050): a separate Vite config so the hub spike is never
// part of the app build (the app's vite.config.ts only builds web/index.html).
//   Dev:     npx vite --config spikes/hub/vite.config.ts
//   Build:   npx vite build --config spikes/hub/vite.config.ts
//   Preview: npx vite preview --config spikes/hub/vite.config.ts
import { fileURLToPath } from "node:url";
import { defineConfig } from "vite";

const root = fileURLToPath(new URL(".", import.meta.url));

export default defineConfig({
  root,
  base: "./",
  build: { target: "es2022", outDir: "dist", emptyOutDir: true, sourcemap: false },
  server: { port: 5199 },
  preview: { port: 5199 },
});
