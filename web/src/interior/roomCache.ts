// Composed rooms, cached by place, recipe version and seed (and the art
// pack's version, which changes what parts look like). Composing takes a few
// ms; walking in and out of the same place reuses the room. A few are kept.
import { composeInterior, interiorSeed, type ArtPack, type InteriorRecipe, type InteriorRoom } from "../../../tools/pixelkit/art/mandate/pack";

const MAX = 4;
const rooms = new Map<string, InteriorRoom>();

export function roomKey(pack: Pick<ArtPack, "version">, recipe: Pick<InteriorRecipe, "id" | "version">, place: number, townSeed: number): string {
  return `${pack.version}|${place}|${recipe.id}@${recipe.version}|${interiorSeed(townSeed, place)}`;
}

/** The room for a place: its recipe, composed from the seed keyed by the town and the place. */
export function roomFor(pack: ArtPack, recipe: InteriorRecipe, place: number, townSeed: number): InteriorRoom {
  const key = roomKey(pack, recipe, place, townSeed);
  let room = rooms.get(key);
  if (room) rooms.delete(key);
  else room = composeInterior(recipe, interiorSeed(townSeed, place), pack.palette, pack.lightness);
  rooms.set(key, room);
  while (rooms.size > MAX) rooms.delete(rooms.keys().next().value as string);
  return room;
}

/** Rooms kept (tests). */
export const cachedRooms = () => rooms.size;
