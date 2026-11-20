// The notebook (DEC-091 "the notebook"; M6 K5): what the player knows about
// the town, organised for finding things rather than as a log. Its own tabs
// (People, Places, Worries by neighbourhood, Policies, Leads) keep each list
// short on a phone; People is the single place for people (./People: the
// key figures pinned, then everyone else, each with Talk and Go to them). Every entry shows its level (in dots and in words, never
// colour alone, DEC-051), where and when it was learned; placeholders show
// what is still unknown with the lead that would reveal it, so the notebook
// doubles as an exploration guide. Opening it tells the engine the new
// entries have been seen (the badge clears); they stay marked while the
// player looks. The engine decides what is known; this only arranges it.
import { useEffect, useMemo, useRef, useState, type KeyboardEvent } from "react";
import type { LeadView } from "../../engine/protocol/LeadView";
import type { PocketView } from "../../engine/protocol/PocketView";
import { formatDate } from "../../i18n/format";
import { placeName } from "../../places/labels";
import { useGame } from "../../state/gameStore";
import { firstNewTab, groupNotebook, itemFacts, itemText, itemTitle, leadText, levelDots, matches, newCounts, NOTEBOOK_TABS, type NotebookItem, type NotebookTab } from "./notebookModel";
import { PeopleSection, loadSheets as engineSheets, type SheetLoader } from "./People";
import "./notebook.css";

const ICON: Record<NotebookTab, string> = { people: "👤", places: "📍", issues: "💬", policies: "📜", leads: "🧭" };
const tabId = (t: NotebookTab) => `notebook-tab-${t}`;
const panelId = (t: NotebookTab) => `notebook-panel-${t}`;

/** A lead: what it would tell, and a way there (auto-routed, DEC-079). */
export function Lead({ lead }: { lead: LeadView }) {
  const tr = useGame((s) => s.tr);
  const busy = useGame((s) => s.busy);
  const visit = useGame((s) => s.visit);
  const to = lead.route?.to ?? lead.to;
  return (
    <div className="nb-lead" data-testid="notebook-lead">
      <p>
        <span className="nb-lead-label">{tr.t("ui-notebook-lead")}: </span>
        {leadText(tr, lead)} <span className="muted small">({tr.t("ui-notebook-lead-gives", { level: tr.t(`knowledge-level-${lead.level}`) })})</span>
      </p>
      {to &&
        (lead.route?.here ? (
          <span className="muted small">{tr.t("ui-notebook-here")}</span>
        ) : (
          <button type="button" className="btn btn-sm" disabled={busy} onClick={() => void visit(to.location)} data-testid="notebook-lead-go">
            {tr.t("ui-notebook-go")}: {placeName(tr, to)}
          </button>
        ))}
    </div>
  );
}

function Item({ item }: { item: NotebookItem }) {
  const tr = useGame((s) => s.tr);
  const busy = useGame((s) => s.busy);
  const openCharacter = useGame((s) => s.openCharacter);
  const e = item.entry;
  const last = e?.history[e.history.length - 1];
  const title = itemTitle(tr, item);
  const facts = itemFacts(tr, item);
  const person = item.subject.type === "person" ? item.subject.person : null;
  // The steps are worth listing only when they came from different places or days.
  const steps = new Set(e?.history.map((h) => `${h.key}|${JSON.stringify(h.args)}|${h.on.day_number}`)).size;
  return (
    <li className={e ? "nb-item" : "nb-item nb-gap"} data-testid={e ? "notebook-entry" : "notebook-gap"} data-subject={item.subject.type} data-level={item.level} data-new={item.isNew ? "true" : undefined}>
      <div className="nb-head">
        <h4 className="nb-title">{title}</h4>
        {item.isNew && (
          <span className="badge nb-new" data-testid="notebook-new">
            {tr.t("ui-notebook-new")}
          </span>
        )}
      </div>
      <p className="nb-level small">
        <span className="nb-dots" aria-hidden="true">
          {levelDots(item.level)}
        </span>{" "}
        {tr.t(`knowledge-level-${item.level}`)}
        {last && (
          <span className="muted nb-from" data-testid="notebook-from">
            {" · "}
            {tr.t("ui-notebook-from", { source: tr.event(last.key, last.args), date: formatDate(tr, last.on) })}
          </span>
        )}
        {e?.as_of && (
          <span className="muted">
            {" · "}
            {tr.t("knowledge-as-of", { month: e.as_of.month, year: e.as_of.year })}
            {e.stale && <> · {tr.t("knowledge-stale")}</>}
          </span>
        )}
      </p>
      {facts.map((f, i) => (
        <p key={i} className="nb-fact">
          {f}
        </p>
      ))}
      {e && steps > 1 && (
        <details className="nb-history">
          <summary>{tr.t("ui-notebook-history")}</summary>
          <ol>
            {e.history.map((h, i) => (
              <li key={i} className="small">
                {tr.t(`knowledge-level-${h.level}`)}: {tr.t("ui-notebook-from", { source: tr.event(h.key, h.args), date: formatDate(tr, h.on) })}
              </li>
            ))}
          </ol>
        </details>
      )}
      {item.lead && <Lead lead={item.lead} />}
      {person !== null && (
        <button type="button" className="link-btn small" disabled={busy} onClick={() => void openCharacter(person)}>
          {tr.t("ui-notebook-person-open", { name: title })}
        </button>
      )}
    </li>
  );
}

function ItemList({ items, testId }: { items: NotebookItem[]; testId?: string }) {
  return (
    <ul className="nb-list" data-testid={testId}>
      {items.map((i) => (
        <Item key={i.key} item={i} />
      ))}
    </ul>
  );
}

/** The notebook page: the pocket's Notebook tab. `loadSheets`: the People cards' character sheets (the engine's; tests pass their own). */
export function Notebook({ loadSheets = engineSheets }: { loadSheets?: SheetLoader }) {
  const tr = useGame((s) => s.tr);
  const notebook = useGame((s) => s.notebook);
  const fresh = useGame((s) => s.notebookFresh);
  const pocket = useGame((s) => s.pocket);
  const busy = useGame((s) => s.busy);
  const loadNotebook = useGame((s) => s.loadNotebook);
  const seeNotebook = useGame((s) => s.seeNotebook);
  const leaveNotebook = useGame((s) => s.leaveNotebook);
  const [choice, setChoice] = useState<{ tab: NotebookTab; sig: string } | null>(null);
  const [q, setQ] = useState("");
  const refs = useRef<Partial<Record<NotebookTab, HTMLButtonElement | null>>>({});

  useEffect(() => {
    void loadNotebook();
    return () => leaveNotebook();
  }, [loadNotebook, leaveNotebook]);

  // Looking is seeing: once per pocket view with something new, tell the engine.
  const tried = useRef<PocketView | null>(null);
  useEffect(() => {
    if (!pocket || pocket.notebook_new === 0 || busy || tried.current === pocket) return;
    tried.current = pocket;
    void seeNotebook();
  }, [pocket, busy, seeNotebook]);

  const groups = useMemo(() => (notebook ? groupNotebook(notebook, new Set(fresh)) : null), [notebook, fresh]);

  // Land where the news is: the player's own choice holds until something new arrives.
  const sig = fresh.join(",");
  const landing = groups ? firstNewTab(groups) : null;
  const tab: NotebookTab = choice && (choice.sig === sig || !landing) ? choice.tab : (landing ?? choice?.tab ?? "people");
  const setTab = (t: NotebookTab) => setChoice({ tab: t, sig });

  const texts = useMemo(() => {
    if (!groups) return null;
    const m = new Map<NotebookItem, string>();
    for (const t of ["people", "places", "issues", "policies"] as const) for (const i of groups[t]) m.set(i, itemText(tr, i));
    return m;
  }, [groups, tr]);

  if (!notebook || !groups || !texts) return <p className="muted small">…</p>;

  const counts = newCounts(groups);
  const freshCount = counts.people + counts.places + counts.issues + counts.policies;
  const searching = q.trim() !== "";
  const keep = (i: NotebookItem) => matches(texts.get(i) ?? "", q);
  const leadKeep = (l: LeadView) => matches(leadText(tr, l).toLowerCase(), q);
  const hits: Record<NotebookTab, number> = {
    people: groups.people.filter(keep).length,
    places: groups.places.filter(keep).length,
    issues: groups.issues.filter(keep).length,
    policies: groups.policies.filter(keep).length,
    leads: groups.leads.open.filter(leadKeep).length + groups.leads.standing.filter(leadKeep).length,
  };
  const elsewhere = NOTEBOOK_TABS.filter((t) => t !== tab && hits[t] > 0);

  const onKey = (e: KeyboardEvent) => {
    const i = NOTEBOOK_TABS.indexOf(tab);
    const n = NOTEBOOK_TABS.length;
    const next = e.key === "ArrowRight" ? NOTEBOOK_TABS[(i + 1) % n] : e.key === "ArrowLeft" ? NOTEBOOK_TABS[(i + n - 1) % n] : e.key === "Home" ? NOTEBOOK_TABS[0] : e.key === "End" ? NOTEBOOK_TABS[n - 1] : undefined;
    if (!next) return;
    e.preventDefault();
    setTab(next);
    refs.current[next]?.focus();
  };

  const body = () => {
    if (tab === "leads") {
      const open = groups.leads.open.filter(leadKeep);
      const standing = groups.leads.standing.filter(leadKeep);
      return (
        <>
          <p className="hint">{tr.t("ui-notebook-leads-help")}</p>
          {open.length + standing.length === 0 && <p className="muted small">{searching ? tr.t("ui-notebook-filter-none", { q }) : tr.t("ui-notebook-empty-leads")}</p>}
          {open.length > 0 && (
            <section aria-labelledby="nb-leads-open">
              <h3 className="sub" id="nb-leads-open">
                {tr.t("ui-notebook-leads-open")}
              </h3>
              <ul className="nb-list" data-testid="notebook-leads-open">
                {open.map((l, i) => (
                  <li key={i} className="nb-item">
                    <Lead lead={l} />
                    <p className="muted small">{formatDate(tr, l.since)}</p>
                  </li>
                ))}
              </ul>
            </section>
          )}
          {standing.length > 0 && (
            <section aria-labelledby="nb-leads-standing">
              <h3 className="sub" id="nb-leads-standing">
                {tr.t("ui-notebook-leads-standing")}
              </h3>
              <ul className="nb-list" data-testid="notebook-leads-standing">
                {standing.map((l, i) => (
                  <li key={i} className="nb-item">
                    <Lead lead={l} />
                  </li>
                ))}
              </ul>
            </section>
          )}
        </>
      );
    }
    const items = groups[tab].filter(keep);
    // People: the single place for people (the key figures pinned, everyone else known, then power and placeholders).
    if (tab === "people") return <PeopleSection items={items} q={q} Rest={ItemList} load={loadSheets} />;
    if (items.length === 0) return <p className="muted small">{searching ? tr.t("ui-notebook-filter-none", { q }) : tr.t(`ui-notebook-empty-${tab}`)}</p>;
    if (tab === "issues") {
      const town = items.filter((i) => i.subject.type === "town_concerns");
      const hoods = items.filter((i) => i.subject.type === "hood_concerns");
      return (
        <>
          {town.length > 0 && (
            <section aria-labelledby="nb-issues-town">
              <h3 className="sub" id="nb-issues-town">
                {tr.t("ui-notebook-town-worries")}
              </h3>
              <ItemList items={town} />
            </section>
          )}
          {hoods.length > 0 && (
            <section aria-labelledby="nb-issues-hoods">
              <h3 className="sub" id="nb-issues-hoods">
                {tr.t("ui-notebook-hood-worries")}
              </h3>
              <ItemList items={hoods} testId="notebook-hoods" />
            </section>
          )}
        </>
      );
    }
    return <ItemList items={items} />;
  };

  return (
    <div className="notebook" data-testid="notebook">
      <h3 className="pocket-tab-title">{tr.t("ui-notebook-title", { town: notebook.zone ?? "" })}</h3>
      {freshCount > 0 && (
        <p className="note" data-testid="notebook-fresh">
          {tr.t("ui-notebook-new-note", { n: freshCount })}
        </p>
      )}
      <label className="nb-search">
        <span className="visually-hidden">{tr.t("ui-notebook-filter")}</span>
        <input className="input" type="search" value={q} onChange={(e) => setQ(e.target.value)} placeholder={tr.t("ui-notebook-filter-placeholder")} data-testid="notebook-search" />
      </label>
      <div className="nb-tabs" role="tablist" aria-label={tr.t("ui-notebook-tabs")} onKeyDown={onKey}>
        {NOTEBOOK_TABS.map((t) => {
          const n = t === "leads" ? 0 : counts[t];
          const dim = searching && hits[t] === 0;
          return (
            <button
              key={t}
              ref={(el) => {
                refs.current[t] = el;
              }}
              type="button"
              role="tab"
              id={tabId(t)}
              aria-selected={tab === t}
              aria-controls={panelId(t)}
              tabIndex={tab === t ? 0 : -1}
              className={["nb-tab", tab === t ? "nb-tab-on" : "", dim ? "nb-tab-dim" : ""].filter(Boolean).join(" ")}
              onClick={() => setTab(t)}
              data-testid={tabId(t)}
            >
              <span className="nb-tab-icon" aria-hidden="true">
                {ICON[t]}
              </span>
              <span className="nb-tab-name">{tr.t(`ui-notebook-tab-${t}`)}</span>
              {n > 0 && (
                <>
                  <span className="badge nb-tab-badge" aria-hidden="true" data-testid={`notebook-tab-badge-${t}`}>
                    {n}
                  </span>
                  <span className="visually-hidden">, {tr.t("ui-notebook-new-note", { n })}</span>
                </>
              )}
            </button>
          );
        })}
      </div>
      <div role="tabpanel" id={panelId(tab)} aria-labelledby={tabId(tab)} className="nb-panel" data-testid={panelId(tab)}>
        {searching && elsewhere.length > 0 && (
          <p className="muted small" data-testid="notebook-elsewhere">
            {tr.t("ui-notebook-filter-elsewhere", { tabs: "" })}
            {elsewhere.map((t, i) => (
              <span key={t}>
                {i > 0 && ", "}
                <button type="button" className="link-btn" onClick={() => setTab(t)}>
                  {tr.t(`ui-notebook-tab-${t}`)} ({hits[t]})
                </button>
              </span>
            ))}
          </p>
        )}
        {body()}
      </div>
    </div>
  );
}
