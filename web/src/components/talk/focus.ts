// Where a conversation opens (M6 note 29): "Talk to them" on the whip's
// list opens the conversation on the bill, straight at the arguments the
// engine offers on it; an agenda verb had in conversation (DEC-114, B5)
// opens it at the verb's choice. Presentation only: which choices there
// are, their chances and costs, are the engine's (`TalkView`).
import { create } from "zustand";

interface TalkFocus {
  /** The bill the next conversation opens on, if any. */
  bill: number | null;
  setBill(bill: number | null): void;
  /** The topic (a choice's id) the next conversation opens at, if any. */
  topic: string | null;
  setTopic(topic: string | null): void;
}

export const useTalkFocus = create<TalkFocus>((set) => ({
  bill: null,
  setBill: (bill) => set({ bill }),
  topic: null,
  setTopic: (topic) => set({ topic }),
}));
