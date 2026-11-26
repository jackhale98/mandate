// The open proposal sheet (DEC-092, task F6): which proposal, the choices and
// sliders the player has set, and the engine's preview of that package. A
// cache of one view (the engine stays authoritative: every cost, support and
// estimate on the sheet is the preview's); a small store of its own so the
// desk, a place's panel, the notebook and a bill can each open it with one call.
import { create } from "zustand";
import { engine, expectKind } from "../../engine/client";
import type { ChoiceValue } from "../../engine/protocol/ChoiceValue";
import type { ParamValue } from "../../engine/protocol/ParamValue";
import type { PolicyPreviewView } from "../../engine/protocol/PolicyPreviewView";

/** How long the sliders must rest before the engine is asked again (as the catalogue's). */
export const PROPOSAL_DEBOUNCE_MS = 150;

/** What the sheet is opened on: a proposal, or a bill drafted from one (to amend it). */
export interface ProposalTarget {
  proposal: number;
  /** The proposal's family id (`ProposalView.family`, `BillView.template`): the preview's `template`. */
  family: string;
  /** A bill drafted from it: the sheet amends it instead of drafting. */
  bill?: number | null;
  choices?: ChoiceValue[];
  params?: ParamValue[];
}

export interface ProposalOpen {
  proposal: number;
  family: string;
  bill: number | null;
  /** What the player picked; an axis left out is the engine's default (or as the engine read it). */
  choices: ChoiceValue[];
  params: ParamValue[];
  /** The engine's preview of the package as set; null until it answers, or when it has none. */
  view: PolicyPreviewView | null;
  /** A preview is being asked for (what is shown is the last answer). */
  busy: boolean;
  /** The engine has no sheet for it (never heard of, or gone). */
  gone: boolean;
  /** The bill drafted or amended from the sheet, once the command was done. */
  done: { kind: "drafted" | "amended"; bill: number } | null;
}

interface ProposalState {
  open: ProposalOpen | null;
  /** Open the sheet and ask the engine for its preview. */
  openProposal(t: ProposalTarget, opts?: { opener?: HTMLElement | null }): Promise<void>;
  /** Pick an option of an axis; the engine previews the package again. */
  choose(axis: string, option: string): Promise<void>;
  /** Set a sponsor's version (their options and sliders) as the package. */
  setPackage(choices: ChoiceValue[], params: ParamValue[]): Promise<void>;
  /** Move the sliders (asked again once they rest). */
  setParams(params: ParamValue[]): void;
  /** Ask again (after a command changed the world). */
  refresh(): Promise<void>;
  /** A draft or an amendment went through. */
  markDone(kind: "drafted" | "amended", bill: number): void;
  /** Back from a bill to its proposal (drafting again). */
  leaveBill(): Promise<void>;
  close(): void;
}

let opener: HTMLElement | null = null;
let seq = 0;
let timer: number | undefined;

async function query(o: Pick<ProposalOpen, "proposal" | "family" | "choices" | "params">): Promise<PolicyPreviewView | null> {
  try {
    return expectKind(
      await engine().request({ kind: "query", payload: { type: "policy_preview", template: o.family, params: o.params, proposal: o.proposal, choices: o.choices } }),
      "policy_preview",
    );
  } catch {
    // Not heard of, or no such proposal: the sheet says so.
    return null;
  }
}

/** The choices with one axis set (the others kept). */
export function withChoice(choices: ChoiceValue[], axis: string, option: string): ChoiceValue[] {
  return [...choices.filter((c) => c.axis !== axis), { axis, option }];
}

export const useProposal = create<ProposalState>((set, get) => {
  /** Ask for the current package; a later ask wins. */
  async function ask(): Promise<void> {
    const o = get().open;
    if (!o) return;
    const mine = ++seq;
    set({ open: { ...o, busy: true } });
    const view = await query(o);
    const now = get().open;
    if (mine !== seq || !now || now.proposal !== o.proposal) return;
    set({ open: { ...now, view: view ?? (now.view && !now.gone ? now.view : null), busy: false, gone: view === null && now.view === null } });
  }
  return {
    open: null,
    async openProposal(t, opts) {
      opener = opts?.opener ?? null;
      window.clearTimeout(timer);
      set({
        open: { proposal: t.proposal, family: t.family, bill: t.bill ?? null, choices: t.choices ?? [], params: t.params ?? [], view: null, busy: true, gone: false, done: null },
      });
      await ask();
    },
    async choose(axis, option) {
      const o = get().open;
      if (!o) return;
      window.clearTimeout(timer);
      set({ open: { ...o, choices: withChoice(o.choices, axis, option), done: null } });
      await ask();
    },
    async setPackage(choices, params) {
      const o = get().open;
      if (!o) return;
      window.clearTimeout(timer);
      set({ open: { ...o, choices, params, done: null } });
      await ask();
    },
    setParams(params) {
      const o = get().open;
      if (!o) return;
      set({ open: { ...o, params, busy: true, done: null } });
      window.clearTimeout(timer);
      timer = window.setTimeout(() => void ask(), PROPOSAL_DEBOUNCE_MS);
    },
    async refresh() {
      await ask();
    },
    markDone(kind, bill) {
      const o = get().open;
      if (o) set({ open: { ...o, done: { kind, bill } } });
    },
    async leaveBill() {
      const o = get().open;
      if (!o) return;
      set({ open: { ...o, bill: null, choices: [], params: [], done: null } });
      await ask();
    },
    close() {
      window.clearTimeout(timer);
      seq++;
      set({ open: null });
      const back = opener;
      opener = null;
      if (back?.isConnected) back.focus();
    },
  };
});
