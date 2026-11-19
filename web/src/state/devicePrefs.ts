// Small presentation choices remembered on this device (localStorage, best
// effort): which pocket tab was open last and whether the pocket is docked
// beside the town on wide screens. Never game state; storage may be blocked
// (private windows), so every read and write is guarded and falls back.

export type PocketTab = "today" | "town" | "places" | "people";
export const POCKET_TABS: readonly PocketTab[] = ["today", "town", "places", "people"];

const TAB_KEY = "mandate.pocketTab";
const DOCK_KEY = "mandate.pocketDocked";

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

/** Wide screens dock sheets beside the town (matches the 900px breakpoint in styles.css). */
export function isWide(): boolean {
  return typeof window !== "undefined" && window.matchMedia?.("(min-width: 900px)").matches === true;
}
