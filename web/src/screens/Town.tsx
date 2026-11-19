// The game's main screen (DEC-088 "Main screen", DEC-079, DEC-086): the
// walkable street, full screen. The town map is a full-screen overlay over it
// (the bar's Map button): tapping a place there closes the map and walks the
// player there in the street, and arriving sends the same command as the
// pocket's Places tab (`visit`). The town card lives in the pocket's Town tab
// and the place list in its Places tab: the text path to every place, so
// nothing is reachable only through the canvas (DEC-051). Both pictures come
// from one layout v2 plan made in the town worker (../components/useTownLayout).
// Where the player is (neighbourhood and place) is a label at the top of the
// street (../components/TownStreet), not in the bar.
import { lazy, Suspense, useEffect, useRef, useState, type KeyboardEvent } from "react";
import type { LocationDto } from "../engine/protocol/LocationDto";
import type { TownPlacesView } from "../engine/protocol/TownPlacesView";
import { useGame } from "../state/gameStore";
import { Loader } from "../components/Brand";
import { TownMap } from "../components/TownMap";
import { say } from "../components/townMapText";
import { useTownLayout, type TownLayout } from "../components/useTownLayout";
import type { WalkRequest } from "../components/TownStreet";

// The walkable street: a chunk of its own (the hub renderer and the pixel-art provider).
const TownStreet = lazy(() => import("../components/TownStreet"));

function Pending({ label }: { label: string }) {
  return (
    <div className="town-map-pending" data-testid="town-map-loading">
      <Loader compact label={label} />
    </div>
  );
}

/** The town map over the street (DEC-086): a modal dialog; Escape or ✕ closes it. */
function TownMapOverlay({ town, layout, onPick }: { town: TownPlacesView; layout: TownLayout; onPick: (location: LocationDto) => void }) {
  const tr = useGame((s) => s.tr);
  const setOpen = useGame((s) => s.setTownMapOpen);
  const ref = useRef<HTMLDivElement>(null);
  // Modal: the screen behind is inert while the map is open; the keyboard starts on the close button and returns to the Map button.
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const back = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    const others = [...(el.parentElement?.children ?? [])].filter((c): c is HTMLElement => c !== el && c instanceof HTMLElement && !c.inert);
    for (const o of others) o.inert = true;
    el.querySelector<HTMLElement>("[data-testid='town-map-close']")?.focus();
    return () => {
      for (const o of others) o.inert = false;
      (document.querySelector<HTMLElement>("[data-testid='town-map-button']") ?? back)?.focus();
    };
  }, []);
  const onKey = (e: KeyboardEvent) => {
    if (e.key !== "Escape") return;
    e.preventDefault();
    e.stopPropagation();
    setOpen(false);
  };
  return (
    <div ref={ref} className="town-map-overlay" role="dialog" aria-modal="true" aria-labelledby="town-map-title" data-testid="town-map-overlay" onKeyDown={onKey}>
      <div className="town-map-overlay-head">
        <h2 id="town-map-title">
          {town.municipality} <span className="muted small">· {say(tr, "ui-town-map-pick")}</span>
        </h2>
        <button type="button" className="btn btn-ghost" onClick={() => setOpen(false)} aria-label={say(tr, "ui-town-map-close")} data-testid="town-map-close">
          ✕
        </button>
      </div>
      <div className="town-map-overlay-body">
        {layout.status === "ready" ? (
          <TownMap town={town} map={layout.map} onPick={(h) => onPick(h.location)} />
        ) : layout.status === "loading" ? (
          <Pending label={say(tr, "ui-town-planning")} />
        ) : (
          <p className="error small" role="alert">
            {layout.message}
          </p>
        )}
      </div>
    </div>
  );
}

function TownScreen({ town }: { town: TownPlacesView }) {
  const tr = useGame((s) => s.tr);
  const layout = useTownLayout(town);
  const mapOpen = useGame((s) => s.townMapOpen);
  const setMapOpen = useGame((s) => s.setTownMapOpen);
  const [walk, setWalk] = useState<WalkRequest | null>(null);
  return (
    <>
      <main id="town-main" className="town-screen" data-testid="town" aria-labelledby="town-heading">
        <h1 id="town-heading" className="visually-hidden">
          {town.municipality}
        </h1>
        {layout.status === "ready" ? (
          <Suspense fallback={<Pending label={say(tr, "ui-town-street-loading")} />}>
            <TownStreet town={town} map={layout.map} walkTo={walk} />
          </Suspense>
        ) : layout.status === "loading" ? (
          <Pending label={say(tr, "ui-town-planning")} />
        ) : (
          <p className="error small town-error" role="alert" data-testid="town-map-error">
            {layout.message}
          </p>
        )}

      </main>
      {mapOpen && (
        <TownMapOverlay
          town={town}
          layout={layout}
          onPick={(location) => {
            setMapOpen(false);
            setWalk((w) => ({ location, id: (w?.id ?? 0) + 1 }));
          }}
        />
      )}
    </>
  );
}

export function Town() {
  const town = useGame((s) => s.town);
  if (!town) {
    return (
      <main className="town-screen" data-testid="town-loading">
        <div className="center">
          <Loader compact />
        </div>
      </main>
    );
  }
  return <TownScreen town={town} />;
}
