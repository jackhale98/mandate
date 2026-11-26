// The doorstep (DEC-114 §3.3): which door the player is at, what the engine
// says is behind it (`DoorView`), and the reply to what they said. A door is
// opened from the street (a house's door, walked to) or from the street's
// rows (quick mode); both go through here, so the two paths are one.
// Presentation only: the engine generates the household, rolls the reply
// and keeps what is remembered.
import { create } from "zustand";
import { engine, expectKind } from "../../engine/client";
import type { DoorChoiceDto } from "../../engine/protocol/DoorChoiceDto";
import type { DoorView } from "../../engine/protocol/DoorView";
import { useGame } from "../../state/gameStore";

export interface DoorReply {
  key: string;
  args: Record<string, string>;
}

interface DoorState {
  /** The door the player is at (its neighbourhood and slot), or null. */
  at: { neighborhood: number; door: number } | null;
  view: DoorView | null;
  /** Their reply to the player's choice, once made. */
  reply: DoorReply | null;
  error: string | null;
  open(neighborhood: number, door: number): Promise<void>;
  close(): void;
  /** Say one of the three things (the engine's `doorstep` action), then read the reply. */
  say(choice: DoorChoiceDto): Promise<void>;
}

const errText = (e: unknown) => (e instanceof Error ? e.message : String(e));

async function fetchDoor(neighborhood: number, door: number): Promise<DoorView | null> {
  return expectKind(await engine().request({ kind: "query", payload: { type: "door", neighborhood, door } }), "door");
}

export const useDoor = create<DoorState>((set, get) => ({
  at: null,
  view: null,
  reply: null,
  error: null,

  async open(neighborhood, door) {
    try {
      const view = await fetchDoor(neighborhood, door);
      set({ at: view ? { neighborhood, door } : null, view, reply: null, error: null });
    } catch (e) {
      set({ error: errText(e) });
    }
  },

  close() {
    set({ at: null, view: null, reply: null, error: null });
  },

  async say(choice) {
    const at = get().at;
    const game = useGame.getState();
    if (!at || get().reply || game.busy) return;
    // The knock is a command like any other: saved, and the pocket refreshed after it.
    await game.command({ type: "act", action: { type: "doorstep", neighborhood: at.neighborhood, door: at.door, choice }, late_night: false });
    const after = useGame.getState();
    if (after.error) {
      set({ error: after.error });
      return;
    }
    // The reply is the action's outcome: shown on the doorstep, not as a toast as well.
    const t = after.toast;
    if (t) after.dismissToast();
    // What they said first stays on the doorstep; the street around it moves on.
    let view = get().view;
    try {
      const fresh = await fetchDoor(at.neighborhood, at.door);
      if (fresh && view) view = { ...view, knocked: fresh.knocked, street: fresh.street };
    } catch {
      // The reply stands; the street's rows refresh with the next view.
    }
    set({ reply: t ? { key: t.key, args: t.args } : null, view });
  },
}));
