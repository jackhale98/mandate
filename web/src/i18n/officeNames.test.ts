/// <reference types="node" />
// No office or body is named in the web code (DEC-098 §1, R2): bodies and
// offices come from the engine's lists (`OfficeView.bodies`, `executive`,
// `CampaignView.nominations`, `PostDto`), and their words from the
// constitution's Fluent keys. This scan fails on the base constitution's
// office and body words anywhere in `web/src` (code, comments, tests,
// styles) outside Fluent keys. Generated protocol types are skipped: they
// carry the engine's own docs and the deprecated aliases.
// Identifiers joined by `_` (a place kind such as `council_chamber`, a
// content id) are registry keys, not words, and are not matched.
import { readdirSync, readFileSync, statSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const SRC = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const HERE = fileURLToPath(import.meta.url);
/** Generated code. */
const GENERATED = ["engine/protocol", "engine/pkg", "dev/protocol"].map((d) => path.join(SRC, d));

/** The words (the base constitution's office and body), as whole words in any case. */
const NAMES = /\b(mayors?|mayoral(ty)?|councils?|councill?ors?)\b/i;
/** A Fluent key in quotes: lowercase words joined by hyphens (`role-councillor`, `body-council`). */
const FLUENT_KEY = /(["'`])[a-z][a-z0-9_]*(?:-[a-z0-9_]+)+\1/g;

function files(dir: string, out: string[] = []): string[] {
  for (const name of readdirSync(dir)) {
    const p = path.join(dir, name);
    if (statSync(p).isDirectory()) {
      if (!GENERATED.includes(p)) files(p, out);
    } else if (/\.(tsx?|css)$/.test(name) && p !== HERE) out.push(p);
  }
  return out;
}

/** Lines that name an office or body outside a Fluent key. */
function offending(text: string): string[] {
  return text.split("\n").filter((line) => NAMES.test(line.replace(FLUENT_KEY, '""')));
}

describe("no office or body names in the web code (DEC-098)", () => {
  it("finds a name in code and comments, and allows Fluent keys and registry ids", () => {
    expect(offending(`if (seat === "mayor") x();`)).toHaveLength(1);
    expect(offending(`// the Council votes`)).toHaveLength(1);
    expect(offending(`o.council.map(f)`)).toHaveLength(1);
    expect(offending(`tr.t("role-councillor"); tr.t('office-mayor'); kind === "council_chamber"; "start.sitting_councillor"`)).toEqual([]);
  });

  it("scans the source and finds none", () => {
    const all = files(SRC);
    expect(all.length).toBeGreaterThan(100);
    const hits = all.flatMap((f) => offending(readFileSync(f, "utf8")).map((l) => `${path.relative(SRC, f)}: ${l.trim()}`));
    expect(hits).toEqual([]);
  });
});
