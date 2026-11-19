/// <reference types="node" />
// Every Fluent key the UI asks for by name exists in the English locale
// (DEC-054: UI strings live in content, with no English fallbacks in code).
// The keys are read from the source: the literal first argument of
// `tr.t(…)` / `tr.has(…)` / `t(…)`, and any "ui-…" string literal (a key
// chosen in a ternary or kept in a table). A key built at run time
// (`place-${kind}`) mostly names something from the engine's views, which the
// content validator checks against the registries; here each such family
// must at least have keys under its prefix, and the families the UI builds
// from its own closed sets (the pocket's tabs) are checked in full.
import { readdirSync, readFileSync, statSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { FluentResource } from "@fluent/bundle";
import { POCKET_TABS } from "../state/devicePrefs";
import { englishFtl } from "./englishLocale";

const SRC = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
/** Generated or test-only code. */
const SKIP_DIRS = new Set(["engine", "fixtures"]);

function sources(dir: string, out: string[] = []): string[] {
  for (const name of readdirSync(dir)) {
    const p = path.join(dir, name);
    if (statSync(p).isDirectory()) {
      if (!SKIP_DIRS.has(name)) sources(p, out);
    } else if (/\.tsx?$/.test(name) && !/\.test\.tsx?$/.test(name) && name !== "englishLocale.ts") out.push(p);
  }
  return out;
}

const CALL = String.raw`\b(?:tr\.t|tr\.has|t)\(\s*`;

/** Keys the UI asks for, with the files that ask. */
function askedKeys(): Map<string, Set<string>> {
  const keys = new Map<string, Set<string>>();
  const add = (k: string, file: string) => {
    let s = keys.get(k);
    if (!s) keys.set(k, (s = new Set()));
    s.add(path.relative(SRC, file));
  };
  for (const file of sources(SRC)) {
    const text = readFileSync(file, "utf8");
    for (const m of text.matchAll(new RegExp(CALL + String.raw`"([a-z][a-z0-9-]*)"`, "g"))) add(m[1] as string, file);
    for (const m of text.matchAll(/["'`](ui-[a-z0-9-]+)["'`]/g)) add(m[1] as string, file);
  }
  return keys;
}

/** Static prefixes of keys built at run time (`place-${kind}` gives "place-"), with a file that builds them. */
function askedPrefixes(): Map<string, string> {
  const out = new Map<string, string>();
  for (const file of sources(SRC)) {
    const text = readFileSync(file, "utf8");
    for (const m of text.matchAll(new RegExp(CALL + "`([a-z][a-z0-9-]*)\\$\\{", "g"))) out.set(m[1] as string, path.relative(SRC, file));
  }
  return out;
}

describe("UI strings are in the English locale", () => {
  const ftl = englishFtl();
  const defined = new Set<string>();
  for (const entry of new FluentResource(ftl).body) if ("id" in entry && typeof entry.id === "string") defined.add(entry.id);

  it("parses, and the scan finds the keys the UI asks for", () => {
    expect(defined.size).toBeGreaterThan(500);
    const asked = askedKeys();
    // A guard against a scan that finds nothing.
    for (const k of ["ui-town-street-label", "ui-pocket-tabs", "ui-end-turn"]) expect(asked.has(k), k).toBe(true);
  });

  it("defines every key the UI asks for", () => {
    const missing = [...askedKeys()].filter(([k]) => !defined.has(k)).map(([k, files]) => `${k} (${[...files].join(", ")})`);
    expect(missing).toEqual([]);
  });

  it("has keys under every prefix the UI builds keys from, and every pocket tab", () => {
    const prefixes = askedPrefixes();
    expect(prefixes.has("place-")).toBe(true);
    const empty = [...prefixes].filter(([p]) => ![...defined].some((k) => k.startsWith(p))).map(([p, f]) => `${p}… (${f})`);
    expect(empty).toEqual([]);
    for (const tab of POCKET_TABS) expect(defined.has(`ui-pocket-tab-${tab}`), tab).toBe(true);
  });

  it("defines each key once across the locale's files", () => {
    const seen = new Set<string>();
    const dup: string[] = [];
    for (const line of ftl.split("\n")) {
      const id = /^([a-z][a-zA-Z0-9_-]*)\s*=/.exec(line)?.[1];
      if (!id) continue;
      if (seen.has(id)) dup.push(id);
      seen.add(id);
    }
    expect(dup).toEqual([]);
  });
});
