// The walkable town (DEC-079, DEC-080, DEC-088): the game's main screen, the
// town's streets full screen, drawn by the hub renderer (../hub/hubView) in
// the pixel-art pack (DEC-084). Tap a building, or a place on the town map
// overlay (`walkTo`), and the player walks there along the streets; arriving
// sends the same command as the pocket's Places tab (`visit`: move there,
// free, and open its panel). With reduced motion there is no walk: the
// player is there at once. Townsfolk walk about, and
// the light follows the turn's slots like the town map's (../hub/turnClock):
// it holds still between actions and eases forward when one is done.
// Presentation only: the engine moves the player and says what a place offers.
// A place with an interior (DEC-088) is walked into from its door (a tap, or
// Enter at it) instead of opening its panel (../interior); the town map, like
// the Places tab, still opens the panel (DEC-051: nothing only by walking).
// Loaded on demand (the renderer and the art are a chunk of their own); the
// pack itself is built in the town worker.
import { useEffect, useRef, useState } from "react";
import type { LocationDto } from "../engine/protocol/LocationDto";
import type { TownPlacesView } from "../engine/protocol/TownPlacesView";
import { DayClock } from "../hub/dayCycle";
import { HubView } from "../hub/hubView";
import { PixelkitArt } from "../hub/render/pixelkitArt";
import { atLocation, locationCell, type TownBuilding, type TownMap } from "../hub/townLayout";
import { LIGHT_EASE_MS, PASSAGE_EASE_MS, shownMinutes } from "../hub/turnClock";
import { artPack } from "../hub/townWorkerClient";
import { Loader } from "./Brand";
import { placeName } from "../places/labels";
import { useGame } from "../state/gameStore";
import { sameLocation } from "../state/routing";
import { useSheetsCover } from "./sheetCover";
import { streetPeople } from "./talk/streetPeople";
import { Interior } from "../interior/Interior";
import { enterable } from "../interior/recipes";

/** Walk to a place picked on the town map; a new `id` is a new request. */
export interface WalkRequest {
  location: LocationDto;
  id: number;
}

export interface TownStreetProps {
  town: TownPlacesView;
  /** The dressed layout v2 map (the same the town map draws). */
  map: TownMap;
  /** The latest place picked on the town map, to walk to. */
  walkTo: WalkRequest | null;
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

export default function TownStreet({ town, map, walkTo }: TownStreetProps) {
  const tr = useGame((s) => s.tr);
  const pocket = useGame((s) => s.pocket);
  const busy = useGame((s) => s.busy);
  const visit = useGame((s) => s.visit);
  const command = useGame((s) => s.command);
  const walking = useGame((s) => s.walking);
  const passage = useGame((s) => s.passage?.phase ?? null);
  const inCampaign = useGame((s) => s.pocket?.in_campaign ?? false);
  // What may cover the street: the map overlay and election night always do; a sheet does where it is a
  // phone's bottom sheet over most of it (measured below), not beside the street on a wide screen.
  const modal = useGame((s) => s.townMapOpen || s.electionNight || s.interior !== null);
  // The door hook (DEC-088): walk into a place's interior, or open its panel.
  const walkIn = useGame((s) => s.walkIn);
  const inside = useGame((s) => s.interior !== null);
  const mapWalk = useRef<LocationDto | null>(null);
  const sheets = useGame((s) => [s.pocketOpen && "pocket", s.openPlace !== null && "place", s.character !== null && "character", s.decision !== null && "decision", s.story !== null && "story", s.talk !== null && "talk"].filter(Boolean).join(" "));
  // The talk hook (DEC-093): people standing in the street to walk up to.
  const peopleAt = useGame((s) => s.peopleAt);
  const openTalk = useGame((s) => s.openTalk);
  const stageRef = useRef<HTMLDivElement>(null);
  const viewRef = useRef<HubView | null>(null);
  const artRef = useRef<PixelkitArt | null>(null);
  const [ready, setReady] = useState(false);
  const [failed, setFailed] = useState<string | null>(null);
  const [status, setStatus] = useState<string | null>(null);
  // The door hint shown to keyboard walkers, announced too.
  const [door, setDoor] = useState<string | null>(null);
  // Who the player stands next to in the street, announced (the talk hook).
  const [near, setNear] = useState<string | null>(null);
  const names = useRef(new Map<string, string>());
  const talkTo = useRef<(key: string) => void>(() => {});
  // The turn's time of day, eased forward when a slot is spent; night and the new morning when a turn ends.
  const minutes = shownMinutes(pocket, passage) ?? 12 * 60;
  const easeMs = passage ? PASSAGE_EASE_MS : LIGHT_EASE_MS;
  const here = pocket?.location.location ?? town.location;

  // The latest values the view's callbacks read (it is made once per map).
  const latest = useRef({ minutes, here, busy });
  const arrive = useRef<(b: TownBuilding | null) => void>(() => {});
  const arriveAt = useRef<(loc: LocationDto, viaMap?: boolean) => void>(() => {});
  useEffect(() => {
    latest.current = { minutes, here, busy };
    talkTo.current = (key) => {
      if (latest.current.busy) return;
      const id = Number(key.slice(1));
      void openTalk(key.startsWith("c") ? { type: "person", person: id } : { type: "ambient", slot: id });
    };
    arrive.current = (b) => {
      const v = viewRef.current;
      if (!v) return;
      let loc: LocationDto | null = null;
      if (b?.home) loc = { type: "home" };
      else if (b && b.place !== null) loc = { type: "place", place: b.place };
      else {
        // Open ground or a house: the streets of the neighbourhood the player is in.
        const d = v.map.districts[v.map.district[v.scene.playerCell] ?? -1];
        if (d) loc = { type: "street", neighborhood: d.id };
      }
      if (loc) arriveAt.current(loc);
    };
    arriveAt.current = (loc, viaMap) => {
      // Arriving where the town map sent the player opens the panel; at a door walked to, the place is entered.
      const fromMap = viaMap ?? (mapWalk.current !== null && sameLocation(mapWalk.current, loc));
      mapWalk.current = null;
      if (latest.current.busy) return;
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
        setStatus(tr.t("ui-town-map-quiet", { place: spot.location.name || tr.t(`place-${spot.location.kind_key}`) }));
        return;
      }
      setStatus(null);
      if (fromMap) void visit(loc);
      else void walkIn(loc, map.seed, enterable);
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
        const { minutes: m, here: loc } = latest.current;
        const art = new PixelkitArt(pack, { night: "moods" });
        art.setClock(m);
        if (!document.getElementById("hub-icon-sprite")) document.body.insertAdjacentHTML("afterbegin", art.iconSprite().replace("<svg ", '<svg id="hub-icon-sprite" '));
        const name = (b: TownBuilding) => (b.home ? tr.t("place-home") : b.place !== null ? b.name || tr.t(`place-${b.kind}`) : null);
        const enter = (b: TownBuilding) => {
          const n = name(b);
          return n ? tr.t("ui-town-street-enter", { place: n }) : null;
        };
        view = new HubView(stage, {
          map: { ...map, start: locationCell(map, loc) ?? map.start },
          art,
          label: name,
          labelIcon: (b) => (b.place !== null || b.home ? art.icon(b.home ? "home" : b.kind) : null),
          townsfolk: 18,
          clock: new DayClock(m, 0, false),
          reducedMotion: reducedMotion(),
          canvasLabel: tr.t("ui-town-street-label", { town: town.municipality }),
          onArrive: (b) => arrive.current(b),
          // Desktop: arrows and WASD walk, Enter at a door goes in (the same as a tap on the building).
          keyboard: true,
          doorHint: enter,
          onDoor: (b) => setDoor(b ? enter(b) : null),
          // Walk up to someone in the street (a tap, or Enter beside them) and talk (DEC-093).
          onTalk: (key) => talkTo.current(key),
          personHint: (key) => tr.t("ui-talk-street-hint", { name: names.current.get(key) ?? tr.t("ui-talk-street-someone") }),
          onPersonNear: (key) => setNear(key === null ? null : tr.t("ui-talk-street-near", { name: names.current.get(key) ?? tr.t("ui-talk-street-someone") })),
        });
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

  // Nothing animates where nobody sees it: behind the map overlay, a modal, or a sheet over most of the
  // street (a phone). A sheet beside the street (wide screens) leaves it live: zoom and pan keep working.
  const sheetCovers = useSheetsCover(stageRef, sheets);
  const covered = modal || sheetCovers;
  useEffect(() => {
    if (viewRef.current) viewRef.current.paused = covered;
  }, [covered, ready]);

  // An auto-routed action (the agenda, a panel's button) walks the player here too; the engine does the
  // moving. Behind a sheet the street is paused, so the player is simply there (no walk nobody sees).
  // Already there (on a neighbourhood's streets: anywhere in it), nobody moves.
  const coveredNow = useRef(covered);
  useEffect(() => {
    coveredNow.current = covered;
  }, [covered]);
  useEffect(() => {
    const v = viewRef.current;
    if (!walking || !ready || !v || (!v.scene.playerMoving && atLocation(v.map, walking.location, v.scene.playerCell))) return;
    const cell = coveredNow.current ? locationCell(v.map, walking.location) : null;
    if (cell !== null) {
      v.scene.placePlayer(cell);
      v.wake();
    } else v.goToLocation(walking.location, false);
  }, [walking, ready]);

  // Where the player stands, for the location label: the neighbourhood under them (from the dressed
  // map's districts) and whether they stand at the engine's location (not walking, at its door).
  const [spot, setSpot] = useState<{ hood: number | null; at: boolean }>({ hood: null, at: true });
  useEffect(() => {
    if (!ready) return;
    const read = () => {
      const v = viewRef.current;
      if (!v) return;
      const cell = v.scene.playerCell;
      const hood = v.map.districts[v.map.district[cell] ?? -1]?.id ?? null;
      const at = !v.scene.playerMoving && atLocation(v.map, latest.current.here, cell);
      setSpot((s) => (s.hood === hood && s.at === at ? s : { hood, at }));
    };
    read();
    const id = window.setInterval(read, 250);
    return () => window.clearInterval(id);
  }, [ready]);
  const engineAt = pocket?.location ?? null;
  const hoodName = (id: number | null) => town.neighborhoods.find((h) => h.id === id)?.name ?? null;
  // Standing at the engine's location: its neighbourhood (a door on a border belongs to its place's); walking: the ground underfoot.
  const zone = (ready && !spot.at ? hoodName(spot.hood) : null) ?? (engineAt?.neighborhood_name || null) ?? hoodName(spot.hood) ?? town.municipality;
  const place = engineAt && engineAt.location.type !== "street" && (!ready || spot.at) ? placeName(tr, engineAt) : null;

  // The people on the street the player is on stand about it, to walk up to and talk to (DEC-093). At a
  // place they are inside: its panel lists them. The engine says who is here; the street only places them.
  const hereNow = pocket?.location.location ?? null;
  // Fetched again after every command (the pocket is new then): meeting a passer-by makes them someone stored.
  const peopleKey = hereNow?.type === "street" ? String(hereNow.neighborhood) : "";
  useEffect(() => {
    const v = viewRef.current;
    if (!v || !ready) return;
    if (!peopleKey || !hereNow) {
      v.setPeople([]);
      return;
    }
    let live = true;
    void peopleAt(hereNow).then((list) => {
      if (!live || viewRef.current !== v) return;
      const people = streetPeople(tr, list);
      names.current = new Map(people.map((p) => [p.key, p.name]));
      v.setPeople(people);
    });
    return () => {
      live = false;
    };
    // `hereNow` is read through `peopleKey` (its neighbourhood).
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [peopleKey, pocket, ready, peopleAt, tr]);

  // A place picked on the town map: walk there (at once with reduced motion), then arrive as on a tap.
  // Each request is walked once (the view is remade when the town is redressed).
  const walked = useRef(0);
  useEffect(() => {
    const v = viewRef.current;
    if (!walkTo || !ready || !v || walkTo.id <= walked.current) return;
    walked.current = walkTo.id;
    mapWalk.current = walkTo.location;
    if (sameLocation(walkTo.location, latest.current.here) || reducedMotion() || !v.goToLocation(walkTo.location, true)) arriveAt.current(walkTo.location, true);
  }, [walkTo, ready]);

  // The engine moved the player some other way (a skipped walk, an action elsewhere): stand there. On a
  // neighbourhood's streets, anywhere in it will do: a tap on open ground walks there and then moves the
  // engine to those streets, and the player stays where they walked to.
  const hereKey = JSON.stringify(here);
  useEffect(() => {
    const v = viewRef.current;
    if (!v || !ready || v.scene.playerMoving) return;
    const loc = JSON.parse(hereKey) as LocationDto;
    if (atLocation(v.map, loc, v.scene.playerCell)) return;
    const cell = locationCell(v.map, loc);
    if (cell !== null && cell !== v.scene.playerCell) {
      v.scene.placePlayer(cell);
      v.wake();
    }
  }, [hereKey, ready]);

  return (
    <div className="town-street" data-testid="town-street">
      <div className="town-street-frame">
        <div ref={stageRef} className="town-street-stage" role="group" aria-roledescription={tr.t("ui-town-street")} aria-label={town.municipality} aria-describedby="town-street-help" data-ready={ready ? "1" : "0"} />
        {!ready && (
          <div className="town-street-loading">
            {failed ? <p className="muted small">{failed}</p> : <Loader compact label={tr.t("ui-town-street-loading")} />}
          </div>
        )}
      </div>
      {inside && <Interior />}
      <div className="town-top">
        {/* Where you are: the neighbourhood, and the place once you stand at it (never blocks taps). */}
        <p className="town-where" data-testid="where">
          <strong className="town-where-zone" data-testid="where-zone">
            {zone}
          </strong>
          {place && (
            <>
              <span aria-hidden="true"> · </span>
              <span className="town-where-place" data-testid="where-place">
                {place}
              </span>
            </>
          )}
        </p>
        {inCampaign && <span className="chip chip-warn town-top-chip">{tr.t("ui-campaign-season")}</span>}
        <p className="town-street-status small" role="status" data-testid="town-street-status">
          {status}
        </p>
      </div>
      {/* Announced only when the neighbourhood changes, not at every step. */}
      <span className="visually-hidden" aria-live="polite" data-testid="where-announce">
        {tr.t("ui-town-where-zone", { zone })}
      </span>
      <span className="visually-hidden" aria-live="polite" data-testid="door-announce">
        {door}
      </span>
      <span className="visually-hidden" aria-live="polite" data-testid="person-announce">
        {near}
      </span>
      <p id="town-street-help" className="visually-hidden">
        {tr.t("ui-town-street-help")}
      </p>
    </div>
  );
}
