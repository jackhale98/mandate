// THROWAWAY SPIKE (DEC-050): the contract both renderers implement. The scene
// code culls and y-sorts once per frame; a renderer only draws the list, so the
// comparison is about drawing cost, not about game logic.
import type { Art } from "./art";

export interface Drawable {
  id: number; // stable per entity (lets a retained-mode renderer keep objects)
  frame: number;
  x: number; // anchor, world px
  y: number;
}

export interface FrameInput {
  camX: number; // world px at the view centre
  camY: number;
  zoom: number; // CSS px per art px
  decals: Drawable[]; // flat, drawn over the ground
  decalCount: number;
  sprites: Drawable[]; // y-sorted, already culled
  spriteCount: number;
  /** Day-part lighting: a multiply tint plus additive glows (null = daylight). */
  tint: number | null;
  glows: Drawable[];
  glowCount: number;
}

export interface HubRenderer {
  readonly name: string;
  readonly info: string;
  resize(cssW: number, cssH: number, dpr: number): void;
  render(f: FrameInput): void;
}

export type RendererFactory = (canvas: HTMLCanvasElement, art: Art) => Promise<HubRenderer>;

/** World->device transform shared by both renderers (pixel-snapped). */
export function viewTransform(f: FrameInput, cssW: number, cssH: number, dpr: number) {
  const s = f.zoom * dpr;
  const tx = Math.round(cssW * dpr * 0.5 - f.camX * s);
  const ty = Math.round(cssH * dpr * 0.5 - f.camY * s);
  return { s, tx, ty };
}
