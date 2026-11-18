import type { NationalView } from "../engine/protocol/NationalView";
import { useGame } from "../state/gameStore";
import { formatCount, formatPct } from "../i18n/format";
import { StatTile, type Delta } from "./StatTile";

/** Change over the last `n` months; `upIsGood` sets the delta colour. */
function delta(series: number[], n: number, upIsGood: boolean | null, fmt: (v: number) => string): Delta | undefined {
  if (series.length <= n) return undefined;
  const now = series[series.length - 1] ?? 0;
  const then = series[series.length - 1 - n] ?? 0;
  const d = now - then;
  const sign = d > 0 ? "+" : d < 0 ? "−" : "±";
  return { text: `${sign}${fmt(Math.abs(d))}`, good: upIsGood === null || Math.abs(d) < 1e-4 ? null : d > 0 === upIsGood };
}

export function NationalEconomy({ e }: { e: NationalView }) {
  const tr = useGame((s) => s.tr);
  const last12 = (v: number[]) => v.slice(-12);
  const pts = (v: number, ago: number) => `${formatPct(v, 1)} · ${tr.t("ui-months-ago", { n: ago })}`;
  return (
    <section className="card" aria-labelledby="nat-econ">
      <div className="card-head">
        <h2 id="nat-econ">{tr.t("ui-economy")}</h2>
        {e.in_recession && (
          <span className="chip chip-warn">
            <span aria-hidden="true">⚠</span> {tr.t("ui-recession")}
          </span>
        )}
      </div>
      <div className="stats">
        <StatTile
          label={tr.t("ui-unemployment")}
          value={formatPct(e.unemployment, 1)}
          delta={delta(e.unemployment_history, 12, false, (v) => formatPct(v, 1))}
          trend={last12(e.unemployment_history)}
          formatPoint={pts}
          trendLabel={tr.t("ui-trend-12m", { what: tr.t("ui-unemployment") })}
        />
        <StatTile
          label={tr.t("ui-inflation")}
          value={formatPct(e.inflation, 1)}
          delta={delta(e.inflation_history, 12, null, (v) => formatPct(v, 1))}
          trend={last12(e.inflation_history)}
          formatPoint={pts}
          trendLabel={tr.t("ui-trend-12m", { what: tr.t("ui-inflation") })}
        />
        <StatTile label={tr.t("ui-interest-rate")} value={formatPct(e.policy_rate, 2)} />
        <StatTile label={tr.t("ui-growth")} value={formatPct(e.gdp_growth_annual, 1)} />
        <StatTile label={tr.t("ui-debt")} value={formatPct(e.debt_pct_gdp, 0)} />
        <StatTile label={tr.t("ui-output")} value={formatCount(e.gdp)} />
      </div>
    </section>
  );
}
