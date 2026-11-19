// The colour scheme the page is drawn in (styles.css: dark by default, light
// when the OS asks for it, either forced by `data-theme` on the root), for
// canvases that pick their own colours.
import { useEffect, useState } from "react";
import type { Scheme } from "./layerPalette";

export function currentScheme(): Scheme {
  const forced = document.documentElement.dataset.theme;
  if (forced === "light" || forced === "dark") return forced;
  return typeof window.matchMedia === "function" && window.matchMedia("(prefers-color-scheme: light)").matches ? "light" : "dark";
}

export function useScheme(): Scheme {
  const [scheme, setScheme] = useState<Scheme>(currentScheme);
  useEffect(() => {
    const update = () => setScheme(currentScheme());
    const mq = typeof window.matchMedia === "function" ? window.matchMedia("(prefers-color-scheme: light)") : null;
    mq?.addEventListener("change", update);
    const mo = new MutationObserver(update);
    mo.observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme"] });
    return () => {
      mq?.removeEventListener("change", update);
      mo.disconnect();
    };
  }, []);
  return scheme;
}
