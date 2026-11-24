import type { ArticleView } from "../engine/protocol/ArticleView";
import type { NewspaperView } from "../engine/protocol/NewspaperView";
import type { OutletView } from "../engine/protocol/OutletView";
import type { LegalView } from "../engine/protocol/LegalView";
import { useGame } from "../state/gameStore";
import type { Translator } from "../i18n/fluent";
import { formatDate } from "../i18n/format";
import { WhyButton } from "./why/WhyButton";

/** An outlet's name (a town paper is named after its office, DEC-079). */
export function outletName(tr: Translator, o: OutletView): string {
  return tr.t(o.name_key, { outlet_place: o.place });
}

/** A story's headline (facts and Fluent keys from the engine, DEC-054). */
export function headline(tr: Translator, a: ArticleView): string {
  return tr.event(a.headline_key, a.args);
}

/**
 * The newspaper (DEC-034, DEC-079: at the newspaper office): what the outlets
 * that reach town are running, with who ran it and how they framed it.
 * Coverage is the outlets' choice, not the truth.
 */
export function Newspaper({ n }: { n: NewspaperView }) {
  const tr = useGame((s) => s.tr);
  return (
    <section className="card" data-testid="newspaper">
      <h2>{tr.t("service-newspaper")}</h2>
      <p className="muted small">
        {n.outlets.map((o) => `${outletName(tr, o)} (${tr.t(`credibility-${o.credibility}`)})`).join(" · ")}
      </p>
      {n.articles.length === 0 ? (
        <p className="muted small">{tr.t("ui-newspaper-empty")}</p>
      ) : (
        <ul className="plain-list articles">
          {n.articles.map((a) => (
            <li key={a.id} className={a.about_you ? "article article-you" : "article"} data-testid="article">
              <p className="article-head">
                <strong>{headline(tr, a)}</strong>
              </p>
              <p className="muted small">
                {outletName(tr, a.outlet)} · {formatDate(tr, a.date)} · <span className={`chip chip-framing chip-${a.framing}`}>{tr.t(`framing-${a.framing}`)}</span>
                {a.about_you && <> · {tr.t("ui-about-you")}</>}
                {a.event !== undefined && <> · </>}
                <WhyButton event={a.event} about={headline(tr, a)} inline />
              </p>
            </li>
          ))}
        </ul>
      )}
      <p className="hint">{tr.t("ui-newspaper-note")}</p>
    </section>
  );
}

/** The latest headlines, coarse (the pocket; the newspaper has the rest). */
export function Headlines({ news }: { news: ArticleView[] }) {
  const tr = useGame((s) => s.tr);
  if (news.length === 0) return null;
  return (
    <section data-testid="pocket-news">
      <h3 className="sub">{tr.t("ui-news")}</h3>
      <ul className="plain small">
        {news.map((a) => (
          <li key={a.id}>
            {headline(tr, a)} <span className="muted">· {outletName(tr, a.outlet)}</span>
          </li>
        ))}
      </ul>
    </section>
  );
}

/** Scandals and cases (DEC-006: a visible track, with warnings in the alerts). */
export function Legal({ l }: { l: LegalView }) {
  const tr = useGame((s) => s.tr);
  if (l.scandals.length === 0 && l.cases.length === 0 && l.run.status === "active") return null;
  return (
    <section className="card" data-testid="pocket-legal">
      <h2>{tr.t("ui-legal")}</h2>
      {l.scandals.length > 0 && (
        <>
          <h3 className="sub">{tr.t("ui-scandals")}</h3>
          <ul className="plain small">
            {l.scandals.map((s) => (
              <li key={s.id} data-testid="scandal">
                {tr.t("ui-scandal-line", {
                  fact: tr.t(s.fact_key),
                  severity: tr.t(`severity-${s.severity}`),
                  date: formatDate(tr, s.broke),
                })}{" "}
                <span className="muted">
                  · {s.running ? tr.t(`heat-${s.heat}`) : tr.t("ui-scandal-over")}
                  {s.response && <> · {tr.t(`response-${s.response.replace(/_/g, "-")}`)}</>}
                </span>{" "}
                <WhyButton event={s.event} about={tr.t(s.fact_key)} inline />
              </li>
            ))}
          </ul>
        </>
      )}
      {l.cases.length > 0 && (
        <>
          <h3 className="sub">{tr.t("ui-cases")}</h3>
          <ul className="plain small">
            {l.cases.map((c) => (
              <li key={c.id} data-testid="case">
                {tr.t("ui-case-line", { fact: tr.t(c.fact_key), stage: tr.t(`case-stage-${c.stage}`) })}
                {c.next && <span className="muted"> · {tr.t("ui-case-next", { date: formatDate(tr, c.next) })}</span>}
                {c.odds && <span className="muted"> · {tr.t(`ui-case-odds-${c.stage === "investigation" ? "charge" : "convict"}`, { risk: tr.t(`risk-${c.odds}`) })}</span>}
                {c.sentence && <span> · {tr.t(`sentence-${c.sentence}`)}{c.years > 0 ? ` (${tr.t("ui-years", { n: c.years })})` : ""}</span>}{" "}
                <WhyButton event={c.event} about={tr.t(c.fact_key)} inline />
              </li>
            ))}
          </ul>
        </>
      )}
    </section>
  );
}

/** In prison, or the career is over (DEC-006): said plainly, with its cause. */
export function RunBanner({ l }: { l: LegalView }) {
  const tr = useGame((s) => s.tr);
  if (l.run.status === "active") return null;
  const fact = l.run.fact_key ? tr.t(l.run.fact_key) : "";
  return (
    <div className={l.run.status === "ended" ? "run-banner run-ended" : "run-banner"} role="status" data-testid="run-banner">
      {l.run.status === "imprisoned" && l.run.until ? (
        <p>
          <strong>{tr.t("ui-in-prison", { date: formatDate(tr, l.run.until) })}</strong> {tr.t("ui-in-prison-why", { fact, n: l.run.years })}
        </p>
      ) : (
        <>
          <h2>{tr.t("ui-career-over")}</h2>
          <p>{tr.t("ui-career-over-why", { fact, reason: tr.t(`ending-${l.run.reason ?? "prison"}`), n: l.run.years })}</p>
          <p className="muted small">{tr.t("ui-career-over-note")}</p>
        </>
      )}
    </div>
  );
}
