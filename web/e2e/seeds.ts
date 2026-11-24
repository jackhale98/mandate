import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

// The seed a browser spec needs, from the repository's seed cache
// (tests/seeds.json). A spec cannot search for a seed cheaply, so it reads the
// cached one; the Rust side evaluates the same requirement and keeps the file
// fresh (`mandate-cli seeds check|refresh`, run by scripts/repin.sh and
// scripts/check.sh fast). Returns the seed as the text the title screen takes.
const file = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../tests/seeds.json");

export function seedFor(name: string): string {
  const cache = JSON.parse(readFileSync(file, "utf8")) as Record<string, { seed: number; needs: string }>;
  const entry = cache[name];
  if (!entry) throw new Error(`requirement "${name}" is not in tests/seeds.json`);
  return String(entry.seed);
}
