import { describe, expect, it } from "vitest";
import type { TopicView } from "../../engine/protocol/TopicView";
import { lobbyBills } from "./TalkSheet";

const topic = (id: string, bill?: number): TopicView => ({
  id,
  name_key: "lobby-choice-plain",
  mechanic: "choice",
  about_required: false,
  about: [],
  odds: 0.4,
  available: true,
  unavailable_key: null,
  asked_this_week: false,
  ...(bill === undefined ? {} : { bill, name_args: { bill_key: `policy-${bill}` } }),
});

describe("lobbying in conversation (M6 note 29)", () => {
  it("groups the engine's arguments by bill, in its order, once each", () => {
    const topics = [topic("choice.lobby.3.plain", 3), topic("topic.worries"), topic("choice.lobby.1.plain", 1), topic("choice.lobby.3.owe", 3)];
    expect(lobbyBills(topics)).toEqual([
      { bill: 3, nameKey: "policy-3" },
      { bill: 1, nameKey: "policy-1" },
    ]);
  });

  it("has no bill to argue when the engine offers none", () => {
    expect(lobbyBills([topic("topic.worries")])).toEqual([]);
  });
});
