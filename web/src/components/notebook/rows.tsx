// Rows, folds and sections: the pieces the pocket is built from (DEC-113 §3).
// A person, a place, an ask, an arc and a bill are each a row of two or three
// short lines that opens in place (or opens a sheet). A folded section is one
// row with its name and a count of what is inside, never a share. A list is
// one Tab stop: Up and Down move between its rows, Enter opens or closes one,
// Tab then goes into the opened row (§4.2). Presentation only: what is in a
// row, its order and its count come from the engine's views.
import { useEffect, useRef, useState, type FocusEvent, type KeyboardEvent, type ReactNode } from "react";
import type { DateView } from "../../engine/protocol/DateView";
import type { Translator } from "../../i18n/fluent";
import { useGame } from "../../state/gameStore";
import "./notebook.css";

/** A list of rows: one Tab stop, arrows move between rows (roving focus). */
export function RowList({ children, testId, label, className = "" }: { children: ReactNode; testId?: string | undefined; label?: string | undefined; className?: string | undefined }) {
  const ref = useRef<HTMLUListElement>(null);
  const mains = () => [...(ref.current?.querySelectorAll<HTMLElement>(":scope > li > .rw-row > .rw-main") ?? [])];
  // Exactly one row of the list is in the Tab order: the last one focused, else the first.
  useEffect(() => {
    const all = mains();
    if (all.length > 0 && !all.some((b) => b.tabIndex === 0)) (all[0] as HTMLElement).tabIndex = 0;
  });
  const onFocus = (e: FocusEvent) => {
    const all = mains();
    if (!all.includes(e.target as HTMLElement)) return;
    for (const b of all) b.tabIndex = b === e.target ? 0 : -1;
  };
  const onKey = (e: KeyboardEvent) => {
    const all = mains();
    const i = all.indexOf(document.activeElement as HTMLElement);
    if (i < 0) return;
    const n = all.length;
    const next = e.key === "ArrowDown" ? all[Math.min(n - 1, i + 1)] : e.key === "ArrowUp" ? all[Math.max(0, i - 1)] : e.key === "Home" ? all[0] : e.key === "End" ? all[n - 1] : undefined;
    if (!next) return;
    e.preventDefault();
    next.focus();
  };
  return (
    <ul ref={ref} className={`rw-rows ${className}`.trim()} data-testid={testId} aria-label={label} onFocus={onFocus} onKeyDown={onKey}>
      {children}
    </ul>
  );
}

export interface RowProps {
  /** An icon or a portrait before the lines (decorative). */
  mark?: ReactNode | undefined;
  title: ReactNode;
  /** Beside the title, muted (a role, a place). */
  role?: ReactNode | undefined;
  /** The second line. */
  sub?: ReactNode | undefined;
  /** A third line: why it matters now. */
  reason?: ReactNode | undefined;
  /** After the lines: when, and one action. Hidden while the row is open (the opened row has the full actions). */
  end?: ReactNode | undefined;
  /** The accessible name of the row (its lines in a sentence). */
  label?: string | undefined;
  /** Opens in place: the opened part is `children`. */
  open?: boolean | undefined;
  onToggle?: (() => void) | undefined;
  /** Opens something else (a sheet) instead of opening in place. */
  onActivate?: (() => void) | undefined;
  /** The row's main button's test id. */
  mainTestId?: string | undefined;
  children?: ReactNode | undefined;
  className?: string | undefined;
  li?: Record<string, string | undefined> | undefined;
  disabled?: boolean | undefined;
}

/** One row: its lines are one button (the whole row is the target); an action may sit at its end. */
export function Row({ mark, title, role, sub, reason, end, label, open, onToggle, onActivate, mainTestId, children, className = "", li = {}, disabled }: RowProps) {
  const expands = onToggle !== undefined;
  return (
    <li className={["rw-li", open ? "rw-li-open" : "", className].filter(Boolean).join(" ")} {...li}>
      <div className="rw-row">
        {mark !== undefined && (
          <span className="rw-mark" aria-hidden="true">
            {mark}
          </span>
        )}
        <button
          type="button"
          className="rw-main"
          tabIndex={-1}
          aria-expanded={expands ? open === true : undefined}
          aria-label={label}
          disabled={disabled}
          onClick={() => (expands ? onToggle?.() : onActivate?.())}
          data-testid={mainTestId}
        >
          <span className="rw-l1">
            <strong className="rw-title">{title}</strong>
            {role && <span className="rw-role">{role}</span>}
          </span>
          {sub && <span className="rw-l2">{sub}</span>}
          {reason && <span className="rw-l3">{reason}</span>}
        </button>
        {end && !open && <span className="rw-end">{end}</span>}
      </div>
      {open && children && <div className="rw-open">{children}</div>}
    </li>
  );
}

/** "5 days" over "by 14 Mar": when something is due, for a row's end. */
export function When({ top, under }: { top: string; under?: string | null }) {
  return (
    <span className="rw-when">
      {top}
      {under && <small>{under}</small>}
    </span>
  );
}

/** Whether a fold is open: what the player chose this session, else its default. */
export function useFold(key: string, byDefault: boolean): [boolean, (open: boolean) => void] {
  const chosen = useGame((s) => s.folds[key]);
  const setFold = useGame((s) => s.setFold);
  return [chosen ?? byDefault, (open: boolean) => setFold(key, open)];
}

/**
 * A section that folds to one row with its name and a count of what is inside
 * (DEC-113 §3.5). Open ones stay open for the session. `count` null: no count
 * (the agenda, the standing).
 */
export function Fold({ id, title, count, countText, hint, open: byDefault = false, children, testId, headTestId }: { id: string; title: string; count: number | null; countText?: string; hint?: ReactNode; open?: boolean; children: ReactNode; testId?: string; headTestId?: string }) {
  const [open, setOpen] = useFold(id, byDefault);
  const bodyId = `fold-${id}`;
  return (
    <section className={open ? "rw-fold rw-fold-open" : "rw-fold"} data-testid={testId} data-fold={id}>
      <h3 className="rw-fold-h">
        <button type="button" className="rw-fold-head" aria-expanded={open} aria-controls={bodyId} onClick={() => setOpen(!open)} data-testid={headTestId ?? `fold-head-${id}`}>
          <span className="rw-fold-title">
            <span className="rw-fold-name">{title}</span>
            {hint && <small>{hint}</small>}
          </span>
          {count !== null && (
            <span className="rw-count">
              {count}
              {countText && <span className="visually-hidden">, {countText}</span>}
            </span>
          )}
          <span className="rw-chev" aria-hidden="true">
            {open ? "⌄" : "›"}
          </span>
        </button>
      </h3>
      {open && (
        <div className="rw-fold-body" id={bodyId}>
          {children}
        </div>
      )}
    </section>
  );
}

/** A section with a heading (open, never folded): "Needs you", "Under way", "Matter now". */
export function Section({ title, children, testId, id }: { title: string; children: ReactNode; testId?: string; id: string }) {
  return (
    <section className="rw-sec" aria-labelledby={id} data-testid={testId}>
      <h3 className="rw-h" id={id}>
        {title}
      </h3>
      {children}
    </section>
  );
}

/** The first `show` rows, then "Show the other N" (DEC-113 §3.5: the engine says how many a section shows). */
export function useShowMore<T>(rows: readonly T[], show: number): { shown: readonly T[]; more: number; showAll: () => void } {
  const [all, setAll] = useState(false);
  const shown = all ? rows : rows.slice(0, show);
  return { shown, more: rows.length - shown.length, showAll: () => setAll(true) };
}

export function ShowMore({ more, onClick, testId }: { more: number; onClick: () => void; testId?: string }) {
  const tr = useGame((s) => s.tr);
  if (more <= 0) return null;
  return (
    <button type="button" className="btn btn-ghost rw-more-btn" onClick={onClick} data-testid={testId ?? "show-more"}>
      {tr.t("ui-show-the-other", { n: more })}
    </button>
  );
}

/** "14 Mar": a date in a row's end. */
export function shortDate(tr: Translator, d: DateView): string {
  return tr.t("ui-date-short", { day: d.day, month: tr.t(`month-short-${d.month}`) });
}

/** The stance as a sign and a short word (DEC-051: never colour alone): + warm, = neutral, − cool. Not arrows: ▲
 *  and ▼ mean a movement elsewhere (a debate's lines, the stat tiles), and a person's stance is a level, not a trend
 *  (owner, 2026-11-25). Not dots, which read as the knowledge meter the Notebook no longer has. */
export function stanceMark(shape: string): string {
  return shape === "up" ? "+" : shape === "down" ? "−" : "=";
}
