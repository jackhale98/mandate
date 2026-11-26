// Fundraising, the press, the bribe and the leak are conversations
// (DEC-114, B5): an agenda card, a place's action or a booth names whom the
// verb is had with and where, and "Talk to them" opens the talk panel on
// them, at the verb's choice (going there first, free). The choice shows its
// chance and costs; the result is the exchange, with its Why. The engine
// says who, where and what it costs (`VerbTalkView`, `OfferView.with`).
import type { LocationView } from "../../engine/protocol/LocationView";
import type { PersonRefView } from "../../engine/protocol/PersonRefView";
import { useGame } from "../../state/gameStore";
import { placeName } from "../../places/labels";
import { roleText } from "../CharacterSheet";
import { personName } from "../Knowing";
import { useTalkFocus } from "./focus";

/** The agenda verbs had in conversation (action keys). */
export const TALK_VERBS: ReadonlySet<string> = new Set(["press", "fundraise", "take_bribe", "leak_file"]);

export function VerbTalk({ action, person, to, here, topic }: { action: string; person: PersonRefView | null | undefined; to: LocationView | null; here: boolean; topic: string | null | undefined }) {
  const tr = useGame((s) => s.tr);
  const busy = useGame((s) => s.busy);
  const shady = useGame((s) => s.office?.player.shady.find((x) => x.action === action));
  const who = person && person.id !== null ? person : null;
  const open = async () => {
    if (!who || who.id === null) return;
    const g = useGame.getState();
    useTalkFocus.getState().setTopic(topic ?? null);
    // Where it is done: go there first (moving is free, DEC-079).
    if (!here && to) {
      if (g.openPlace) useGame.setState({ openPlace: to.location });
      await g.command({ type: "move_to", location: to.location });
    }
    await useGame.getState().openTalk({ type: "person", person: who.id });
  };
  return (
    <div className="action-form" data-testid={`verb-talk-${action}`}>
      <p className="tradeoff">{tr.t(`tradeoff-${action.replace(/_/g, "-")}`)}</p>
      {who ? (
        <p className="small" data-testid="verb-talk-with">
          {tr.t(here || !to ? "ui-verb-talk-here" : "ui-verb-talk-at", { name: personName(tr, who.name), role: roleText(tr, who), place: to ? placeName(tr, to) : "" })}
        </p>
      ) : (
        <p className="muted small" data-testid="verb-talk-nobody">
          {tr.t(shady?.unavailable_key ?? "ui-verb-talk-nobody")}
        </p>
      )}
      <button type="button" className="btn btn-primary btn-block" disabled={busy || !who} onClick={() => void open()} data-testid="verb-talk-open">
        <span aria-hidden="true">💬 </span>
        {tr.t("ui-verb-talk")}
      </button>
    </div>
  );
}
