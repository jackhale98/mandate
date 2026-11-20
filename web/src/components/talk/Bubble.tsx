// A pixel speech bubble (dialogue mode, DEC-093): a reply page, the player's
// question, or "…" while the person thinks. In the street it is placed over
// a speaker's sprite by `placeBubble` (following the camera, kept on screen:
// clamped at the sides, flipped below the speaker near the top); beside a
// portrait it sits in the layout with its tail to the side. The text is in
// the DOM in full (the typewriter only hides what is not revealed yet); the
// conversation's live region reads it to screen readers, so the bubble
// itself is hidden from them.
import type { Ref } from "react";
import type { Page } from "./dialogueModel";

/** The tail: a stepped pixel point, 5×3 cells of 3 px (ink edge, paper inside); its top row covers the bubble's edge. */
function Tail() {
  return (
    <svg className="dlg-tail" width="20" height="12" viewBox="0 0 15 9" shapeRendering="crispEdges" aria-hidden="true">
      <path className="dlg-tail-ink" d="M0 0h3v3H0zM12 0h3v3h-3zM3 3h3v3H3zM9 3h3v3H9zM6 6h3v3H6z" />
      <path className="dlg-tail-paper" d="M3 0h9v3H3zM6 3h3v3H6z" />
    </svg>
  );
}

export interface BubbleProps {
  who: "person" | "player";
  /** What is said: a reply page (stage directions in italics), or plain text. */
  page: Page | string | null;
  /** Characters revealed so far (the typewriter); Infinity: all. */
  typed?: number;
  /** More pages follow (a ▼ shows). */
  more?: boolean;
  /** Thinking: "…" (waiting for the reply). */
  thinking?: boolean;
  /** Beside a portrait (tail to the side) rather than over a sprite. */
  side?: boolean;
  onAdvance?: () => void;
  testId?: string;
  bubbleRef?: Ref<HTMLDivElement>;
}

export function Bubble({ who, page, typed = Infinity, more = false, thinking = false, side = false, onAdvance, testId, bubbleRef }: BubbleProps) {
  const segs = page === null ? [] : typeof page === "string" ? [{ text: page, stage: false }] : page;
  let left = typed;
  return (
    <div ref={bubbleRef} className={`dlg-bubble dlg-bubble-${who}${side ? " dlg-bubble-side" : ""}`} data-testid={testId} aria-hidden="true"
      onClick={(e) => {
        e.stopPropagation();
        onAdvance?.();
      }}
    >
      <div className="dlg-bubble-body">
        {thinking ? (
          <p className="dlg-thinking">
            <span>.</span>
            <span>.</span>
            <span>.</span>
          </p>
        ) : (
          segs.map((s, i) => {
            // The typewriter: what is revealed, then the rest kept in place (so the bubble does not grow as it types).
            const shown = Math.max(0, Math.min(s.text.length, left));
            left -= s.text.length + 1;
            return (
              <p key={i} className={s.stage ? "dlg-stage" : "dlg-line"} data-stage={s.stage ? "1" : undefined}>
                {s.text.slice(0, shown)}
                {shown < s.text.length && <span className="dlg-untyped">{s.text.slice(shown)}</span>}
              </p>
            );
          })
        )}
        {more && <span className="dlg-more-mark">▼</span>}
      </div>
      <Tail />
    </div>
  );
}

/** The screen edges a street bubble keeps inside (viewport px). */
export interface BubbleBounds {
  left: number;
  top: number;
  right: number;
  bottom: number;
}

/**
 * Place a street bubble over a speaker: centred above the head at `at`
 * (viewport px), clamped to `bounds` at the sides; where there is no room
 * above, below the speaker's feet (`sprite`: the sprite's height on screen)
 * with the tail flipped; failing both, at the top. The tail points at the
 * speaker wherever the bubble is clamped to.
 */
export function placeBubble(el: HTMLElement, at: { x: number; y: number }, sprite: number, bounds: BubbleBounds) {
  const w = el.offsetWidth;
  const h = el.offsetHeight;
  if (w === 0 || h === 0) return;
  const tail = 9;
  const gap = 2;
  const left = Math.round(Math.min(Math.max(at.x - w / 2, bounds.left), Math.max(bounds.left, bounds.right - w)));
  let top = at.y - gap - tail - h;
  let flip = false;
  if (top < bounds.top) {
    const below = at.y + sprite + gap + tail;
    if (below + h <= bounds.bottom) {
      top = below;
      flip = true;
    } else top = bounds.top;
  }
  el.style.transform = `translate(${left}px, ${Math.round(top)}px)`;
  el.dataset.flip = flip ? "1" : "0";
  el.style.setProperty("--tail-x", `${Math.round(Math.min(Math.max(at.x - left, 14), w - 14))}px`);
}
