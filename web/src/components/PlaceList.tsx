// The Notebook's Places tab (DEC-113; DEC-051, DEC-079): every place in town
// as a button, reachable without walking, and what is known of places. The
// places that matter now come first, with the engine's reasons and in its
// order (`SpotView.now`, `rank`); then each neighbourhood is a row with its
// kind, ward and worry, opening to its places (the one the player is in is
// open). Choosing a place sends the same command as arriving on foot (`visit`:
// go there, free, and open its panel). This is quick mode's home.
import type { SpotView } from "../engine/protocol/SpotView";
import type { TownHoodView } from "../engine/protocol/TownHoodView";
import { hoodLabel, placeIcon, placeName } from "../places/labels";
import { useGame } from "../state/gameStore";
import { districtLabel } from "./posts";
import { Fold, Row, RowList, Section } from "./notebook/rows";
import { reasonText } from "./notebook/People";
import { itemFacts, itemTitle, type NotebookItem } from "./notebook/notebookModel";

/** One place: the whole row goes there (the same `visit` as walking in). */
export function Spot({ s }: { s: SpotView }) {
  const tr = useGame((st) => st.tr);
  const busy = useGame((st) => st.busy);
  const visit = useGame((st) => st.visit);
  const name = placeName(tr, s.location);
  const kind = s.location.name && s.location.kind_key !== "streets" ? tr.t(`place-${s.location.kind_key}`) : null;
  // A short summary of what you can do and learn here (withdrawing a bill is a footnote).
  const what = [...s.actions.filter((a) => a !== "withdraw").map((a) => tr.t(`short-${a}`)), ...s.verbs.map((k) => tr.t(k)), ...s.services.map((x) => tr.t(`short-${x}`))];
  const reason = s.now?.find((r) => r.kind !== "here");
  return (
    <li className="rw-li">
      <div className="rw-row">
        <span className="rw-mark" aria-hidden="true">
          {placeIcon(s.location.kind_key)}
        </span>
        <button
          type="button"
          className={s.here ? "rw-main spot spot-here" : "rw-main spot"}
          tabIndex={-1}
          aria-current={s.here ? "location" : undefined}
          disabled={busy}
          onClick={() => void visit(s.location.location)}
          data-testid={`spot-${s.location.kind_key}`}
        >
          <span className="rw-l1">
            <strong className="rw-title spot-name">{name}</strong>
            {kind && <span className="rw-role">{kind}</span>}
          </span>
          {what.length > 0 && <span className="rw-l2 spot-what">{what.join(" · ")}</span>}
          {reason && <span className="rw-l3">{reasonText(tr, reason)}</span>}
          {s.note && <span className="rw-l3">{tr.event(s.note.key, s.note.args)}</span>}
        </button>
        <span className="rw-end">
          {s.here ? (
            <span className="spot-marker" data-testid="you-are-here">
              <span aria-hidden="true">📍 </span>
              {tr.t("ui-you-are-here")}
            </span>
          ) : (
            <span className="rw-chev" aria-hidden="true">
              ›
            </span>
          )}
        </span>
      </div>
    </li>
  );
}

/** A neighbourhood: its kind, ward and worry on the fold's row; its places inside. */
function Hood({ h, spots, worries, open }: { h: TownHoodView; spots: SpotView[]; worries: NotebookItem | null; open: boolean }) {
  const tr = useGame((s) => s.tr);
  const name = hoodLabel(tr, h.name, h.name_key, h.name_args);
  const bits = [tr.t(`hood-${h.kind}`)];
  if (h.ward_number !== null) bits.push(districtLabel(tr, h.ward_number, "local"));
  if (h.is_home) bits.push(tr.t("ui-home-hood"));
  const worry = h.top_concern_key ? tr.t("ui-town-hood-worry", { concern: tr.t(h.top_concern_key) }) : null;
  const doing = (s: SpotView) => s.actions.length + s.services.length + s.verbs.length > 0;
  const active = spots.filter(doing);
  const quiet = spots.filter((s) => !doing(s));
  return (
    <div className="hood" data-testid="town-hood" data-hood={h.id}>
      <Fold
        id={`hood-${h.id}`}
        title={name}
        hint={
          <>
            <span data-testid="town-hood-ward">{bits.join(" · ")}</span>
            {worry && <> · {worry}</>}
          </>
        }
        count={spots.length}
        countText={tr.t("ui-places-count", { n: spots.length })}
        open={open}
      >
        {(worries || h.note) && (
          <div className="rw-pad">
            {worries && itemFacts(tr, worries).map((f, i) => <p key={i}>{f}</p>)}
            {h.note && <p className="rw-know">{tr.event(h.note.key, h.note.args)}</p>}
          </div>
        )}
        {active.length > 0 && (
          <RowList label={name}>
            {active.map((s, i) => (
              <Spot key={i} s={s} />
            ))}
          </RowList>
        )}
        {/* Places with nothing to do yet are named, as the map draws them quiet. */}
        {quiet.length > 0 && <p className="muted small hood-quiet rw-pad">{tr.t("ui-also-here", { list: quiet.map((s) => placeName(tr, s.location)).join(" · ") })}</p>}
      </Fold>
    </div>
  );
}

/** What is known of places (hidden places found, sectors, sites, services, layers): a row each. */
function Known({ items }: { items: NotebookItem[] }) {
  const tr = useGame((s) => s.tr);
  if (items.length === 0) return null;
  return (
    <Fold id="places-known" title={tr.t("ui-places-known")} count={items.length} testId="notebook-places-known">
      <RowList className="nb-entry-list">
        {items.map((i) => (
          <Row
            key={i.key}
            title={itemTitle(tr, i)}
            sub={[...itemFacts(tr, i), i.entry?.note ? tr.event(i.entry.note.key, i.entry.note.args) : ""].filter(Boolean).join(" · ") || null}
            li={{ "data-testid": "notebook-entry", "data-subject": i.subject.type, "data-new": i.isNew ? "true" : undefined }}
          />
        ))}
      </RowList>
    </Fold>
  );
}

/** The Places tab. `items`: the notebook's entries filed under Places. */
export function PlaceList({ items = [] }: { items?: NotebookItem[] }) {
  const tr = useGame((s) => s.tr);
  const town = useGame((s) => s.town);
  if (!town) return null;
  // The engine's order: the places that matter now first (its reasons say why), then the town's own order.
  const all = town.neighborhoods.flatMap((h) => h.spots);
  // Where the player stands is one of them (the engine's `here` reason): the keyboard lands there first.
  const now = all.filter((s) => (s.now?.length ?? 0) > 0).sort((a, b) => (a.rank ?? 0) - (b.rank ?? 0));
  const shownNow = new Set(now);
  const worries = new Map<number, NotebookItem>();
  for (const i of items) if (i.subject.type === "hood_concerns") worries.set(i.subject.hood, i);
  const known = items.filter((i) => i.subject.type !== "hood_concerns");
  return (
    <div className="nb-places" data-testid="place-list">
      {now.length > 0 && (
        <Section id="nb-places-now" title={tr.t("ui-places-now")} testId="places-now">
          <RowList>
            {now.map((s, i) => (
              <Spot key={i} s={s} />
            ))}
          </RowList>
        </Section>
      )}
      {town.neighborhoods.map((h) => {
        const spots = h.spots.filter((s) => !shownNow.has(s));
        // The neighbourhood the player is in is open (§3.5).
        const here = h.spots.some((s) => s.here);
        return <Hood key={h.id} h={h} spots={spots} worries={worries.get(h.id) ?? null} open={here} />;
      })}
      <Known items={known} />
    </div>
  );
}
