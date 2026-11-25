// Under the town map with a layer on (DEC-086, M6 note 11): the layer's
// legend (always shown: colour never carries identity alone), the reading of
// the neighbourhood or district tapped, and, in its place, the card of the
// person tapped (who they are, where the player knows to find them, their
// full card and "Go to them") or the list of the people sharing one mark.
// Every entry is the engine's; this words and arranges them.
import type { TownAreaView } from "../../engine/protocol/TownAreaView";
import type { TownLegendBinView } from "../../engine/protocol/TownLegendBinView";
import type { TownMapLayerView } from "../../engine/protocol/TownMapLayerView";
import type { TownMapLayersView } from "../../engine/protocol/TownMapLayersView";
import type { TownMarkerView } from "../../engine/protocol/TownMarkerView";
import type { TownShadeOptionView } from "../../engine/protocol/TownShadeOptionView";
import type { Translator } from "../../i18n/fluent";
import { placeName } from "../../places/labels";
import { roleText } from "../CharacterSheet";
import { Unknown } from "../Knowing";
import { levelDots } from "../notebook/notebookModel";
import { postTitle } from "../posts";
import { MarkerFace, markerName } from "./TownLayerOverlay";
import { areaName, hoodName, partyCss, partyName, shadeColours } from "./layerModel";

/** Where the player knows to find someone, in words (null for the player). */
export function whereText(tr: Translator, m: TownMarkerView): string | null {
  if (m.is_player) return null;
  if (m.seen) return tr.event(m.seen.key, m.seen.args);
  if (m.at) return tr.t("ui-found-at", { place: placeName(tr, m.at) });
  return tr.t("ui-town-map-where-unknown");
}

/** Their role, their seat's district and their party, in words (never colour alone). */
export function PersonFacts({ tr, m, view }: { tr: Translator; m: TownMarkerView; view: TownMapLayersView }) {
  const party = partyName(view.parties, m.party);
  return (
    <span className="tml-facts">
      {roleText(tr, m.person)}
      {m.seat && <> · {postTitle(tr, m.seat)}</>}
      {party && (
        <>
          {" · "}
          <span className="party-chip">
            <span className="party-dot" style={{ background: partyCss(view.parties, m.party) }} aria-hidden="true" />
            {party}
          </span>
        </>
      )}
    </span>
  );
}

/** One person as a row that opens their card (the cluster's list, the layer's list). */
export function PersonRow({ tr, m, view, onPerson }: { tr: Translator; m: TownMarkerView; view: TownMapLayersView; onPerson: (m: TownMarkerView) => void }) {
  return (
    <li className="tml-row">
      <MarkerFace m={m} parties={view.parties} small />
      <span className="tml-row-text">
        <button type="button" className="link-btn tml-row-name" onClick={() => onPerson(m)} data-testid="town-map-row-person" data-person={m.person.id ?? undefined}>
          {markerName(tr, m)}
        </button>
        <span className="muted small">
          <PersonFacts tr={tr} m={m} view={view} />
        </span>
      </span>
    </li>
  );
}

interface CardProps {
  tr: Translator;
  m: TownMarkerView;
  view: TownMapLayersView;
  busy: boolean;
  onClose: () => void;
  onOpenCard: (m: TownMarkerView) => void;
  onGo: (m: TownMarkerView) => void;
}

/** The person tapped: who they are, where the player knows to find them, their full card and the way to them. */
export function PersonCard({ tr, m, view, busy, onClose, onOpenCard, onGo }: CardProps) {
  const name = markerName(tr, m);
  const where = whereText(tr, m);
  return (
    <section className="tml-card" aria-labelledby="tml-card-name" data-testid="town-map-person-card" data-person={m.person.id ?? undefined}>
      <MarkerFace m={m} parties={view.parties} />
      <div className="tml-card-text">
        <h3 id="tml-card-name" className="tml-card-name">
          {name}
        </h3>
        <p className="small tml-card-facts">
          <PersonFacts tr={tr} m={m} view={view} />
        </p>
        {where && (
          <p className="small tml-card-where" data-testid="town-map-person-where" data-basis={m.basis}>
            {where}
          </p>
        )}
        {!m.is_player && (
          <p className="small muted tml-card-level">
            <span aria-hidden="true">{levelDots(m.person.level)}</span> {tr.t(`knowledge-level-${m.person.level}`)}
          </p>
        )}
      </div>
      <div className="tml-card-actions">
        {m.person.id !== null && (
          <button type="button" className="btn btn-sm" disabled={busy} onClick={() => onOpenCard(m)} data-testid="town-map-person-open">
            {tr.t("ui-town-map-person-open")}
          </button>
        )}
        {!m.is_player && m.person.id !== null && (
          <button type="button" className="btn btn-sm btn-primary" disabled={busy} aria-label={tr.t("ui-people-go-to", { name })} onClick={() => onGo(m)} data-testid="town-map-person-go">
            <span aria-hidden="true">📍 </span>
            {tr.t("ui-people-go")}
          </button>
        )}
        <button type="button" className="btn btn-sm btn-ghost" onClick={onClose} aria-label={tr.t("ui-close")} data-testid="town-map-person-close">
          ✕
        </button>
      </div>
    </section>
  );
}

/** A group of people (one mark, or those with no known place) as a list. */
export function PeopleList({ tr, title, people, view, onPerson, onClose }: { tr: Translator; title: string; people: TownMarkerView[]; view: TownMapLayersView; onPerson: (m: TownMarkerView) => void; onClose: () => void }) {
  return (
    <section className="tml-group" aria-labelledby="tml-group-title" data-testid="town-map-cluster-list">
      <div className="tml-group-head">
        <h3 id="tml-group-title" className="tml-card-name">
          {title} <span className="muted">({people.length})</span>
        </h3>
        <button type="button" className="btn btn-sm btn-ghost" onClick={onClose} aria-label={tr.t("ui-close")}>
          ✕
        </button>
      </div>
      <ul className="plain-list tml-rows">
        {people.map((m, i) => (
          <PersonRow key={m.person.id ?? `a${i}`} tr={tr} m={m} view={view} onPerson={onPerson} />
        ))}
      </ul>
    </section>
  );
}

function PartyBin({ tr, b, view }: { tr: Translator; b: TownLegendBinView; view: TownMapLayersView }) {
  const name = partyName(view.parties, b.party) ?? (b.label_key ? tr.t(b.label_key) : "");
  return (
    <li>
      <span className="party-dot" style={{ background: partyCss(view.parties, b.party) }} aria-hidden="true" />
      {name} <span className="muted">({b.count})</span>
    </li>
  );
}

interface LegendProps {
  tr: Translator;
  view: TownMapLayersView;
  layer: TownMapLayerView;
  option: TownShadeOptionView | null;
  /** The neighbourhood being read (shaded layers). */
  hood: number | null;
  /** The district being read (boundaries layers). */
  area: TownAreaView | null;
  /** People the layer lists but cannot place, if any (markers layers). */
  unplaced: number;
  onUnplaced: () => void;
  onPerson: (m: TownMarkerView) => void;
}

/** The layer's legend, with the reading of what was tapped. */
export function LayerLegend({ tr, view, layer, option, hood, area, unplaced, onUnplaced, onPerson }: LegendProps) {
  const label = tr.t(layer.label_key, layer.args);
  const data = layer.data;
  const colours = option ? shadeColours(option) : [];
  const i = hood === null ? -1 : view.hoods.findIndex((h) => h.id === hood);
  const read = i >= 0 ? view.hoods[i] : undefined;
  const cell = i >= 0 ? option?.cells[i] : undefined;
  // A magnitude reads from the most to the least; categories keep the engine's order.
  const bins = option ? option.bins.map((b, k) => ({ b, k })) : [];
  if (option?.scale === "sequential") bins.reverse();
  return (
    <section className="tml-legend" aria-label={`${tr.t("ui-map-legend")}: ${label}`} data-testid="town-map-legend" data-layer={layer.id}>
      {data.kind === "shaded" && (
        <div className="tml-readout" data-testid="town-map-readout" aria-live="polite">
          {read && option ? (
            <>
              <span className={cell?.class == null ? "map-swatch tml-hatch" : "map-swatch"} style={cell?.class == null ? undefined : { background: colours[cell.class] }} aria-hidden="true" />
              <span className="tml-readout-text">
                <strong>{hoodName(tr, read)}</strong>{" "}
                {cell?.label_key ? (
                  <span data-testid="town-map-readout-value">
                    {tr.t(option.label_key)}: {tr.t(cell.label_key)}
                  </span>
                ) : (
                  <span data-testid="town-map-readout-value">{tr.t("ui-town-map-unknown")}</span>
                )}
                {(data.details[i]?.length ?? 0) > 0 ? (
                  <span className="muted tml-readout-detail">{tr.t("ui-town-map-readout-worries", { list: (data.details[i] ?? []).map((k) => tr.t(k)).join(", ") })}</span>
                ) : (
                  data.knowledge[i] && (
                    <span className="tml-readout-detail">
                      <Unknown k={data.knowledge[i]} text="knowledge-concerns-unknown" />
                    </span>
                  )
                )}
              </span>
            </>
          ) : (
            <span className="muted">{tr.t("ui-town-map-readout-hint")}</span>
          )}
        </div>
      )}
      {data.kind === "boundaries" && area && (
        <div className="tml-readout" data-testid="town-map-readout" aria-live="polite">
          <span className="tml-readout-text">
            <strong>{areaName(tr, area)}</strong>
            {area.yours && <span className="you-chip">{tr.t("ui-town-map-area-yours")}</span>}
            {area.home && !area.yours && <span className="you-chip">{tr.t("ui-town-map-area-home")}</span>}{" "}
            <span className="tml-readout-detail">
              {tr.t("ui-town-map-area-hoods", {
                list: view.hoods
                  .filter((h) => area.hoods.includes(h.id))
                  .map((h) => hoodName(tr, h))
                  .join(", "),
              })}
            </span>
            <span className="tml-readout-detail">
              {area.holder ? (
                <>
                  <button type="button" className="link-btn" onClick={() => area.holder && onPerson(area.holder)} data-testid="town-map-readout-member">
                    {markerName(tr, area.holder)}
                  </button>
                  <span className="muted">
                    {" · "}
                    <PersonFacts tr={tr} m={area.holder} view={view} />
                  </span>
                </>
              ) : (
                tr.t("ui-town-map-area-vacant")
              )}
            </span>
          </span>
        </div>
      )}
      <div className="tml-legend-head">
        <strong>{label}</strong> <span className="muted">· {tr.t(layer.unit_key, layer.args)}</span>
      </div>
      <ul className="tml-bins">
        {data.kind !== "shaded" && data.legend.map((b, k) => <PartyBin key={k} tr={tr} b={b} view={view} />)}
        {data.kind === "shaded" &&
          bins.map(({ b, k }) => (
            <li key={k} className={cell?.class === k ? "on" : undefined}>
              <span className="map-swatch" style={{ background: colours[k] }} aria-hidden="true" />
              {b.label_key ? tr.t(b.label_key) : ""} <span className="muted">({b.count})</span>
            </li>
          ))}
        {data.kind === "shaded" && option && option.unknown > 0 && (
          <li className={read && cell?.class == null ? "on" : undefined} data-testid="town-map-legend-unknown">
            <span className="map-swatch tml-hatch" aria-hidden="true" />
            {/* Never counted: nothing adds up how much is still to learn (DEC-091, 2026-11-25). */}
            {tr.t("ui-town-map-unknown")}
          </li>
        )}
      </ul>
      {data.kind === "markers" && unplaced > 0 && (
        <p className="tml-unplaced small">
          <button type="button" className="link-btn" onClick={onUnplaced} data-testid="town-map-unplaced">
            {tr.t("ui-town-map-unplaced")} ({unplaced})
          </button>
        </p>
      )}
      {layer.note_key && <p className="tml-note muted">{tr.t(layer.note_key, layer.args)}</p>}
    </section>
  );
}
