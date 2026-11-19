// The time of day in the game comes from the turn, not a clock (DEC-086,
// owner 2026-11-19): morning to evening as the turn's slots are used, then
// evening (or late night once only the late-night slot is left). The light
// holds still between actions; the town map and the walkable street read the
// same time. The prototype keeps its free-running clock.
import type { PocketView } from "../engine/protocol/PocketView";
import { clockForDayPart } from "./dayCycle";

/** Minutes after midnight for where the player is in the turn. */
export function turnMinutes(p: Pick<PocketView, "slots_left" | "slots_total" | "late_night_available">): number {
  if (p.slots_left <= 0) return clockForDayPart(p.late_night_available ? "evening" : "late_night", p.late_night_available ? 0.75 : 0.2);
  const used = Math.max(0, p.slots_total - p.slots_left) / Math.max(1, p.slots_total);
  return used < 0.5 ? clockForDayPart("morning", 0.35 + used) : clockForDayPart("afternoon", (used - 0.5) * 1.6);
}

/** A new turn's first light (no slot used yet). */
export const MORNING_MINUTES = turnMinutes({ slots_left: 1, slots_total: 1, late_night_available: false });

/** The middle of the night between two turns (lamps lit, the streets nearly empty). */
export const NIGHT_MINUTES = clockForDayPart("late_night", 0.55);

/** How long the light takes to move to the next time after an action, ms (instant with reduced motion). */
export const LIGHT_EASE_MS = 1400;
