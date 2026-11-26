// The career card (M6 note 23), at the end of a run (in the epilogue): the
// character and the world, the opening with its rating and the facts behind
// it, each race with its rating at the time and its result, the highest post
// and years served, the notable decisions and how each matter ended, how the
// run ended, and wins against the wins the ratings expected; the world code
// and the build. "Copy" gives the text block; "Share" the share sheet with an
// image, or saves the image and copies the text where sharing is missing.
// The view is the engine's (`CareerCardView`); nothing is computed here.
import { useState } from "react";
import type { CareerCardView } from "../../engine/protocol/CareerCardView";
import type { Translator } from "../../i18n/fluent";
import { useGame } from "../../state/gameStore";
import { cardImage } from "./cardImage";
import { arcLines, cardText, decisionLines, endedWhen, endingText, endingTitle, figureText, highestText, openingPlace, raceHead, raceOutcome, ratingText, whoText, worldText, yearsText } from "./cardText";
import { copyText, shareCard, type Shared } from "./share";
import "./card.css";

type Status = { kind: "idle" } | { kind: "copied" } | { kind: "copy-failed" } | { kind: "shared"; how: Shared };

const SHARED_KEY: Record<Shared, string | null> = {
  shared: "ui-card-shared",
  cancelled: null,
  saved: "ui-card-saved",
  "saved-only": "ui-card-saved-only",
  copied: "ui-card-copied",
  failed: "ui-card-copy-failed",
};

/** The card's sections, as the engine's view gives them. */
export function CardBody({ tr, c }: { tr: Translator; c: CareerCardView }) {
  const decisions = decisionLines(tr, c);
  const arcs = arcLines(tr, c);
  return (
    <div className="career-card" data-testid="career-card" data-unfinished={c.unfinished ? "true" : "false"}>
      <header className="career-card-head">
        <p className="career-card-kicker">
          {tr.t("ui-card")}
          {c.unfinished && (
            <span className="career-card-unfinished" data-testid="card-unfinished">
              {tr.t("ui-card-unfinished")}
            </span>
          )}
        </p>
        <h3 className="career-card-name">{c.name}</h3>
        <p className="muted">{whoText(tr, c)}</p>
        <p className="muted small">{worldText(tr, c)}</p>
      </header>
      <section>
        <h4 className="sub">{tr.t("ui-card-opening")}</h4>
        <p data-testid="card-opening">{openingPlace(tr, c)}</p>
        <p className="muted small" data-testid="card-opening-rating">
          {ratingText(tr, c.opening.rating)}
        </p>
      </section>
      <section>
        <h4 className="sub">{tr.t("ui-card-races")}</h4>
        {c.races.length === 0 ? (
          <p className="muted">{tr.t("ui-card-races-none")}</p>
        ) : (
          <ul className="plain-list" data-testid="card-races">
            {c.races.map((r, i) => (
              <li key={i} data-outcome={r.outcome}>
                <strong>{raceHead(tr, r)}</strong>: {raceOutcome(tr, r)}
                <br />
                <span className="muted small">{ratingText(tr, r.rating)}</span>
              </li>
            ))}
          </ul>
        )}
      </section>
      <section>
        <h4 className="sub">{tr.t("ui-card-highest")}</h4>
        <p data-testid="card-highest">
          {highestText(tr, c)} · {yearsText(tr, c)}
        </p>
      </section>
      <section>
        <h4 className="sub">{tr.t("ui-card-decisions")}</h4>
        {decisions.length === 0 ? (
          <p className="muted">{tr.t("ui-card-decisions-none")}</p>
        ) : (
          <ul className="plain-list small">
            {decisions.map((d, i) => (
              <li key={i}>{d}</li>
            ))}
          </ul>
        )}
      </section>
      <section>
        <h4 className="sub">{tr.t("ui-card-arcs")}</h4>
        {arcs.length === 0 ? (
          <p className="muted">{tr.t("ui-card-arcs-none")}</p>
        ) : (
          <ul className="plain-list small" data-testid="card-arcs">
            {arcs.map((a, i) => (
              <li key={i}>{a}</li>
            ))}
          </ul>
        )}
      </section>
      <section>
        <h4 className="sub">{tr.t("ui-card-ended")}</h4>
        <p data-testid="card-ended">
          <strong>{endingTitle(tr, c)}</strong>, {endedWhen(tr, c)}
        </p>
        <p className="muted small">{endingText(tr, c)}</p>
      </section>
      <section className="career-card-figure">
        <h4 className="sub">{tr.t("ui-card-figure")}</h4>
        <p data-testid="card-figure">{figureText(tr, c)}</p>
      </section>
      <section>
        <h4 className="sub">{tr.t("ui-card-code")}</h4>
        {c.code ? (
          <>
            <p className="career-card-code" data-testid="card-code">
              {c.code}
            </p>
            <p className="hint">{tr.t("ui-card-code-help")}</p>
          </>
        ) : (
          <p className="muted">{tr.t("ui-card-code-none")}</p>
        )}
        <p className="muted small" data-testid="card-build">
          {tr.t("ui-card-build", { build: c.build })}
        </p>
      </section>
    </div>
  );
}

/** The card with its Copy and Share buttons (loaded from the engine once the act is over). */
export function CareerCard() {
  const tr = useGame((s) => s.tr);
  const c = useGame((s) => s.careerCard);
  const loading = useGame((s) => s.cardLoading);
  const progress = useGame((s) => s.progress);
  const [status, setStatus] = useState<Status>({ kind: "idle" });
  if (!c) {
    return loading ? (
      <section className="card" role="status" aria-live="polite" data-testid="card-loading">
        <p>{tr.t(progress?.stage ?? "ui-card-loading")}</p>
        {progress && <progress max={1} value={progress.fraction} />}
      </section>
    ) : null;
  }
  const text = cardText(tr, c);
  const title = tr.t(c.unfinished ? "ui-card-text-title-unfinished" : "ui-card-text-title");
  const copy = async () => setStatus((await copyText(text)) ? { kind: "copied" } : { kind: "copy-failed" });
  const share = async () => setStatus({ kind: "shared", how: await shareCard(text, title, () => cardImage(tr, c)) });
  const said = status.kind === "copied" ? "ui-card-copied" : status.kind === "copy-failed" ? "ui-card-copy-failed" : status.kind === "shared" ? SHARED_KEY[status.how] : null;
  const failed = status.kind === "copy-failed" || (status.kind === "shared" && status.how === "failed");
  return (
    <section className="card career-card-wrap" aria-label={tr.t("ui-card")}>
      <CardBody tr={tr} c={c} />
      <div className="career-card-actions">
        <button type="button" className="btn btn-primary" onClick={() => void copy()} data-testid="card-copy">
          {tr.t("ui-card-copy")}
        </button>
        <button type="button" className="btn" onClick={() => void share()} data-testid="card-share">
          {tr.t("ui-card-share")}
        </button>
      </div>
      <p className="small" aria-live="polite" data-testid="card-status" data-status={status.kind === "shared" ? status.how : status.kind}>
        {said ? tr.t(said) : ""}
      </p>
      {failed && <textarea className="input career-card-text" readOnly rows={8} value={text} aria-label={tr.t("ui-card")} data-testid="card-text" />}
    </section>
  );
}
