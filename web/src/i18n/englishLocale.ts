/// <reference types="node" />
// Tests only: the English locale as the engine serves it (every `.ftl` of the
// base pack, in path order, joined as `ContentBundle::locale_text` joins
// them), read from content/ so a test sees the real strings without the
// engine. Not imported by the app.
import { readdirSync, readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { Translator } from "./fluent";

export const ENGLISH_LOCALE_DIR = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../../content/base/locale/en");

/** The English Fluent text, concatenated in path order. */
export function englishFtl(): string {
  return readdirSync(ENGLISH_LOCALE_DIR)
    .filter((f) => f.endsWith(".ftl"))
    .sort()
    .map((f) => readFileSync(path.join(ENGLISH_LOCALE_DIR, f), "utf8"))
    .join("\n");
}

let cached: Translator | null = null;
/** A translator over the English locale (made once per test file). */
export function englishTranslator(): Translator {
  cached ??= new Translator("en", englishFtl());
  return cached;
}
