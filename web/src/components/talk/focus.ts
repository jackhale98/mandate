// Where a conversation opens (M6 note 29): "Talk to them" on the whip's
// list opens the conversation on the bill, straight at the arguments the
// engine offers on it. Presentation only: which arguments there are, their
// chances and costs, are the engine's (`TalkView`).
import { create } from "zustand";

interface TalkFocus {
  /** The bill the next conversation opens on, if any. */
  bill: number | null;
  setBill(bill: number | null): void;
}

export const useTalkFocus = create<TalkFocus>((set) => ({
  bill: null,
  setBill: (bill) => set({ bill }),
}));
