import type { SideEffectView } from "../engine/protocol/SideEffectView";
import { useGame } from "../state/gameStore";

/** A policy's simulated downsides (DEC-073), with a size label in words. */
export function TradeOffs({ effects }: { effects: SideEffectView[] }) {
  const tr = useGame((s) => s.tr);
  if (effects.length === 0) return null;
  return (
    <>
      <h4 className="sub">{tr.t("ui-tradeoffs")}</h4>
      <ul className="plain tradeoff-list" data-testid="policy-tradeoffs">
        {effects.map((e, i) => (
          <li key={i}>
            {tr.t(`sidefx-${e.kind}`, {
              group: tr.t(`sidefx-group-${e.group}${e.older ? "-older" : ""}`),
              size: tr.t(`sidefx-size-${e.size}`),
            })}
          </li>
        ))}
      </ul>
    </>
  );
}
