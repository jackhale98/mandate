// A person's bust from the pixel-art pack (DEC-084): the street sprite's
// front-facing idle frame, composed from the same character parts and the
// same look (`lookFromSeed(variant)`, ./look), cropped to the head and
// shoulders. Loaded on demand (the pack is built once per session in the
// town worker, ../../hub/townWorkerClient), so the conversation's first
// paint never waits on it. Presentation only.
import { artPack } from "../../hub/townWorkerClient";
import { composeCharacter, lookFromSeed, PLAYER_LOOK } from "../../../../tools/pixelkit/art/mandate/pack";
import { rgbaTable, toRgba } from "../../../../tools/pixelkit/src/runtime";

/** A bust, as RGBA pixels (art px). */
export interface Bust {
  w: number;
  h: number;
  rgba: Uint8ClampedArray;
}

/** Rows of the 16×24 frame kept: the head and the shoulders. */
export const BUST_ROWS = 16;

const busts = new Map<number, Promise<Bust>>();
let table: Uint8Array | null = null;

/** The bust for a look (`player`: the player's own). Kept for the session. */
export function bust(variant: number, player = false): Promise<Bust> {
  const key = player ? -1 : variant >>> 0;
  let b = busts.get(key);
  if (!b) {
    b = artPack().then((pack) => {
      table ??= rgbaTable(pack.palette.colors);
      const sheet = composeCharacter(player ? PLAYER_LOOK : lookFromSeed(key), pack.palette, pack.lightness);
      // Facing the viewer, standing still.
      const frame = sheet.frames[0]?.[0];
      if (!frame) throw new Error("no character frame");
      // From the top of the hair down: the same crop for everyone, so a tall person's bust is not shorter.
      let top = 0;
      while (top < frame.h && !frame.px.subarray(top * frame.w, (top + 1) * frame.w).some((v) => v !== 0)) top++;
      top = Math.max(0, top - 1);
      const h = BUST_ROWS;
      const px = new Uint8Array(frame.w * h);
      for (let y = 0; y < h && top + y < frame.h; y++) px.set(frame.px.subarray((top + y) * frame.w, (top + y + 1) * frame.w), y * frame.w);
      return { w: frame.w, h, rgba: toRgba({ w: frame.w, h, px }, table) };
    });
    b.catch(() => busts.delete(key));
    busts.set(key, b);
  }
  return b;
}
