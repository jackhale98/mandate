// Where a conversation is shown (dialogue mode, DEC-093): in the street, when
// the person stands there by the player and the street is in sight, or over
// a portrait otherwise. The walkable street (../TownStreet) offers itself as
// a stage while it is mounted; the conversation (./TalkSheet) asks it to
// frame the two speakers, dims the rest and anchors the speech bubbles to
// their sprites. Presentation only: who is here and what is said come from
// the engine's views.
import { create } from "zustand";

export interface Point {
  x: number;
  y: number;
}

/** Where the speakers' heads are on screen (viewport CSS px), and the street's box. */
export interface StageAnchors {
  player: Point;
  person: Point;
  /** How tall the sprites are on screen (CSS px): bubbles sit above the heads. */
  scale: number;
  bounds: { left: number; top: number; right: number; bottom: number };
}

export interface TalkStage {
  /** The street can frame a conversation with this person now: they stand there by the player, in sight. */
  canFrame(key: string): boolean;
  /** Frame the player and the person, facing each other, in the street above `boxTop` (the dialogue box's top edge, viewport px); pause walking. */
  begin(key: string, boxTop: number): void;
  /** The dialogue box's top edge moved (viewport px). */
  cover(boxTop: number): void;
  /** Someone is speaking (a reply being revealed): they move a little. */
  speaking(who: "player" | "person" | null): void;
  /** The conversation ended: the camera goes back to following the player, walking resumes. */
  end(): void;
  anchors(): StageAnchors | null;
  /** Called after every drawn frame while a conversation is framed (the bubbles follow the camera). */
  onFrame(fn: () => void): () => void;
  /** The look of someone standing in the street (the walker's), if they do. */
  lookOf(key: string): number | null;
  /** Give the keyboard back to the street. */
  focus(): void;
}

export type TalkMode = "street" | "portrait";

interface StageState {
  stage: TalkStage | null;
  /** How the open conversation is shown (null: none open). The street pauses behind a portrait. */
  mode: TalkMode | null;
}

export const useTalkStage = create<StageState>(() => ({ stage: null, mode: null }));

export function setTalkStage(stage: TalkStage | null) {
  useTalkStage.setState({ stage });
}

export function setTalkMode(mode: TalkMode | null) {
  if (useTalkStage.getState().mode !== mode) useTalkStage.setState({ mode });
}
