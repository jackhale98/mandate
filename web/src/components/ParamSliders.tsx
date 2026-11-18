import type { ParamValue } from "../engine/protocol/ParamValue";
import type { ParamView } from "../engine/protocol/ParamView";
import { useGame } from "../state/gameStore";

function decimals(step: number): number {
  const s = String(step);
  const i = s.indexOf(".");
  return i < 0 ? 0 : s.length - i - 1;
}

/** Range inputs for a template's parameters. The engine validates and clamps (it is authoritative). */
export function ParamSliders({ defs, values, onChange }: { defs: ParamView[]; values: ParamValue[]; onChange: (v: ParamValue[]) => void }) {
  const tr = useGame((s) => s.tr);
  return (
    <div className="params">
      {defs.map((d) => {
        const v = values.find((x) => x.id === d.id)?.value ?? d.default;
        const id = `param-${d.id}`;
        return (
          <label key={d.id} className="param" htmlFor={id}>
            <span className="param-head">
              <span>{tr.t(d.name_key)}</span>
              <span className="num">
                {v.toFixed(decimals(d.step))} <span className="muted">{tr.t(d.unit_key)}</span>
              </span>
            </span>
            <input
              id={id}
              type="range"
              min={d.min}
              max={d.max}
              step={d.step}
              value={v}
              onChange={(e) => {
                const value = Number(e.target.value);
                const rest = values.filter((x) => x.id !== d.id);
                onChange([...rest, { id: d.id, value }]);
              }}
            />
          </label>
        );
      })}
    </div>
  );
}
