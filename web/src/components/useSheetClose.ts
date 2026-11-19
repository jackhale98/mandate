import { useCallback, useEffect, useRef, useState, type RefCallback, type RefObject } from "react";
import { registerSheet } from "./sheetGestures";

/** Matches the `sheet-down` animation in styles.css. */
const CLOSE_MS = 200;

function reducedMotion(): boolean {
  return typeof window !== "undefined" && window.matchMedia?.("(prefers-reduced-motion: reduce)").matches === true;
}

/**
 * Lets a sheet slide away before it unmounts: `requestClose` adds the closing
 * class, then calls `close` once the animation has run. With reduced motion it
 * closes at once.
 *
 * Put the returned `sheetRef` on the sheet's element: while it is mounted,
 * Escape closes the top sheet and, on phones, a downward swipe closes a bottom
 * sheet (`sheetGestures.ts`). A sheet that needs its element too passes its own
 * ref object as `elementRef`, and it is kept up to date.
 */
export function useSheetClose(
  close: () => void,
  elementRef?: RefObject<HTMLElement | null>,
): [closing: boolean, requestClose: () => void, sheetRef: RefCallback<HTMLElement>] {
  const [closing, setClosing] = useState(false);
  const timer = useRef<number | undefined>(undefined);
  useEffect(() => () => window.clearTimeout(timer.current), []);
  const requestClose = useCallback(() => {
    if (reducedMotion()) {
      close();
      return;
    }
    setClosing(true);
    window.clearTimeout(timer.current);
    timer.current = window.setTimeout(() => {
      setClosing(false);
      close();
    }, CLOSE_MS);
  }, [close]);
  // The gestures read the latest close through a ref, so the element is registered once.
  const latest = useRef(requestClose);
  useEffect(() => {
    latest.current = requestClose;
  });
  const sheetRef = useCallback(
    (el: HTMLElement | null) => {
      if (elementRef) elementRef.current = el;
      if (!el) return;
      const unregister = registerSheet(el, () => latest.current());
      return () => {
        unregister();
        if (elementRef) elementRef.current = null;
      };
    },
    [elementRef],
  );
  return [closing, requestClose, sheetRef];
}
