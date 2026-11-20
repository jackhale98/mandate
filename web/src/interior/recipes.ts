// The interior recipe of a place (DEC-088, DEC-098 §4). Recipes are content:
// `content/base/interiors.jsonc` is their single source of truth, typed and
// validated in `mandate-content`. The engine embeds the content and sends a
// place's recipe in its place view (`interior`, the content's own JSON), so
// the client draws whatever the loaded packs say; the pixel-art tool reads
// the same file for its sheets and tests. A view without one, or one this
// client cannot read, keeps the place panel.
import { normalizeRecipe, type InteriorRecipe } from "../../../tools/pixelkit/art/mandate/interiorsRecipes";

const cache = new WeakMap<object, InteriorRecipe | null>();

/** The recipe a place view carries (`PlacePanelView.interior`), or null. */
export function recipeFromView(json: unknown): InteriorRecipe | null {
  if (typeof json !== "object" || json === null) return null;
  const hit = cache.get(json);
  if (hit !== undefined) return hit;
  const r = json as Partial<InteriorRecipe>;
  const ok = typeof r.id === "string" && typeof r.version === "number" && typeof r.cols === "number" && typeof r.rows === "number" && Array.isArray(r.pieces) && Array.isArray(r.floors) && Array.isArray(r.walls) && typeof r.roles === "object";
  const out = ok ? normalizeRecipe(r as InteriorRecipe) : null;
  cache.set(json, out);
  return out;
}

/** The recipe id when the client can draw a place view's interior (`walkIn`'s test), else null. */
export const enterable = (json: unknown): string | null => recipeFromView(json)?.id ?? null;
