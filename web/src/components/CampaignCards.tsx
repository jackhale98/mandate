import type { CampaignView } from "../engine/protocol/CampaignView";
import type { CommandDto } from "../engine/protocol/CommandDto";
import type { NominationView } from "../engine/protocol/NominationView";
import type { RaceView } from "../engine/protocol/RaceView";
import type { ResultView } from "../engine/protocol/ResultView";
import { useGame } from "../state/gameStore";
import { formatDate } from "../i18n/format";
import { PartyChip, PollChart, ResultBars, WhyBars } from "./ElectionCharts";
import { postTitle } from "./posts";

export function Calendar({ c }: { c: CampaignView }) {
  const tr = useGame((s) => s.tr);
  const busy = useGame((s) => s.busy);
  const perform = useGame((s) => s.perform);
  // One option per post the engine says the player may stand for (seats
  // before offices), then standing down; the engine marks the one chosen.
  const retired = c.filing?.type === "retire";
  const options: { id: string; label: string; disabled: boolean; on: boolean; command: CommandDto }[] = c.nominations.map((n, i) => {
    const post = postTitle(tr, n.post);
    return {
      id: `post-${i}`,
      label: tr.t(n.held ? "ui-file-defend" : "ui-file-post", { post }),
      disabled: !n.held && !n.open,
      on: n.chosen,
      command: { type: "file", post: n.post, retire: false },
    };
  });
  options.push({ id: "retire", label: tr.t("ui-file-retire"), disabled: false, on: retired, command: { type: "file", post: null, retire: true } });
  // Why a post is closed to the player, for the first such post.
  const closed = c.nominations.find((n) => !n.open && n.reason_key);
  const hint = c.in_campaign
    ? tr.t("ui-filing-closed")
    : closed?.reason_key
      ? tr.t(closed.reason_key, { post: postTitle(tr, closed.post), min: closed.min_standing ?? 0 })
      : tr.t("ui-filing-help");
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
            <label key={o.id} className={o.on ? "seg seg-on" : "seg"} aria-disabled={o.disabled} data-testid={`file-${o.id}`}>
              <input type="radio" name="filing" checked={o.on} disabled={o.disabled} onChange={() => void perform(o.command)} />
              {o.label}
            </label>
          ))}
        </div>
        <p className="hint">{hint}</p>
      </fieldset>
      {c.refusal && (
        <p className="note" data-testid="nomination-refused">
          {tr.event(c.refusal.key, c.refusal.args)}
        </p>
      )}
      {c.nominations
        .filter((n) => !n.held)
        .map((n, i) => (
          <NominationNotes key={i} n={n} c={c} />
        ))}
    </section>
  );
}

/** A post the player could file for: who holds it and whether they stand
 * again (in words), the party's rule for its own holder, and the Standing it
 * asks against the player's. All from the engine's view. */
function NominationNotes({ n, c }: { n: NominationView; c: CampaignView }) {
  const tr = useGame((s) => s.tr);
  const holder = n.holder ?? "";
  return (
    <div className="nomination-notes" data-testid="nomination-notes">
      <h3 className="sub">{postTitle(tr, n.post)}</h3>
      <p className="small">
        {n.holder !== null ? (
          <>
            {tr.t("ui-nomination-holder", { holder })} <PartyChip parties={c.parties} party={n.holder_party} />
            {n.holder_outlook_key && <span className="muted"> · {tr.t(n.holder_outlook_key)}</span>}
          </>
        ) : (
          tr.t("ui-nomination-vacant")
        )}
      </p>
      {n.holder_same_party && <p className="small">{tr.t("ui-nomination-backs-holder")}</p>}
      {n.warning_key && (
        <p className="small">
          <strong>{tr.t(n.warning_key, { holder })}</strong>
        </p>
      )}
      {n.gate_standing !== null && (
        <p className="small">{tr.t("ui-nomination-gate", { min: Math.round(n.gate_standing), standing: Math.round(n.standing) })}</p>
      )}
    </div>
  );
}

export function RaceCard({ race, c }: { race: RaceView; c: CampaignView }) {
  const tr = useGame((s) => s.tr);
  const avg = race.poll_average;
  return (
    <section className="card" data-testid={race.is_player_race ? "my-race" : undefined}>
      <div className="card-head">
        <h2>{postTitle(tr, race.post)}</h2>
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
        <h2>{postTitle(tr, r.post)}</h2>
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

/** The campaign at the party office: your race (public polls), the other races, results, parties. */
export function CampaignHub() {
  const tr = useGame((s) => s.tr);
  const c = useGame((s) => s.campaign);
  if (!c) return null;
  const mine = c.races.find((r) => r.is_player_race);
  const others = c.races.filter((r) => !r.is_player_race);
  return (
    <>
      {c.in_campaign && !mine && <p className="note">{tr.t("ui-not-running")}</p>}
      {c.refusal && <p className="note">{tr.event(c.refusal.key, c.refusal.args)}</p>}
      {mine && <RaceCard race={mine} c={c} />}
      {others.length > 0 && (
        <section className="card">
          <h2>{tr.t("ui-other-races")}</h2>
          <ul className="region-list">
            {others.map((r, i) => {
              const avg = r.poll_average;
              return (
                <li key={i} className="other-race">
                  <span>{postTitle(tr, r.post)}</span>
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
    </>
  );
}

/** Election night: the player's result first (or that they were on no
 * ballot), what became of the post they held, the offices' results, then
 * who won each other seat. Shown whether or not the player stood. */
export function ElectionNight() {
  const tr = useGame((s) => s.tr);
  const c = useGame((s) => s.campaign);
  const open = useGame((s) => s.electionNight);
  const close = useGame((s) => s.closeElectionNight);
  if (!open || !c) return null;
  const mine = c.results.filter((r) => r.player_ran);
  // The rest of the night: the offices' results (the town's headline races).
  const rest = c.results.filter((r) => !r.player_ran && r.post.kind === "office");
  const seats = c.results.filter((r) => !r.player_ran && r.post.kind !== "office");
  const outcome = c.seat_outcome;
  return (
    <div className="modal-backdrop" role="dialog" aria-modal="true" aria-labelledby="election-night-title" data-testid="election-night">
      <div className="modal">
        <div className="sheet-head">
          <h2 id="election-night-title">{tr.t("ui-election-night")}</h2>
          <button type="button" className="btn btn-sm btn-ghost" onClick={close}>
            {tr.t("ui-continue-game")}
          </button>
        </div>
        {mine.length === 0 && (
          <p className="verdict" data-testid="night-not-running">
            {tr.t("ui-night-not-running")}
          </p>
        )}
        {outcome && (
          <p className="note" data-testid="night-seat">
            {tr.t(outcome.key, { post: postTitle(tr, outcome.post), winner: outcome.winner ?? "" })}
          </p>
        )}
        {mine.map((r, i) => (
          <ResultCard key={i} r={r} c={c} headline />
        ))}
        {rest.map((r, i) => (
          <ResultCard key={`m${i}`} r={r} c={c} />
        ))}
        {seats.length > 0 && (
          <section className="card" data-testid="night-seats">
            <h2>{tr.t("ui-night-other-seats")}</h2>
            <ul className="region-list">
              {seats.map((r, i) => {
                const w = r.candidates.find((cd) => cd.is_winner);
                return (
                  <li key={i} className="other-race">
                    <span>{postTitle(tr, r.post)}</span>
                    <span className="small">
                      {w && (
                        <>
                          {tr.t("ui-night-won-by", { winner: w.name })} <PartyChip parties={c.parties} party={w.party} />
                        </>
                      )}
                    </span>
                  </li>
                );
              })}
            </ul>
          </section>
        )}
        <button type="button" className="btn btn-primary btn-block" onClick={close}>
          {tr.t("ui-continue-game")}
        </button>
      </div>
    </div>
  );
}
