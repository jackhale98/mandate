// The pocket's Places tab (DEC-088 main screen, DEC-051, DEC-079): every place
// in town as a button, by neighbourhood, reachable without walking. Choosing
// one sends the same command as arriving on foot in the street (`visit`: go
// there, free, and open its panel). This is the accessible path to every
// place and quick mode's future home. Places with nothing to do yet are named
// under "Also here".
import type { SpotView } from "../engine/protocol/SpotView";
import { placeIcon, placeName } from "../places/labels";
import { useGame } from "../state/gameStore";
import { say } from "./townMapText";

function Spot({ s }: { s: SpotView }) {
  const tr = useGame((st) => st.tr);
  const busy = useGame((st) => st.busy);
  const visit = useGame((st) => st.visit);
  const name = placeName(tr, s.location);
  const kind = tr.t(`place-${s.location.kind_key}`);
  // A short summary of what you can do and learn here (withdrawing a bill is a footnote).
  const what = [...s.actions.filter((a) => a !== "withdraw").map((a) => tr.t(`short-${a}`)), ...s.services.map((x) => tr.t(`short-${x}`))];
  return (
    <li>
      <button
        type="button"
        className={s.here ? "spot spot-here" : "spot"}
        aria-current={s.here ? "location" : undefined}
        disabled={busy}
        onClick={() => void visit(s.location.location)}
        data-testid={`spot-${s.location.kind_key}`}
      >
        <span className="spot-icon" aria-hidden="true">
          {placeIcon(s.location.kind_key)}
        </span>
        <span className="spot-text">
          <strong className="spot-name">{name}</strong>
          {s.location.name && s.location.kind_key !== "streets" && <span className="muted small">{kind}</span>}
          <span className="spot-what small">{what.join(" · ")}</span>
        </span>
        {s.here && (
          <span className="spot-marker" data-testid="you-are-here">
            <span aria-hidden="true">📍 </span>
            {tr.t("ui-you-are-here")}
          </span>
        )}
      </button>
    </li>
  );
}

export function PlaceList() {
  const tr = useGame((s) => s.tr);
  const town = useGame((s) => s.town);
  if (!town) return null;
  return (
    <>
      <p className="hint places-help">{say(tr, "ui-places-help")}</p>
      <div className="town-grid" data-testid="place-list">
        {town.neighborhoods.map((h) => {
          const active = h.spots.filter((s) => s.actions.length + s.services.length > 0);
          const quiet = h.spots.filter((s) => s.actions.length + s.services.length === 0);
          return (
            <section key={h.id} className={`hood hood-${h.kind}`} aria-labelledby={`hood-${h.id}`}>
              <div className="hood-head">
                <h3 id={`hood-${h.id}`}>{h.name}</h3>
                <span className="muted small">{tr.t(`hood-${h.kind}`)}</span>
                {h.is_home && <span className="chip chip-muted">{tr.t("ui-home-hood")}</span>}
                {h.in_ward && <span className="chip chip-muted">{tr.t("ui-your-ward")}</span>}
              </div>
              <ul className="spots">
                {active.map((s, i) => (
                  <Spot key={i} s={s} />
                ))}
              </ul>
              {quiet.length > 0 && <p className="muted small hood-quiet">{tr.t("ui-also-here", { list: quiet.map((s) => placeName(tr, s.location)).join(" · ") })}</p>}
            </section>
          );
        })}
      </div>
    </>
  );
}
