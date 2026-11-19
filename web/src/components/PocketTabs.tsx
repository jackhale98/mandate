// The pocket's tabs (DEC-088 main screen): Today, Town, Places, People. A
// WAI-ARIA tab list (arrow keys, Home and End move between tabs; the open
// tab is the one in the Tab order). Badges count what waits on a tab and say
// so in words for screen readers. The choice is remembered per device
// (../state/devicePrefs).
import { useRef, type KeyboardEvent } from "react";
import { POCKET_TABS, type PocketTab } from "../state/devicePrefs";
import { useGame } from "../state/gameStore";

export const pocketTabId = (t: PocketTab) => `pocket-tab-${t}`;
export const pocketPanelId = (t: PocketTab) => `pocket-panel-${t}`;

const ICON: Record<PocketTab, string> = { today: "📅", town: "🏛️", places: "📍", people: "👥" };

/** A count on a tab, with what it counts in words (read after the tab's name). */
export interface TabBadge {
  n: number;
  text: string;
}

export function PocketTabs({ tab, onChange, badges }: { tab: PocketTab; onChange: (t: PocketTab) => void; badges: Partial<Record<PocketTab, TabBadge>> }) {
  const tr = useGame((s) => s.tr);
  const refs = useRef<Partial<Record<PocketTab, HTMLButtonElement | null>>>({});
  const onKey = (e: KeyboardEvent) => {
    const i = POCKET_TABS.indexOf(tab);
    const n = POCKET_TABS.length;
    const next =
      e.key === "ArrowRight" ? POCKET_TABS[(i + 1) % n] : e.key === "ArrowLeft" ? POCKET_TABS[(i + n - 1) % n] : e.key === "Home" ? POCKET_TABS[0] : e.key === "End" ? POCKET_TABS[n - 1] : undefined;
    if (!next) return;
    e.preventDefault();
    onChange(next);
    refs.current[next]?.focus();
  };
  return (
    <div className="tabs pocket-tabs" role="tablist" aria-label={tr.t("ui-pocket-tabs")} onKeyDown={onKey}>
      {POCKET_TABS.map((t) => {
        const b = badges[t];
        return (
          <button
            key={t}
            ref={(el) => {
              refs.current[t] = el;
            }}
            type="button"
            role="tab"
            id={pocketTabId(t)}
            aria-selected={tab === t}
            aria-controls={pocketPanelId(t)}
            tabIndex={tab === t ? 0 : -1}
            className={tab === t ? "tab tab-on pocket-tab" : "tab pocket-tab"}
            onClick={() => onChange(t)}
            data-testid={`pocket-tab-${t}`}
          >
            <span className="pocket-tab-icon" aria-hidden="true">
              {ICON[t]}
            </span>
            <span className="pocket-tab-name">{tr.t(`ui-pocket-tab-${t}`)}</span>
            {b && b.n > 0 && (
              <>
                <span className="badge pocket-tab-badge" aria-hidden="true" data-testid={`pocket-tab-badge-${t}`}>
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
