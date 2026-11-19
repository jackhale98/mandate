// An exchange as it was spoken (DEC-093): the player's question, the reply
// in the speaker's voice (an opener as a stage direction, then what they
// say), the outcome, then what the player learned (new notebook entries,
// DEC-091) and the leads it opened. Lines are Fluent keys with the facts
// captured when they were said (`ExchangeView`); nothing is composed here.
import type { ExchangeView } from "../../engine/protocol/ExchangeView";
import type { LeadView } from "../../engine/protocol/LeadView";
import { useGame } from "../../state/gameStore";
import { placeName } from "../../places/labels";
import { personName } from "../Knowing";
import { aboutText, learnedText } from "./talkText";

function Lead({ l }: { l: LeadView }) {
  const tr = useGame((s) => s.tr);
  const busy = useGame((s) => s.busy);
  const visit = useGame((s) => s.visit);
  const openTalk = useGame((s) => s.openTalk);
  const to = l.route?.to ?? null;
  return (
    <li className="talk-lead">
      <span>
        <span aria-hidden="true">→ </span>
        {tr.event(l.key, l.args)}
      </span>
      {to && !l.route?.here && (
        <button
          type="button"
          className="btn btn-sm btn-ghost"
          disabled={busy}
          onClick={() => {
            void openTalk(null);
            // The lead's place: go there and open it (free).
            void visit(to.location);
          }}
        >
          {tr.t("ui-talk-lead-go")} · {placeName(tr, to)}
        </button>
      )}
    </li>
  );
}

export function ExchangeCard({ x }: { x: ExchangeView }) {
  const tr = useGame((s) => s.tr);
  const topic = tr.t(x.topic_key);
  const about = x.about_label || x.about_label_key ? aboutText(tr, x.about_label, x.about_label_key) : "";
  const name = personName(tr, x.speaker.name);
  return (
    <section className="card talk-exchange" aria-labelledby="talk-said-title" data-testid="talk-exchange">
      <h3 className="sub" id="talk-said-title">
        {tr.t("ui-talk-said")}
      </h3>
      <p className="talk-you small" data-testid="talk-you">
        {about ? tr.t("ui-talk-you-asked-about", { topic, about }) : tr.t("ui-talk-you-asked", { topic })}
      </p>
      {/* The reply, announced as it arrives. */}
      <div className="talk-dialogue" aria-live="polite" data-testid="talk-dialogue">
        {x.parts.map((p, i) => (
          <p key={i} className={p.part === "opener" ? "talk-stage" : "talk-line"} data-part={p.part}>
            {tr.event(p.key, p.args)}
          </p>
        ))}
      </div>
      <p className="talk-outcome small" data-testid="talk-outcome">
        {tr.event(x.outcome_key, x.outcome_args)}
        {x.relationship && <> {tr.t(x.relationship === "warmer" ? "ui-talk-warmer" : "ui-talk-cooler", { name })}</>}
      </p>
      <h3 className="sub">{tr.t("ui-talk-learned")}</h3>
      {x.learned.length === 0 ? (
        <p className="muted small">{tr.t("ui-talk-learned-none")}</p>
      ) : (
        <ul className="plain-list talk-learned" data-testid="talk-learned">
          {x.learned.map((l, i) => (
            <li key={i}>
              <span aria-hidden="true">✎ </span>
              {learnedText(tr, l)} <span className="chip chip-muted talk-level">{tr.t(`knowledge-level-${l.level}`)}</span>
            </li>
          ))}
        </ul>
      )}
      {x.learned.length > 0 && <p className="hint">{tr.t("ui-talk-in-notebook")}</p>}
      {x.leads.length > 0 && (
        <>
          <h3 className="sub">{tr.t("ui-talk-leads")}</h3>
          <ul className="plain-list talk-leads" data-testid="talk-leads">
            {x.leads.map((l, i) => (
              <Lead key={i} l={l} />
            ))}
          </ul>
        </>
      )}
    </section>
  );
}
