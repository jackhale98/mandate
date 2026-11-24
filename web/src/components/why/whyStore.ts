// The open "Why?" panel: which event was asked about and the engine's answer.
// A cache of one view (the engine stays authoritative); a small store of its
// own so any surface can open the panel with one call.
import { create } from "zustand";
import { engine, expectKind } from "../../engine/client";
import type { FactorView } from "../../engine/protocol/FactorView";
import type { WhyView } from "../../engine/protocol/WhyView";

/** The engine's factor decomposition of a race, shown beside its chain as the result view gave it. */
export interface WhyFactors {
  factors: FactorView[];
  /** −1: from the loser's side (what cost them), as the result card shows it. */
  sign: 1 | -1;
}

export interface WhyOpen {
  event: number;
  /** The engine's answer; null while it is asked for, or when it has none. */
  view: WhyView | null;
  busy: boolean;
  /** The engine has nothing on the event (the record no longer keeps it). */
  gone: boolean;
  factors: WhyFactors | null;
}

interface WhyState {
  open: WhyOpen | null;
  /** Ask why an event happened and open the panel on the answer. */
  ask(event: number, opts?: { opener?: HTMLElement | null; factors?: WhyFactors | null }): Promise<void>;
  /** Ask again, as far back as the engine walks. */
  further(): Promise<void>;
  close(): void;
}

/** What had the keyboard when the panel opened: it gets it back on close. */
let opener: HTMLElement | null = null;

async function query(event: number, depth: number | null): Promise<WhyView | null> {
  try {
    return expectKind(await engine().request({ kind: "query", payload: { type: "why", event, depth } }), "why");
  } catch {
    // No such event, or one the player never sees: the panel says so.
    return null;
  }
}

export const useWhy = create<WhyState>((set, get) => ({
  open: null,
  async ask(event, opts) {
    opener = opts?.opener ?? null;
    set({ open: { event, view: null, busy: true, gone: false, factors: opts?.factors ?? null } });
    const view = await query(event, null);
    // Closed, or another event asked about, meanwhile.
    if (get().open?.event !== event) return;
    set({ open: { event, view, busy: false, gone: view === null, factors: opts?.factors ?? null } });
  },
  async further() {
    const w = get().open;
    if (!w?.view || w.busy) return;
    set({ open: { ...w, busy: true } });
    const view = await query(w.event, w.view.max_depth);
    if (get().open?.event !== w.event) return;
    // If the longer walk fails, what was shown stays.
    set({ open: { ...w, view: view ?? w.view, busy: false } });
  },
  close() {
    set({ open: null });
    const back = opener;
    opener = null;
    if (back?.isConnected) back.focus();
  },
}));
