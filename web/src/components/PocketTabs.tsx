// The pocket's switch (DEC-113): Today and the Notebook, each with its count
// (what needs you, the engine's `needs_count`; what is new in the notebook).
// On a phone it is a WAI-ARIA tab list of two (arrow keys, Home and End move
// between them; the open side is the one in the Tab order). On a wide screen
// both sides are open at once (owner's answer 4), so it becomes two buttons,
// each showing its own panel: Today stays on the right, Notebook opens (or
// reopens) the panel on the left. `data-on` says which is showing either way.
// The side is remembered per device (../state/devicePrefs).
import { useRef, type KeyboardEvent } from "react";
import type { PocketView } from "../engine/protocol/PocketView";
import { POCKET_TABS, type PocketSide } from "../state/devicePrefs";
import { useGame } from "../state/gameStore";

export const pocketTabId = (t: PocketSide) => `pocket-tab-${t}`;
export const pocketPanelId = (t: PocketSide) => `pocket-panel-${t}`;

export function PocketTabs({ side, onChange, p, wide, bookOpen }: { side: PocketSide; onChange: (t: PocketSide) => void; p: PocketView; wide: boolean; bookOpen: boolean }) {
  const tr = useGame((s) => s.tr);
  const refs = useRef<Partial<Record<PocketSide, HTMLButtonElement | null>>>({});
  const badge: Record<PocketSide, { n: number; text: string; icon: string }> = {
    today: { n: p.needs_count, text: tr.t("ui-needs-count", { n: p.needs_count }), icon: "" },
    notebook: { n: p.notebook_new, text: tr.t("ui-notebook-new-count", { n: p.notebook_new }), icon: "✎ " },
  };
  const onKey = (e: KeyboardEvent) => {
    if (wide) return;
    const i = POCKET_TABS.indexOf(side);
    const n = POCKET_TABS.length;
    const next =
      e.key === "ArrowRight" ? POCKET_TABS[(i + 1) % n] : e.key === "ArrowLeft" ? POCKET_TABS[(i + n - 1) % n] : e.key === "Home" ? POCKET_TABS[0] : e.key === "End" ? POCKET_TABS[n - 1] : undefined;
    if (!next) return;
    e.preventDefault();
    onChange(next);
    refs.current[next]?.focus();
  };
  return (
    <div className="pocket-switch" role={wide ? "group" : "tablist"} aria-label={tr.t("ui-pocket-tabs")} onKeyDown={onKey}>
      {POCKET_TABS.map((t) => {
        const b = badge[t];
        const on = wide ? (t === "today" ? true : bookOpen) : side === t;
        const tabProps = wide
          ? { "aria-expanded": t === "notebook" ? bookOpen : undefined, "aria-controls": t === "notebook" ? "pocket-book" : undefined }
          : { role: "tab", "aria-selected": side === t, "aria-controls": pocketPanelId(t), tabIndex: side === t ? 0 : -1 };
        return (
          <button
            key={t}
            ref={(el) => {
              refs.current[t] = el;
            }}
            type="button"
            id={pocketTabId(t)}
            {...tabProps}
            className="pocket-sw"
            data-on={on ? "true" : "false"}
            onClick={() => onChange(t)}
            data-testid={pocketTabId(t)}
          >
            <span className="pocket-sw-name">{tr.t(`ui-pocket-tab-${t}`)}</span>
            {b.n > 0 && (
              <>
                <span className="badge" aria-hidden="true" data-testid={`pocket-tab-badge-${t}`}>
                  {b.icon}
                  {b.n}
                </span>
                <span className="visually-hidden">, {b.text}</span>
              </>
            )}
          </button>
        );
      })}
    </div>
  );
}
