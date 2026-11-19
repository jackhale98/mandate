import type { KnowledgeView } from "../engine/protocol/KnowledgeView";
import type { LeadView } from "../engine/protocol/LeadView";
import type { Translator } from "../i18n/fluent";
import { useGame } from "../state/gameStore";

// Knowledge is earned (DEC-091): the engine redacts what the player does not
// know and says so; these render its placeholders. The UI never decides what
// is known. The notebook page itself is K5.

/** A name as the player knows it: empty from the engine means a stranger. */
export function personName(tr: Translator, name: string): string {
  return name === "" ? tr.t("knowledge-person-unnamed") : name;
}

/** A lead toward knowing more, when the engine gives one. */
export function LeadLine({ lead }: { lead: LeadView | null }) {
  const tr = useGame((s) => s.tr);
  if (!lead) return null;
  return (
    <span className="muted small" data-testid="knowledge-lead">
      {" "}
      · {tr.t("knowledge-lead")}: {tr.event(lead.key, lead.args)}
    </span>
  );
}

/** "Not known yet — ask around", with the lead if there is one. */
export function Unknown({ k, text }: { k: KnowledgeView; text: string }) {
  const tr = useGame((s) => s.tr);
  return (
    <span className="muted" data-testid="knowledge-placeholder">
      {tr.t(text)}
      <LeadLine lead={k.lead} />
    </span>
  );
}
