// The Notebook's Town tab (DEC-113; DEC-077 §1): the town's character
// (industry, crisis, who runs things, the rival and their plan, priorities),
// its headline numbers, gossip, the loose ends the player picked up and the
// latest news, each once. Everything is as the player knows it (DEC-091): what
// is not known is simply not listed (no placeholders, nothing counted); a lead
// shows on the thing it is about, and those picked up in conversation are
// together under "Loose ends". The neighbourhoods are rows in Places. The UI
// never decides what is known.
import type { ArcLeadView } from "../engine/protocol/ArcLeadView";
import type { LeadView } from "../engine/protocol/LeadView";
import type { PersonRefView } from "../engine/protocol/PersonRefView";
import type { PocketView } from "../engine/protocol/PocketView";
import type { RivalPlanView } from "../engine/protocol/RivalPlanView";
import type { TownCharacterView } from "../engine/protocol/TownCharacterView";
import type { Translator } from "../i18n/fluent";
import { formatMoney, formatPct } from "../i18n/format";
import { useGame } from "../state/gameStore";
import { roleText } from "./CharacterSheet";
import { LeadLine, personName } from "./Knowing";
import { Headlines } from "./Newspaper";
import { StatTile } from "./StatTile";
import { Fold, Row, RowList } from "./notebook/rows";
import { ArcLead, Lead } from "./notebook/Leads";
import { itemFacts, itemTitle, type NotebookItem } from "./notebook/notebookModel";

/** Someone in a list: their name as known (a role for a stranger), opening their sheet. */
export function Who({ p }: { p: PersonRefView }) {
  const tr = useGame((s) => s.tr);
  const busy = useGame((s) => s.busy);
  const openCharacter = useGame((s) => s.openCharacter);
  return (
    <button type="button" className="link-btn" disabled={busy || p.id === null} onClick={() => p.id !== null && void openCharacter(p.id)}>
      {personName(tr, p.name)}
      {p.rumoured && <span className="muted small"> ({tr.t("knowledge-rumoured")})</span>}
      <span className="muted small"> · {roleText(tr, p)}</span>
    </button>
  );
}

/** What the rival is planning, as far as the player knows (`rival-plan-*`). */
export function rivalPlanText(tr: Translator, plan: RivalPlanView, rival: PersonRefView | null): string {
  // A districted seat by its number, any other post by its constitution label.
  const p = plan.post;
  const seat = !p ? "" : p.constituency_number !== null ? tr.t("rival-seat-ward", { ward: p.constituency_number }) : tr.t("rival-seat-office", { post: tr.t(p.label_key) });
  return tr.t(plan.key, { rival: personName(tr, rival?.name ?? ""), seat, focus: plan.focus_key ? tr.t(plan.focus_key) : "" });
}

function signed(v: number): string {
  const n = Math.round(v * 100);
  return n > 0 ? `+${n}` : String(n);
}

/** The town's character (DEC-077 §1): what it lives on, what it is going through, and who matters. */
function Character({ c }: { c: TownCharacterView }) {
  const tr = useGame((s) => s.tr);
  return (
    <section className="card town-card" data-testid="town-card" aria-labelledby="town-card-title">
      <h2 id="town-card-title">{tr.t("ui-town-card")}</h2>
      <dl className="facts">
        <div>
          <dt>{tr.t("ui-defining-industry")}</dt>
          <dd data-testid="town-industry">{c.industry ? `${tr.t(c.industry.name_key)} · ${tr.t("ui-industry-share", { pct: Math.round(c.industry_share * 100) })}` : tr.t("ui-nobody")}</dd>
        </div>
        <div>
          <dt>{tr.t("ui-live-crisis")}</dt>
          <dd data-testid="town-crisis">
            {c.crisis ? (
              <>
                <strong>{tr.t(c.crisis.name_key)}</strong> · {tr.t(`crisis-severity-${c.crisis.severity_band}`)}, {tr.t(`crisis-trend-${c.crisis.trend}`)}{" "}
                <span className="muted small">({tr.t("ui-crisis-since", { month: c.crisis.since.month, year: String(c.crisis.since.year) })})</span>
              </>
            ) : (
              <span className="muted">{tr.t("ui-no-crisis")}</span>
            )}
          </dd>
        </div>
        {(c.broker || c.broker_knowledge.level !== "unknown") && (
          <div>
            <dt>{tr.t("ui-power-broker-line")}</dt>
            <dd data-testid="town-broker">{c.broker ? <Who p={c.broker} /> : tr.t("ui-nobody")}</dd>
          </div>
        )}
        <div>
          <dt>{tr.t("ui-rival-line")}</dt>
          <dd data-testid="town-rival">
            {c.rival ? <Who p={c.rival} /> : tr.t("ui-nobody")}
            {c.rival_plan && (
              <span className="town-rival-plan small" data-testid="town-rival-plan">
                {rivalPlanText(tr, c.rival_plan, c.rival)}
                {c.rival_plan.knowledge.level !== "detailed" && <LeadLine lead={c.rival_plan.knowledge.lead} />}
              </span>
            )}
          </dd>
        </div>
        {(c.priorities.length > 0 || c.priorities_knowledge.level !== "unknown") && (
          <div>
            <dt>{tr.t("ui-priorities")}</dt>
            <dd data-testid="town-priorities">{c.priorities.length > 0 ? c.priorities.map((p) => tr.t(p.name_key)).join(" · ") : tr.t("ui-nobody")}</dd>
          </div>
        )}
      </dl>
    </section>
  );
}

/** Headline numbers, coarse and last month's (DEC-079 rule 3). */
function Figures({ p }: { p: PocketView }) {
  const tr = useGame((s) => s.tr);
  const h = p.headlines;
  return (
    <section data-testid="town-headlines" aria-labelledby="town-headlines-title">
      <h3 className="sub" id="town-headlines-title">
        {tr.t("ui-headlines")}
      </h3>
      <div className="stats stats-2">
        <StatTile label={tr.t("ui-unemployment")} value={formatPct(h.unemployment, 0)} />
        <StatTile label={tr.t("ui-rent")} value={formatMoney(h.monthly_rent)} />
        <StatTile label={tr.t("ui-gov-approval")} value={signed(h.gov_approval)} />
        <StatTile label={tr.t("ui-you-in-ward")} value={`${Math.round(h.awareness_ward * 100)}%`} delta={{ text: `${tr.t("ui-awareness")} · ${signed(h.approval_ward)} ${tr.t("ui-approval")}`, good: null }} />
      </div>
      {h.top_concern_keys.length > 0 && <p className="small">{tr.t("ui-top-concerns", { list: h.top_concern_keys.map((k) => tr.t(k)).join(", ") })}</p>}
      <p className="hint">{tr.t("ui-headlines-note")}</p>
    </section>
  );
}

/** Gossip and other things known about the town that the card does not already say. */
function Talk({ items }: { items: NotebookItem[] }) {
  const tr = useGame((s) => s.tr);
  if (items.length === 0) return null;
  return (
    <Fold id="town-talk" title={tr.t("ui-town-talk")} count={items.length} testId="notebook-town-talk">
      <RowList className="nb-entry-list">
        {items.map((i) => (
          <Row key={i.key} title={itemTitle(tr, i)} sub={itemFacts(tr, i).join(" · ") || null} li={{ "data-testid": "notebook-entry", "data-subject": i.subject.type, "data-new": i.isNew ? "true" : undefined }} />
        ))}
      </RowList>
    </Fold>
  );
}

/** Loose ends: the leads picked up in conversation and the paper, and what is going on somewhere (an arc only heard of). */
function LooseEnds({ leads, arcs }: { leads: LeadView[]; arcs: ArcLeadView[] }) {
  const tr = useGame((s) => s.tr);
  const n = leads.length + arcs.length;
  if (n === 0) return null;
  return (
    <Fold id="town-leads" title={tr.t("ui-loose-ends")} count={n} testId="notebook-loose-ends">
      <div className="rw-pad" data-testid="notebook-leads-open">
        {arcs.map((l) => (
          <ArcLead key={`arc-${l.arc}`} lead={l} />
        ))}
        {leads.map((l, i) => (
          <Lead key={i} lead={l} />
        ))}
      </div>
    </Fold>
  );
}

/** The Town tab. `items`: the notebook's entries filed under Town; `leads`, `arcs`: loose ends. */
export function TownTab({ items = [], leads = [], arcs = [] }: { items?: NotebookItem[]; leads?: LeadView[]; arcs?: ArcLeadView[] }) {
  const tr = useGame((s) => s.tr);
  const town = useGame((s) => s.town);
  const p = useGame((s) => s.pocket);
  if (!town || !p) return null;
  // The card says who runs things and what the rival plans; the town's worries are in the numbers.
  const talk = items.filter((i) => i.subject.type === "secret");
  return (
    <>
      <h3 className="pocket-tab-title">
        {town.municipality} <span className="muted small">· {tr.t("ui-town")}</span>
      </h3>
      {town.character && <Character c={town.character} />}
      <Figures p={p} />
      <LooseEnds leads={leads} arcs={arcs} />
      <Talk items={talk} />
      {p.news.length > 0 && (
        <Fold id="town-news" title={tr.t("ui-news")} count={p.news.length}>
          <div className="rw-pad">
            <Headlines news={p.news} />
          </div>
        </Fold>
      )}
    </>
  );
}
