// Stat tile (dataviz: label · value · signed delta · 12-point trend). The
// sparkline is muted with the current period in the accent; hover/focus shows
// a tooltip, and every value is also in the tile's accessible label.
import { useId, useState } from "react";

export interface Delta {
  text: string;
  /** true = good, false = bad, null = neutral */
  good: boolean | null;
}

interface Props {
  label: string;
  value: string;
  delta?: Delta | undefined;
  trend?: number[] | undefined;
  formatPoint?: (v: number, monthsAgo: number) => string;
  trendLabel?: string;
}

const W = 120;
const H = 32;

export function StatTile({ label, value, delta, trend, formatPoint, trendLabel }: Props) {
  const [hover, setHover] = useState<number | null>(null);
  const id = useId();
  const pts = trend && trend.length >= 2 ? trend : null;
  let path = "";
  let coords: [number, number][] = [];
  if (pts) {
    const min = Math.min(...pts);
    const max = Math.max(...pts);
    const span = max - min || 1;
    coords = pts.map((v, i) => [(i / (pts.length - 1)) * (W - 6) + 3, H - 4 - ((v - min) / span) * (H - 8)]);
    path = coords.map(([x, y], i) => `${i ? "L" : "M"}${x.toFixed(1)},${y.toFixed(1)}`).join("");
  }
  const last = coords[coords.length - 1];
  const hovered = hover !== null ? coords[hover] : undefined;
  const pick = (clientX: number, rect: DOMRect) => {
    if (!pts) return;
    const t = (clientX - rect.left) / rect.width;
    setHover(Math.max(0, Math.min(pts.length - 1, Math.round(t * (pts.length - 1)))));
  };

  return (
    <div className="stat" role="group" aria-labelledby={`${id}-l`}>
      <span id={`${id}-l`} className="stat-label">
        {label}
      </span>
      <span className="stat-value">{value}</span>
      {delta && (
        <span className={`stat-delta ${delta.good === null ? "" : delta.good ? "delta-good" : "delta-bad"}`}>
          <span aria-hidden="true">{delta.good === null ? "•" : delta.good ? "▲" : "▼"}</span> {delta.text}
        </span>
      )}
      {pts && (
        <div className="spark-wrap">
          <svg
            className="spark"
            viewBox={`0 0 ${W} ${H}`}
            role="img"
            aria-label={trendLabel ?? label}
            tabIndex={0}
            onPointerMove={(e) => pick(e.clientX, e.currentTarget.getBoundingClientRect())}
            onPointerLeave={() => setHover(null)}
            onFocus={() => setHover(pts.length - 1)}
            onBlur={() => setHover(null)}
            onKeyDown={(e) => {
              if (e.key === "ArrowLeft") setHover((h) => Math.max(0, (h ?? pts.length - 1) - 1));
              if (e.key === "ArrowRight") setHover((h) => Math.min(pts.length - 1, (h ?? 0) + 1));
            }}
          >
            <path d={path} className="spark-line" />
            {hovered && <line x1={hovered[0]} x2={hovered[0]} y1={0} y2={H} className="spark-cross" />}
            {last && <circle cx={last[0]} cy={last[1]} r={3} className="spark-now" />}
            {hovered && <circle cx={hovered[0]} cy={hovered[1]} r={3} className="spark-hover" />}
          </svg>
          {hover !== null && formatPoint && (
            <span className="spark-tip" role="status">
              {formatPoint(pts[hover] ?? 0, pts.length - 1 - hover)}
            </span>
          )}
        </div>
      )}
    </div>
  );
}
