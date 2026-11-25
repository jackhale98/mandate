// The arc sheet (A3; DEC-109, DEC-113 §3.9): one matter as the player knows
// it, from the engine's `ArcView`. Where it stands and its clock in a
// sentence; what the player can do in it now, each beat a row whose button
// goes straight to the command or route that takes it (a decision's sheet,
// the ask's answer, a conversation, the place where the visit is done); who
// is in it; and what has happened. Below "known" the view is a lead and the
// sheet shows nothing else. Heat is never drawn (STAGED_ARCS §1.6). Every
// beat is reachable from here without walking or the map (DEC-051 parity).
// Opened from an arc's row on Today, a reason on a person, or the story so far.
import { useEffect, useRef, type ReactNode } from "react";
import type { ArcBeatView } from "../engine/protocol/ArcBeatView";
import type { ArcCastView } from "../engine/protocol/ArcCastView";
import type { ArcView } from "../engine/protocol/ArcView";
import { formatDate } from "../i18n/format";
import { placeIcon, placeName } from "../places/labels";
import { useGame } from "../state/gameStore";
import { capital } from "./DecisionSheet";
import { personName } from "./Knowing";
import { useSheetClose } from "./useSheetClose";
import { WhyButton } from "./why/WhyButton";
import { AskRow } from "./asks/Asks";
import { useScene } from "./exchange/sceneStore";
import { ArcLead } from "./notebook/Leads";
import { Face } from "./notebook/People";
import { Fold, Row, RowList, Section, When, shortDate } from "./notebook/rows";

const BEAT_MARK: Record<string, string> = { decision: "❗", ask: "🤝", talk: "💬", visit: "🚶", exchange: "🎤" };

/** What a beat needs, in a few words: where or with whom, and the time it takes. */
function needsLine(tr: ReturnType<typeof useGame.getState>["tr"], b: ArcBeatView): string {
  const parts: string[] = [];
  if (b.needs.person) parts.push(personName(tr, b.needs.person.name) || tr.t(b.needs.person.role_key));
  if (b.needs.place) parts.push(placeName(tr, b.needs.place));
  parts.push(b.needs.halves === 0 ? tr.t("ui-arc-cost-free") : b.needs.halves === 1 ? tr.t("ui-verb-cost-half") : tr.t("ui-verb-cost-slot"));
  return parts.join(" · ");
}

/**
 * One thing the player can do in the arc, as the command that does it. A
 * scene (DEC-112) opens the scene's screen from this row, as a decision's
 * sheet does: it says there whether it can be begun now.
 */
function Beat({ b, arc }: { b: ArcBeatView; arc: number }) {
  const tr = useGame((s) => s.tr);
  const busy = useGame((s) => s.busy);
  const pocket = useGame((s) => s.pocket);
  const openDecision = useGame((s) => s.openDecision);
  const openTalk = useGame((s) => s.openTalk);
  const visit = useGame((s) => s.visit);
  const openPanel = useGame((s) => s.openPanel);
  const openArc = useGame((s) => s.openArc);
  // An ask beat is the ask itself: opened, it is answered here (its price read first).
  if (b.kind === "ask" && b.ask !== null) {
    const a = pocket?.asks.find((x) => x.id === b.ask);
    if (a) return <AskRow a={a} />;
  }
  const label = capital(tr.event(b.label.key, b.label.args));
  const blocked = !b.available && b.blocked_key ? tr.t(b.blocked_key) : null;
  // The deadline goes in the line, so the title keeps the room the button leaves it.
  const due = b.days_left !== null ? tr.t("ui-days-left", { n: b.days_left }) : null;
  const sub = [needsLine(tr, b), due, blocked].filter(Boolean).join(" · ");
  let act: { text: string; run: () => void } | null = null;
  if (b.kind === "exchange") {
    const beat = `${b.stage}.${b.beat}`;
    act = { text: tr.t("ui-go"), run: () => void useScene.getState().openScene(arc, beat) };
  } else if (b.kind === "decision" && b.decision !== null) {
    const id = b.decision;
    act = { text: tr.t("ui-choose"), run: () => void openDecision(id) };
  } else if (b.kind === "talk" && b.needs.person?.id != null) {
    const person = b.needs.person.id;
    act = { text: tr.t("ui-talk"), run: () => void openTalk({ type: "person", person }) };
  } else if (b.kind === "visit") {
    const to = b.route?.to ?? b.needs.place;
    if (to) {
      const here = b.route?.here === true;
      act = {
        text: here ? tr.t("ui-arc-do-here") : tr.t("ui-go"),
        run: () => {
          void openArc(null);
          void (here ? openPanel(to.location) : visit(to.location));
        },
      };
    }
  }
  return (
    <Row
      mark={BEAT_MARK[b.kind] ?? "•"}
      title={label}
      sub={sub}
      label={`${label}. ${sub}`}
      onActivate={act?.run}
      disabled={!act || busy}
      end={
        <>
          {!act && b.days_left !== null && b.due && <When top={tr.t("ui-days-left", { n: b.days_left })} under={shortDate(tr, b.due)} />}
          {act && (
            <button type="button" className={b.kind === "decision" ? "btn btn-sm btn-primary rw-go" : "btn btn-sm rw-go"} tabIndex={-1} disabled={busy || (!b.available && b.kind !== "exchange")} aria-label={`${act.text}: ${label}`} onClick={act.run} data-testid={`arc-beat-act-${b.beat}`}>
              {act.text}
            </button>
          )}
        </>
      }
      li={{ "data-testid": `arc-beat-${b.beat}`, "data-kind": b.kind, "data-available": b.available ? "true" : "false", "data-verb": b.verb ?? undefined, "data-topic": b.topic ?? undefined }}
    />
  );
}

/** Someone, somewhere or a place in the arc, as a row. */
function Cast({ c }: { c: ArcCastView }) {
  const tr = useGame((s) => s.tr);
  const busy = useGame((s) => s.busy);
  const openTalk = useGame((s) => s.openTalk);
  const visit = useGame((s) => s.visit);
  const openArc = useGame((s) => s.openArc);
  const p = c.person;
  if (p && p.id !== null) {
    const id = p.id;
    const name = personName(tr, p.name) || tr.t(p.role_key);
    return (
      <Row
        mark={<Face id={id} />}
        title={name}
        sub={tr.t(p.role_key)}
        label={`${name}. ${tr.t(p.role_key)}`}
        onActivate={() => void openTalk({ type: "person", person: id })}
        end={
          <button type="button" className="btn btn-sm rw-go" tabIndex={-1} disabled={busy} aria-label={tr.t("ui-talk-to", { name })} onClick={() => void openTalk({ type: "person", person: id })}>
            {tr.t("ui-talk")}
          </button>
        }
        li={{ "data-testid": "arc-cast", "data-role": c.role }}
      />
    );
  }
  const at = c.at;
  const go = at
    ? () => {
        void openArc(null);
        void visit(at.location);
      }
    : undefined;
  return (
    <Row
      mark={at ? placeIcon(at.kind_key) : "🗺️"}
      title={at ? placeName(tr, at) : c.name}
      sub={at && at.kind_key !== "streets" ? at.neighborhood_name : undefined}
      onActivate={go}
      disabled={!go || busy}
      end={
        go ? (
          <button type="button" className="btn btn-sm rw-go" tabIndex={-1} disabled={busy} onClick={go}>
            {tr.t("ui-go")}
          </button>
        ) : undefined
      }
      li={{ "data-testid": "arc-cast", "data-role": c.role }}
    />
  );
}

function Body({ v }: { v: ArcView }) {
  const tr = useGame((s) => s.tr);
  // Below "known": a lead, and nothing else (A3).
  if (v.lead || !v.title) {
    return <div className="arc-lead" data-testid="arc-lead-only">{v.lead && <ArcLead lead={v.lead} />}</div>;
  }
  const stage = v.stage ? tr.event(v.stage.text.key, v.stage.text.args) : null;
  // The newest event is the stage's own line, shown above; the rest is how it came to this.
  const earlier = v.history.filter((e) => e.id !== v.stage?.event);
  const content: ReactNode[] = [];
  if (v.ended) {
    content.push(
      <p key="ended" className="arc-clock" data-testid="arc-ended">
        {v.ended.text ? tr.event(v.ended.text.key, v.ended.text.args) : tr.t("ui-arc-ended")} {tr.t("ui-arc-ended-on", { date: formatDate(tr, v.ended.on) })}
      </p>,
    );
  } else if (v.clock) {
    content.push(
      <p key="clock" className="arc-clock" data-testid="arc-clock">
        {tr.event(v.clock.text.key, v.clock.text.args)} ({formatDate(tr, v.clock.due)})
      </p>,
    );
  }
  if (stage)
    content.push(
      <div key="stage" className="arc-story" data-testid="arc-stage">
        <p>{stage}</p>
        <WhyButton event={v.stage?.event} about={stage} />
      </div>,
    );
  return (
    <>
      {content}
      {!v.ended && (
        <Section id="arc-do" title={tr.t("ui-arc-can-do")} testId="arc-beats">
          {v.beats.length === 0 && !v.turn ? (
            <p className="today-empty">{tr.t("ui-arc-nothing-now")}</p>
          ) : (
            <RowList>
              {v.beats.map((b) => (
                <Beat key={`${b.stage}-${b.beat}`} b={b} arc={v.id} />
              ))}
              {v.turn?.state === "ahead" && (
                <Row className="rw-muted" mark="⌛" title={tr.t("ui-arc-turn")} sub={v.turn.on ? tr.t("ui-arc-turn-on", { date: formatDate(tr, v.turn.on) }) : tr.t("ui-arc-turn-ahead")} disabled li={{ "data-testid": "arc-turn" }} />
              )}
            </RowList>
          )}
        </Section>
      )}
      {v.cast.length > 0 && (
        <Section id="arc-cast" title={tr.t("ui-arc-cast")} testId="arc-cast-list">
          <RowList>
            {v.cast.map((c) => (
              <Cast key={c.role} c={c} />
            ))}
          </RowList>
        </Section>
      )}
      {v.headline && (
        <p className="small muted" data-testid="arc-headline">
          {tr.t("ui-arc-paper", { headline: tr.event(v.headline.text_key, v.headline.args) })}
        </p>
      )}
      {earlier.length > 0 && (
        <Fold id={`arc-history-${v.id}`} title={tr.t("ui-arc-history")} count={earlier.length}>
          <div className="rw-pad">
            <ol className="events small" data-testid="arc-history">
              {earlier.map((e) => (
                <li key={e.id}>
                  <time className="muted">{formatDate(tr, e.date)}</time>
                  <span>{tr.event(e.text_key, e.args)}</span>
                </li>
              ))}
            </ol>
          </div>
        </Fold>
      )}
    </>
  );
}

export function ArcSheet() {
  const tr = useGame((s) => s.tr);
  const v = useGame((s) => s.arc);
  const openArc = useGame((s) => s.openArc);
  const [closing, requestClose, sheetRef] = useSheetClose(() => void openArc(null));
  const title = useRef<HTMLHeadingElement>(null);
  const opener = useRef<Element | null>(null);
  const id = v?.id ?? null;
  // Opening moves the keyboard to the title; closing gives it back to what opened the sheet (§4.3).
  useEffect(() => {
    if (id === null) return;
    opener.current = document.activeElement;
    const raf = requestAnimationFrame(() => title.current?.focus());
    return () => {
      cancelAnimationFrame(raf);
      const back = opener.current;
      if (back instanceof HTMLElement && back.isConnected) back.focus();
    };
  }, [id]);
  if (!v) return null;
  const name = v.title ? capital(tr.event(v.title.key, v.title.args)) : tr.t("ui-arc-unknown");
  const where = v.at?.name ?? null;
  const head = [v.ended ? tr.t("ui-arc-state-ended") : tr.t("ui-under-way"), where, v.days !== null && !v.ended ? tr.t("ui-arc-days-on", { n: v.days }) : null].filter(Boolean).join(" · ");
  return (
    <>
      <div className={closing ? "sheet-scrim arc-scrim sheet-closing" : "sheet-scrim arc-scrim"} onClick={requestClose} aria-hidden="true" />
      <aside ref={sheetRef} className={closing ? "sheet arc-sheet sheet-closing" : "sheet arc-sheet"} aria-labelledby="arc-title" data-testid="arc-sheet" data-arc={v.id}>
        <div className="sheet-head">
          <div>
            {v.title && <p className="muted small">{head}</p>}
            <h2 id="arc-title" ref={title} tabIndex={-1}>
              {name}
            </h2>
          </div>
          <button type="button" className="btn btn-ghost" onClick={requestClose} aria-label={tr.t("ui-close")}>
            ✕
          </button>
        </div>
        <div className="sheet-body pocket-body">
          <Body v={v} />
        </div>
      </aside>
    </>
  );
}
