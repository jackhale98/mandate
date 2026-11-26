import { useGame } from "../state/gameStore";
import { Loader } from "../components/Brand";
import { formatDate } from "../i18n/format";
import type { Translator } from "../i18n/fluent";
import type { DraftView } from "../engine/protocol/DraftView";
import type { OpeningView } from "../engine/protocol/OpeningView";

/**
 * The words of an opening's tag (what the next draft is varied against): the
 * matter (an industry's sector, else a hazard), the town's class and the post.
 * Labels only; the tag itself is the engine's.
 */
export function tagWords(tr: Translator, o: OpeningView): string[] {
  const m = o.tag.matter;
  const matter = m.includes(".") ? m.replace(".", "-") : `hazard-${m}`;
  return [matter, `class-${o.tag.town_class}`, o.post?.role_key ?? ""].filter((k) => k !== "" && tr.has(k)).map((k) => tr.t(k));
}

/** The seat the player would hold, in the post's own words (its role word, and its district when it has one). */
function seatLine(tr: Translator, o: OpeningView): string | null {
  if (!o.post) return null;
  const role = tr.t(o.post.role_key);
  return o.post.constituency_number !== null ? tr.t("ui-draft-seat-district", { role, n: String(o.post.constituency_number) }) : tr.t("ui-draft-seat", { role });
}

/** One opening: a real town and a real situation, with its line (built from facts on the day play begins). */
function OpeningCard({ o, index, onChoose, busy }: { o: OpeningView; index: number; onChoose: (i: number) => void; busy: boolean }) {
  const tr = useGame((s) => s.tr);
  const seat = seatLine(tr, o);
  return (
    <li className="card draft-opening" data-testid="draft-opening" data-template={o.template} data-town={o.town}>
      <header className="draft-head">
        <h2 className="draft-town">{o.town_name}</h2>
        <span className="draft-title muted">{tr.event(o.title_key, o.args)}</span>
      </header>
      {o.pitch_key && (
        <p className="draft-pitch" data-testid="draft-pitch">
          {tr.event(o.pitch_key, o.args)}
        </p>
      )}
      {seat && (
        <p className="draft-seat small" data-testid="draft-seat">
          {seat}
        </p>
      )}
      <ul className="draft-tags" aria-label={tr.t("ui-draft-tags")}>
        {tagWords(tr, o).map((w) => (
          <li key={w} className="chip">
            {w}
          </li>
        ))}
      </ul>
      <button type="button" className={index === 0 ? "btn btn-primary btn-block" : "btn btn-block"} disabled={busy} onClick={() => onChoose(index)} data-testid="draft-choose">
        {tr.t("ui-draft-choose", { town: o.town_name })}
      </button>
    </li>
  );
}

/** The openings once the audition is done. */
export function DraftOpenings({ draft, busy, onChoose }: { draft: DraftView; busy: boolean; onChoose: (i: number) => void }) {
  const tr = useGame((s) => s.tr);
  if (draft.openings.length === 0) return <p className="muted" data-testid="draft-none">{tr.t("ui-draft-none")}</p>;
  return (
    <>
      <p className="muted small" data-testid="draft-date">
        {tr.t("ui-draft-opens", { date: formatDate(tr, draft.opens) })}
      </p>
      <ol className="draft-list" data-testid="draft-list">
        {draft.openings.map((o, i) => (
          <OpeningCard key={`${o.town}-${o.template}`} o={o} index={i} onChoose={onChoose} busy={busy} />
        ))}
      </ol>
    </>
  );
}

/**
 * The draft (DEC-101 §3; STAGED_ARCS §6.6): the new world's openings, best first, each a town, the seat the player
 * would hold, the situation's line and its tag. The engine auditions them one fork a message, so the screen shows
 * progress while it works; it computes nothing itself.
 */
export function Draft() {
  const tr = useGame((s) => s.tr);
  const draft = useGame((s) => s.draft);
  const progress = useGame((s) => s.draftProgress);
  const busy = useGame((s) => s.busy);
  const choose = useGame((s) => s.chooseOpening);
  const reroll = useGame((s) => s.rerollDraft);
  const leave = useGame((s) => s.leaveDraft);
  const working = draft === null;
  return (
    <main className="title draft" data-testid="draft">
      <header className="draft-hero">
        <h1>{tr.t("ui-draft-heading")}</h1>
        <p className="muted">{tr.t("ui-draft-intro")}</p>
      </header>
      {working ? (
        <section className="card draft-working" role="status" aria-live="polite" data-testid="draft-progress">
          <Loader compact decorative />
          <span>{tr.t("progress-draft")}</span>
          <progress max={Math.max(1, progress?.total ?? 1)} value={progress?.done ?? 0} />
        </section>
      ) : (
        <DraftOpenings draft={draft} busy={busy} onChoose={(i) => void choose(i)} />
      )}
      <div className="draft-actions">
        {!working && draft.openings.length > 0 && (
          <button type="button" className="btn" disabled={busy} onClick={() => void choose(0)} data-testid="draft-surprise">
            {tr.t("ui-draft-surprise")}
          </button>
        )}
        <button type="button" className="btn" disabled={busy && !working} onClick={() => void reroll()} data-testid="draft-reroll">
          {tr.t("ui-draft-reroll")}
        </button>
        <button type="button" className="btn btn-ghost" onClick={leave} data-testid="draft-back">
          {tr.t("ui-back")}
        </button>
      </div>
    </main>
  );
}
