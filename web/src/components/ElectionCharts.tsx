// Election visuals (dataviz contract): party colour follows the party, never
// rank; every coloured mark has a text label beside it (light-mode aqua sits
// under 3:1, so labels are mandatory); one axis; hover shows exact values.
import { useId, useState } from "react";
import type { PartyView } from "../engine/protocol/PartyView";
import type { RaceView } from "../engine/protocol/RaceView";
import type { ResultView } from "../engine/protocol/ResultView";
import type { FactorView } from "../engine/protocol/FactorView";
import { useGame } from "../state/gameStore";
import { formatDate, formatCount } from "../i18n/format";

export function partyColour(parties: PartyView[], party: number | null): string {
  const p = party === null ? undefined : parties.find((x) => x.id === party);
  return p ? `var(--party-${p.colour % 3})` : "var(--party-none)";
}

export function PartyChip({ parties, party }: { parties: PartyView[]; party: number | null }) {
  const tr = useGame((s) => s.tr);
  const p = party === null ? undefined : parties.find((x) => x.id === party);
  return (
    <span className="party-chip">
      <span className="party-dot" style={{ background: partyColour(parties, party) }} aria-hidden="true" />
      {p ? p.name : tr.t("ui-independent")}
    </span>
  );
}

const pct = (v: number) => `${Math.round(v * 100)}%`;

/** Poll shares over the campaign, one line per candidate, direct-labelled at the end. */
export function PollChart({ race, parties }: { race: RaceView; parties: PartyView[] }) {
  const tr = useGame((s) => s.tr);
  const [hover, setHover] = useState<number | null>(null);
  const id = useId();
  const polls = race.polls;
  if (polls.length === 0) return <p className="muted small">{tr.t("ui-no-polls")}</p>;
  const W = 320;
  const H = 150;
  const padL = 34;
  const padR = 92;
  const padT = 10;
  const padB = 22;
  const days = polls.map((p) => p.day);
  const d0 = Math.min(...days);
  const d1 = Math.max(...days, d0 + 1);
  const maxShare = Math.min(1, Math.max(0.5, ...polls.flatMap((p) => p.shares)) + 0.05);
  const x = (d: number) => padL + ((d - d0) / (d1 - d0)) * (W - padL - padR);
  const y = (v: number) => padT + (1 - v / maxShare) * (H - padT - padB);
  const ticks = [0, 0.25, 0.5, 0.75, 1].filter((t) => t <= maxShare + 1e-9);
  const last = polls[polls.length - 1];
  const avg = race.poll_average ?? last?.shares ?? [];
  // Spread end labels so they never collide (min 13px apart).
  const ends = race.candidates
    .map((_c, k) => ({ k, y: y(avg[k] ?? 0) }))
    .sort((a, b) => a.y - b.y);
  for (let i = 1; i < ends.length; i++) {
    const prev = ends[i - 1];
    const cur = ends[i];
    if (prev && cur && cur.y - prev.y < 13) cur.y = prev.y + 13;
  }
  const hp = hover === null ? null : polls[hover];
  const label = `${tr.t("ui-poll-trend")}: ${race.candidates.map((c, k) => `${c.name} ${pct(avg[k] ?? 0)}`).join(", ")}`;
  return (
    <div className="poll-chart">
      <svg
        viewBox={`0 0 ${W} ${H}`}
        role="img"
        aria-labelledby={`${id}-t`}
        tabIndex={0}
        onPointerMove={(e) => {
          const r = e.currentTarget.getBoundingClientRect();
          const px = ((e.clientX - r.left) / r.width) * W;
          let best = 0;
          let bd = Infinity;
          polls.forEach((p, i) => {
            const d = Math.abs(x(p.day) - px);
            if (d < bd) {
              bd = d;
              best = i;
            }
          });
          setHover(best);
        }}
        onPointerLeave={() => setHover(null)}
        onBlur={() => setHover(null)}
        onKeyDown={(e) => {
          if (e.key === "ArrowLeft") setHover((h) => Math.max(0, (h ?? polls.length) - 1));
          if (e.key === "ArrowRight") setHover((h) => Math.min(polls.length - 1, (h ?? -1) + 1));
        }}
      >
        <title id={`${id}-t`}>{label}</title>
        {ticks.map((t) => (
          <g key={t}>
            <line className="grid" x1={padL} x2={W - padR} y1={y(t)} y2={y(t)} />
            <text className="axis" x={padL - 4} y={y(t) + 3} textAnchor="end">
              {pct(t)}
            </text>
          </g>
        ))}
        {race.candidates.map((c, k) => {
          const pts = polls.map((p) => `${x(p.day).toFixed(1)},${y(p.shares[k] ?? 0).toFixed(1)}`).join(" ");
          return (
            <g key={c.politician}>
              <polyline className="poll-line" points={pts} style={{ stroke: partyColour(parties, c.party) }} />
              {polls.map((p, i) => (
                <circle
                  key={i}
                  className="poll-dot"
                  cx={x(p.day)}
                  cy={y(p.shares[k] ?? 0)}
                  r={hover === i ? 4.5 : 3}
                  style={{ fill: partyColour(parties, c.party) }}
                />
              ))}
            </g>
          );
        })}
        {ends.map(({ k, y: ly }) => {
          const c = race.candidates[k];
          if (!c) return null;
          return (
            <text key={c.politician} className="end-label" x={W - padR + 6} y={ly + 4}>
              {`${c.name.split(" ").slice(-1)[0] ?? c.name} ${pct(avg[k] ?? 0)}`}
            </text>
          );
        })}
        {hp && <line className="cross" x1={x(hp.day)} x2={x(hp.day)} y1={padT} y2={H - padB} />}
      </svg>
      {hp && (
        <div className="chart-tip" role="status">
          <strong>{formatDate(tr, hp.date)}</strong> · {hp.pollster}
          {race.candidates.map((c, k) => (
            <div key={c.politician}>
              <span className="party-dot" style={{ background: partyColour(parties, c.party) }} aria-hidden="true" /> {c.name}{" "}
              <span className="num">{pct(hp.shares[k] ?? 0)}</span>
            </div>
          ))}
          <div className="muted">{tr.t("ui-undecided", { pct: Math.round(hp.undecided * 100) })}</div>
        </div>
      )}
      <details className="table-view">
        <summary>{tr.t("ui-poll-table")}</summary>
        <table>
          <thead>
            <tr>
              <th>{tr.t("ui-date-col")}</th>
              <th>{tr.t("ui-pollster")}</th>
              {race.candidates.map((c) => (
                <th key={c.politician}>{c.name}</th>
              ))}
              <th>{tr.t("ui-undecided-col")}</th>
            </tr>
          </thead>
          <tbody>
            {polls.map((p, i) => (
              <tr key={i}>
                <td>{formatDate(tr, p.date)}</td>
                <td>{p.pollster}</td>
                {p.shares.map((s, k) => (
                  <td key={k} className="num">
                    {pct(s)}
                  </td>
                ))}
                <td className="num">{pct(p.undecided)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </details>
    </div>
  );
}

/** Final shares as labelled bars, with the final poll average as a tick. */
export function ResultBars({ result, parties }: { result: ResultView; parties: PartyView[] }) {
  const tr = useGame((s) => s.tr);
  const sorted = [...result.candidates].sort((a, b) => b.share - a.share);
  return (
    <ul className="result-bars">
      {sorted.map((c) => (
        <li key={c.name} className={c.is_winner ? "winner" : ""}>
          <div className="result-head">
            <span>
              {c.is_winner && <span aria-label={tr.t("ui-winner")}>✓ </span>}
              <strong>{c.name}</strong> {c.is_player && <span className="chip chip-muted">{tr.t("ui-you")}</span>}
              {c.was_incumbent && <span className="muted small"> · {tr.t("ui-incumbent")}</span>}
            </span>
            <span className="num">
              {pct(c.share)} <span className="muted small">({formatCount(c.votes)})</span>
            </span>
          </div>
          <div className="result-track" aria-hidden="true">
            <span className="result-fill" style={{ width: `${c.share * 100}%`, background: partyColour(parties, c.party) }} />
            {c.final_poll !== null && <span className="poll-tick" style={{ left: `${c.final_poll * 100}%` }} />}
          </div>
          <div className="muted small">
            <PartyChip parties={parties} party={c.party} />
            {c.final_poll !== null && <> · {tr.t("ui-final-poll", { pct: Math.round(c.final_poll * 100) })}</>}
          </div>
        </li>
      ))}
    </ul>
  );
}

/** Why the winner won: each factor's contribution to the margin (diverging, labelled). */
export function WhyBars({ factors, sign = 1 }: { factors: FactorView[]; sign?: 1 | -1 }) {
  const tr = useGame((s) => s.tr);
  if (factors.length === 0) return null;
  const max = Math.max(0.05, ...factors.map((f) => Math.abs(f.margin)));
  return (
    <ul className="diverge">
      {factors.map((f) => {
        const v = (sign * f.margin) / max;
        const pts = Math.round(sign * f.margin * 100);
        return (
          <li key={f.factor}>
            <span>{tr.t(`factor-${f.factor}`)}</span>
            <span className="dv-track" aria-hidden="true">
              <span className={v >= 0 ? "dv-fill dv-pos" : "dv-fill dv-neg"} style={v >= 0 ? { left: "50%", width: `${v * 50}%` } : { right: "50%", width: `${-v * 50}%` }} />
            </span>
            <span className="num">{pts > 0 ? `+${pts}` : pts}</span>
          </li>
        );
      })}
    </ul>
  );
}
