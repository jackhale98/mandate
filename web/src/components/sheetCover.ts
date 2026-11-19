// Sheets over the street (DEC-080 "Battery"): on a phone an open sheet (the
// pocket, a place panel, a character, a decision, the story) is a bottom
// sheet that can cover most of the street, and then nothing should animate
// behind it. On wide screens sheets stand beside the street, which stays
// visible and live (zoom and pan must keep working with a place panel open).
// `useSheetsCover` says when one sheet covers more than SHEET_COVER of the
// street's area.
import { useEffect, useState, type RefObject } from "react";

/** The share of the street a sheet may cover before the street pauses. */
export const SHEET_COVER = 0.6;

export interface Box {
  left: number;
  top: number;
  right: number;
  bottom: number;
}

/** The share (0–1) of `under`'s area that `over` covers. */
export function coverShare(under: Box, over: Box): number {
  const area = (under.right - under.left) * (under.bottom - under.top);
  if (!(area > 0)) return 0;
  const w = Math.max(0, Math.min(under.right, over.right) - Math.max(under.left, over.left));
  const h = Math.max(0, Math.min(under.bottom, over.bottom) - Math.max(under.top, over.top));
  return (w * h) / area;
}

/**
 * Whether an open sheet covers most of `stage`, measured as sheets open, slide
 * in and resize, and on window resize. `open` names the sheets open now (a new
 * value re-measures); empty: none.
 */
export function useSheetsCover(stage: RefObject<HTMLElement | null>, open: string): boolean {
  const [covers, setCovers] = useState(false);
  useEffect(() => {
    if (!open) return;
    const sheets = () => [...document.querySelectorAll<HTMLElement>(".sheet:not(.sheet-closing)")];
    const measure = () => {
      const s = stage.current?.getBoundingClientRect();
      setCovers(!!s && sheets().some((p) => coverShare(s, p.getBoundingClientRect()) > SHEET_COVER));
    };
    const ro = new ResizeObserver(measure);
    const watched = new Set<HTMLElement>();
    const watch = () => {
      for (const p of sheets()) {
        if (watched.has(p)) continue;
        ro.observe(p);
        watched.add(p);
      }
      measure();
    };
    watch();
    // A sheet mounts with the same render and slides in: measure again once it is there and in place.
    const raf = requestAnimationFrame(watch);
    document.addEventListener("animationend", watch, true);
    window.addEventListener("resize", measure);
    return () => {
      cancelAnimationFrame(raf);
      ro.disconnect();
      document.removeEventListener("animationend", watch, true);
      window.removeEventListener("resize", measure);
    };
  }, [stage, open]);
  // None open covers nothing (the last measure is kept for the next opening, which measures again).
  return !!open && covers;
}
