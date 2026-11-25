import type { DateView } from "../engine/protocol/DateView";
import type { Translator } from "./fluent";

export function formatDate(tr: Translator, d: DateView): string {
  return tr.t("ui-date", { day: d.day, month: tr.t(`month-${d.month}`), year: String(d.year) });
}

/** A day with its weekday ("Saturday 14 July 2031"): the day played (DEC-114), as the engine names it (`weekday_key`). */
export function formatDay(tr: Translator, d: DateView, weekdayKey: string): string {
  if (!weekdayKey) return formatDate(tr, d);
  return tr.t("ui-date-weekday", { weekday: tr.t(weekdayKey), day: d.day, month: tr.t(`month-${d.month}`), year: String(d.year) });
}

/** Short ("Sat 14 Jul"), for a narrow bar. */
export function formatDayShort(tr: Translator, d: DateView, weekdayKey: string): string {
  const wd = weekdayKey ? tr.t(weekdayKey.replace("weekday-", "weekday-short-")) : "";
  return tr.t("ui-date-weekday-short", { weekday: wd, day: d.day, month: tr.t(`month-short-${d.month}`) }).trim();
}

export function formatCount(n: number): string {
  return new Intl.NumberFormat("en", { notation: n >= 1e6 ? "compact" : "standard", maximumFractionDigits: 2 }).format(n);
}

/** A whole count with the locale's grouping, never abbreviated (a before-and-after must show the change). */
export function formatWhole(n: number): string {
  return new Intl.NumberFormat("en", { maximumFractionDigits: 0 }).format(n);
}

export function formatPct(v: number, digits = 1): string {
  return new Intl.NumberFormat("en", { style: "percent", minimumFractionDigits: digits, maximumFractionDigits: digits }).format(v);
}

export function formatMoney(v: number): string {
  return new Intl.NumberFormat("en", { notation: Math.abs(v) >= 1e5 ? "compact" : "standard", maximumFractionDigits: Math.abs(v) >= 1e5 ? 1 : 0 }).format(v);
}
