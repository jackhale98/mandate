// The interior recipes as the Node tools see them (sheets, tests): read from
// the game's content, `content/base/interiors.jsonc`, their single source of
// truth (DEC-098 §4). The game bundles the same file (web/src/interior). Node
// only: the pack and the composer take recipes as arguments.
import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { parseInteriorRecipes, recipeFor, type InteriorRecipe } from "./interiorsRecipes";

/** The repository root (tools/pixelkit/art/mandate → four up). */
export const REPO_ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../../..");
export const INTERIORS_FILE = path.join(REPO_ROOT, "content/base/interiors.jsonc");

/** Every recipe in the base content, in file order. */
export const RECIPES: readonly InteriorRecipe[] = parseInteriorRecipes(readFileSync(INTERIORS_FILE, "utf8"));

const need = (place: string): InteriorRecipe => {
  const r = recipeFor(RECIPES, place);
  if (!r) throw new Error(`content/base/interiors.jsonc has no recipe for ${place}`);
  return r;
};

export const COUNCIL_CHAMBER = need("council_chamber");
export const TOWN_HALL = need("town_hall");
export const DINER = need("diner");
