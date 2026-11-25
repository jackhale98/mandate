// A scene of an arc (DEC-112; EXCHANGE_SCENES §7): a debate, played in the
// conversation view. The other side's portrait and their bubble (the prompt)
// above a dialogue box; in the box the audience strip (one chip a group: its
// name, a mark on a five-step scale drawn as a shape, an arrow when it moved,
// "?" where the player cannot know its lean), then the lines the player can
// say, each with whom it lands with and what it risks shown before choosing,
// and at most one locked line that names a route. After a choice the line goes
// up in the player's bubble, the room's reaction and the other side's answer
// follow, and Continue opens the next round; after three rounds, the result
// card (won, split, lost; how each group moved; what the paper will say; any
// promise made) with "Why?" on the closing event.
//
// Quick mode (§7.3) is the same scene as one plain sheet: the audience as a
// list, the prompt as a paragraph, the lines as a list with their tags written
// out, no stage and no bubbles, and no Continue between rounds. The choices,
// the numbers and the commands are the same.
//
// The UI computes nothing (rule 4): groups, leans, ranges, steps, chances and
// the result are the engine's `ArcSceneView`; this file lays them out.
//
// Keyboard (§7.4): one Tab stop for the list; arrows, Home and End move;
// Enter chooses; digits 1 to 5 choose a row; Right arrow opens a row's detail
// and Left arrow or Escape leaves it; Escape on the list asks before walking
// out. Screen reader: the prompt is read by a polite live region, then the
// number of choices; a row's name is its label and its tags in words; the
// strip is a list; the result's table is a table. Focus goes to the list when
// a round opens and to the result when the scene ends.
import { useEffect, useLayoutEffect, useMemo, useRef, useState, type KeyboardEvent as ReactKeyboardEvent, type ReactNode } from "react";
import type { ArcSceneView } from "../../engine/protocol/ArcSceneView";
import type { SceneLeanView } from "../../engine/protocol/SceneLeanView";
import type { SceneOptionView } from "../../engine/protocol/SceneOptionView";
import type { SceneRoundView } from "../../engine/protocol/SceneRoundView";
import { formatDate } from "../../i18n/format";
import type { Translator } from "../../i18n/fluent";
import { placeName } from "../../places/labels";
import { useGame } from "../../state/gameStore";
import { Portrait } from "../portrait/Portrait";
import { lookVariant } from "../portrait/look";
import { WhyButton } from "../why/WhyButton";
import { arrows, chipName, groupName, landsText, leanText, optionName, pointsText, resultChange, say, shortNote } from "./sceneText";
import { useScene, type Said } from "./sceneStore";
import "./exchange.css";

const reducedMotion = () => typeof matchMedia === "function" && matchMedia("(prefers-reduced-motion: reduce)").matches;
const TYPE_STEP = 3;
const TYPE_MS = 24;

/** The scene layer: shown while a scene is open (from Today or an arc's sheet). */
export function SceneSheet() {
  const open = useScene((s) => s.open);
  const v = useScene((s) => s.view);
  if (!open || !v) return null;
  return <Scene key={`${v.arc}:${v.beat}`} v={v} />;
}

type Step = { kind: "list" } | { kind: "more"; id: string } | { kind: "leave" };

/** The other side's name, or their role where the player has not heard of them. */
function otherName(tr: Translator, v: ArcSceneView): string {
  const o = v.other;
  if (!o) return "";
  return o.name || tr.t(o.role_key);
}

const MARKS = ["◀◀", "◀", "◆", "▶", "▶▶"];

/** A lean in a result's cell: its mark and points for the eyes, its words for a screen reader. */
function leanCell(tr: Translator, lean: SceneLeanView | null): ReactNode {
  if (!lean) return tr.t("ui-scene-not-known");
  return (
    <>
      <span aria-hidden="true" className="scn-cell-lean">
        {MARKS[lean.step + 2]} {lean.points > 0 ? `+${lean.points}` : lean.points < 0 ? `−${-lean.points}` : "0"}
      </span>
      <span className="visually-hidden">{leanText(tr, lean)}</span>
    </>
  );
}

/** What the player said in a round, and the other side's answer, in the words shown at the time. */
function roundWords(tr: Translator, v: ArcSceneView, r: SceneRoundView, s: Said | undefined) {
  const args = { ...v.args, ...(s?.option.args ?? {}) };
  return {
    // Without the line as it was shown (a reload mid-scene), the room alone: never a sentence with a hole in it.
    said: r.said_key && s ? tr.event(r.said_key, args) : null,
    reply: r.reply_key && s ? tr.event(r.reply_key, args) : null,
    room: tr.t(r.room_key),
  };
}

function Scene({ v }: { v: ArcSceneView }) {
  const tr = useGame((s) => s.tr);
  const busy = useGame((s) => s.busy);
  const plain = useScene((s) => s.plain);
  const said = useScene((s) => s.said);
  const { begin, say: sayLine, leave, close, setPlain } = useScene.getState();

  const [step, setStep] = useState<Step>({ kind: "list" });
  // Rounds whose reaction has been seen (the staged scene shows each before the next round).
  const [seen, setSeen] = useState(v.played.length);
  const last = v.played.at(-1) ?? null;
  const reacting = !plain && v.played.length > seen && last !== null;
  const showResult = v.phase === "done" && !reacting;

  const listRef = useRef<HTMLUListElement>(null);
  const resultRef = useRef<HTMLDivElement>(null);
  const boxRef = useRef<HTMLElement>(null);
  const [active, setActive] = useState(0);
  const [lockedOpen, setLockedOpen] = useState(false);

  // The prompt types out (a tap shows it whole; reduced motion: whole at once).
  const prompt = v.prompt ? say(tr, v.prompt.text) : "";
  const promptKey = `${v.round}:${v.prompt?.text.key ?? ""}:${plain ? 1 : 0}`;
  const [shownPrompt, setShownPrompt] = useState({ key: promptKey, typed: Infinity });
  // A new prompt types from its start (the derived-state pattern: no effect, no flash of the old one).
  if (shownPrompt.key !== promptKey) setShownPrompt({ key: promptKey, typed: reducedMotion() || plain ? Infinity : 0 });
  const typed = shownPrompt.typed;
  const setTyped = (f: number | ((t: number) => number)) => setShownPrompt((s) => ({ ...s, typed: typeof f === "function" ? f(s.typed) : f }));
  const typing = typed < prompt.length && !reacting;
  useEffect(() => {
    if (!typing) return;
    const id = window.setInterval(() => setShownPrompt((s) => ({ ...s, typed: Math.min(prompt.length, s.typed + TYPE_STEP) })), TYPE_MS);
    return () => window.clearInterval(id);
  }, [typing, prompt.length]);

  const other = otherName(tr, v);
  const title = say(tr, v.title);
  const words = last ? roundWords(tr, v, last, said[last.round]) : null;

  // ------------------------------------------------------------------ focus

  const round = v.phase === "under_way" && !reacting ? v.round : 0;
  useEffect(() => {
    // A round opens: the list; the scene ends: the result; a reaction: its Continue.
    if (showResult) resultRef.current?.focus({ preventScroll: true });
    else {
      const items = [...(boxRef.current?.querySelectorAll<HTMLElement>("[data-choice]") ?? [])];
      (items[0] ?? null)?.focus({ preventScroll: true });
    }
  }, [round, reacting, showResult, v.phase, step.kind, plain]);

  // The focus goes back to whatever opened the scene.
  useLayoutEffect(() => {
    const opener = document.activeElement;
    return () => {
      if (opener instanceof HTMLElement && opener.isConnected) opener.focus({ preventScroll: true });
    };
  }, []);

  // ------------------------------------------------------------------ acts

  const choose = (o: SceneOptionView) => {
    if (busy) return;
    setStep({ kind: "list" });
    setLockedOpen(false);
    void sayLine(o);
  };
  const shut = () => {
    if (v.phase === "under_way") setStep({ kind: "leave" });
    else close();
  };

  // Escape: out of a row's detail, else out of the leaving question, else asks before walking out (or closes).
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape" || e.defaultPrevented || document.querySelector(".why-sheet")) return;
      e.preventDefault();
      e.stopPropagation();
      if (step.kind !== "list") setStep({ kind: "list" });
      else shut();
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  });

  const options = v.phase === "under_way" && !reacting ? v.options : [];
  const onListKey = (e: ReactKeyboardEvent) => {
    if (e.key === "ArrowLeft" && step.kind === "more") {
      e.preventDefault();
      setStep({ kind: "list" });
      return;
    }
    const items = [...(listRef.current?.querySelectorAll<HTMLElement>("[data-choice]") ?? [])];
    if (items.length === 0) return;
    const i = Math.max(0, items.indexOf(document.activeElement as HTMLElement));
    const n = items.length;
    const move = (to: number) => {
      e.preventDefault();
      items[to]?.focus();
      setActive(to);
    };
    if (e.key === "ArrowDown") return move((i + 1) % n);
    if (e.key === "ArrowUp") return move(i <= 0 ? n - 1 : i - 1);
    if (e.key === "Home") return move(0);
    if (e.key === "End") return move(n - 1);
    const digit = /^[1-9]$/.test(e.key) ? Number(e.key) - 1 : -1;
    if (digit >= 0 && step.kind === "list") {
      const o = options[digit];
      if (o) {
        e.preventDefault();
        choose(o);
      }
      return;
    }
    if (e.key === "ArrowRight" && step.kind === "list") {
      const id = items[i]?.dataset.choice ?? "";
      if (options.some((o) => o.id === id)) {
        e.preventDefault();
        setStep({ kind: "more", id });
      }
    }
  };

  // ------------------------------------------------------------------ parts

  const strip = (
    <ul className={`scn-strip${plain ? " scn-strip-plain" : ""}`} aria-label={tr.t("ui-scene-audience")} data-testid="scene-strip">
      {v.groups.map((g) => (
        <li
          key={g.key}
          className={`scn-chip scn-size-${g.size}`}
          aria-label={chipName(tr, g)}
          data-testid="scene-chip"
          data-group={g.key}
          data-known={g.lean ? "true" : "false"}
          data-step={g.lean ? String(g.lean.step) : undefined}
          data-moved={g.moved_steps === null ? undefined : String(g.moved_steps)}
        >
          <span className="scn-chip-name" aria-hidden="true">
            {say(tr, plain ? g.label : g.short)}
          </span>
          {plain ? (
            <span className="scn-chip-lean" aria-hidden="true">
              {tr.t(g.lean_key)}
              {g.moved_steps !== null && g.moved_steps !== 0 && ` · ${tr.t(g.moved_steps > 0 ? "ui-scene-moved-up" : "ui-scene-moved-down")}`}
            </span>
          ) : (
            <span className="scn-scale" aria-hidden="true" data-step={g.lean ? String(g.lean.step) : "?"}>
              <span className="scn-scale-line" />
              <span className="scn-mark">{g.lean ? "◆" : "?"}</span>
              {g.moved_steps !== null && g.moved_steps !== 0 && <span className="scn-moved">{g.moved_steps > 0 ? "+" : "−"}</span>}
            </span>
          )}
        </li>
      ))}
    </ul>
  );

  // A line's tags: its chance and its cost first (always exact, never cut off a narrow row), then whom it lands with.
  const tagsOf = (o: SceneOptionView, spelled: boolean): ReactNode[] => {
    const out: ReactNode[] = [];
    if (o.chance !== null)
      out.push(
        <span key="chance" className="scn-tag scn-tag-chance">
          {spelled ? tr.t("ui-scene-chance", { pct: o.chance }) : tr.t("ui-scene-chance-short", { pct: o.chance })}
        </span>,
      );
    const cost = o.notes.map((n) => shortNote(tr, n)).find((x) => x !== null);
    if (cost)
      out.push(
        <span key="cost" className="scn-tag scn-tag-cost">
          {cost}
        </span>,
      );
    for (const t of o.tags)
      out.push(
        spelled ? (
          <span key={t.group} className="scn-tag">
            {landsText(tr, v, t, true)}
          </span>
        ) : (
          <span key={t.group} className="scn-tag" data-range={t.exact ? undefined : "true"}>
            <b>{arrows(t)}</b> {groupName(tr, v, t.group, true)}
          </span>
        ),
      );
    return out;
  };

  const optionRow = (o: SceneOptionView, i: number) => (
    <li
      key={o.id}
      className="scn-opt-li"
      data-testid="scene-option"
      data-option={o.id}
      data-kind={o.kind}
      data-safe={o.safe ? "true" : undefined}
      // The engine's previews, for tests that hold the result inside them (shown = rolled).
      data-stands={JSON.stringify(o.stands)}
      data-fails={JSON.stringify(o.fails)}
    >
      <button
        type="button"
        className="dlg-choice scn-opt"
        data-choice={o.id}
        tabIndex={active === i ? 0 : -1}
        aria-label={optionName(tr, v, o)}
        aria-disabled={busy || undefined}
        aria-keyshortcuts={String(i + 1)}
        onFocus={() => setActive(i)}
        onClick={() => choose(o)}
      >
        <span className="scn-opt-text">
          <span className="scn-opt-label">
            {!plain && (
              <span className="scn-num" aria-hidden="true">
                {i + 1}
              </span>
            )}
            {say(tr, o.label)}
          </span>
          <span className="scn-tags" aria-hidden="true">
            {tagsOf(o, plain)}
          </span>
        </span>
      </button>
      <button type="button" className="scn-more" tabIndex={-1} aria-label={tr.t("ui-scene-more", { line: say(tr, o.label) })} onClick={() => setStep({ kind: "more", id: o.id })} data-testid="scene-more">
        ›
      </button>
    </li>
  );

  const lockedRow = v.locked && (
    <li className="scn-locked" data-testid="scene-locked">
      {lockedOpen || plain ? (
        <p className="scn-locked-text">
          <span aria-hidden="true">🔒 </span>
          {say(tr, v.locked.text)}
        </p>
      ) : (
        <button type="button" className="link-btn scn-locked-open" tabIndex={-1} onClick={() => setLockedOpen(true)} data-testid="scene-locked-open">
          <span aria-hidden="true">🔒 </span>
          {tr.t("ui-scene-locked-open")}
        </button>
      )}
    </li>
  );

  const more = (id: string) => {
    const o = v.options.find((x) => x.id === id);
    if (!o) return null;
    return (
      <div className="scn-detail" data-testid="scene-detail" onKeyDown={onListKey}>
        <div className="dlg-step-head">
          <button type="button" className="dlg-back" data-choice="back" onClick={() => setStep({ kind: "list" })} data-testid="scene-back">
            <span aria-hidden="true">‹ </span>
            {tr.t("ui-dlg-back")}
          </button>
          <h3 className="dlg-step-title">{say(tr, o.label)}</h3>
        </div>
        <p className="scn-detail-h">{o.chance !== null ? tr.t("ui-scene-if-stands", { pct: o.chance }) : tr.t("ui-scene-lands")}</p>
        <ul className="scn-detail-list" data-testid="scene-detail-stands">
          {o.stands.map((c) => (
            <li key={c.group} data-group={c.group} data-lo={c.lo} data-hi={c.hi} data-exact={c.exact ? "true" : "false"}>
              <span>{groupName(tr, v, c.group)}</span> <b>{pointsText(tr, c)}</b>
              {!c.exact && <span className="scn-muted"> · {tr.t("ui-scene-not-known-how")}</span>}
            </li>
          ))}
        </ul>
        {o.fails.length > 0 && (
          <>
            <p className="scn-detail-h">{tr.t("ui-scene-if-fails")}</p>
            <ul className="scn-detail-list" data-testid="scene-detail-fails">
              {o.fails.map((c) => (
                <li key={c.group} data-group={c.group} data-lo={c.lo} data-hi={c.hi}>
                  <span>{groupName(tr, v, c.group)}</span> <b>{pointsText(tr, c)}</b>
                </li>
              ))}
            </ul>
          </>
        )}
        {(o.notes.length > 0 || o.answer_key) && (
          <ul className="scn-notes">
            {o.notes.map((n, i) => (
              <li key={i}>{say(tr, n)}</li>
            ))}
            {o.answer_key && <li>{tr.t(o.answer_key)}</li>}
          </ul>
        )}
        <p className="scn-muted small">{tr.t("ui-scene-points")}</p>
        <button type="button" className="btn btn-sm btn-primary" onClick={() => choose(o)} disabled={busy} data-testid="scene-say">
          {tr.t("ui-scene-say-this")}
        </button>
      </div>
    );
  };

  const result = showResult && v.result && (
    <div className="scn-result" ref={resultRef} tabIndex={-1} aria-labelledby="scn-result-h" data-testid="scene-result" data-outcome={v.result.outcome}>
      <h3 id="scn-result-h" className="scn-outcome">
        {tr.t(v.result.outcome_key)}
      </h3>
      <p>{say(tr, v.result.text)}</p>
      <table className="scn-table" data-testid="scene-result-table">
        <caption className="visually-hidden">{tr.t("ui-scene-moved-caption")}</caption>
        <thead>
          <tr>
            <th scope="col">{tr.t("ui-scene-group")}</th>
            <th scope="col">{tr.t("ui-scene-before")}</th>
            <th scope="col">{tr.t("ui-scene-after")}</th>
            <th scope="col">{tr.t("ui-scene-change")}</th>
          </tr>
        </thead>
        <tbody>
          {v.result.groups.map((g) => (
            <tr key={g.group} data-group={g.group} data-steps={g.steps}>
              <th scope="row">{groupName(tr, v, g.group)}</th>
              <td>{leanCell(tr, g.before)}</td>
              <td>{leanCell(tr, g.after)}</td>
              <td>
                <span aria-hidden="true">{arrows({ lo_steps: g.steps, hi_steps: g.steps })} </span>
                {resultChange(tr, g)}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      <p className="scn-paper" data-testid="scene-paper">
        <span className="scn-label">{tr.t("ui-scene-paper")}</span> “{say(tr, v.result.headline)}”
      </p>
      {v.result.promises.length > 0 && (
        <div data-testid="scene-promises">
          <p className="scn-label">{tr.t("ui-scene-promised")}</p>
          <ul className="scn-notes">
            {v.result.promises.map((p) => (
              <li key={p.ask}>
                {say(tr, p.title)} · {tr.t("ui-scene-due", { date: formatDate(tr, p.due) })}
              </li>
            ))}
          </ul>
        </div>
      )}
      <div className="scn-actions">
        <WhyButton event={v.result.event} about={tr.t(v.result.outcome_key)} />
        <button type="button" className="btn btn-sm btn-primary" onClick={close} data-testid="scene-close">
          {tr.t("ui-close")}
        </button>
      </div>
    </div>
  );

  // What is in the box's list, by phase.
  const list = (): ReactNode => {
    if (step.kind === "leave")
      return (
        <div className="scn-leave" data-testid="scene-leave">
          <p className="dlg-status">{tr.t("ui-scene-leave-ask")}</p>
          <ul className="dlg-choices" ref={listRef} onKeyDown={onListKey}>
            <li className="dlg-item">
              <button type="button" className="dlg-choice" data-choice="stay" onClick={() => setStep({ kind: "list" })} data-testid="scene-stay">
                {tr.t("ui-scene-stay")}
              </button>
            </li>
            <li className="dlg-item">
              <button
                type="button"
                className="dlg-choice"
                data-choice="walk"
                onClick={() => {
                  setStep({ kind: "list" });
                  void leave();
                }}
                data-testid="scene-walk-out"
              >
                {tr.t("ui-scene-walk-out")}
              </button>
            </li>
          </ul>
        </div>
      );
    if (step.kind === "more") return more(step.id);
    if (showResult) return result;
    if (reacting && words) {
      const done = v.phase === "done";
      return (
        <>
          <p className="dlg-status scn-room" data-testid="scene-room">
            <i>{words.room}</i>
          </p>
          <ul className="dlg-choices" ref={listRef} onKeyDown={onListKey}>
            <li className="dlg-item dlg-item-continue">
              <button type="button" className="dlg-choice" data-choice="continue" onClick={() => setSeen(v.played.length)} data-testid="scene-continue">
                {done ? tr.t("ui-scene-to-result") : tr.t("ui-scene-next-round", { n: v.round, of: v.rounds })}
              </button>
            </li>
          </ul>
        </>
      );
    }
    if (v.phase === "open" || v.phase === "ahead")
      return (
        <>
          <p className="dlg-status" data-testid="scene-offer">
            {say(tr, v.offer)}
          </p>
          {v.blocked_key && <p className="dlg-status dlg-status-out">{tr.t(v.blocked_key)}</p>}
          <ul className="dlg-choices" ref={listRef} onKeyDown={onListKey}>
            {v.phase === "open" && (
              <li className="dlg-item dlg-item-go">
                <button type="button" className="dlg-choice" data-choice="begin" aria-disabled={!v.available || busy || undefined} onClick={() => v.available && !busy && void begin()} data-testid="scene-begin">
                  <span className="dlg-choice-label">{tr.t("ui-scene-begin", { place: v.place ? placeName(tr, v.place) : "" })}</span>
                  <span className="dlg-tag">{tr.t(v.halves <= 1 ? "ui-verb-cost-half" : "ui-verb-cost-slot")}</span>
                </button>
              </li>
            )}
            <li className="dlg-item dlg-item-bye">
              <button type="button" className="dlg-choice" data-choice="later" onClick={close} data-testid="scene-later">
                {tr.t("ui-scene-not-now")}
              </button>
            </li>
          </ul>
        </>
      );
    return (
      <ul className="dlg-choices scn-options" ref={listRef} onKeyDown={onListKey} aria-label={tr.t("ui-scene-choices")} data-testid="scene-options">
        {options.map(optionRow)}
        {lockedRow}
      </ul>
    );
  };

  // The live region: the prompt, then the number of choices; after a choice, how the room took it and the answer.
  const live = useMemo(() => {
    if (showResult && v.result) return `${tr.t(v.result.outcome_key)}. ${say(tr, v.result.text)}`;
    if (reacting && words) return [words.room, words.reply ? tr.t("ui-dlg-says", { name: other, line: words.reply }) : null].filter(Boolean).join(" ");
    if (v.phase === "under_way" && v.prompt && step.kind === "list") return `${tr.t("ui-dlg-says", { name: other, line: prompt })} ${tr.t("ui-scene-n-choices", { n: v.options.length })}`;
    return "";
  }, [showResult, reacting, words, v, step.kind, tr, other, prompt]);

  const roundLine = v.phase === "under_way" ? tr.t("ui-scene-round", { n: reacting && last ? last.round : v.round, of: v.rounds }) : v.phase === "done" ? tr.t(v.result?.outcome_key ?? "exchange-outcome-split") : v.last_day ? tr.t("ui-scene-on", { date: formatDate(tr, v.last_day) }) : "";

  const head = (
    <header className="dlg-head scn-head">
      <p className="dlg-sub" data-testid="scene-round">
        {roundLine}
      </p>
      <button type="button" className="link-btn scn-mode" onClick={() => setPlain(!plain)} data-testid="scene-plain-toggle" aria-pressed={plain}>
        {tr.t(plain ? "ui-scene-staged" : "ui-scene-plain")}
      </button>
      <button type="button" className="dlg-close" onClick={shut} aria-label={v.phase === "under_way" ? tr.t("ui-scene-leave") : tr.t("ui-close")} data-testid="scene-x">
        ✕
      </button>
    </header>
  );

  const livePara = (
    <p className="visually-hidden" aria-live="polite" data-testid="scene-live">
      {live}
    </p>
  );

  const common = {
    "data-testid": "scene",
    "data-phase": v.phase,
    "data-round": String(v.round),
    "data-mode": plain ? "plain" : "stage",
  };

  if (plain) {
    // Quick mode: one sheet, the same choices as a plain list.
    const shown = v.phase === "under_way" ? v.played : [];
    return (
      <div className="scn-layer scn-plain-layer dlg-layer" {...common}>
        <section ref={boxRef} className="scn-plain" role="dialog" aria-labelledby="scn-title" aria-busy={busy || undefined}>
          <h2 id="scn-title" className="scn-title">
            {title}
          </h2>
          {head}
          {livePara}
          <div className="scn-plain-body">
            <h3 className="scn-label">{tr.t("ui-scene-audience")}</h3>
            {strip}
            {shown.length > 0 && v.played.at(-1) && (
              <div className="scn-plain-said" data-testid="scene-said">
                {(() => {
                  const r = v.played.at(-1) as SceneRoundView;
                  const w = roundWords(tr, v, r, said[r.round]);
                  return (
                    <>
                      {w.said && <p>{tr.t("ui-scene-you-said", { line: w.said })}</p>}
                      <p>
                        <i>{w.room}</i>
                      </p>
                      {w.reply && <p>{tr.t("ui-dlg-says", { name: other, line: w.reply })}</p>}
                    </>
                  );
                })()}
              </div>
            )}
            {v.phase === "under_way" && v.prompt && (
              <p className="scn-plain-prompt" data-testid="scene-prompt">
                <b>{other}:</b> {prompt}
                <WhyButton event={v.prompt.event} inline about={prompt} />
              </p>
            )}
            {list()}
          </div>
        </section>
      </div>
    );
  }

  // The staged scene: the other side and their bubble above the box.
  const bubble = reacting && words ? words.reply : v.phase === "under_way" ? prompt : v.phase === "open" ? null : null;
  const shownText = !reacting && v.phase === "under_way" ? prompt.slice(0, typed) : bubble;
  return (
    <div className="scn-layer dlg-layer dlg-portrait" {...common}>
      <div
        className="scn-stage"
        aria-hidden="true"
        onClick={() => {
          if (typing) setTyped(Infinity);
        }}
      >
        <div className="scn-stage-row">
          <Portrait variant={v.other?.id != null ? lookVariant(`c${v.other.id}`) : 100} className="dlg-face scn-face" />
          <div className="scn-bubbles">
            {shownText && (
              <div className="dlg-bubble dlg-bubble-person dlg-bubble-side scn-bubble" data-testid="scene-bubble">
                <div className="dlg-bubble-body">
                  <p>
                    {shownText}
                    {typing && <span className="dlg-untyped">{prompt.slice(typed)}</span>}
                  </p>
                </div>
              </div>
            )}
            {reacting && words?.said && (
              <div className="dlg-bubble dlg-bubble-player scn-bubble scn-bubble-you" data-testid="scene-you">
                <div className="dlg-bubble-body">
                  <p>{tr.t("ui-scene-you-said", { line: words.said })}</p>
                </div>
              </div>
            )}
          </div>
        </div>
      </div>
      <div className="scn-box-wrap">
        <div className="dlg-plate dlg-frame">
          <h2 className="dlg-name" id="scn-title">
            {other || title}
          </h2>
        </div>
        <section ref={boxRef} className="dlg-box dlg-frame scn-box" role="dialog" aria-label={title} aria-busy={busy || undefined}>
          {head}
          {livePara}
          {strip}
          {v.phase === "under_way" && !reacting && v.prompt && (
            <p className="visually-hidden" data-testid="scene-prompt">
              {other}: {prompt}
            </p>
          )}
          <div className="dlg-body scn-body">
            {v.phase === "under_way" && !reacting && v.prompt?.event != null && step.kind === "list" && (
              <p className="scn-why-prompt">
                <WhyButton event={v.prompt.event} inline about={prompt} />
              </p>
            )}
            {list()}
          </div>
        </section>
      </div>
    </div>
  );
}
