import type { SpotView } from "../engine/protocol/SpotView";
import { useGame } from "../state/gameStore";
import { placeIcon, placeName } from "../places/labels";

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

/**
 * The town (M5 interim, DEC-079): a schematic of neighbourhoods with their
 * places as tappable buildings. It is also a plain list of sections and
 * buttons, so every place is reachable without the picture (DEC-051).
 */
export function Town() {
  const tr = useGame((s) => s.tr);
  const town = useGame((s) => s.town);
  if (!town) {
    return (
      <main className="game-body">
        <div className="center">
          <div className="spinner" aria-hidden="true" />
        </div>
      </main>
    );
  }
  return (
    <main className="game-body town" data-testid="town">
      <h1 className="town-title">
        {town.municipality} <span className="muted small">· {tr.t("ui-town-help")}</span>
      </h1>
      <div className="town-grid">
        {town.neighborhoods.map((h) => {
          const active = h.spots.filter((s) => s.actions.length + s.services.length > 0);
          const quiet = h.spots.filter((s) => s.actions.length + s.services.length === 0);
          return (
            <section key={h.id} className={`hood hood-${h.kind}`} aria-labelledby={`hood-${h.id}`}>
              <div className="hood-head">
                <h2 id={`hood-${h.id}`}>{h.name}</h2>
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
    </main>
  );
}
