// Closing sheets without the close button (DEC-048 phones, DEC-051 keyboard):
// - on phones a bottom sheet follows a downward drag from its head (or from its
//   body when that is scrolled to the top) and closes past a threshold or on a
//   quick flick, otherwise it springs back;
// - Escape closes the sheet on top.
// Presentation only: closing a sheet is the same as pressing its close button.

/** Bottom sheets: below this width sheets dock as side panels instead. */
const PHONE = "(max-width: 899px)";
/** Movement (px) before a gesture counts as a drag rather than a tap. */
const SLOP = 8;
/** A flick: release speed in px per ms. */
const FLICK = 0.5;
/** Spring-back duration (ms). */
const SPRING_MS = 220;

interface OpenSheet {
  el: HTMLElement;
  close: () => void;
  order: number;
}

const openSheets: OpenSheet[] = [];
let opened = 0;

function matches(query: string): boolean {
  return typeof window !== "undefined" && window.matchMedia?.(query).matches === true;
}

/** The sheet drawn on top: highest z-index, then the most recently opened. */
export function topSheet(): OpenSheet | undefined {
  let best: OpenSheet | undefined;
  let bestZ = -Infinity;
  for (const s of openSheets) {
    const z = Number.parseInt(getComputedStyle(s.el).zIndex, 10);
    const zi = Number.isFinite(z) ? z : 0;
    if (!best || zi > bestZ || (zi === bestZ && s.order > best.order)) {
      best = s;
      bestZ = zi;
    }
  }
  return best;
}

function onKeyDown(e: KeyboardEvent) {
  if (e.key !== "Escape" || e.defaultPrevented) return;
  // A modal dialog (election night) owns the keyboard while it is open.
  if (document.querySelector('[aria-modal="true"]')) return;
  const top = topSheet();
  if (!top) return;
  e.preventDefault();
  top.close();
}

/**
 * Registers an open sheet for Escape and swipe-to-close. `close` is the sheet's
 * own (animated) close. Returns the cleanup to run when the sheet unmounts.
 */
export function registerSheet(el: HTMLElement, close: () => void): () => void {
  const entry: OpenSheet = { el, close, order: ++opened };
  openSheets.push(entry);
  if (openSheets.length === 1) document.addEventListener("keydown", onKeyDown);
  const detach = attachSwipe(el, close);
  return () => {
    detach();
    const i = openSheets.indexOf(entry);
    if (i >= 0) openSheets.splice(i, 1);
    if (openSheets.length === 0) document.removeEventListener("keydown", onKeyDown);
  };
}

interface Drag {
  id: number;
  x: number;
  y: number;
  fromHead: boolean;
  engaged: boolean;
  dead: boolean;
  dy: number;
  lastY: number;
  lastT: number;
  /** Downward speed, px per ms (smoothed). */
  v: number;
}

/** Swipe down to close a bottom sheet. Pointer events track the finger; a
 * non-passive touchmove guard stops the page from scrolling (or refreshing)
 * once the drag is the sheet's. */
export function attachSwipe(el: HTMLElement, close: () => void): () => void {
  let g: Drag | null = null;
  let springTimer: number | undefined;
  const scrim = (): HTMLElement | null => {
    const s = el.previousElementSibling;
    return s instanceof HTMLElement && s.classList.contains("sheet-scrim") ? s : null;
  };
  const head = () => el.querySelector(":scope > .sheet-head");

  const follow = (dy: number) => {
    el.style.transform = dy > 0 ? `translateY(${dy}px)` : "";
    const s = scrim();
    if (s) s.style.opacity = dy > 0 ? String(Math.max(0, 1 - dy / Math.max(1, el.offsetHeight))) : "";
  };
  const springBack = () => {
    const s = scrim();
    if (!matches("(prefers-reduced-motion: reduce)")) {
      const t = `transform ${SPRING_MS}ms cubic-bezier(0.2, 0.8, 0.2, 1)`;
      el.style.transition = t;
      if (s) s.style.transition = `opacity ${SPRING_MS}ms ease-out`;
    }
    follow(0);
    window.clearTimeout(springTimer);
    springTimer = window.setTimeout(() => {
      el.style.transition = "";
      if (s) s.style.transition = "";
    }, SPRING_MS + 20);
  };
  /** A drag that ends over a button must not also press it. */
  const swallowClick = () => {
    const stop = (e: Event) => {
      e.stopPropagation();
      e.preventDefault();
    };
    window.addEventListener("click", stop, { capture: true, once: true });
    window.setTimeout(() => window.removeEventListener("click", stop, { capture: true }), 0);
  };

  const onDown = (e: PointerEvent) => {
    g = null;
    if (!e.isPrimary || (e.pointerType === "mouse" && e.button !== 0)) return;
    if (!matches(PHONE) || el.classList.contains("sheet-closing")) return;
    const target = e.target instanceof Element ? e.target : null;
    if (!target || target.closest("input, select, textarea, [contenteditable], .table-scroll")) return;
    const fromHead = head()?.contains(target) ?? false;
    if (!fromHead && el.scrollTop > 0) return;
    g = { id: e.pointerId, x: e.clientX, y: e.clientY, fromHead, engaged: false, dead: false, dy: 0, lastY: e.clientY, lastT: e.timeStamp, v: 0 };
  };

  const onMove = (e: PointerEvent) => {
    if (!g || g.dead || e.pointerId !== g.id) return;
    const dx = e.clientX - g.x;
    const dy = e.clientY - g.y;
    if (!g.engaged) {
      if ((Math.abs(dx) > SLOP && Math.abs(dx) > Math.abs(dy)) || dy < -SLOP) {
        g.dead = true;
        return;
      }
      if (dy < SLOP) return;
      if (!g.fromHead && el.scrollTop > 0) {
        g.dead = true;
        return;
      }
      g.engaged = true;
      window.clearTimeout(springTimer);
      el.style.transition = "none";
      const s = scrim();
      if (s) s.style.transition = "none";
      try {
        el.setPointerCapture(e.pointerId);
      } catch {
        // The pointer may already be gone; the drag still follows its events.
      }
    }
    const dt = Math.max(1, e.timeStamp - g.lastT);
    g.v = 0.6 * ((e.clientY - g.lastY) / dt) + 0.4 * g.v;
    g.lastY = e.clientY;
    g.lastT = e.timeStamp;
    g.dy = Math.max(0, dy - SLOP);
    follow(g.dy);
  };

  const onEnd = (e: PointerEvent) => {
    if (!g || e.pointerId !== g.id) return;
    const d = g;
    g = null;
    if (!d.engaged) return;
    swallowClick();
    // A pause before letting go is not a flick.
    const v = e.timeStamp - d.lastT > 100 ? 0 : d.v;
    const far = d.dy > Math.min(160, el.offsetHeight * 0.33);
    const flick = v > FLICK && d.dy > 24;
    if (e.type === "pointerup" && (far || flick)) {
      // The closing animation runs on from where the finger left the sheet.
      el.style.transition = "";
      const s = scrim();
      if (s) s.style.transition = "";
      close();
    } else {
      springBack();
    }
  };

  const onTouchMove = (e: TouchEvent) => {
    if (!g || g.dead || !e.cancelable) return;
    if (g.engaged) {
      e.preventDefault();
      return;
    }
    const t = e.touches[0];
    if (!t) return;
    const dx = t.clientX - g.x;
    const dy = t.clientY - g.y;
    if (dy > 0 && dy >= Math.abs(dx) && (g.fromHead || el.scrollTop <= 0)) e.preventDefault();
  };

  el.addEventListener("pointerdown", onDown);
  el.addEventListener("pointermove", onMove);
  el.addEventListener("pointerup", onEnd);
  el.addEventListener("pointercancel", onEnd);
  el.addEventListener("touchmove", onTouchMove, { passive: false });
  return () => {
    window.clearTimeout(springTimer);
    el.removeEventListener("pointerdown", onDown);
    el.removeEventListener("pointermove", onMove);
    el.removeEventListener("pointerup", onEnd);
    el.removeEventListener("pointercancel", onEnd);
    el.removeEventListener("touchmove", onTouchMove);
  };
}
