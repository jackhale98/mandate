/// <reference types="node" />
// The corpus renderer's runner: `mandate-cli content doctor --corpus <dir>`
// runs this file under Vitest with MANDATE_CORPUS=<dir>, so the text is
// rendered by the game's own Fluent code (the Translator and the screens'
// helpers, with the app's module resolution). Without MANDATE_CORPUS it is
// skipped: it is a tool, not a test of the app.
import { readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import { englishFtl } from "../../i18n/englishLocale";
import { Translator } from "../../i18n/fluent";
import { countItems, markdown, renderAll, type RawItem } from "./corpusRender";

const dir = process.env.MANDATE_CORPUS;

describe.skipIf(!dir)("render the corpus", () => {
  it("renders raw.jsonl into corpus.jsonl and corpus.md", () => {
    const d = dir ?? "";
    const raw: RawItem[] = readFileSync(path.join(d, "raw.jsonl"), "utf8")
      .split("\n")
      .filter((l) => l.trim() !== "")
      .map((l) => JSON.parse(l) as RawItem);
    const tr = new Translator("en", englishFtl());
    const items = renderAll(tr, raw);
    writeFileSync(path.join(d, "corpus.jsonl"), items.map((i) => JSON.stringify(i)).join("\n") + "\n");
    const counts = countItems(items);
    writeFileSync(path.join(d, "corpus.md"), markdown(items, counts));
    writeFileSync(path.join(d, "rendered-counts.json"), JSON.stringify(counts, null, 2));
    expect(items.length).toBeGreaterThan(0);
  });
});
