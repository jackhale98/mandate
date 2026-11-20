// The dialogue hook (DEC-093, DEC-088): what a conversation's presentation
// (speech bubbles, a dialogue box, camera framing) attaches to when the
// player talks to someone inside a place. The interior publishes its stage
// here while it is shown; the talk layer reads it and never reaches into the
// room's code:
//
//   const stage = interiorStage();          // null in the street
//   stage?.frame("c12");                    // they and the player turn to each other
//   const box = stage?.anchorOf("c12");     // CSS px in stage.element: where to hang a bubble
//   const me = stage?.anchorOf("player");
//   onInteriorStage((s) => ...);            // shown, or gone (null)
//
// Person keys are the street's: `c<character id>` or `a<ambient slot>`, the
// same `TalkTargetDto` the store's `openTalk` takes. The interior frames the
// person the store's talk is for by itself; `frame(null)` ends it.

export interface InteriorStage {
  /** The element the room is drawn in (anchors are relative to it). */
  readonly element: HTMLElement;
  /** The place inside (an engine place id). */
  readonly place: number;
  /** Who is shown, by key (people the engine says are here; scenery staff are not listed). */
  people(): string[];
  /** Where someone (or "player") is drawn: CSS px in `element`, the head's top to the feet. */
  anchorOf(key: string): { x: number; y: number; w: number; h: number } | null;
  /** Frame a conversation on someone (they and the player face each other), or end it (null). */
  frame(key: string | null): void;
}

let current: InteriorStage | null = null;
const listeners = new Set<(s: InteriorStage | null) => void>();

/** The interior on screen, or null. */
export const interiorStage = (): InteriorStage | null => current;

/** Hear when an interior is shown or gone. Returns an unsubscribe. */
export function onInteriorStage(f: (s: InteriorStage | null) => void): () => void {
  listeners.add(f);
  return () => listeners.delete(f);
}

/** The interior publishes itself (and null when it goes). */
export function setInteriorStage(s: InteriorStage | null) {
  current = s;
  for (const f of listeners) f(s);
}
