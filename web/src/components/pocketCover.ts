// The pocket sheet over the street (DEC-080 "Battery"): on a phone the open
// pocket can cover most of the street, and then nothing should animate behind
// it. `usePocketCovers` says when it covers more than POCKET_COVER of the
// street's area (docked beside the town on desktop it covers none).
import { useEffect, useState, type RefObject } from "react";

/** The share of the street the pocket may cover before the street pauses. */
export const POCKET_COVER = 0.6;

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

/** Whether the open pocket covers most of `stage` (measured as it opens, slides in, resizes, and on window resize). */
export function usePocketCovers(stage: RefObject<HTMLElement | null>, open: boolean): boolean {
  const [covers, setCovers] = useState(false);
  useEffect(() => {
    if (!open) return;
    const sheet = () => document.querySelector<HTMLElement>("[data-testid='pocket']");
    const measure = () => {
      const s = stage.current;
      const p = sheet();
      setCovers(!!s && !!p && !p.classList.contains("sheet-closing") && coverShare(s.getBoundingClientRect(), p.getBoundingClientRect()) > POCKET_COVER);
    };
    const ro = new ResizeObserver(measure);
    let watched: HTMLElement | null = null;
    const watch = () => {
      const p = sheet();
      if (p && p !== watched) {
        if (watched) ro.unobserve(watched);
        ro.observe(p);
        watched = p;
      }
      measure();
    };
    watch();
    // The sheet mounts with the same render and slides in: measure again once it is there and in place.
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
  // Closed, it covers nothing (the last measure is kept for the next opening, which measures again).
  return open && covers;
}
