// The talk panel (DEC-093): talking to someone by topic. Topics are buttons
// with honest odds (DEC-073: the engine's, the ones it rolls) and the cost;
// what is unavailable says why. A reply shows as a short dialogue in the
// speaker's voice, then what the player learned (the notebook's entries,
// DEC-091) and the leads. Everything shown comes from the engine's views
// (`TalkView`, `ExchangeView`); the UI decides nothing. Reached from the
// place panel's people, the pocket's People tab and the walkable street, so
// the List view and quick mode reach the same conversations (DEC-051).
import { useLayoutEffect, useRef, useState } from "react";
import type { TopicView } from "../../engine/protocol/TopicView";
import type { AboutChoiceView } from "../../engine/protocol/AboutChoiceView";
import type { TalkView } from "../../engine/protocol/TalkView";
import { useGame } from "../../state/gameStore";
import { placeName } from "../../places/labels";
import { useSheetClose } from "../useSheetClose";
import { roleText } from "../CharacterSheet";
import { ExchangeCard } from "./ExchangeCard";
import { aboutText, oddsText } from "./talkText";
import "./talk.css";

/** The cost line: half a slot (or the half left over), the late slot, or out of time. */
function Cost({ v }: { v: TalkView }) {
  const tr = useGame((s) => s.tr);
  if (v.out_of_time)
    return (
      <p className="talk-out" role="status" data-testid="talk-out-of-time">
        <span aria-hidden="true">⌛ </span>
        {v.late_night ? tr.t("talk-why-late-used") : tr.t("ui-talk-out-of-time")}
      </p>
    );
  return (
    <p className="tradeoff talk-cost" data-testid="talk-cost">
      {v.late_night ? tr.t("ui-talk-cost-late") : v.half_slot_left ? tr.t("ui-talk-cost-half-left") : tr.t(v.cost_halves <= 1 ? "ui-talk-cost-half" : "ui-talk-cost-slot", { left: v.slots_left })}
    </p>
  );
}

/** Subjects by kind, in the order the engine lists them. */
function groups(list: readonly AboutChoiceView[]): [string, AboutChoiceView[]][] {
  const out: [string, AboutChoiceView[]][] = [];
  for (const c of list) {
    const g = out.find(([k]) => k === c.kind);
    if (g) g[1].push(c);
    else out.push([c.kind, [c]]);
  }
  return out;
}

/** One topic: a button, or (for a topic about something) the subject to choose and an Ask button. */
function Topic({ t }: { t: TopicView }) {
  const tr = useGame((s) => s.tr);
  const busy = useGame((s) => s.busy);
  const talkAbout = useGame((s) => s.talkAbout);
  // The subject chosen, by what it is (the list can change when the view is refreshed).
  const first = t.about_required ? (t.about.find((c) => c.available) ?? t.about[0] ?? null) : null;
  const [pick, setPick] = useState<string>(first ? JSON.stringify(first.about) : "");
  const choice: AboutChoiceView | null = (pick ? t.about.find((c) => JSON.stringify(c.about) === pick) : null) ?? (t.about_required ? (first ?? null) : null);
  const available = choice ? choice.available : t.available;
  const odds = choice ? choice.odds : t.odds;
  const why = choice ? choice.unavailable_key : t.unavailable_key;
  const label = tr.t(t.name_key);
  const meta = (
    <>
      {odds !== null && (
        <strong className="talk-odds" data-testid="talk-odds">
          {oddsText(tr, odds)}
        </strong>
      )}
      {t.asked_this_week && <span className="muted small talk-asked">{tr.t("ui-talk-asked")}</span>}
    </>
  );
  const reason = !available && why && (
    <p className="muted small talk-why" data-testid="talk-why">
      {tr.t(why)}
    </p>
  );
  if (t.about.length === 0)
    return (
      <li className="talk-topic" data-testid={`talk-topic-${t.id}`}>
        <button type="button" className="btn talk-topic-btn" disabled={busy || !available} onClick={() => void talkAbout(t.id, null)}>
          <span className="talk-topic-name">{label}</span>
          {meta}
        </button>
        {reason}
      </li>
    );
  const id = `talk-about-${t.id.replace(/\W/g, "-")}`;
  return (
    <li className="talk-topic talk-topic-about" data-testid={`talk-topic-${t.id}`}>
      <div className="talk-topic-head">
        <span className="talk-topic-name">{label}</span>
        {meta}
      </div>
      <div className="talk-topic-row">
        <label className="visually-hidden" htmlFor={id}>
          {tr.t("ui-talk-about")}
        </label>
        <select id={id} className="input talk-about" value={choice ? JSON.stringify(choice.about) : ""} onChange={(e) => setPick(e.target.value)} data-testid="talk-about">
          {!t.about_required && <option value="">{tr.t("ui-talk-about-none")}</option>}
          {/* Grouped by kind where there are several: a bill and the policy it drafts share a name. */}
          {groups(t.about).map(([kind, list]) => {
            const options = list.map((c) => (
              <option key={JSON.stringify(c.about)} value={JSON.stringify(c.about)}>
                {aboutText(tr, c.label, c.label_key)}
              </option>
            ));
            return groups(t.about).length > 1 ? (
              <optgroup key={kind} label={tr.t(`ui-talk-kind-${kind}`)}>
                {options}
              </optgroup>
            ) : (
              options
            );
          })}
        </select>
        <button type="button" className="btn btn-primary talk-ask" disabled={busy || !available} onClick={() => void talkAbout(t.id, choice ? choice.about : null)} data-testid="talk-ask">
          {tr.t("ui-talk-ask")}
        </button>
      </div>
      {reason}
    </li>
  );
}

/** The talk panel: a sheet over the place panel (bottom sheet on phones, side panel from 900px). */
export function TalkSheet() {
  const tr = useGame((s) => s.tr);
  const v = useGame((s) => s.talk);
  const x = useGame((s) => s.exchange);
  const late = useGame((s) => s.talkLate);
  const busy = useGame((s) => s.busy);
  const openTalk = useGame((s) => s.openTalk);
  const setLate = useGame((s) => s.setTalkLate);
  const goToTalk = useGame((s) => s.goToTalk);
  const sheet = useRef<HTMLElement>(null);
  const [closing, requestClose, sheetRef] = useSheetClose(() => void openTalk(null), sheet);
  const who = v ? (v.person.id ?? `a${v.person.ambient_slot ?? ""}`) : null;
  // Someone new starts at the top; a new reply is scrolled to (it is at the top of the body).
  useLayoutEffect(() => {
    if (sheet.current) sheet.current.scrollTop = 0;
  }, [who, x?.id]);
  if (!v) return null;
  // A passer-by is a stranger until the conversation starts.
  const title = v.person.name ? tr.t("ui-talk-with", { name: v.person.name }) : tr.t("ui-talk-with-stranger");
  return (
    <>
      <div className={closing ? "sheet-scrim talk-scrim sheet-closing" : "sheet-scrim talk-scrim"} onClick={requestClose} aria-hidden="true" />
      <aside ref={sheetRef} className={closing ? "sheet talk-sheet sheet-closing" : "sheet talk-sheet"} aria-labelledby="talk-title" data-testid="talk-panel">
        <div className="sheet-head">
          <div className="talk-head">
            <h2 id="talk-title">{title}</h2>
            <p className="muted small">
              {roleText(tr, v.person)}
              {roleText(tr, v.person) && " · "}
              <span data-testid="talk-opinion">{tr.t(`ui-talk-opinion-${v.opinion_band}`)}</span>
            </p>
          </div>
          <div className="sheet-tools">
            <button type="button" className="btn btn-ghost" onClick={requestClose} aria-label={tr.t("ui-close")}>
              ✕
            </button>
          </div>
        </div>
        <div className="sheet-body talk-body">
          {x && <ExchangeCard x={x} />}
          {!v.here && (
            <div className="note" data-testid="talk-away">
              {v.route?.to ? (
                <>
                  <p>{tr.t("ui-talk-away", { place: placeName(tr, v.route.to) })}</p>
                  <button type="button" className="btn btn-primary" disabled={busy} onClick={() => void goToTalk()} data-testid="talk-go">
                    {tr.t("ui-talk-go", { place: placeName(tr, v.route.to) })}
                  </button>
                </>
              ) : (
                <p>{tr.t("ui-talk-nowhere")}</p>
              )}
            </div>
          )}
          <section className="card talk-topics" aria-labelledby="talk-topics-title" data-testid="talk-topics">
            <h3 className="sub" id="talk-topics-title">
              {x ? tr.t("ui-talk-again") : tr.t("ui-talk-topics")}
            </h3>
            {v.here && <Cost v={v} />}
            {v.late_night_possible && (
              <label className="check">
                <input type="checkbox" checked={late} disabled={busy} onChange={(e) => void setLate(e.target.checked)} data-testid="talk-late" />
                {tr.t("ui-talk-late")}
              </label>
            )}
            <ul className="talk-topic-list">
              {v.topics.map((t) => (
                <Topic key={`${who}-${t.id}`} t={t} />
              ))}
            </ul>
          </section>
        </div>
      </aside>
    </>
  );
}
