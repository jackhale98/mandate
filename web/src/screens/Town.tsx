// The game's main screen (DEC-088 "Main screen", DEC-079, DEC-086): the
// walkable street, full screen. The map is a full-screen overlay over it
// (the bar's Map button) with Town | Country at its top: on the town map,
// tapping a place closes the map and walks the player there in the street,
// and arriving sends the same command as the pocket's Places tab (`visit`);
// on the country map (DEC-021), tapping a town opens its panel (travel
// between towns is M7). The town card lives in the pocket's Town tab
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
import { CountryMap } from "../components/CountryMap";
import { MunicipalityPanel } from "../components/MunicipalityPanel";
import type { MapScope } from "../state/gameStore";
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

const SCOPES: readonly MapScope[] = ["town", "country"];

/** Town | Country: which map the overlay shows (a tab list; arrows, Home and End move between them). */
function ScopeTabs({ scope, onChange }: { scope: MapScope; onChange: (s: MapScope) => void }) {
  const tr = useGame((s) => s.tr);
  const refs = useRef<Partial<Record<MapScope, HTMLButtonElement | null>>>({});
  const onKey = (e: KeyboardEvent) => {
    const i = SCOPES.indexOf(scope);
    const next = e.key === "ArrowRight" ? SCOPES[(i + 1) % SCOPES.length] : e.key === "ArrowLeft" ? SCOPES[(i + SCOPES.length - 1) % SCOPES.length] : e.key === "Home" ? SCOPES[0] : e.key === "End" ? SCOPES[SCOPES.length - 1] : undefined;
    if (!next) return;
    e.preventDefault();
    onChange(next);
    refs.current[next]?.focus();
  };
  return (
    <div className="segmented segmented-sm map-scope" role="tablist" aria-label={tr.t("ui-map-scope")} onKeyDown={onKey}>
      {SCOPES.map((s) => (
        <button
          key={s}
          ref={(el) => {
            refs.current[s] = el;
          }}
          type="button"
          role="tab"
          id={`map-scope-${s}`}
          aria-selected={scope === s}
          aria-controls="map-scope-panel"
          tabIndex={scope === s ? 0 : -1}
          className={scope === s ? "seg seg-on" : "seg"}
          onClick={() => onChange(s)}
          data-testid={`map-scope-${s}`}
        >
          {tr.t(`ui-map-scope-${s}`)}
        </button>
      ))}
    </div>
  );
}

/**
 * The map overlay over the street: a modal dialog with the town map (DEC-086)
 * or the country's (DEC-021), chosen at its top and remembered per device;
 * Escape or ✕ closes it (Escape closes an open town's panel first).
 */
function TownMapOverlay({ town, layout, onPick }: { town: TownPlacesView; layout: TownLayout; onPick: (location: LocationDto) => void }) {
  const tr = useGame((s) => s.tr);
  const setOpen = useGame((s) => s.setTownMapOpen);
  const scope = useGame((s) => s.mapScope);
  const setScope = useGame((s) => s.setMapScope);
  const [selected, setSelected] = useState<number | null>(null);
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
    // A town's panel open over the country map closes first (the sheets' own Escape stands aside for a modal).
    if (scope === "country" && selected !== null) setSelected(null);
    else setOpen(false);
  };
  return (
    <div ref={ref} className="town-map-overlay" role="dialog" aria-modal="true" aria-labelledby="town-map-title" data-testid="town-map-overlay" data-scope={scope} onKeyDown={onKey}>
      <div className="town-map-overlay-head">
        <h2 id="town-map-title">
          {scope === "town" ? (
            <>
              {town.municipality} <span className="muted small">· {tr.t("ui-town-map-pick")}</span>
            </>
          ) : (
            <>
              {tr.t("ui-map-country-title")} <span className="muted small">· {tr.t("ui-map-country-pick")}</span>
            </>
          )}
        </h2>
        <ScopeTabs scope={scope} onChange={setScope} />
        <button type="button" className="btn btn-ghost" onClick={() => setOpen(false)} aria-label={tr.t("ui-town-map-close")} data-testid="town-map-close">
          ✕
        </button>
      </div>
      <div className="town-map-overlay-body" role="tabpanel" id="map-scope-panel" aria-labelledby={`map-scope-${scope}`}>
        {scope === "country" ? (
          <CountryMap selected={selected} onSelect={setSelected} />
        ) : layout.status === "ready" ? (
          <TownMap town={town} map={layout.map} onPick={(h) => onPick(h.location)} />
        ) : layout.status === "loading" ? (
          <Pending label={tr.t("ui-town-planning")} />
        ) : (
          <p className="error small" role="alert">
            {layout.message}
          </p>
        )}
      </div>
      {scope === "country" && selected !== null && <MunicipalityPanel id={selected} onClose={() => setSelected(null)} />}
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
          <Suspense fallback={<Pending label={tr.t("ui-town-street-loading")} />}>
            <TownStreet town={town} map={layout.map} walkTo={walk} />
          </Suspense>
        ) : layout.status === "loading" ? (
          <Pending label={tr.t("ui-town-planning")} />
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
