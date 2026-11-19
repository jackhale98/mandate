// Map | List on the town screen (DEC-086, DEC-051): the map is the default;
// the list is the text view of the same places. The choice is remembered per
// device (localStorage, best effort).
import { useCallback, useRef, useState, type KeyboardEvent } from "react";
import { useGame } from "../state/gameStore";
import { say } from "./townMapText";

export type TownViewMode = "map" | "list";

const KEY = "mandate.townView";

function readMode(): TownViewMode {
  try {
    return localStorage.getItem(KEY) === "list" ? "list" : "map";
  } catch {
    return "map";
  }
}

/** The town view the player chose last on this device (the map by default). */
export function useTownViewMode(): [TownViewMode, (m: TownViewMode) => void] {
  const [mode, setMode] = useState<TownViewMode>(readMode);
  const set = useCallback((m: TownViewMode) => {
    setMode(m);
    try {
      localStorage.setItem(KEY, m);
    } catch {
      // Storage blocked: the choice lasts for this visit.
    }
  }, []);
  return [mode, set];
}

export const townPanelId = (m: TownViewMode) => `town-panel-${m}`;
export const townTabId = (m: TownViewMode) => `town-tab-${m}`;

/** The tabs (a WAI-ARIA tab list: arrow keys move between them). */
export function TownViewTabs({ mode, onChange }: { mode: TownViewMode; onChange: (m: TownViewMode) => void }) {
  const tr = useGame((s) => s.tr);
  const refs = useRef<Record<TownViewMode, HTMLButtonElement | null>>({ map: null, list: null });
  const modes: TownViewMode[] = ["map", "list"];
  const onKey = (e: KeyboardEvent) => {
    if (e.key !== "ArrowLeft" && e.key !== "ArrowRight" && e.key !== "Home" && e.key !== "End") return;
    e.preventDefault();
    const next: TownViewMode = e.key === "Home" ? "map" : e.key === "End" ? "list" : mode === "map" ? "list" : "map";
    onChange(next);
    refs.current[next]?.focus();
  };
  return (
    <div className="tabs town-tabs" role="tablist" aria-label={say(tr, "ui-town-view")} onKeyDown={onKey}>
      {modes.map((m) => (
        <button
          key={m}
          ref={(el) => {
            refs.current[m] = el;
          }}
          type="button"
          role="tab"
          id={townTabId(m)}
          aria-selected={mode === m}
          aria-controls={townPanelId(m)}
          tabIndex={mode === m ? 0 : -1}
          className={mode === m ? "tab tab-on" : "tab"}
          onClick={() => onChange(m)}
          data-testid={`town-tab-${m}`}
        >
          <span aria-hidden="true">{m === "map" ? "🗺️ " : "☰ "}</span>
          {say(tr, m === "map" ? "ui-town-view-map" : "ui-town-view-list")}
          {m === "list" && <span className="visually-hidden"> ({say(tr, "ui-town-view-list-hint")})</span>}
        </button>
      ))}
    </div>
  );
}
