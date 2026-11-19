import { useLayoutEffect, useRef } from "react";
import type { DecisionView } from "../engine/protocol/DecisionView";
import type { EffectLineView } from "../engine/protocol/EffectLineView";
import type { BeatView } from "../engine/protocol/BeatView";
import { useGame } from "../state/gameStore";
import type { Translator } from "../i18n/fluent";
import { formatDate } from "../i18n/format";
import { placeIcon, placeName } from "../places/labels";
import { useSheetClose } from "./useSheetClose";
import { roleText, subjectText } from "./CharacterSheet";

/** One trade-off or aftermath line (facts and Fluent keys from the engine). */
function Line({ l }: { l: EffectLineView }) {
  const tr = useGame((s) => s.tr);
  return <li className={`effect effect-${l.tone}`}>{tr.event(l.key, l.args)}</li>;
}

/** A line of dialogue: the speaker may quote a memory ("You promised the bridge in '31"). */
function beatText(tr: Translator, d: DecisionView, b: BeatView): string {
  const args: Record<string, string> = { ...d.args };
  if (b.quote) args.memory = tr.t(b.quote.key, { subject: subjectText(tr, b.quote.subject), year: String(b.quote.year) });
  return tr.event(b.key, args);
}

/** Titles are noun phrases (memories quote them); headings start with a capital. */
export function capital(s: string): string {
  return s.charAt(0).toUpperCase() + s.slice(1);
}

/** The deadline, in days, with the date. */
export function deadlineText(tr: Translator, d: DecisionView): string {
  return `${tr.t("ui-decide-by", { date: formatDate(tr, d.deadline) })} · ${tr.t("ui-decide-days", { n: d.days_left })}`;
}

/**
 * A decision (DEC-077 §4): the situation, who is involved, each option with
 * its costs and gains (DEC-073), the deadline and what happens without an
 * answer; after choosing, an aftermath card with what each effect did. The
 * scene's conversation is shown where it happens (DEC-079); the decision can
 * be answered from anywhere.
 */
export function DecisionSheet() {
  const tr = useGame((s) => s.tr);
  const d = useGame((s) => s.decision);
  const open = useGame((s) => s.openDecision);
  const answer = useGame((s) => s.answer);
  const visit = useGame((s) => s.visit);
  const openCharacter = useGame((s) => s.openCharacter);
  const busy = useGame((s) => s.busy);
  const [closing, requestClose] = useSheetClose(() => void open(null));
  const sheet = useRef<HTMLElement>(null);
  useLayoutEffect(() => {
    if (sheet.current) sheet.current.scrollTop = 0;
  }, [d?.id, d?.status]);
  if (!d) return null;
  const pending = d.status === "pending";
  const fallback = d.options.find((o) => o.id === d.default_option);
  return (
    <>
      <div className={closing ? "sheet-scrim decision-scrim sheet-closing" : "sheet-scrim decision-scrim"} onClick={requestClose} aria-hidden="true" />
      <aside ref={sheet} className={closing ? "sheet decision-sheet sheet-closing" : "sheet decision-sheet"} aria-labelledby="decision-title" data-testid="decision-sheet">
        <div className="sheet-head">
          <div>
            <p className="muted small">
              <span className={d.kind === "climax" ? "chip chip-warn" : "chip chip-muted"}>{tr.t(`ui-decision-kind-${d.kind}`)}</span>
              {d.storyline_key && <> · {tr.t(d.storyline_key)}</>}
            </p>
            <h2 id="decision-title" className="decision-title">
              {capital(tr.event(d.title_key, d.args))}
            </h2>
            {pending && (
              <p className="small" data-testid="decision-deadline">
                {deadlineText(tr, d)}
              </p>
            )}
          </div>
          <div className="sheet-tools">
            <button type="button" className="btn btn-ghost" onClick={requestClose} aria-label={tr.t("ui-close")}>
              ✕
            </button>
          </div>
        </div>
        <div className="sheet-body">
          {d.aftermath && (
            <section className="card aftermath" data-testid="decision-aftermath" aria-live="polite">
              <h3 className="sub">{tr.t("ui-aftermath")}</h3>
              <p>
                <strong>{d.aftermath.by_default ? tr.t("ui-aftermath-default") : tr.event(d.aftermath.label_key, d.args)}</strong>
              </p>
              <p>{tr.event(d.aftermath.key, d.args)}</p>
              {d.aftermath.lines.length > 0 && (
                <ul className="plain effect-list">
                  {d.aftermath.lines.map((l, i) => (
                    <Line key={i} l={l} />
                  ))}
                </ul>
              )}
            </section>
          )}
          <section className="card">
            <p data-testid="decision-situation">{tr.event(d.situation_key, d.args)}</p>
            {d.people.length > 0 && (
              <>
                <h3 className="sub">{tr.t("ui-who-involved")}</h3>
                <ul className="people">
                  {d.people.map((p) => (
                    <li key={p.id ?? p.name}>
                      <button type="button" className="link-btn person" onClick={() => p.id !== null && void openCharacter(p.id)}>
                        <span>{p.name}</span>
                        <span className="muted small">{roleText(tr, p)}</span>
                      </button>
                    </li>
                  ))}
                </ul>
              </>
            )}
            {d.reasons.length > 0 && (
              <>
                <h3 className="sub">{tr.t("ui-why-these")}</h3>
                <ul className="plain small">
                  {d.reasons.map((r, i) => (
                    <li key={i}>{tr.event(r.key, r.args)}</li>
                  ))}
                </ul>
              </>
            )}
          </section>
          {d.scene.place && (
            <section className="card scene" data-testid="decision-scene">
              <h3 className="sub">
                <span aria-hidden="true">{placeIcon(d.scene.place.kind_key)} </span>
                {tr.t("ui-scene-at", { place: placeName(tr, d.scene.place) })}
              </h3>
              {d.scene.here ? (
                <ul className="plain quotes">
                  {d.scene.beats.map((b, i) => (
                    <li key={i} className={b.quote ? `memory memory-${b.quote.tone}` : undefined}>
                      {b.speaker && <strong>{b.speaker}: </strong>}
                      {beatText(tr, d, b)}
                    </li>
                  ))}
                </ul>
              ) : (
                pending && (
                  <p className="small">
                    {tr.t("ui-scene-hint")}{" "}
                    <button type="button" className="btn btn-sm btn-ghost" disabled={busy} onClick={() => d.scene.place && void visit(d.scene.place.location)}>
                      {tr.t("ui-scene-go")}
                    </button>
                  </p>
                )
              )}
            </section>
          )}
          {pending && (
            <section className="card" data-testid="decision-options">
              <h3 className="sub">{tr.t("ui-choose")}</h3>
              <ol className="options">
                {d.options.map((o) => (
                  <li key={o.id} className="option">
                    <p className="option-label">
                      <strong>{tr.event(o.label_key, d.args)}</strong>
                    </p>
                    <div className="option-tradeoffs">
                      {o.gains.length > 0 && (
                        <div>
                          <h4 className="sub small">{tr.t("ui-gains")}</h4>
                          <ul className="plain effect-list">
                            {o.gains.map((l, i) => (
                              <Line key={i} l={l} />
                            ))}
                          </ul>
                        </div>
                      )}
                      {o.costs.length > 0 && (
                        <div>
                          <h4 className="sub small">{tr.t("ui-costs")}</h4>
                          <ul className="plain effect-list">
                            {o.costs.map((l, i) => (
                              <Line key={i} l={l} />
                            ))}
                          </ul>
                        </div>
                      )}
                    </div>
                    {o.unavailable_key && <p className="small muted">{tr.t(o.unavailable_key)}</p>}
                    <button type="button" className="btn btn-sm" disabled={busy || o.unavailable_key !== null} onClick={() => void answer(d.id, o.id)} data-testid={`option-${o.id}`}>
                      {tr.t("ui-choose")}
                    </button>
                  </li>
                ))}
              </ol>
              {fallback && <p className="note small">{tr.t("ui-if-you-dont", { option: tr.event(fallback.label_key, d.args) })}</p>}
            </section>
          )}
        </div>
      </aside>
    </>
  );
}
