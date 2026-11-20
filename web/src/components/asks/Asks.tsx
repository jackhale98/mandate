// Asks and promises (DEC-097 §2; ENGAGEMENT §3.9): the Today block (offers
// with Accept and Decline, accepted asks and promises with their next need,
// progress, deadline and a way to do it), casework at the services desk, and
// the story sheet's promises. Everything shown is the engine's: what an ask
// needs, how far along it is, where it is done, the casework odds and what
// keeping or breaking it does (the UI never computes an outcome).
import type { AskView } from "../../engine/protocol/AskView";
import type { EffectLineView } from "../../engine/protocol/EffectLineView";
import type { PocketView } from "../../engine/protocol/PocketView";
import type { Translator } from "../../i18n/fluent";
import { formatDate } from "../../i18n/format";
import { placeIcon, placeName } from "../../places/labels";
import { useGame } from "../../state/gameStore";
import { capital } from "../DecisionSheet";
import { personName } from "../Knowing";
import { TalkButton } from "../talk/TalkButton";

/** The ask in a few words, as a heading. */
export function askTitle(tr: Translator, a: AskView): string {
  return capital(tr.event(a.title_key, a.args));
}

/** Who asks, in words. */
export function askerText(tr: Translator, a: AskView): string {
  const x = a.asker;
  switch (x.type) {
    case "person":
      return x.person.name ? personName(tr, x.person.name) : tr.t(x.person.role_key);
    case "party":
      return x.voice ? tr.t("ui-ask-asker-party", { party: x.name, name: personName(tr, x.voice.name) }) : x.name;
    case "outlet":
      return x.name;
    case "constituency":
    case "electorate":
      return tr.event(x.name_key, a.args);
  }
}

/** Its status in words (an offer left unanswered lapsed). */
export function askStatusText(tr: Translator, a: AskView): string {
  return tr.t(`ui-ask-status-${a.status === "declined" && a.unanswered ? "lapsed" : a.status}`);
}

function Lines({ title, lines }: { title: string; lines: EffectLineView[] }) {
  const tr = useGame((s) => s.tr);
  if (lines.length === 0) return null;
  return (
    <div>
      <h4 className="sub small">{title}</h4>
      <ul className="plain effect-list">
        {lines.map((l, i) => (
          <li key={i} className={`effect effect-${l.tone}`}>
            {tr.event(l.key, l.args)}
          </li>
        ))}
      </ul>
    </div>
  );
}

/** When it must be answered (an offer) or kept (accepted). */
function deadline(tr: Translator, a: AskView): string {
  return a.status === "offered" && a.respond_by ? tr.t("ui-ask-respond-by", { date: formatDate(tr, a.respond_by), n: a.days_left }) : tr.t("ui-ask-due", { date: formatDate(tr, a.due), n: a.days_left });
}

/** What to do next, with a way to do it: go there, talk to them, or sort it out (with the engine's odds). */
function NextStep({ a }: { a: AskView }) {
  const tr = useGame((s) => s.tr);
  const busy = useGame((s) => s.busy);
  const visit = useGame((s) => s.visit);
  const casework = useGame((s) => s.casework);
  const title = askTitle(tr, a);
  const current = a.needs.find((n) => n.state === "current");
  return (
    <div className="ask-next">
      <p className="small" data-testid="ask-need">
        {tr.t("ui-ask-next", { need: tr.event(a.need_key, a.args) })}
        {a.progress && <span className="muted"> · {tr.t("ui-ask-progress", { done: a.progress.done, total: a.progress.total })}</span>}
      </p>
      {current?.watch && <p className="muted small">{tr.t("ui-ask-watch")}</p>}
      <div className="bill-actions">
        {a.odds !== null ? (
          <>
            <p className="tradeoff small" data-testid="ask-odds">
              <strong>{tr.t("ui-ask-odds", { pct: Math.round(a.odds * 100) })}</strong>
            </p>
            <button
              type="button"
              className="btn btn-sm btn-primary"
              disabled={busy}
              aria-label={tr.t("ui-ask-casework-label", { ask: title, pct: Math.round(a.odds * 100) })}
              onClick={() => void casework(a.id)}
              data-testid={`ask-casework-${a.id}`}
            >
              {tr.t("ui-ask-casework")}
            </button>
          </>
        ) : (
          a.how?.to &&
          !a.how.here && (
            <button
              type="button"
              className="btn btn-sm btn-ghost"
              disabled={busy}
              aria-label={tr.t("ui-ask-go-label", { place: placeName(tr, a.how.to), ask: title })}
              onClick={() => a.how?.to && void visit(a.how.to.location)}
              data-testid={`ask-go-${a.id}`}
            >
              <span aria-hidden="true">{placeIcon(a.how.to.kind_key)} </span>
              {placeName(tr, a.how.to)}
            </button>
          )
        )}
        {a.person && a.person.id !== null && <TalkButton person={a.person.id} name={a.person.name} />}
      </div>
    </div>
  );
}

/** One ask or promise: what, who, by when, the next step, and what keeping or breaking it does. */
export function AskItem({ a }: { a: AskView }) {
  const tr = useGame((s) => s.tr);
  const busy = useGame((s) => s.busy);
  const respond = useGame((s) => s.respondAsk);
  const title = askTitle(tr, a);
  const offered = a.status === "offered";
  const open = offered || a.status === "accepted";
  const headId = `ask-${a.id}-title`;
  return (
    <li className="ask" data-testid={`ask-${a.id}`} data-status={a.status} aria-labelledby={headId}>
      <div className="ask-head">
        <strong id={headId}>{title}</strong>
        <span className="muted small">
          {" "}
          · {a.promise ? tr.t("ui-ask-promise") : tr.t("ui-ask-from", { who: askerText(tr, a) })}
        </span>
      </div>
      <p className="muted small">
        <span className={a.status === "broken" ? "chip chip-warn" : "chip chip-muted"} data-testid="ask-status">
          {askStatusText(tr, a)}
        </span>
        {a.public && <> · {tr.t("ui-ask-public")}</>}
        {a.big && <> · {tr.t("ui-ask-big")}</>}
        {open && <> · {deadline(tr, a)}</>}
        {a.resolved_on && <> · {formatDate(tr, a.resolved_on)}</>}
      </p>
      {a.outcome_key && <p className="small">{tr.event(a.outcome_key, a.args)}</p>}
      {open && <p className="small">{tr.event(a.detail_key, a.args)}</p>}
      {a.status === "accepted" && <NextStep a={a} />}
      {offered && (
        <>
          <p className="small">{tr.t("ui-ask-next", { need: tr.event(a.need_key, a.args) })}</p>
          <div className="bill-actions">
            <button type="button" className="btn btn-sm btn-primary" disabled={busy} aria-label={tr.t("ui-ask-accept-label", { ask: title })} onClick={() => void respond(a.id, true)} data-testid={`ask-accept-${a.id}`}>
              {tr.t("ui-ask-accept")}
            </button>
            <button type="button" className="btn btn-sm" disabled={busy} aria-label={tr.t("ui-ask-decline-label", { ask: title })} onClick={() => void respond(a.id, false)} data-testid={`ask-decline-${a.id}`}>
              {tr.t("ui-ask-decline")}
            </button>
          </div>
        </>
      )}
      {open && (a.keep_lines.length > 0 || a.break_lines.length > 0 || a.decline_lines.length > 0) && (
        <details className="ask-details">
          <summary className="small">{tr.t("ui-ask-details")}</summary>
          <div className="option-tradeoffs">
            <Lines title={tr.t("ui-ask-if-kept")} lines={a.keep_lines} />
            <Lines title={tr.t("ui-ask-if-broken")} lines={a.break_lines} />
            <Lines title={tr.t("ui-ask-if-declined")} lines={a.decline_lines} />
          </div>
        </details>
      )}
    </li>
  );
}

/** The Today tab's "Asks and promises" block, under Decisions. */
export function TodayAsks({ p }: { p: PocketView }) {
  const tr = useGame((s) => s.tr);
  const busy = useGame((s) => s.busy);
  const openPromises = useGame((s) => s.openPromises);
  return (
    <section className="card" data-testid="pocket-asks" aria-labelledby="pocket-asks-title">
      <h2 id="pocket-asks-title">{tr.t("ui-asks")}</h2>
      {p.can_promise && (
        <button type="button" className="btn btn-sm" disabled={busy} onClick={() => void openPromises(true)} data-testid="open-promises">
          {tr.t("ui-promise-make")}
        </button>
      )}
      {p.asks.length === 0 ? (
        <p className="muted small">{tr.t("ui-asks-none")}</p>
      ) : (
        <ul className="plain-list asks">
          {p.asks.map((a) => (
            <AskItem key={a.id} a={a} />
          ))}
        </ul>
      )}
      {p.asks_recent.length > 0 && (
        <>
          <h3 className="sub">{tr.t("ui-asks-recent")}</h3>
          <ul className="plain-list asks" data-testid="asks-recent">
            {p.asks_recent.map((a) => (
              <AskItem key={a.id} a={a} />
            ))}
          </ul>
        </>
      )}
    </section>
  );
}

/** Casework at the services desk (the place panel where it is offered): each accepted ask that needs it, with the engine's odds. */
export function CaseworkHere() {
  const tr = useGame((s) => s.tr);
  const pocket = useGame((s) => s.pocket);
  const cases = (pocket?.asks ?? []).filter((a) => a.status === "accepted" && a.odds !== null);
  return (
    <section className="card" data-testid="place-casework" aria-labelledby="place-casework-title">
      <h2 id="place-casework-title">{tr.t("ui-casework")}</h2>
      <p className="hint">{tr.t("ui-casework-help")}</p>
      {cases.length === 0 ? (
        <p className="muted small">{tr.t("ui-casework-none")}</p>
      ) : (
        <ul className="plain-list asks">
          {cases.map((a) => (
            <AskItem key={a.id} a={a} />
          ))}
        </ul>
      )}
    </section>
  );
}

/** The story sheet's promises: every campaign promise made, how it stands, and the career's counts. */
export function StoryPromises() {
  const tr = useGame((s) => s.tr);
  const asks = useGame((s) => s.asks);
  if (!asks) return null;
  const t = asks.tally;
  return (
    <section className="card" data-testid="story-promises" aria-labelledby="story-promises-title">
      <h3 className="sub" id="story-promises-title">
        {tr.t("ui-story-promises")}
      </h3>
      <p className="muted small">{tr.t("ui-story-promises-tally", { made: t.promises_made, kept: t.promises_kept, broken: t.promises_broken })}</p>
      {asks.promises.length === 0 ? (
        <p className="muted small">{tr.t("ui-story-promises-none")}</p>
      ) : (
        <ul className="plain-list asks">
          {asks.promises.map((a) => (
            <AskItem key={a.id} a={a} />
          ))}
        </ul>
      )}
      <p className="muted small" data-testid="story-asks-tally">
        {tr.t("ui-story-asks-tally", { accepted: t.accepted, kept: t.kept, broken: t.broken, declined: t.declined })}
      </p>
    </section>
  );
}
