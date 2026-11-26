// The Notebook (DEC-113; DEC-091 "the notebook"): the pocket's side for
// looking things up. One row of four tabs, People, Places, Town and Policies,
// with search at its end; the old Town and Places tabs live here now. What is
// known is listed and nothing else: no placeholders, no meter, nothing counts
// how much has been learned (DEC-091, 2026-11-25). A gap is one sentence from
// the engine, shown only where it changes something; a search for something
// not known answers with the way to find out. While a search is typed the tabs
// give way to one flat list grouped by kind. Opening the Notebook tells the
// engine the new entries have been seen (the badge clears); they stay marked
// while the player looks. The engine decides what is known; this arranges it.
import { useEffect, useMemo, useRef, type KeyboardEvent, type Ref } from "react";
import type { PocketView } from "../../engine/protocol/PocketView";
import { useGame } from "../../state/gameStore";
import { NOTEBOOK_TABS, type NotebookTab } from "../../state/devicePrefs";
import { PlaceList, Spot } from "../PlaceList";
import { TownTab } from "../TownCard";
import { placeName } from "../../places/labels";
import { BillRow } from "./Bills";
import { Lead } from "./Leads";
import { PeopleList, PeopleTab, roleLine } from "./People";
import { Row, RowList, Section } from "./rows";
import { groupNotebook, itemFacts, itemText, itemTitle, matches, newCounts, personItems, subjectKey, type NotebookGroups, type NotebookItem } from "./notebookModel";
import "./notebook.css";
import { useProposal } from "../proposal/proposalStore";

export const notebookTabId = (t: NotebookTab) => `notebook-tab-${t}`;
export const notebookPanelId = (t: NotebookTab) => `notebook-panel-${t}`;

/** The notebook filed by tab, with what stays marked new while the player looks; null until loaded. */
export function useNotebookGroups(): { groups: NotebookGroups; fresh: ReadonlySet<string>; freshPeople: ReadonlySet<number> } | null {
  const notebook = useGame((s) => s.notebook);
  const freshList = useGame((s) => s.notebookFresh);
  return useMemo(() => {
    if (!notebook) return null;
    const fresh = new Set(freshList);
    const freshPeople = new Set<number>();
    for (const k of fresh) {
      const s = JSON.parse(k) as { type: string; person?: number };
      if (s.type === "person" && s.person !== undefined) freshPeople.add(s.person);
    }
    return { groups: groupNotebook(notebook, fresh), fresh, freshPeople };
  }, [notebook, freshList]);
}

/** New in each tab (the tabs' badges). */
export function useTabCounts(): Record<NotebookTab, number> {
  const g = useNotebookGroups();
  const people = useGame((s) => s.people);
  if (!g) return { people: 0, places: 0, town: 0, policies: 0 };
  const peopleNew = people ? people.rows.filter((r) => r.new_this_turn || g.freshPeople.has(r.id)).length : 0;
  return newCounts(g.groups, peopleNew);
}

/** The Notebook's one row of tabs, with the search glass at its end (a field while searching). */
export function NotebookTabs() {
  const tr = useGame((s) => s.tr);
  const tab = useGame((s) => s.notebookTab);
  const setTab = useGame((s) => s.setPocketTab);
  const q = useGame((s) => s.notebookQuery);
  const setQ = useGame((s) => s.setNotebookQuery);
  const counts = useTabCounts();
  const refs = useRef<Partial<Record<NotebookTab, HTMLButtonElement | null>>>({});
  const field = useRef<HTMLInputElement>(null);
  const glass = useRef<HTMLButtonElement>(null);
  const searching = q !== null;
  useEffect(() => {
    if (searching) field.current?.focus();
  }, [searching]);
  const onKey = (e: KeyboardEvent) => {
    const i = NOTEBOOK_TABS.indexOf(tab);
    const n = NOTEBOOK_TABS.length;
    const next = e.key === "ArrowRight" ? NOTEBOOK_TABS[(i + 1) % n] : e.key === "ArrowLeft" ? NOTEBOOK_TABS[(i + n - 1) % n] : e.key === "Home" ? NOTEBOOK_TABS[0] : e.key === "End" ? NOTEBOOK_TABS[n - 1] : undefined;
    if (!next) return;
    e.preventDefault();
    setTab(next);
    refs.current[next]?.focus();
  };
  if (searching)
    return (
      <div className="nb-search" role="search">
        <label className="visually-hidden" htmlFor="nb-search-field">
          {tr.t("ui-notebook-filter")}
        </label>
        <input
          ref={field}
          id="nb-search-field"
          className="input"
          type="search"
          value={q}
          onChange={(e) => setQ(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Escape") {
              e.stopPropagation();
              setQ(null);
              requestAnimationFrame(() => glass.current?.focus());
            }
          }}
          placeholder={tr.t("ui-notebook-filter-placeholder")}
          data-testid="notebook-search"
        />
        <button type="button" className="btn btn-ghost pocket-icon" aria-label={tr.t("ui-notebook-search-close")} onClick={() => setQ(null)} data-testid="notebook-search-close">
          ✕
        </button>
      </div>
    );
  return (
    <div className="nb-tabs">
      <div className="nb-tablist" role="tablist" aria-label={tr.t("ui-notebook-tabs")} onKeyDown={onKey}>
        {NOTEBOOK_TABS.map((t) => {
          const n = counts[t];
          return (
            <button
              key={t}
              ref={(el) => {
                refs.current[t] = el;
              }}
              type="button"
              role="tab"
              id={notebookTabId(t)}
              aria-selected={tab === t}
              aria-controls={notebookPanelId(t)}
              tabIndex={tab === t ? 0 : -1}
              className={tab === t ? "nb-tab nb-tab-on" : "nb-tab"}
              onClick={() => setTab(t)}
              data-testid={notebookTabId(t)}
            >
              <span className="nb-tab-name">{tr.t(`ui-notebook-tab-${t}`)}</span>
              {n > 0 && (
                <>
                  <span className="badge" aria-hidden="true" data-testid={`notebook-tab-badge-${t}`}>
                    {n}
                  </span>
                  <span className="visually-hidden">, {tr.t("ui-notebook-new-note", { n })}</span>
                </>
              )}
            </button>
          );
        })}
      </div>
      <button ref={glass} type="button" className="nb-find" aria-label={tr.t("ui-notebook-filter")} title={tr.t("ui-notebook-filter")} onClick={() => setQ("")} data-testid="notebook-search-open">
        <span aria-hidden="true">🔍</span>
      </button>
    </div>
  );
}

/** Entries as rows: a title and the facts known (worries, support, proposals, places found). */
export function EntryRows({ items, testId }: { items: NotebookItem[]; testId?: string }) {
  const tr = useGame((s) => s.tr);
  const office = useGame((s) => s.office);
  // A proposal still on the desk opens its sheet (DEC-092).
  const sheetOf = (i: NotebookItem) => {
    const s = i.subject;
    const p = s.type === "proposal" ? office?.proposals.find((x) => x.id === s.proposal) : undefined;
    return p ? () => void useProposal.getState().openProposal({ proposal: p.id, family: p.family }, { opener: document.activeElement instanceof HTMLElement ? document.activeElement : null }) : undefined;
  };
  return (
    <RowList className="nb-entry-list" testId={testId}>
      {items.map((i) => (
        <Row
          key={i.key}
          title={
            <>
              {itemTitle(tr, i)}
              {i.isNew && (
                <span className="badge rw-new" data-testid="notebook-new">
                  {tr.t("ui-notebook-new")}
                </span>
              )}
            </>
          }
          sub={itemFacts(tr, i).join(" · ") || null}
          reason={i.entry?.note ? tr.event(i.entry.note.key, i.entry.note.args) : null}
          onActivate={sheetOf(i)}
          li={{ "data-testid": "notebook-entry", "data-subject": i.subject.type, "data-new": i.isNew ? "true" : undefined }}
        />
      ))}
    </RowList>
  );
}

/** The Policies tab: the player's bills, then what is known of who supports what. */
function PoliciesTab({ items, p }: { items: NotebookItem[]; p: PocketView }) {
  const tr = useGame((s) => s.tr);
  if (items.length === 0 && p.bills.length === 0) return <p className="today-empty">{tr.t("ui-notebook-empty-policies")}</p>;
  return (
    <>
      {p.bills.length > 0 && (
        <Section id="nb-bills" title={tr.t("ui-your-bills")} testId="notebook-bills">
          <RowList>
            {p.bills.map((b) => (
              <BillRow key={b.id} b={b} p={p} testId={`policy-bill-${b.id}`} />
            ))}
          </RowList>
        </Section>
      )}
      {items.length > 0 && (
        <Section id="nb-support" title={tr.t("ui-policies-known")} testId="notebook-policies">
          <EntryRows items={items} />
        </Section>
      )}
    </>
  );
}

/** While a search is typed: one flat list grouped by kind (people, places, worries, policies), and the way to find out what is not known. */
function SearchResults({ q, groups, freshPeople }: { q: string; groups: NotebookGroups; freshPeople: ReadonlySet<number> }) {
  const tr = useGame((s) => s.tr);
  const people = useGame((s) => s.people);
  const town = useGame((s) => s.town);
  const pocket = useGame((s) => s.pocket);
  if (q.trim() === "") return <p className="muted small">{tr.t("ui-notebook-search-hint")}</p>;
  const items = personItems(groups.people);
  const who = (people?.rows ?? []).filter((r) => matches(`${r.name} ${roleLine(tr, r)}`, q));
  const spots = (town?.neighborhoods ?? []).flatMap((h) => h.spots).filter((s) => matches(`${placeName(tr, s.location)} ${tr.t(`place-${s.location.kind_key}`)}`, q));
  const keep = (i: NotebookItem) => matches(itemText(tr, i), q);
  const worries = [...groups.places, ...groups.town].filter((i) => (i.subject.type === "hood_concerns" || i.subject.type === "town_concerns") && keep(i));
  const known = [...groups.places.filter((i) => i.subject.type !== "hood_concerns"), ...groups.town.filter((i) => i.subject.type !== "town_concerns"), ...groups.policies].filter(keep);
  const bills = (pocket?.bills ?? []).filter((b) => matches(tr.t(b.name_key), q));
  const gaps = groups.gaps.filter((g) => matches(`${tr.event(g.title_key, g.args)} ${g.lead ? tr.event(g.lead.key, g.lead.args) : ""}`, q));
  const none = who.length + spots.length + worries.length + known.length + bills.length + gaps.length === 0;
  return (
    <div className="nb-results" data-testid="notebook-results">
      {none && <p className="today-empty">{tr.t("ui-notebook-filter-none", { q })}</p>}
      {who.length > 0 && (
        <Section id="nb-r-people" title={tr.t("ui-notebook-tab-people")} testId="notebook-results-people">
          <PeopleList rows={who} show={who.length} items={items} fresh={freshPeople} />
        </Section>
      )}
      {spots.length > 0 && (
        <Section id="nb-r-places" title={tr.t("ui-notebook-tab-places")} testId="notebook-results-places">
          <RowList>
            {spots.map((s, i) => (
              <Spot key={i} s={s} />
            ))}
          </RowList>
        </Section>
      )}
      {worries.length > 0 && (
        <Section id="nb-r-worries" title={tr.t("ui-search-worries")} testId="notebook-results-worries">
          <EntryRows items={worries} testId="notebook-hoods" />
        </Section>
      )}
      {(known.length > 0 || bills.length > 0) && pocket && (
        <Section id="nb-r-known" title={tr.t("ui-search-known")} testId="notebook-results-known">
          {bills.length > 0 && (
            <RowList>
              {bills.map((b) => (
                <BillRow key={b.id} b={b} p={pocket} testId={`search-bill-${b.id}`} />
              ))}
            </RowList>
          )}
          {known.length > 0 && <EntryRows items={known} />}
        </Section>
      )}
      {gaps.length > 0 && (
        <Section id="nb-r-gaps" title={tr.t("ui-search-find-out")} testId="notebook-results-gaps">
          {gaps.map((g) => (
            <div key={subjectKey(g.subject)} className="nb-gap-answer" data-testid="notebook-gap-answer">
              <p className="small">{tr.t("ui-search-not-known", { what: tr.event(g.title_key, g.args) })}</p>
              {g.lead && <Lead lead={g.lead} />}
            </div>
          ))}
        </Section>
      )}
    </div>
  );
}

/** The Notebook's body: the open tab's panel (or the search's results). `focusRef`: where "Skip to Places" puts the keyboard. */
export function Notebook({ panelRef }: { panelRef?: Ref<HTMLDivElement> }) {
  const tr = useGame((s) => s.tr);
  const tab = useGame((s) => s.notebookTab);
  const setTab = useGame((s) => s.setPocketTab);
  const pocket = useGame((s) => s.pocket);
  const busy = useGame((s) => s.busy);
  const q = useGame((s) => s.notebookQuery);
  const land = useGame((s) => s.folds["notebook-land"] === true);
  const setFold = useGame((s) => s.setFold);
  const loadNotebook = useGame((s) => s.loadNotebook);
  const seeNotebook = useGame((s) => s.seeNotebook);
  const leaveNotebook = useGame((s) => s.leaveNotebook);
  const g = useNotebookGroups();
  const counts = useTabCounts();
  const arcLeads = useGame((s) => s.notebook?.arc_leads);

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

  // Opened from "You learned N new things": land on the first tab with something new (once).
  useEffect(() => {
    // Waits for the counts: the notebook just fetched may not have them yet. Leaving the Notebook forgets it.
    if (!land || !g) return;
    const first = NOTEBOOK_TABS.find((t) => counts[t] > 0);
    if (!first) return;
    setTab(first);
    setFold("notebook-land", false);
  }, [land, g, counts, setTab, setFold]);
  useEffect(() => () => useGame.getState().setFold("notebook-land", false), []);

  const body = () => {
    if (!g || !pocket) return <p className="muted small">…</p>;
    if (q !== null) return <SearchResults q={q} groups={g.groups} freshPeople={g.freshPeople} />;
    switch (tab) {
      case "people":
        return <PeopleTab items={personItems(g.groups.people)} fresh={g.freshPeople} />;
      case "places":
        return <PlaceList items={g.groups.places} />;
      case "town":
        return <TownTab items={g.groups.town} leads={g.groups.leads} arcs={arcLeads ?? []} />;
      case "policies":
        return <PoliciesTab items={g.groups.policies} p={pocket} />;
    }
  };
  return (
    <div className="notebook" data-testid="notebook">
      <div ref={panelRef} role="tabpanel" id={notebookPanelId(tab)} aria-labelledby={q === null ? notebookTabId(tab) : undefined} aria-label={q !== null ? tr.t("ui-notebook-filter") : undefined} tabIndex={-1} className="nb-panel" data-testid={notebookPanelId(tab)}>
        {body()}
      </div>
    </div>
  );
}
