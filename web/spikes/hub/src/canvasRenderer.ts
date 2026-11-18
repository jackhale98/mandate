// THROWAWAY SPIKE (DEC-050): Canvas 2D renderer. Same approach as the
// strategic map (web/src/map/WorldMap.tsx): the static ground is baked once
// into an offscreen canvas; each frame does a full redraw of the visible
// ground rect plus the culled, y-sorted sprite list from one atlas.
//
// Dirty rects were considered and rejected: the camera follows the player, so
// almost every frame scrolls the whole view, and ambient NPCs are spread over
// the screen. A full redraw is one drawImage for the ground plus one per sprite.
import type { Art } from "./art";
import { viewTransform, type FrameInput, type HubRenderer, type RendererFactory } from "./renderer";

export const createRenderer: RendererFactory = async (canvas: HTMLCanvasElement, art: Art) => {
  const ctx = canvas.getContext("2d", { alpha: false });
  if (!ctx) throw new Error("Canvas 2D unavailable");
  const c: CanvasRenderingContext2D = ctx;
  let cssW = 1;
  let cssH = 1;
  let dpr = 1;
  const { atlas, ground, frames } = art;
  const tintCss = (t: number) => `rgb(${(t >> 16) & 255},${(t >> 8) & 255},${t & 255})`;

  const r: HubRenderer = {
    name: "canvas",
    info: "Canvas 2D",
    resize(w, h, d) {
      cssW = w;
      cssH = h;
      dpr = d;
      canvas.width = Math.round(w * d);
      canvas.height = Math.round(h * d);
    },
    render(f: FrameInput) {
      const { s, tx, ty } = viewTransform(f, cssW, cssH, dpr);
      c.setTransform(1, 0, 0, 1, 0, 0);
      c.imageSmoothingEnabled = false;
      c.fillStyle = "#1d2a1d";
      c.fillRect(0, 0, canvas.width, canvas.height);
      c.setTransform(s, 0, 0, s, tx, ty);
      // Ground: only the visible source rect.
      const vx0 = Math.max(0, Math.floor(-tx / s));
      const vy0 = Math.max(0, Math.floor(-ty / s));
      const vx1 = Math.min(ground.width, Math.ceil((canvas.width - tx) / s));
      const vy1 = Math.min(ground.height, Math.ceil((canvas.height - ty) / s));
      if (vx1 > vx0 && vy1 > vy0) c.drawImage(ground, vx0, vy0, vx1 - vx0, vy1 - vy0, vx0, vy0, vx1 - vx0, vy1 - vy0);
      for (let i = 0; i < f.decalCount; i++) {
        const d = f.decals[i] as FrameInput["decals"][number];
        const fr = frames[d.frame];
        if (fr) c.drawImage(atlas, fr.x, fr.y, fr.w, fr.h, Math.round(d.x - fr.ax), Math.round(d.y - fr.ay), fr.w, fr.h);
      }
      for (let i = 0; i < f.spriteCount; i++) {
        const d = f.sprites[i] as FrameInput["sprites"][number];
        const fr = frames[d.frame];
        if (fr) c.drawImage(atlas, fr.x, fr.y, fr.w, fr.h, Math.round(d.x - fr.ax), Math.round(d.y - fr.ay), fr.w, fr.h);
      }
      if (f.tint !== null) {
        c.setTransform(1, 0, 0, 1, 0, 0);
        c.globalCompositeOperation = "multiply";
        c.fillStyle = tintCss(f.tint);
        c.fillRect(0, 0, canvas.width, canvas.height);
        c.globalCompositeOperation = "lighter";
        c.setTransform(s, 0, 0, s, tx, ty);
        for (let i = 0; i < f.glowCount; i++) {
          const d = f.glows[i] as FrameInput["glows"][number];
          const fr = frames[d.frame];
          if (fr) c.drawImage(atlas, fr.x, fr.y, fr.w, fr.h, d.x - fr.ax, d.y - fr.ay, fr.w, fr.h);
        }
        c.globalCompositeOperation = "source-over";
      }
    },
  };
  return r;
};
