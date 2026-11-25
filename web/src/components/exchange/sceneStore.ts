// The scene on screen (DEC-112; EXCHANGE_SCENES §7): which scene is open, the
// engine's view of it (`ArcSceneView`), and the commands that play it. The
// store decides nothing: it sends `ArcExchange` (inside `Go` to begin, as a
// visit beat is begun) and asks the engine for the view again. What it keeps
// of its own is what it showed: the prompt each round was put with and the
// line the player chose, so the reaction can be said with the same words.
import { create } from "zustand";
import { engine, expectKind } from "../../engine/client";
import type { ArcSceneView } from "../../engine/protocol/ArcSceneView";
import type { CommandDto } from "../../engine/protocol/CommandDto";
import type { SceneOptionView } from "../../engine/protocol/SceneOptionView";
import type { TextView } from "../../engine/protocol/TextView";
import { useGame } from "../../state/gameStore";

/** What was on screen when a round was answered. */
export interface Said {
  prompt: TextView;
  option: SceneOptionView;
}

const PLAIN_KEY = "mandate.scene.plain";

function readPlain(): boolean {
  try {
    return globalThis.localStorage?.getItem(PLAIN_KEY) === "1";
  } catch {
    return false;
  }
}

interface SceneState {
  /** The scene open, by its run and `<stage>.<beat>`. */
  open: { arc: number; beat: string } | null;
  view: ArcSceneView | null;
  /** Quick mode: the scene as a plain list (§7.3). Remembered on this device. */
  plain: boolean;
  /** Each round answered, as it was shown (by round number). */
  said: Record<number, Said>;
  /** Open a scene (from Today or the arc's sheet): the sheets over it give way. */
  openScene(arc: number, beat: string): Promise<void>;
  refresh(): Promise<void>;
  /** Turn up: go to its place and begin, in one request. */
  begin(): Promise<void>;
  say(option: SceneOptionView): Promise<void>;
  /** Walk out: the rounds left are forfeited. */
  leave(): Promise<void>;
  close(): void;
  setPlain(plain: boolean): void;
}

async function query(arc: number, beat: string): Promise<ArcSceneView | null> {
  try {
    return expectKind(await engine().request({ kind: "query", payload: { type: "arc_exchange", arc, beat } }), "arc_exchange");
  } catch {
    return null;
  }
}

export const useScene = create<SceneState>((set, get) => {
  /** Send a command for the open scene, then read the scene again. The scene says what happened: no toast over it. */
  const send = async (cmd: CommandDto) => {
    const game = useGame.getState();
    await game.command(cmd);
    useGame.getState().dismissToast();
    await get().refresh();
  };
  const act = (o: { arc: number; beat: string }, a: Extract<CommandDto, { type: "arc_exchange" }>["act"]): CommandDto => ({ type: "arc_exchange", arc: o.arc, beat: o.beat, act: a });
  return {
    open: null,
    view: null,
    plain: readPlain(),
    said: {},

    async openScene(arc, beat) {
      const g = useGame.getState();
      g.setPocketOpen(false);
      if (g.arc) void g.openArc(null);
      const view = await query(arc, beat);
      set({ open: { arc, beat }, view, said: {} });
    },

    async refresh() {
      const o = get().open;
      if (!o) return;
      const view = await query(o.arc, o.beat);
      // Gone (its arc ended and the player knows nothing of it): the scene closes.
      if (!view) set({ open: null, view: null, said: {} });
      else set({ view });
    },

    async begin() {
      const o = get().open;
      const v = get().view;
      if (!o || !v) return;
      const to = v.route?.to ?? v.place;
      const begin = act(o, { type: "begin" });
      await send(to && !v.route?.here ? { type: "go", location: to.location, then: begin } : begin);
    },

    async say(option) {
      const o = get().open;
      const v = get().view;
      if (!o || !v || !v.prompt) return;
      set({ said: { ...get().said, [v.round]: { prompt: v.prompt.text, option } } });
      await send(act(o, { type: "say", option: option.id }));
    },

    async leave() {
      const o = get().open;
      if (!o) return;
      await send(act(o, { type: "leave" }));
    },

    close() {
      set({ open: null, view: null, said: {} });
    },

    setPlain(plain) {
      try {
        globalThis.localStorage?.setItem(PLAIN_KEY, plain ? "1" : "0");
      } catch {
        // A private window: the choice lasts the session.
      }
      set({ plain });
    },
  };
});
