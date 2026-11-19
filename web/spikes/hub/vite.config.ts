// THROWAWAY SPIKE (DEC-050): a separate Vite config so the hub spike is never
// part of the app build (the app's vite.config.ts only builds web/index.html).
// Two pages:
//   index.html    the walkable town prototype on the real engine (M6 groundwork)
//   compare.html  the original Canvas 2D vs PixiJS comparison (DEC-080 evidence)
//   Dev:     npx vite --config spikes/hub/vite.config.ts
//   Build:   npx vite build --config spikes/hub/vite.config.ts
//   Preview: npx vite preview --config spikes/hub/vite.config.ts
// The pixel-art tool's pack (tools/pixelkit, DEC-084) is built here, at build
// time, and served to the page as `virtual:pixelkit-pack` (?art=pixelkit).
import { fileURLToPath } from "node:url";
import { defineConfig, type Plugin } from "vite";
import { buildPack } from "../../../tools/pixelkit/art/mandate/pack";
import { packToJson } from "../../../tools/pixelkit/src/core/serial";

const root = fileURLToPath(new URL(".", import.meta.url));
const repo = fileURLToPath(new URL("../../..", import.meta.url));
const serial = fileURLToPath(new URL("../../../tools/pixelkit/src/core/serial.ts", import.meta.url));

/** `virtual:pixelkit-pack`: the built art pack (palette, tiles, props, lighting) as data. */
function pixelkitPack(): Plugin {
  const id = "virtual:pixelkit-pack";
  return {
    name: "pixelkit-pack",
    resolveId: (s) => (s === id ? `\0${id}` : null),
    load(s) {
      if (s !== `\0${id}`) return null;
      const t0 = performance.now();
      const json = packToJson(buildPack());
      const ms = Math.round(performance.now() - t0);
      return `import { packFromJson } from ${JSON.stringify(serial)};\nexport const buildMs = ${ms};\nexport const bytes = ${json.length};\nexport default packFromJson(${JSON.stringify(json)});\n`;
    },
  };
}

export default defineConfig({
  root,
  base: "./",
  plugins: [pixelkitPack()],
  // The town page runs the engine in its Web Worker, as the app does.
  worker: { format: "es" },
  build: {
    target: "es2022",
    outDir: "dist",
    emptyOutDir: true,
    sourcemap: false,
    rollupOptions: { input: { index: fileURLToPath(new URL("index.html", import.meta.url)), compare: fileURLToPath(new URL("compare.html", import.meta.url)) } },
  },
  server: { port: 5199, fs: { allow: [repo] } },
  preview: { port: 5199 },
});
