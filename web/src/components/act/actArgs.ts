import type { FluentVariable } from "@fluent/bundle";
import type { Translator } from "../../i18n/fluent";

/**
 * The engine's arguments for a message read directly with `tr.t`: numbers as
 * numbers (Fluent's plural variants match numbers, not strings), and each
 * `<name>_key` argument as its text under `<name>`.
 */
export function actArgs(tr: Translator, args: Record<string, string>): Record<string, FluentVariable> {
  const out: Record<string, FluentVariable> = {};
  for (const [k, v] of Object.entries(args)) {
    if (k.endsWith("_key")) out[k.slice(0, -4)] = tr.t(v);
    else out[k] = /^-?\d+$/.test(v) ? Number(v) : v;
  }
  return out;
}
