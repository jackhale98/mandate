// Small presentation choices remembered on this device (localStorage, best
// effort): which side of the pocket was open last and which Notebook tab,
// whether the pocket is docked beside the town on wide screens, and which map
// the map overlay showed. Never game state; storage may be blocked (private
// windows), so every read and write is guarded and falls back.

// The pocket has two sides under one switch (DEC-113): Today, for doing, and
// the Notebook, for reference. The Notebook has one row of four tabs, which
// took in the old Town and Places tabs (and people, DEC-088).
export type PocketSide = "today" | "notebook";
export const POCKET_TABS: readonly PocketSide[] = ["today", "notebook"];

export type NotebookTab = "people" | "places" | "town" | "policies";
export const NOTEBOOK_TABS: readonly NotebookTab[] = ["people", "places", "town", "policies"];

/** What the pocket can be opened on: a side, or a Notebook tab (which opens the Notebook there). */
export type PocketTab = PocketSide | NotebookTab;

const TAB_KEY = "mandate.pocketTab";
const BOOK_KEY = "mandate.notebookTab";
const DOCK_KEY = "mandate.pocketDocked";
const MAP_KEY = "mandate.mapScope";

/** The map overlay's two maps: the player's town (DEC-086) and the country (DEC-021). */
export type MapScope = "town" | "country";

function read(key: string): string | null {
  try {
    return typeof localStorage === "undefined" ? null : localStorage.getItem(key);
  } catch {
    return null;
  }
}

function write(key: string, value: string) {
  try {
    localStorage.setItem(key, value);
  } catch {
    // Storage blocked: the choice lasts for this visit.
  }
}

/** Where a pocket tab opens: its side, and the Notebook tab when it names one. */
export function placeTab(tab: PocketTab): { side: PocketSide; book: NotebookTab | null } {
  if (tab === "today" || tab === "notebook") return { side: tab, book: null };
  return { side: "notebook", book: tab };
}

/** Tabs that are gone, and where their content went (the old four tabs, and the People tab before them). */
const MOVED: Readonly<Record<string, PocketTab>> = { people: "people", town: "town", places: "places" };

/** The pocket side chosen last on this device (Today at first; an old Town, Places or People tab: the Notebook). */
export function readPocketTab(): PocketSide {
  const v = read(TAB_KEY);
  if (v === "today" || v === "notebook") return v;
  const moved = v !== null ? MOVED[v] : undefined;
  return moved ? placeTab(moved).side : "today";
}

export function writePocketTab(side: PocketSide) {
  write(TAB_KEY, side);
}

/** The Notebook tab chosen last (People at first; a pocket last left on the old Town or Places tab opens there). */
export function readNotebookTab(): NotebookTab {
  const v = read(BOOK_KEY);
  const found = NOTEBOOK_TABS.find((t) => t === v);
  if (found) return found;
  const old = read(TAB_KEY);
  return old === "town" || old === "places" ? old : "people";
}

export function writeNotebookTab(tab: NotebookTab) {
  write(BOOK_KEY, tab);
}

/** Whether the pocket stays docked open beside the town on wide screens. */
export function readPocketDocked(): boolean {
  return read(DOCK_KEY) === "1";
}

export function writePocketDocked(docked: boolean) {
  write(DOCK_KEY, docked ? "1" : "0");
}

/** The map the overlay showed last on this device (the town at first). */
export function readMapScope(): MapScope {
  return read(MAP_KEY) === "country" ? "country" : "town";
}

export function writeMapScope(scope: MapScope) {
  write(MAP_KEY, scope);
}

const FOLD_KEY = "mandate.folds";

/** The folds opened or closed this session (DEC-113 §3.5: an opened fold stays open for the session; a reload keeps it). */
export function readFolds(): Record<string, boolean> {
  try {
    const v = typeof sessionStorage === "undefined" ? null : sessionStorage.getItem(FOLD_KEY);
    const parsed: unknown = v ? JSON.parse(v) : {};
    return parsed && typeof parsed === "object" ? (parsed as Record<string, boolean>) : {};
  } catch {
    return {};
  }
}

export function writeFolds(folds: Record<string, boolean>) {
  try {
    sessionStorage.setItem(FOLD_KEY, JSON.stringify(folds));
  } catch {
    // Storage blocked: the folds last for this page.
  }
}

/** Wide screens dock sheets beside the town (matches the 900px breakpoint in styles.css). */
export function isWide(): boolean {
  return typeof window !== "undefined" && window.matchMedia?.("(min-width: 900px)").matches === true;
}
