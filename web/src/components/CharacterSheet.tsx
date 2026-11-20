import { useLayoutEffect, useRef } from "react";
import type { SubjectView } from "../engine/protocol/SubjectView";
import type { PersonRefView } from "../engine/protocol/PersonRefView";
import { useGame } from "../state/gameStore";
import type { Translator } from "../i18n/fluent";
import { placeName } from "../places/labels";
import { useSheetClose } from "./useSheetClose";
import { PartyChip } from "./ElectionCharts";
import { Unknown, personName } from "./Knowing";

/** A memory's or favour's subject, in words (a bill, a seat). */
export function subjectText(tr: Translator, s: SubjectView | null): string {
  if (!s) return "";
  return tr.t(s.key, { ...(s.n === null ? {} : { n: s.n }), ...(s.post ? { post: tr.t(s.post.label_key) } : {}) });
}

/**
 * What kind of thing a subject is, for a memory line's selector
 * (`$subject_kind`): a bill (a policy's name, a title), a decision (its
 * title, a noun phrase), a seat, or none (the memory is about nothing in
 * particular, and the line leaves the subject out).
 */
export function subjectKind(s: SubjectView | null): "bill" | "decision" | "seat" | "none" {
  if (!s) return "none";
  if (s.key.startsWith("policy-")) return "bill";
  if (s.key.startsWith("subject-")) return "seat";
  return "decision";
}

/** A memory as a line: `key` is the sheet's (`memory-…`) or a speaker's quote of it (`scene-quote-…`). */
export function memoryText(tr: Translator, key: string, m: { subject: SubjectView | null; year: number }): string {
  return tr.t(key, { subject: subjectText(tr, m.subject), subject_kind: subjectKind(m.subject), year: String(m.year) });
}

/** The key of a speaker quoting a memory to the player's face (first person, no parenthesised year). */
export function quoteKey(memoryKey: string): string {
  return memoryKey.replace(/^memory-/, "scene-quote-");
}

/** Someone's role line: their office or occupation (and ward). */
export function roleText(tr: Translator, p: Pick<PersonRefView, "role_key">): string {
  return p.role_key ? tr.t(p.role_key) : "";
}

/**
 * A character sheet (DEC-031): who they are, how they see you, what you owe
 * each other, and what they remember. Opens over the place panel, with the
 * same sheet pattern (bottom sheet on phones, side panel from 900px).
 */
export function CharacterSheet() {
  const tr = useGame((s) => s.tr);
  const c = useGame((s) => s.character);
  const open = useGame((s) => s.openCharacter);
  const visit = useGame((s) => s.visit);
  const campaign = useGame((s) => s.campaign);
  const busy = useGame((s) => s.busy);
  const sheet = useRef<HTMLElement>(null);
  const [closing, requestClose, sheetRef] = useSheetClose(() => void open(null), sheet);
  useLayoutEffect(() => {
    if (sheet.current) sheet.current.scrollTop = 0;
  }, [c?.id]);
  if (!c) return null;
  return (
    <>
      <div className={closing ? "sheet-scrim character-scrim sheet-closing" : "sheet-scrim character-scrim"} onClick={requestClose} aria-hidden="true" />
      <aside ref={sheetRef} className={closing ? "sheet character-sheet sheet-closing" : "sheet character-sheet"} aria-labelledby="character-title" data-testid="character-sheet">
        <div className="sheet-head">
          <div>
            <h2 id="character-title">{personName(tr, c.name)}</h2>
            <p className="muted small">
              {roleText(tr, c)}
              {c.ward_number !== null && <> · {tr.t("ui-ward-n", { n: c.ward_number })}</>} · {tr.t(`tier-${c.tier}`)}
            </p>
          </div>
          <div className="sheet-tools">
            <button type="button" className="btn btn-ghost" onClick={requestClose} aria-label={tr.t("ui-close")}>
              ✕
            </button>
          </div>
        </div>
        <div className="sheet-body">
          <section className="card">
            <h3 className="sub">{tr.t("ui-character")}</h3>
            <p className="small">{tr.t("ui-age-home", { age: c.age, hood: c.home_neighborhood })}</p>
            {c.party !== null && campaign && <PartyChip parties={campaign.parties} party={c.party} />}
            {c.place && (
              <p className="small">
                {tr.t("ui-found-at", { place: placeName(tr, c.place) })}{" "}
                <button type="button" className="btn btn-sm btn-ghost" disabled={busy} onClick={() => c.place && void visit(c.place.location)}>
                  {tr.t("ui-go-here")}
                </button>
              </p>
            )}
            <div className="tags" data-testid="character-personality">
              {c.is_rival && <span className="chip chip-warn">{tr.t("ui-your-rival")}</span>}
              {c.is_broker && <span className="chip">{tr.t("ui-power-broker")}</span>}
              {c.personality.map((k) => (
                <span key={k} className="chip chip-muted">
                  {tr.t(k)}
                </span>
              ))}
            </div>
            {c.traits.length > 0 && (
              <p className="small">
                {tr.t("ui-traits")}: {c.traits.map((k) => tr.t(k)).join(" · ")}
              </p>
            )}
            {c.expertise_key && (
              <p className="small muted">
                {tr.t("ui-expertise")}: {tr.t(c.expertise_key)}
              </p>
            )}
            {c.goals && c.goals.length > 0 && <p className="small">{tr.t("knowledge-goals", { list: c.goals.map((g) => tr.t(g)).join(" · ") })}</p>}
            {(c.knowledge.level === "unknown" || c.knowledge.level === "heard") && (
              <p className="small" data-testid="character-unknown">
                <Unknown k={c.knowledge} text="knowledge-character-unknown" />
              </p>
            )}
          </section>
          {!c.is_player && (
            <section className="card" data-testid="character-relationship">
              <h3 className="sub">{tr.t("ui-relationship")}</h3>
              <ul className="plain small">
                <li>{tr.t(`opinion-${c.opinion_band}`)}</li>
                <li>{tr.t(`trust-${c.trust_band}`)}</li>
                {c.bonds.length > 0 && <li>{c.bonds.map((b) => tr.t(`bond-${b}`)).join(" · ")}</li>}
                {c.owes_you > 0 && <li>{tr.t("ui-owes-you", { n: c.owes_you })}</li>}
                {c.you_owe > 0 && <li>{tr.t("ui-you-owe-them", { n: c.you_owe })}</li>}
              </ul>
              {c.favours.length > 0 && (
                <>
                  <h4 className="sub">{tr.t("ui-favours")}</h4>
                  <ul className="plain small">
                    {c.favours.map((f, i) => (
                      <li key={i}>
                        {f.you_owe ? "→ " : "← "}
                        {tr.t(f.context_key, { subject: subjectText(tr, f.subject) })} ({f.since_year}){f.count > 1 ? ` ×${f.count}` : ""}
                      </li>
                    ))}
                  </ul>
                </>
              )}
              <h4 className="sub">{tr.t("ui-remembers")}</h4>
              {c.memories.length === 0 ? (
                <p className="muted small">{tr.t("ui-remembers-nothing")}</p>
              ) : (
                <ul className="plain small quotes" data-testid="character-memories">
                  {c.memories.map((m, i) => (
                    <li key={i} className={`memory memory-${m.tone}`}>
                      {memoryText(tr, m.key, m)}
                    </li>
                  ))}
                </ul>
              )}
            </section>
          )}
        </div>
      </aside>
    </>
  );
}
