import { useGame } from "../../state/gameStore";
import { useWhy, type WhyFactors } from "./whyStore";
import "./why.css";

/**
 * The "Why?" control on an outcome: opens the panel on the event the view
 * names. Renders nothing when the view names none (the engine gives an id
 * only for an event the player may ask about). `about`: the outcome in
 * words, for the control's accessible name where several share a list.
 * `label`: `ui-why-led` where "Why?" is already taken (the bill card).
 */
export function WhyButton({
  event,
  about,
  label = "ui-why-ask",
  factors = null,
  inline = false,
}: {
  event: number | null | undefined;
  about?: string;
  label?: "ui-why-ask" | "ui-why-led";
  factors?: WhyFactors | null;
  /** In a line of text or a list row: a small link instead of a button. */
  inline?: boolean;
}) {
  const tr = useGame((s) => s.tr);
  const ask = useWhy((s) => s.ask);
  if (event === null || event === undefined) return null;
  return (
    <button
      type="button"
      className={inline ? "link-btn why-btn why-btn-inline" : "btn btn-sm btn-ghost why-btn"}
      aria-label={about ? tr.t("ui-why-about", { label: tr.t(label), what: about }) : undefined}
      onClick={(e) => {
        // A control inside a row that is itself tappable (the turn's card) only opens the panel.
        e.stopPropagation();
        void ask(event, { opener: e.currentTarget, factors });
      }}
      data-testid="why-button"
      data-event={event}
    >
      {tr.t(label)}
    </button>
  );
}
