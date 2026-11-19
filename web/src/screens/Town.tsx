import type { SpotView } from "../engine/protocol/SpotView";
import { useGame } from "../state/gameStore";
import { placeIcon, placeName } from "../places/labels";
import { Loader } from "../components/Brand";
import type { TownCharacterView } from "../engine/protocol/TownCharacterView";
import type { PersonRefView } from "../engine/protocol/PersonRefView";
import { roleText } from "../components/CharacterSheet";
import { Unknown, personName } from "../components/Knowing";
import { TownMap } from "../components/TownMap";
import { TownViewTabs, townPanelId, townTabId, useTownViewMode } from "../components/TownViewTabs";

function Who({ p }: { p: PersonRefView }) {
  const tr = useGame((s) => s.tr);
  const busy = useGame((s) => s.busy);
  const openCharacter = useGame((s) => s.openCharacter);
  return (
    <button type="button" className="link-btn" disabled={busy || p.id === null} onClick={() => p.id !== null && void openCharacter(p.id)}>
      {personName(tr, p.name)}
      {p.rumoured && <span className="muted small"> ({tr.t("knowledge-rumoured")})</span>}
      <span className="muted small"> · {roleText(tr, p)}</span>
    </button>
  );
}

/** The town's character (DEC-077 §1): what it lives on, what it is going through, and who matters. */
function TownCard({ c }: { c: TownCharacterView }) {
  const tr = useGame((s) => s.tr);
  return (
    <section className="card town-card" data-testid="town-card" aria-labelledby="town-card-title">
      <h2 id="town-card-title">{tr.t("ui-town-card")}</h2>
      <dl className="facts">
        <div>
          <dt>{tr.t("ui-defining-industry")}</dt>
          <dd data-testid="town-industry">{c.industry ? `${tr.t(c.industry.name_key)} · ${tr.t("ui-industry-share", { pct: Math.round(c.industry_share * 100) })}` : tr.t("ui-nobody")}</dd>
        </div>
        <div>
          <dt>{tr.t("ui-live-crisis")}</dt>
          <dd data-testid="town-crisis">
            {c.crisis ? (
              <>
                <strong>{tr.t(c.crisis.name_key)}</strong> · {tr.t(`crisis-severity-${c.crisis.severity_band}`)}, {tr.t(`crisis-trend-${c.crisis.trend}`)} <span className="muted small">({tr.t("ui-crisis-since", { month: c.crisis.since.month, year: String(c.crisis.since.year) })})</span>
              </>
            ) : (
              <span className="muted">{tr.t("ui-no-crisis")}</span>
            )}
          </dd>
        </div>
        <div>
          <dt>{tr.t("ui-power-broker-line")}</dt>
          <dd data-testid="town-broker">{c.broker ? <Who p={c.broker} /> : c.broker_knowledge.level === "unknown" ? <Unknown k={c.broker_knowledge} text="knowledge-broker-unknown" /> : tr.t("ui-nobody")}</dd>
        </div>
        <div>
          <dt>{tr.t("ui-rival-line")}</dt>
          <dd data-testid="town-rival">{c.rival ? <Who p={c.rival} /> : tr.t("ui-nobody")}</dd>
        </div>
        <div>
          <dt>{tr.t("ui-priorities")}</dt>
          <dd data-testid="town-priorities">
            {c.priorities.length > 0 ? c.priorities.map((p) => tr.t(p.name_key)).join(" · ") : c.priorities_knowledge.level === "unknown" ? <Unknown k={c.priorities_knowledge} text="knowledge-priorities-unknown" /> : tr.t("ui-nobody")}
          </dd>
        </div>
      </dl>
    </section>
  );
}

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
  // DEC-086: the town map is the main view; the list is its text view (Map | List, remembered per device).
  const [mode, setMode] = useTownViewMode();
  if (!town) {
    return (
      <main className="game-body">
        <div className="center">
          <Loader compact />
        </div>
      </main>
    );
  }
  return (
    <main className="game-body town" data-testid="town">
      <h1 className="town-title">
        {town.municipality} <span className="muted small">· {tr.t("ui-town-help")}</span>
      </h1>
      <div className="town-layout">
      {town.character && <TownCard c={town.character} />}
      <div className="town-main">
      <TownViewTabs mode={mode} onChange={setMode} />
      {mode === "map" ? (
        <div role="tabpanel" id={townPanelId("map")} aria-labelledby={townTabId("map")}>
          <TownMap town={town} />
        </div>
      ) : (
      <div className="town-grid" role="tabpanel" id={townPanelId("list")} aria-labelledby={townTabId("list")}>
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
      )}
      </div>
      </div>
    </main>
  );
}
