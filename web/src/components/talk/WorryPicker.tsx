// The worry raised on a canvass (DEC-091 §1.6, "ignorance costs
// effectiveness"): "no particular issue" by default, else one of the worries
// the player knows of in that neighbourhood. The list and the match come from
// the engine's canvass preview (`CanvassView`: the same `canvass_match` the
// command applies, shown as far as the player knows the neighbourhood); the
// UI computes no bonus.
import { useEffect, useState } from "react";
import type { CanvassView } from "../../engine/protocol/CanvassView";
import type { CanvassWorryView } from "../../engine/protocol/CanvassWorryView";
import type { Translator } from "../../i18n/fluent";
import { useGame } from "../../state/gameStore";
import { LeadLine } from "../Knowing";
import "./talk.css";

/** The engine's canvass preview for a neighbourhood, fetched again when the game moves on. */
export function useCanvassPreview(hood: number | null): CanvassView | null {
  const preview = useGame((s) => s.previewCanvass);
  const pocket = useGame((s) => s.pocket);
  const [v, setV] = useState<CanvassView | null>(null);
  useEffect(() => {
    if (hood === null) return;
    let live = true;
    void preview(hood).then((x) => {
      if (live) setV(x);
    });
    return () => {
      live = false;
    };
  }, [hood, pocket, preview]);
  return v && v.neighborhood === hood ? v : null;
}

function worryLabel(tr: Translator, w: CanvassWorryView): string {
  const bits = [tr.t(w.name_key)];
  if (w.top) bits.push(tr.t("ui-canvass-worry-top"));
  else if (w.source === "town") bits.push(tr.t("ui-canvass-worry-town"));
  if (w.fit_band) bits.push(tr.t(`ui-canvass-fit-${w.fit_band}`));
  return bits.join(" · ");
}

/** "×1.45": the engine's multiplier, as shown. */
function times(fit: number): string {
  return `×${fit.toFixed(2)}`;
}

/** The select and what the chosen worry would do here (from the engine's preview). */
export function WorryPicker({ view, issue, onChange }: { view: CanvassView | null; issue: string | null; onChange: (issue: string | null) => void }) {
  const tr = useGame((s) => s.tr);
  const worries = view?.worries ?? [];
  const chosen = worries.find((w) => w.id === issue) ?? null;
  // A worry no longer listed (another neighbourhood) falls back to none.
  useEffect(() => {
    if (issue !== null && view && !chosen) onChange(null);
  }, [issue, view, chosen, onChange]);
  return (
    <div className="worry-picker" data-testid="worry-picker">
      <label className="field">
        <span>{tr.t("ui-canvass-worry")}</span>
        <select className="input" value={chosen?.id ?? ""} onChange={(e) => onChange(e.target.value === "" ? null : e.target.value)} data-testid="canvass-worry">
          <option value="">{tr.t("ui-canvass-worry-none")}</option>
          {worries.map((w) => (
            <option key={w.id} value={w.id}>
              {worryLabel(tr, w)}
            </option>
          ))}
        </select>
      </label>
      <p className="small muted canvass-fit" data-testid="canvass-fit">
        {chosen ? (
          chosen.fit !== null && chosen.fit_band ? (
            tr.t("ui-canvass-effect", { issue: tr.t(chosen.name_key), fit: tr.t(`ui-canvass-fit-${chosen.fit_band}`), times: times(chosen.fit) })
          ) : chosen.fit_band ? (
            tr.t("ui-canvass-effect-band", { issue: tr.t(chosen.name_key), fit: tr.t(`ui-canvass-fit-${chosen.fit_band}`) })
          ) : (
            tr.t("ui-canvass-effect-unknown", { issue: tr.t(chosen.name_key) })
          )
        ) : worries.length === 0 && view ? (
          <>
            {tr.t("ui-canvass-no-worries")}
            <LeadLine lead={view.knowledge.lead} />
          </>
        ) : (
          tr.t("ui-canvass-general")
        )}
      </p>
    </div>
  );
}
