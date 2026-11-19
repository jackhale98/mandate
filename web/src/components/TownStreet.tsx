// The walkable town (DEC-079, DEC-080, DEC-088): the town map zoomed down
// into its streets, drawn by the hub renderer (../hub/hubView) in the
// pixel-art pack (DEC-084). Tap a building and the player walks there along
// the streets; arriving sends the same command the List view's button does
// (`visit`: move there, free, and open its panel). Townsfolk walk about, and
// the light follows the turn's slots like the town map's (../hub/turnClock):
// it holds still between actions and eases forward when one is done.
// Presentation only: the engine moves the player and says what a place offers.
// Loaded on demand (the renderer and the art are a chunk of their own); the
// pack itself is built in the town worker.
import { useEffect, useRef, useState } from "react";
import type { LocationDto } from "../engine/protocol/LocationDto";
import type { TownPlacesView } from "../engine/protocol/TownPlacesView";
import { DayClock } from "../hub/dayCycle";
import { HubView } from "../hub/hubView";
import { PixelkitArt } from "../hub/render/pixelkitArt";
import { locationCell, type TownBuilding, type TownMap } from "../hub/townLayout";
import { LIGHT_EASE_MS, PASSAGE_EASE_MS, shownMinutes } from "../hub/turnClock";
import { artPack } from "../hub/townWorkerClient";
import { Loader } from "./Brand";
import { useGame } from "../state/gameStore";
import { sameLocation } from "../state/routing";
import { say } from "./townMapText";

export interface TownStreetProps {
  town: TownPlacesView;
  /** The dressed layout v2 map (the same the town map draws). */
  map: TownMap;
  /** Where to look first (tiles), or null: at the player. */
  at: { x: number; y: number } | null;
  onBack: () => void;
}

/** What tests see (`window.__townStreet`). */
interface StreetProbe {
  view: HubView;
  /** The time of day drawn now, and where it is going (minutes). */
  readonly minutes: number;
  readonly target: number;
  /** Where the player stands in the engine's terms when the walk ends (the last arrival), for tests. */
  arrivals: number;
}
declare global {
  interface Window {
    __townStreet?: StreetProbe | null;
  }
}

const reducedMotion = () => typeof matchMedia === "function" && matchMedia("(prefers-reduced-motion: reduce)").matches;

export default function TownStreet({ town, map, at, onBack }: TownStreetProps) {
  const tr = useGame((s) => s.tr);
  const pocket = useGame((s) => s.pocket);
  const busy = useGame((s) => s.busy);
  const visit = useGame((s) => s.visit);
  const command = useGame((s) => s.command);
  const walking = useGame((s) => s.walking);
  const passage = useGame((s) => s.passage?.phase ?? null);
  const covered = useGame((s) => s.openPlace !== null || s.character !== null || s.decision !== null || s.story !== null);
  const stageRef = useRef<HTMLDivElement>(null);
  const viewRef = useRef<HubView | null>(null);
  const artRef = useRef<PixelkitArt | null>(null);
  const [ready, setReady] = useState(false);
  const [failed, setFailed] = useState<string | null>(null);
  const [status, setStatus] = useState<string | null>(null);
  // The turn's time of day, eased forward when a slot is spent; night and the new morning when a turn ends.
  const minutes = shownMinutes(pocket, passage) ?? 12 * 60;
  const easeMs = passage ? PASSAGE_EASE_MS : LIGHT_EASE_MS;
  const here = pocket?.location.location ?? town.location;

  // The latest values the view's callbacks read (it is made once per map).
  const latest = useRef({ minutes, here, at, busy });
  const arrive = useRef<(b: TownBuilding | null) => void>(() => {});
  useEffect(() => {
    latest.current = { minutes, here, at, busy };
    arrive.current = (b) => {
      const v = viewRef.current;
      if (!v || latest.current.busy) return;
      let loc: LocationDto | null = null;
      if (b?.home) loc = { type: "home" };
      else if (b && b.place !== null) loc = { type: "place", place: b.place };
      else {
        // Open ground or a house: the streets of the neighbourhood the player is in.
        const d = v.map.districts[v.map.district[v.scene.playerCell] ?? -1];
        if (d) loc = { type: "street", neighborhood: d.id };
      }
      if (!loc) return;
      const probe = window.__townStreet;
      if (probe) probe.arrivals++;
      if (loc.type === "street") {
        setStatus(null);
        if (!sameLocation(loc, latest.current.here)) void command({ type: "move_to", location: loc });
        return;
      }
      // As on the map and in the list: a place with nothing to do yet says so.
      const spot = town.neighborhoods.flatMap((h) => h.spots).find((s) => loc && sameLocation(s.location.location, loc));
      if (spot && spot.actions.length + spot.services.length === 0) {
        setStatus(say(tr, "ui-town-map-quiet", { place: spot.location.name || tr.t(`place-${spot.location.kind_key}`) }));
        return;
      }
      setStatus(null);
      void visit(loc);
    };
  });

  useEffect(() => {
    const stage = stageRef.current;
    if (!stage) return;
    let live = true;
    let view: HubView | null = null;
    artPack()
      .then((pack) => {
        if (!live) return;
        const { minutes: m, here: loc, at: look } = latest.current;
        const art = new PixelkitArt(pack, { night: "moods" });
        art.setClock(m);
        if (!document.getElementById("hub-icon-sprite")) document.body.insertAdjacentHTML("afterbegin", art.iconSprite().replace("<svg ", '<svg id="hub-icon-sprite" '));
        view = new HubView(stage, {
          map: { ...map, start: locationCell(map, loc) ?? map.start },
          art,
          label: (b) => (b.home ? tr.t("place-home") : b.place !== null ? b.name || tr.t(`place-${b.kind}`) : null),
          labelIcon: (b) => (b.place !== null || b.home ? art.icon(b.home ? "home" : b.kind) : null),
          townsfolk: 18,
          clock: new DayClock(m, 0, false),
          reducedMotion: reducedMotion(),
          canvasLabel: say(tr, "ui-town-street-label", { town: town.municipality }),
          onArrive: (b) => arrive.current(b),
        });
        if (look) view.centreOn(look.x, look.y);
        viewRef.current = view;
        artRef.current = art;
        window.__townStreet = {
          view,
          get minutes() {
            return view?.clock.minutes ?? 0;
          },
          get target() {
            return view?.clock.target ?? 0;
          },
          arrivals: 0,
        };
        setReady(true);
      })
      .catch((e: unknown) => live && setFailed(e instanceof Error ? e.message : String(e)));
    return () => {
      live = false;
      view?.destroy();
      viewRef.current = null;
      artRef.current = null;
      window.__townStreet = null;
      setReady(false);
    };
    // The view is made once per map; it reads the rest through `latest`.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [map]);

  // The light: ease forward to the turn's time (instant with reduced motion); repaint the moods once, at the target.
  useEffect(() => {
    const v = viewRef.current;
    const art = artRef.current;
    if (!ready || !v || !art) return;
    if (Math.abs(v.clock.target - minutes) < 0.5) return;
    const quick = reducedMotion();
    v.easeClockTo(minutes, quick ? 0 : easeMs);
    if (art.setClock(minutes)) {
      v.renderer.crossFade(quick ? 0 : Math.min(900, easeMs));
      v.wake();
    }
  }, [minutes, easeMs, ready]);

  // Nothing animates behind a sheet.
  useEffect(() => {
    if (viewRef.current) viewRef.current.paused = covered;
  }, [covered, ready]);

  // An auto-routed action (the agenda) walks the player here too; the engine does the moving.
  useEffect(() => {
    if (walking && ready) viewRef.current?.goToLocation(walking.location, false);
  }, [walking, ready]);

  // The engine moved the player some other way (a skipped walk, an action elsewhere): stand there.
  const hereKey = JSON.stringify(here);
  useEffect(() => {
    const v = viewRef.current;
    if (!v || !ready || v.scene.playerMoving) return;
    const cell = locationCell(v.map, JSON.parse(hereKey) as LocationDto);
    if (cell !== null && cell !== v.scene.playerCell) {
      v.scene.placePlayer(cell);
      v.wake();
    }
  }, [hereKey, ready]);

  return (
    <div className="town-street" data-testid="town-street">
      <div className="town-street-frame">
        <div ref={stageRef} className="town-street-stage" role="group" aria-roledescription={say(tr, "ui-town-street")} aria-label={town.municipality} aria-describedby="town-street-help" data-ready={ready ? "1" : "0"} />
        {!ready && (
          <div className="town-street-loading">
            {failed ? <p className="muted small">{failed}</p> : <Loader compact label={say(tr, "ui-town-street-loading")} />}
          </div>
        )}
        <button type="button" className="btn btn-sm town-map-walk" onClick={onBack} data-testid="town-walk-back">
          <span aria-hidden="true">🗺️ </span>
          {say(tr, "ui-town-walk-back")}
        </button>
      </div>
      <p className="town-map-status small" role="status" data-testid="town-street-status">
        {status}
      </p>
      <p id="town-street-help" className="muted small town-map-help">
        {say(tr, "ui-town-street-help")}
      </p>
    </div>
  );
}
