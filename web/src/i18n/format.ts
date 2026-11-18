import type { DateView } from "../engine/protocol/DateView";
import type { Translator } from "./fluent";

export function formatDate(tr: Translator, d: DateView): string {
  return tr.t("ui-date", { day: d.day, month: tr.t(`month-${d.month}`), year: String(d.year) });
}

export function formatCount(n: number): string {
  return new Intl.NumberFormat("en", { notation: n >= 1e6 ? "compact" : "standard", maximumFractionDigits: 2 }).format(n);
}

export function formatPct(v: number, digits = 1): string {
  return new Intl.NumberFormat("en", { style: "percent", minimumFractionDigits: digits, maximumFractionDigits: digits }).format(v);
}

export function formatMoney(v: number): string {
  return new Intl.NumberFormat("en", { notation: Math.abs(v) >= 1e5 ? "compact" : "standard", maximumFractionDigits: Math.abs(v) >= 1e5 ? 1 : 0 }).format(v);
}
