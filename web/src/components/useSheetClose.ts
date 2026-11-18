import { useCallback, useEffect, useRef, useState } from "react";

/** Matches the `sheet-down` animation in styles.css. */
const CLOSE_MS = 200;

function reducedMotion(): boolean {
  return typeof window !== "undefined" && window.matchMedia?.("(prefers-reduced-motion: reduce)").matches === true;
}

/**
 * Lets a sheet slide away before it unmounts: `requestClose` adds the closing
 * class, then calls `close` once the animation has run. With reduced motion it
 * closes at once.
 */
export function useSheetClose(close: () => void): [closing: boolean, requestClose: () => void] {
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
  return [closing, requestClose];
}
