import { useGame } from "../state/gameStore";

/** The Mandate mark (DEC-081): an M drawn as a stepped poll line whose right peak rises to a flag. */
export function LogoMark({
  size = 64,
  title,
}: {
  size?: number;
  title?: string;
}) {
  return (
    <svg
      className="logo-mark"
      viewBox="0.9 0 16 16"
      width={size}
      height={size}
      shapeRendering="crispEdges"
      role={title ? "img" : undefined}
      aria-label={title}
      aria-hidden={title ? undefined : true}
    >
      <path
        d="M2.5 15 V7.5 H4.5 V9.5 H6.5 V11.5 H8.5 V8.5 H10.5 V5.5 H12.5 V3.5 H13.5 V15"
        fill="none"
        stroke="var(--accent)"
        strokeWidth="1.5"
        strokeLinejoin="miter"
        strokeLinecap="square"
      />
      <rect x="13" y="0" width="1" height="3" fill="var(--text)" />
      <rect x="14" y="0" width="2" height="2" fill="var(--logo-flag)" />
    </svg>
  );
}

const RUNGS = [
  { y: 13, key: "loader-local", fallback: "Local" },
  { y: 10, key: "loader-regional", fallback: "Regional" },
  { y: 7, key: "loader-national", fallback: "National" },
  { y: 4, key: "loader-presidency", fallback: "Presidency" },
] as const;

/**
 * Shown before the engine (and so the translations) has loaded, hence the
 * English fallbacks.
 *
 * The loading animation (DEC-081): the career ladder's rungs light from local
 * office to the presidency. `compact` drops the tier labels for in-screen waits.
 */
export function Loader({
  label,
  compact = false,
  decorative = false,
}: {
  label?: string;
  compact?: boolean;
  /** Inside another live region: draw only, announce nothing. */
  decorative?: boolean;
}) {
  const tr = useGame((s) => s.tr);
  const unit = compact ? 6 : 10;
  return (
    <div
      className={compact ? "loader loader-compact" : "loader"}
      role={decorative ? undefined : "status"}
      aria-busy={decorative ? undefined : true}
      aria-hidden={decorative ? true : undefined}
    >
      <div
        className="loader-ladder"
        style={{ height: 16 * unit }}
        aria-hidden="true"
      >
        <svg
          viewBox="0 0 10 16"
          width={10 * unit}
          height={16 * unit}
          shapeRendering="crispEdges"
        >
          <rect x="1" y="1" width="1" height="14" fill="var(--text)" />
          <rect x="8" y="1" width="1" height="14" fill="var(--text)" />
          {RUNGS.map((r, i) => (
            <rect
              key={r.key}
              className={`rung rung-${i + 1}`}
              x="2"
              y={r.y}
              width="6"
              height="1"
              fill="var(--accent)"
            />
          ))}
        </svg>
        {!compact && (
          <div className="loader-tiers">
            {RUNGS.map((r, i) => (
              <span
                key={r.key}
                className={`rung rung-${i + 1}`}
                style={{ top: r.y * unit - 1 }}
              >
                {tr.has(r.key) ? tr.t(r.key) : r.fallback}
              </span>
            ))}
          </div>
        )}
      </div>
      {label ? (
        <p className="loader-label">{label}</p>
      ) : (
        !decorative && (
          <span className="visually-hidden">{tr.has("ui-loading") ? tr.t("ui-loading") : "Loading…"}</span>
        )
      )}
    </div>
  );
}
