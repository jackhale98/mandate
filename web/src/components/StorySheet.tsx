import { useGame } from "../state/gameStore";
import { formatDate } from "../i18n/format";
import { useSheetClose } from "./useSheetClose";
import { capital } from "./DecisionSheet";
import { StoryPromises } from "./asks/Asks";

/** The story so far (DEC-033): the chapter and its tension, storylines, and what happened. */
export function StorySheet() {
  const tr = useGame((s) => s.tr);
  const story = useGame((s) => s.story);
  const open = useGame((s) => s.openStory);
  const openDecision = useGame((s) => s.openDecision);
  const [closing, requestClose, sheetRef] = useSheetClose(() => void open(false));
  if (!story) return null;
  const ch = story.chapter;
  const tension = Math.round(story.tension * 100);
  return (
    <>
      <div className={closing ? "sheet-scrim sheet-closing" : "sheet-scrim"} onClick={requestClose} aria-hidden="true" />
      <aside ref={sheetRef} className={closing ? "sheet story-sheet sheet-closing" : "sheet story-sheet"} aria-labelledby="story-title" data-testid="story-sheet">
        <div className="sheet-head">
          <div>
            <h2 id="story-title">{tr.t("ui-story-so-far")}</h2>
            {ch && <p className="muted small">{tr.t("ui-chapter", { n: ch.index, kind: tr.t(`chapter-kind-${ch.kind}`) })}</p>}
          </div>
          <button type="button" className="btn btn-ghost" onClick={requestClose} aria-label={tr.t("ui-close")}>
            ✕
          </button>
        </div>
        <div className="sheet-body">
          <section className="card">
            <ul className="bars">
              <li>
                <span>{tr.t("ui-tension")}</span>
                <span className="bar" aria-hidden="true">
                  <span className="bar-fill bar-risk" style={{ width: `${Math.max(0, Math.min(100, tension))}%` }} />
                </span>
                <span className="num">{tension}</span>
              </li>
            </ul>
            {story.profile_key && <p className="muted small">{tr.t("ui-storyteller-label", { name: tr.t(story.profile_key) })}</p>}
          </section>
          {story.storylines.length > 0 && (
            <section className="card">
              <h3 className="sub">{tr.t("ui-storylines")}</h3>
              {story.storylines.map((s, i) => (
                <div key={i} className="storyline">
                  <h4 className="sub">{tr.t(s.title_key)}</h4>
                  <ol className="events small">
                    {s.stages.map((e, j) => (
                      <li key={j}>
                        <time className="muted">{formatDate(tr, e.date)}</time>
                        <span>{tr.event(e.text_key, e.args)}</span>
                      </li>
                    ))}
                  </ol>
                </div>
              ))}
            </section>
          )}
          <StoryPromises />
          {story.past.length > 0 && (
            <section className="card">
              <h3 className="sub">{tr.t("ui-decisions")}</h3>
              <ul className="plain-list">
                {story.past.map((d) => (
                  <li key={d.id}>
                    <button type="button" className="link-btn" onClick={() => void openDecision(d.id)}>
                      {capital(tr.event(d.title_key, d.args))}
                    </button>
                    {d.aftermath && <span className="muted small"> · {d.aftermath.by_default ? tr.t("ui-aftermath-default") : tr.event(d.aftermath.label_key, d.args)}</span>}
                  </li>
                ))}
              </ul>
            </section>
          )}
          <section className="card">
            <h3 className="sub">{tr.t("ui-recent")}</h3>
            <ol className="events small" data-testid="story-entries">
              {story.entries.map((e) => (
                <li key={e.id}>
                  <time className="muted">{formatDate(tr, e.date)}</time>
                  <span>{tr.event(e.text_key, e.args)}</span>
                </li>
              ))}
            </ol>
          </section>
        </div>
      </aside>
    </>
  );
}
