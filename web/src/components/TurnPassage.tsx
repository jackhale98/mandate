// The night between two turns (owner, 2026-11-19): ending a turn eases the
// town into night (lamps and windows on, the streets emptying), a small
// card shows the new date and how long the turn was, then dawn brings the
// new turn's morning. The engine runs the turn meanwhile: the card keeps
// the loader until it answers, and the passage lasts at least about two
// seconds. Once the engine has answered, a tap, a click or a key skips to
// the end. With reduced motion it is a quick fade of the card. The town map
// and the street read the phase from the store (`passage`) for their light;
// the List view gets the card alone. Whatever opens after a turn waits for
// it (`afterPassage`). The date is announced (a live region); focus stays
// where it was.
import { useEffect, useRef } from "react";
import { formatDay } from "../i18n/format";
import type { Translator } from "../i18n/fluent";
import { useGame, type Passage } from "../state/gameStore";
import { Loader } from "./Brand";

/** Phase lengths, ms: [dusk, the least time to dawn from the start, dawn]; quick: reduced motion. */
export const PASSAGE_MS = { dusk: 800, night: 1300, dawn: 800 };
export const PASSAGE_QUICK_MS = { dusk: 120, night: 450, dawn: 200 };

const reducedMotion = () => typeof matchMedia === "function" && matchMedia("(prefers-reduced-motion: reduce)").matches;

/** How long the turn was, in words ("A week later"): the engine's `turn_days`, formatted. */
export function passageSpan(tr: Translator, days: number): string {
  if (days === 1) return tr.t("ui-passage-day");
  if (days === 7) return tr.t("ui-passage-week");
  if (days > 7 && days % 7 === 0) return tr.t("ui-passage-weeks", { n: days / 7 });
  return tr.t("ui-passage-days", { n: days });
}

export function TurnPassage() {
  const tr = useGame((s) => s.tr);
  const passage = useGame((s) => s.passage);
  const setPhase = useGame((s) => s.setPassagePhase);
  const finish = useGame((s) => s.finishPassage);
  const started = useRef<{ id: number; at: number } | null>(null);
  const quick = reducedMotion();

  const phase = passage?.phase ?? null;
  const answered = passage?.to != null;
  const id = passage?.id ?? null;

  // The timeline: dusk → night (held until the engine answers and the least time has passed) → dawn → done.
  useEffect(() => {
    if (id === null || phase === null) return;
    if (started.current?.id !== id) started.current = { id, at: performance.now() };
    const s = started.current;
    const ms = quick ? PASSAGE_QUICK_MS : PASSAGE_MS;
    let t = 0;
    if (phase === "dusk") t = window.setTimeout(() => setPhase("night"), ms.dusk);
    else if (phase === "night" && answered) t = window.setTimeout(() => setPhase("dawn"), Math.max(0, ms.dusk + ms.night - (performance.now() - s.at)));
    else if (phase === "dawn") t = window.setTimeout(finish, ms.dawn);
    return () => window.clearTimeout(t);
  }, [id, phase, answered, quick, setPhase, finish]);

  // A tap, a click or a key skips to the end once the engine has answered (Tab and modifiers keep their jobs).
  useEffect(() => {
    if (id === null || !answered) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Tab" || e.key === "Shift" || e.key === "Control" || e.key === "Alt" || e.key === "Meta") return;
      finish();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [id, answered, finish]);

  if (!passage) return null;
  return (
    <div
      className={`turn-passage turn-passage-${passage.phase}${quick ? " turn-passage-quick" : ""}`}
      data-testid="turn-passage"
      data-phase={passage.phase}
      data-quick={quick ? "1" : "0"}
      onClick={() => answered && finish()}
    >
      <div className="turn-passage-card" role="status" aria-live="polite" aria-atomic="true">
        <PassageCard p={passage} tr={tr} />
      </div>
    </div>
  );
}

function PassageCard({ p, tr }: { p: Passage; tr: Translator }) {
  if (!p.to) {
    return (
      <>
        <span className="turn-passage-date">{formatDay(tr, p.from, p.fromWeekday)}</span>
        <Loader compact decorative />
        <span className="visually-hidden">{tr.t("ui-passage-working")}</span>
      </>
    );
  }
  return (
    <>
      <span className="turn-passage-span">{passageSpan(tr, p.days)}</span>
      {/* What the fortnight held (DEC-114): the engine's lines, with their dates. */}
      {p.fortnight.length > 0 && (
        <ul className="turn-passage-fortnight small" aria-label={tr.t("ui-passage-fortnight")} data-testid="turn-passage-fortnight">
          {p.fortnight.map((e) => (
            <li key={e.id}>{tr.event(e.text_key, e.args)}</li>
          ))}
        </ul>
      )}
      <strong className="turn-passage-date" data-testid="turn-passage-date">
        {formatDay(tr, p.to, p.toWeekday)}
      </strong>
      <span className="muted small turn-passage-skip">{tr.t("ui-passage-skip")}</span>
    </>
  );
}
