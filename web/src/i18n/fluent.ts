// Fluent rendering (DEC-054). The engine emits keys + args; text lives in the
// content pack's .ftl files, fetched from the engine (single source, offline).
import { FluentBundle, FluentResource, type FluentVariable } from "@fluent/bundle";

export class Translator {
  private bundle: FluentBundle;
  readonly errors: string[] = [];

  constructor(locale: string, ftl: string) {
    this.bundle = new FluentBundle(locale, { useIsolating: false });
    const errs = this.bundle.addResource(new FluentResource(ftl));
    for (const e of errs) this.errors.push(String(e));
  }

  has(key: string): boolean {
    return this.bundle.hasMessage(key);
  }

  /** Format an engine event: args named `*_key` are Fluent keys, translated
   * first and passed without the suffix (DEC-054). */
  event(key: string, args: Record<string, string>): string {
    const resolved: Record<string, FluentVariable> = {};
    for (const [k, v] of Object.entries(args)) {
      if (k.endsWith("_key")) continue;
      // Counts select plural variants, which match numbers, not strings.
      if (k === "n" && /^\d+$/.test(v)) resolved[k] = Number(v);
      else resolved[k] = v;
    }
    // Keys see the plain arguments too (an outlet's name takes its place).
    const plain = { ...resolved };
    for (const [k, v] of Object.entries(args)) {
      if (k.endsWith("_key")) resolved[k.slice(0, -4)] = this.t(v, plain);
    }
    return this.t(key, resolved);
  }

  /** Format a message; unknown keys render as `⟦key⟧` so gaps are visible. */
  t(key: string, args?: Record<string, FluentVariable>): string {
    const msg = this.bundle.getMessage(key);
    if (!msg?.value) return `⟦${key}⟧`;
    const errors: Error[] = [];
    const out = this.bundle.formatPattern(msg.value, args ?? null, errors);
    return out;
  }
}

/** Fallback used before the engine has delivered locale text. */
export const EMPTY_TRANSLATOR = new Translator("en", "");
