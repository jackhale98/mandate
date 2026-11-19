// Small presentation choices remembered on this device (localStorage, best
// effort): which pocket tab was open last, whether the pocket is docked
// beside the town on wide screens, and which map the map overlay showed. Never game state; storage may be blocked
// (private windows), so every read and write is guarded and falls back.

export type PocketTab = "today" | "town" | "places" | "notebook" | "people";
export const POCKET_TABS: readonly PocketTab[] = ["today", "town", "places", "notebook", "people"];

const TAB_KEY = "mandate.pocketTab";
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

/** The pocket tab chosen last on this device (Today at first). */
export function readPocketTab(): PocketTab {
  const v = read(TAB_KEY);
  return POCKET_TABS.find((t) => t === v) ?? "today";
}

export function writePocketTab(tab: PocketTab) {
  write(TAB_KEY, tab);
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

/** Wide screens dock sheets beside the town (matches the 900px breakpoint in styles.css). */
export function isWide(): boolean {
  return typeof window !== "undefined" && window.matchMedia?.("(min-width: 900px)").matches === true;
}
