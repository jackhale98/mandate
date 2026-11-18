import type { CampaignView } from "../engine/protocol/CampaignView";
import type { FilingDto } from "../engine/protocol/FilingDto";
import type { RaceView } from "../engine/protocol/RaceView";
import type { ResultView } from "../engine/protocol/ResultView";
import { useGame } from "../state/gameStore";
import { formatDate } from "../i18n/format";
import { PartyChip, PollChart, ResultBars, WhyBars } from "../components/ElectionCharts";
import { Agenda } from "./Office";

function seatTitle(tr: ReturnType<typeof useGame.getState>["tr"], seat: string, ward: number | null): string {
  return seat === "mayor" ? tr.t("ui-race-mayor") : tr.t("ui-race-ward", { n: ward ?? 0 });
}

function Calendar({ c }: { c: CampaignView }) {
  const tr = useGame((s) => s.tr);
  const busy = useGame((s) => s.busy);
  const command = useGame((s) => s.command);
  const options: { id: FilingDto; key: string; disabled: boolean }[] = [
    { id: "council", key: c.player_in_office ? "ui-file-defend" : "ui-file-council", disabled: false },
    { id: "mayor", key: "ui-file-mayor", disabled: !c.mayor_nomination_open },
    { id: "retire", key: "ui-file-retire", disabled: false },
  ];
  const current = c.filing === "default" ? "council" : c.filing;
  return (
    <section className="card" data-testid="election-calendar">
      <div className="card-head">
        <h2>{tr.t("ui-next-election")}</h2>
        {c.days_to_election !== null && <span className="chip">{tr.t("ui-days-left", { n: c.days_to_election })}</span>}
      </div>
      {c.election_day && (
        <p>
          <strong>{formatDate(tr, c.election_day)}</strong>
          {c.campaign_opens && !c.in_campaign && (
            <span className="muted"> · {tr.t("ui-campaign-opens", { date: formatDate(tr, c.campaign_opens) })}</span>
          )}
        </p>
      )}
      {c.player_party !== null && (
        <p className="small">
          {tr.t("ui-your-party")}: <PartyChip parties={c.parties} party={c.player_party} />
        </p>
      )}
      <fieldset className="field" disabled={c.in_campaign || busy}>
        <legend>{tr.t("ui-your-plans")}</legend>
        <div className="segmented segmented-sm" role="radiogroup">
          {options.map((o) => (
            <label key={o.id} className={current === o.id ? "seg seg-on" : "seg"} aria-disabled={o.disabled}>
              <input
                type="radio"
                name="filing"
                checked={current === o.id}
                disabled={o.disabled}
                onChange={() => void command({ type: "file", filing: o.id })}
              />
              {tr.t(o.key)}
            </label>
          ))}
        </div>
        <p className="hint">{c.in_campaign ? tr.t("ui-filing-closed") : !c.mayor_nomination_open ? tr.t("ui-mayor-needs-standing") : tr.t("ui-filing-help")}</p>
      </fieldset>
    </section>
  );
}

function RaceCard({ race, c }: { race: RaceView; c: CampaignView }) {
  const tr = useGame((s) => s.tr);
  const avg = race.poll_average;
  return (
    <section className="card" data-testid={race.is_player_race ? "my-race" : undefined}>
      <div className="card-head">
        <h2>{seatTitle(tr, race.seat, race.ward_number)}</h2>
        {race.is_player_race && <span className="chip">{tr.t("ui-your-race")}</span>}
      </div>
      <ul className="candidate-list">
        {race.candidates.map((cd, k) => (
          <li key={cd.politician}>
            <span>
              <strong>{cd.name}</strong>
              {cd.is_player && <span className="chip chip-muted"> {tr.t("ui-you")}</span>}
              {cd.is_incumbent && <span className="muted small"> · {tr.t("ui-incumbent")}</span>}
              <br />
              <PartyChip parties={c.parties} party={cd.party} />
            </span>
            <span className="num big">{avg ? `${Math.round((avg[k] ?? 0) * 100)}%` : "—"}</span>
          </li>
        ))}
      </ul>
      {race.player_awareness !== null && (
        <ul className="meters">
          <li>
            <span>{tr.t("ui-know-you")}</span>
            <span className="meter" aria-hidden="true">
              <span className="meter-fill" style={{ width: `${Math.round(race.player_awareness * 100)}%` }} />
            </span>
            <span className="num">{Math.round(race.player_awareness * 100)}%</span>
          </li>
        </ul>
      )}
      <h3 className="sub">{tr.t("ui-polls")}</h3>
      <PollChart race={race} parties={c.parties} />
      <p className="hint">{tr.t("ui-polls-note")}</p>
    </section>
  );
}

export function ResultCard({ r, c, headline = false }: { r: ResultView; c: CampaignView; headline?: boolean }) {
  const tr = useGame((s) => s.tr);
  return (
    <section className="card" data-testid={r.player_ran ? "my-result" : undefined}>
      <div className="card-head">
        <h2>{seatTitle(tr, r.seat, r.ward_number)}</h2>
        <span className="muted small">
          {formatDate(tr, r.date)} · {tr.t("ui-turnout", { pct: Math.round(r.turnout * 100) })}
        </span>
      </div>
      {headline && r.player_ran && <p className={r.player_won ? "verdict verdict-win" : "verdict verdict-loss"}>{tr.t(r.player_won ? "ui-you-won" : "ui-you-lost")}</p>}
      <ResultBars result={r} parties={c.parties} />
      {r.decomposition.length > 0 && (
        <>
          <h3 className="sub">{tr.t("ui-why-result")}</h3>
          {/* From the player's side when they lost: what cost them, what helped. */}
          <p className="hint">{tr.t(r.player_ran && !r.player_won ? "ui-why-lost-help" : "ui-why-result-help")}</p>
          <WhyBars factors={r.decomposition} sign={r.player_ran && !r.player_won ? -1 : 1} />
        </>
      )}
    </section>
  );
}

export function Campaign() {
  const tr = useGame((s) => s.tr);
  const c = useGame((s) => s.campaign);
  const o = useGame((s) => s.office);
  if (!c || !o) return null;
  const mine = c.races.find((r) => r.is_player_race);
  const others = c.races.filter((r) => !r.is_player_race);
  return (
    <main className="game-body">
      <Calendar c={c} />
      {c.in_campaign && !mine && <p className="note">{tr.t("ui-not-running")}</p>}
      {mine && <RaceCard race={mine} c={c} />}
      {mine && <Agenda o={o} campaigning />}
      {others.length > 0 && (
        <section className="card">
          <h2>{tr.t("ui-other-races")}</h2>
          <ul className="region-list">
            {others.map((r, i) => {
              const avg = r.poll_average;
              return (
                <li key={i} className="other-race">
                  <span>{seatTitle(tr, r.seat, r.ward_number)}</span>
                  <span className="small">
                    {r.candidates.map((cd, k) => (
                      <span key={cd.politician} className="other-cand">
                        <PartyChip parties={c.parties} party={cd.party} /> {cd.name}
                        {avg && <span className="num"> {Math.round((avg[k] ?? 0) * 100)}%</span>}
                      </span>
                    ))}
                  </span>
                </li>
              );
            })}
          </ul>
        </section>
      )}
      {c.results.length > 0 && (
        <>
          <h2 className="section-title">{tr.t("ui-last-results")}</h2>
          {c.results.map((r, i) => (
            <ResultCard key={i} r={r} c={c} />
          ))}
        </>
      )}
      <section className="card">
        <h2>{tr.t("ui-parties")}</h2>
        <ul className="plain party-list">
          {c.parties.map((p) => (
            <li key={p.id}>
              <PartyChip parties={c.parties} party={p.id} /> <span className="muted small">{p.stances.map((s) => tr.t(`conviction-${s}`)).join(" · ")}</span>
            </li>
          ))}
        </ul>
      </section>
    </main>
  );
}

/** Election night: the player's result first, then the town's. */
export function ElectionNight() {
  const tr = useGame((s) => s.tr);
  const c = useGame((s) => s.campaign);
  const open = useGame((s) => s.electionNight);
  const close = useGame((s) => s.closeElectionNight);
  if (!open || !c) return null;
  const mine = c.results.filter((r) => r.player_ran);
  const rest = c.results.filter((r) => !r.player_ran && r.seat === "mayor");
  return (
    <div className="modal-backdrop" role="dialog" aria-modal="true" aria-labelledby="election-night-title" data-testid="election-night">
      <div className="modal">
        <div className="sheet-head">
          <h2 id="election-night-title">{tr.t("ui-election-night")}</h2>
          <button type="button" className="btn btn-sm btn-ghost" onClick={close}>
            {tr.t("ui-continue-game")}
          </button>
        </div>
        {mine.map((r, i) => (
          <ResultCard key={i} r={r} c={c} headline />
        ))}
        {rest.map((r, i) => (
          <ResultCard key={`m${i}`} r={r} c={c} />
        ))}
        <button type="button" className="btn btn-primary btn-block" onClick={close}>
          {tr.t("ui-continue-game")}
        </button>
      </div>
    </div>
  );
}
