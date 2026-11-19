// "Talk" beside someone in a list (the place panel's people, the pocket's
// People tab): opens the talk panel for them (DEC-093). Someone elsewhere is
// fine too: the panel says where they are and walks the player there. A
// passer-by (no id yet) is met as the conversation starts.
import { useGame } from "../../state/gameStore";
import { personName } from "../Knowing";

export function TalkButton({ person, slot = null, name }: { person: number | null; slot?: number | null; name: string }) {
  const tr = useGame((s) => s.tr);
  const busy = useGame((s) => s.busy);
  const openTalk = useGame((s) => s.openTalk);
  if (person === null && slot === null) return null;
  return (
    <button
      type="button"
      className="btn btn-sm talk-btn"
      disabled={busy}
      aria-label={tr.t("ui-talk-to", { name: personName(tr, name) })}
      onClick={() => void openTalk(person !== null ? { type: "person", person } : { type: "ambient", slot: slot ?? 0 })}
      data-testid="talk-button"
    >
      <span aria-hidden="true">💬 </span>
      {tr.t("ui-talk")}
    </button>
  );
}
