// Inside a place (DEC-088): the room the player walked into from the street,
// over the street, single screen. The room is the recipe composed from a
// keyed seed (presentation, DEC-085); who is in it and what it offers come
// from the engine's place view (DEC-079): people go on the recipe's spots by
// how the view says they are here (./cast), hotspots show only what the view
// offers now (late ones late at night), and both run through the existing
// flows: talking through the store's `openTalk`, actions through the place
// panel's forms (./HotspotSheet). A short fade in and out (none with reduced
// motion). The dialogue mode attaches through ./stage.
import { useEffect, useMemo, useRef, useState } from "react";
import type { HotspotDef } from "../../../tools/pixelkit/art/mandate/interiorsRecipes";
import type { ArtPack } from "../../../tools/pixelkit/art/mandate/pack";
import { artPack } from "../hub/townWorkerClient";
import { LIGHT_EASE_MS, PASSAGE_EASE_MS, shownMinutes } from "../hub/turnClock";
import { useGame } from "../state/gameStore";
import { useSheetsCover } from "../components/sheetCover";
import { streetPeople } from "../components/talk/streetPeople";
import { placeName } from "../places/labels";
import { castRoom, type CastPerson } from "./cast";
import { HotspotSheet, hotspotOffers } from "./HotspotSheet";
import { InteriorView, type Near, type RoomPerson } from "./interiorView";
import { recipeFromView } from "./recipes";
import { roomFor } from "./roomCache";
import { setInteriorStage } from "./stage";
import "./interior.css";

/** What tests see (`window.__interior`). */
interface InteriorProbe {
  view: InteriorView;
  place: number;
  recipe: string;
  people: RoomPerson[];
  hotspots: string[];
}
declare global {
  interface Window {
    __interior?: InteriorProbe | null;
  }
}

const reducedMotion = () => typeof matchMedia === "function" && matchMedia("(prefers-reduced-motion: reduce)").matches;
/** The fade in and out, ms (matches interior.css). */
export const FADE_MS = 220;
/** After this, the staff thins to one of each (the evening). */
const EVENING = 19 * 60;

function hash(s: string): number {
  let h = 0x811c9dc5;
  for (let i = 0; i < s.length; i++) {
    h ^= s.charCodeAt(i);
    h = Math.imul(h, 0x01000193) >>> 0;
  }
  return h;
}

export function Interior() {
  const tr = useGame((s) => s.tr);
  const inside = useGame((s) => s.interior);
  const v = useGame((s) => s.interiorPlace);
  const pocket = useGame((s) => s.pocket);
  const passage = useGame((s) => s.passage?.phase ?? null);
  const busy = useGame((s) => s.busy);
  const openTalk = useGame((s) => s.openTalk);
  const talkTo = useGame((s) => s.talkTo);
  const leaveInterior = useGame((s) => s.leaveInterior);
  const modal = useGame((s) => s.townMapOpen || s.electionNight);
  const sheets = useGame((s) => [s.pocketOpen && "pocket", s.openPlace !== null && "place", s.character !== null && "character", s.decision !== null && "decision", s.story !== null && "story", s.talk !== null && "talk"].filter(Boolean).join(" "));
  const stageRef = useRef<HTMLDivElement>(null);
  const viewRef = useRef<InteriorView | null>(null);
  const [pack, setPack] = useState<ArtPack | null>(null);
  const [failed, setFailed] = useState<string | null>(null);
  const [hot, setHot] = useState<HotspotDef | null>(null);
  const [leaving, setLeaving] = useState(false);
  const [near, setNear] = useState<string | null>(null);
  const [ready, setReady] = useState(false);
  const minutes = shownMinutes(pocket, passage) ?? 12 * 60;
  const easeMs = passage ? PASSAGE_EASE_MS : LIGHT_EASE_MS;
  const late = (pocket?.slots_left ?? 1) <= 0;
  const recipe = recipeFromView(v?.interior);
  const placeLabel = v ? placeName(tr, v.location) : "";

  useEffect(() => {
    let live = true;
    artPack()
      .then((p) => live && setPack(p))
      .catch((e: unknown) => live && setFailed(e instanceof Error ? e.message : String(e)));
    return () => {
      live = false;
    };
  }, []);

  // The room: composed once per place, recipe version and seed (the view is fetched again after every command).
  const recipeKey = recipe ? `${recipe.id}@${recipe.version}` : "";
  const room = useMemo(
    () => (pack && recipe && inside ? roomFor(pack, recipe, inside.place, inside.townSeed) : null),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [pack, recipeKey, inside?.place, inside?.townSeed],
  );

  // The hotspots offered now: the engine's view filters the recipe's (late ones only late at night).
  const hotspots = useMemo(() => (room && v ? room.hotspots.filter((h) => (!h.late || late) && (() => {
    const o = hotspotOffers(h, v);
    return o.actions.length + o.services.length > 0;
  })()) : []), [room, v, late]);

  // The people present on their spots (the engine says who; this says where).
  const people = useMemo((): RoomPerson[] => {
    if (!room || !recipe || !v) return [];
    const entries = streetPeople(tr, v.people);
    const byKey = new Map(entries.map((e) => [e.key, e]));
    const cast: CastPerson[] = [];
    v.people.forEach((p, i) => {
      const key = p.id !== null ? `c${p.id}` : p.ambient_slot !== null ? `a${p.ambient_slot}` : null;
      if (key && byKey.has(key)) cast.push({ key, hereAs: v.here_as[i] ?? "visitor" });
    });
    return castRoom(room, recipe, cast, { fullStaff: minutes < EVENING }).map((pl) => {
      const e = byKey.get(pl.key);
      return { ...pl, label: e?.label ?? null, variant: e?.variant ?? 100 + (hash(`${room.seed}:${pl.key}`) % 997) };
    });
  }, [room, recipe, v, tr, minutes]);

  // The latest values the view's callbacks read (it is made once per room).
  const latest = useRef({ busy, hotspots, people, sheets: "" });
  const leave = () => {
    if (reducedMotion()) leaveInterior();
    else setLeaving(true);
  };
  const leaveRef = useRef(leave);
  useEffect(() => {
    latest.current = { busy, hotspots, people, sheets: [sheets, hot ? "hotspot" : ""].filter(Boolean).join(" ") };
    leaveRef.current = leave;
  });

  // The room's view: made once per room.
  useEffect(() => {
    const stage = stageRef.current;
    if (!stage || !room || !pack || !inside) return;
    const hint = (n: NonNullable<Near>): string | null => {
      if (n.kind === "person") return tr.t("ui-interior-hint-person", { name: latest.current.people.find((p) => p.key === n.key)?.label ?? "" });
      if (n.kind === "hotspot") return tr.t("ui-interior-hint-hotspot", { what: tr.t(`interior-hotspot-${n.id}`) });
      return tr.t("ui-interior-hint-door");
    };
    const view = new InteriorView(stage, {
      room,
      pack,
      minutes,
      reducedMotion: reducedMotion(),
      label: tr.t("ui-interior-label", { place: placeLabel }),
      hotspots: latest.current.hotspots,
      hotspotLabel: (h) => tr.t(`interior-hotspot-${h.id}`),
      doorLabel: tr.t("ui-interior-door"),
      onTalk: (key) => {
        if (latest.current.busy) return;
        const id = Number(key.slice(1));
        view.frame(key);
        void openTalk(key.startsWith("c") ? { type: "person", person: id } : { type: "ambient", slot: id });
      },
      onHotspot: (h) => setHot(h),
      // Escape or the door while a sheet is open (a conversation, a hotspot) is the sheet's, not the room's.
      onLeave: () => {
        if (!latest.current.sheets) leaveRef.current();
      },
      onNear: (n) => {
        if (!n) setNear(null);
        else if (n.kind === "person") setNear(tr.t("ui-interior-near-person", { name: latest.current.people.find((p) => p.key === n.key)?.label ?? "" }));
        else if (n.kind === "hotspot") setNear(tr.t("ui-interior-near-hotspot", { what: tr.t(`interior-hotspot-${n.id}`) }));
        else setNear(tr.t("ui-interior-near-door"));
      },
      hint,
    });
    viewRef.current = view;
    view.setPeople(latest.current.people);
    view.focus();
    setReady(true);
    setInteriorStage({
      element: stage,
      place: inside.place,
      people: () => latest.current.people.filter((p) => !p.extra).map((p) => p.key),
      anchorOf: (key) => view.anchorOf(key),
      frame: (key) => view.frame(key),
    });
    return () => {
      setInteriorStage(null);
      view.destroy();
      viewRef.current = null;
      window.__interior = null;
      setReady(false);
    };
    // The view is made once per room; it reads the rest through `latest`.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [room, pack]);

  useEffect(() => {
    viewRef.current?.setPeople(people);
  }, [people, ready]);
  useEffect(() => {
    viewRef.current?.setHotspots(hotspots);
  }, [hotspots, ready]);
  useEffect(() => {
    viewRef.current?.setMinutes(minutes, reducedMotion() ? 0 : easeMs);
  }, [minutes, easeMs, ready]);
  // A conversation through the store is framed in the room (and ends with it).
  const talkKey = talkTo ? (talkTo.type === "person" ? `c${talkTo.person}` : `a${talkTo.slot}`) : null;
  useEffect(() => {
    viewRef.current?.frame(talkKey);
  }, [talkKey, ready]);

  // Nothing animates where nobody sees it.
  const sheetCovers = useSheetsCover(stageRef, [sheets, hot ? "hotspot" : ""].filter(Boolean).join(" "));
  const covered = modal || sheetCovers || hot !== null;
  useEffect(() => {
    if (viewRef.current) viewRef.current.paused = covered;
  }, [covered, ready]);

  useEffect(() => {
    const view = viewRef.current;
    if (!view || !inside) return;
    window.__interior = { view, place: inside.place, recipe: inside.recipe, people, hotspots: hotspots.map((h) => h.id) };
  }, [people, hotspots, inside, ready]);

  // A recipe the bundle lacks (or a failed pack): back to the street; the panel is the way in.
  useEffect(() => {
    if (inside && v && !recipe) leaveInterior();
  }, [inside, v, recipe, leaveInterior]);

  if (!inside) return null;
  return (
    <div className={`interior${leaving ? " interior-leaving" : ""}${reducedMotion() ? " interior-quick" : ""}`} data-testid="interior" data-place={inside.place} data-recipe={inside.recipe} data-ready={ready ? "1" : "0"}>
      <div ref={stageRef} className="interior-stage" />
      {failed && <p className="muted small interior-failed">{failed}</p>}
      <div className="interior-bar">
        <button type="button" className="btn btn-sm interior-leave" onClick={() => leave()} data-testid="interior-leave">
          {tr.t("ui-interior-leave")}
        </button>
      </div>
      <span className="visually-hidden" aria-live="polite" data-testid="interior-announce">
        {near ?? tr.t("ui-interior-entered", { place: placeLabel })}
      </span>
      <div
        className="interior-veil"
        aria-hidden="true"
        onAnimationEnd={() => {
          if (leaving) leaveInterior();
        }}
      />
      {hot && v && (
        <HotspotSheet
          hotspot={hot}
          view={v}
          onClose={() => {
            setHot(null);
            viewRef.current?.focus();
          }}
        />
      )}
    </div>
  );
}
