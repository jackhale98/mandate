// Tests only: a town map layers view shaped like the engine's
// (`mandate_protocol::town_layers`), small enough to read. Three
// neighbourhoods in two districts; five people, one of them heard of only and
// one with no known place; the worries of one neighbourhood unknown.
import type { KnowledgeView } from "../../engine/protocol/KnowledgeView";
import type { TownMapLayersView } from "../../engine/protocol/TownMapLayersView";
import type { TownMarkerView } from "../../engine/protocol/TownMarkerView";
import type { PostDto } from "../../engine/protocol/PostDto";

const at = { kind: "municipality" as const, tier: "local", id: 4, name: "Ashford", name_key: "jurisdiction-municipality" };

export const seat = (n: number): PostDto => ({ kind: "seat", id: "body_one", at, constituency_number: n, label_key: "body-council", role_key: "role-councillor" });

/** A marker for person `id`: known, a member of the home body, of party 0, at its chamber, unless overridden. */
export function marker(id: number, over: Partial<TownMarkerView> & { name?: string; level?: TownMarkerView["person"]["level"]; role?: string } = {}): TownMarkerView {
  const { name, level, role, ...rest } = over;
  return {
    person: { id, ambient_slot: null, name: name ?? `Person ${id}`, role_key: role ?? "role-councillor", tier: "principal", level: level ?? "known", rumoured: false, can_talk: false, has_news: false },
    is_player: false,
    party: 0,
    seat: null,
    at: { location: { type: "place", place: 7 }, kind_key: "council_chamber", name: "", neighborhood: 10, neighborhood_name: "Ashford" },
    basis: "works",
    seen: null,
    filters: ["all"],
    ...rest,
  };
}

const known: KnowledgeView = { level: "known", as_of: null, stale: false, lead: null };
const unknown: KnowledgeView = { level: "unknown", as_of: null, stale: false, lead: null };

const you = marker(1, { name: "Dana Reyes", is_player: true, seat: seat(1), at: null, basis: "self", filters: [] });
const holder = marker(2, { name: "", level: "unknown", party: 1, seat: seat(2), filters: [] });

export const layersView: TownMapLayersView = {
  at,
  date: { day_number: 30, year: 2026, month: 1, day: 30 },
  hoods: [
    { id: 10, name: "Ashford", name_key: "ui-hood-centre", name_args: { centre_town: "Ashford" } },
    { id: 11, name: "Eastfield" },
    { id: 12, name: "Riverside" },
  ],
  parties: [
    { id: 0, name: "Civic Union", short: "CU", colour: 0 },
    { id: 1, name: "Labour Alliance", short: "LA", colour: 1 },
  ],
  layers: [
    {
      id: "town_map_layer.districts",
      label_key: "town-map-layer-districts",
      unit_key: "town-map-layer-districts-unit",
      note_key: "town-map-layer-districts-note",
      args: { tier: "local" },
      source: "districts",
      scale: "party",
      data: {
        kind: "boundaries",
        areas: [
          { number: 1, label_key: "ui-post-seat", label_args: { n: "1", tier: "local" }, hoods: [10, 11], yours: true, home: true, seat: seat(1), holder: you },
          { number: 2, label_key: "ui-post-seat", label_args: { n: "2", tier: "local" }, hoods: [12], yours: false, home: false, seat: seat(2), holder },
        ],
        legend: [
          { label_key: null, party: 0, count: 1 },
          { label_key: null, party: 1, count: 1 },
        ],
      },
    },
    {
      id: "town_map_layer.people",
      label_key: "town-map-layer-people",
      unit_key: "town-map-layer-people-unit",
      note_key: "town-map-layer-people-note",
      args: { tier: "local" },
      source: "people",
      scale: "party",
      data: {
        kind: "markers",
        markers: [
          marker(3, { name: "Ines Okafor", seat: seat(3), filters: ["all", "body", "met", "party:0"] }),
          marker(4, { name: "Tomas Brandt", party: 1, seat: seat(4), level: "heard", filters: ["all", "body", "party:1"] }),
          marker(5, {
            name: "Ruth Calder",
            role: "occupation-union_steward",
            party: null,
            at: { location: { type: "place", place: 9 }, kind_key: "union_hall", name: "", neighborhood: 11, neighborhood_name: "Eastfield" },
            filters: ["all", "met"],
          }),
          marker(6, {
            name: "Sam Whit",
            role: "occupation-shopkeeper",
            party: null,
            level: "heard",
            at: { location: { type: "street", neighborhood: 12 }, kind_key: "streets", name: "", neighborhood: 12, neighborhood_name: "Riverside" },
            basis: "last_seen",
            seen: {
              key: "seen-last",
              args: { place: "Riverside", kind_key: "place-streets", part_key: "day-part-morning" },
              at: { location: { type: "street", neighborhood: 12 }, kind_key: "streets", name: "", neighborhood: 12, neighborhood_name: "Riverside" },
              part: "morning",
              part_key: "day-part-morning",
              on: { day_number: 23, year: 2026, month: 1, day: 23 },
            },
            filters: ["all"],
          }),
          marker(7, { name: "Lena Marsh", role: "occupation-journalist", party: null, level: "heard", at: null, basis: "unknown", filters: ["all"] }),
        ],
        filters: [
          { id: "all", label_key: "town-map-filter-all", party: null, count: 5 },
          { id: "body", label_key: "body-council", party: null, count: 2 },
          { id: "party:0", label_key: null, party: 0, count: 1 },
          { id: "party:1", label_key: null, party: 1, count: 1 },
          { id: "met", label_key: "town-map-filter-met", party: null, count: 2 },
        ],
        legend: [
          { label_key: null, party: 0, count: 1 },
          { label_key: null, party: 1, count: 1 },
          { label_key: "town-map-legend-no-party", party: null, count: 3 },
        ],
      },
    },
    {
      id: "town_map_layer.worries",
      label_key: "town-map-layer-worries",
      unit_key: "town-map-layer-worries-unit",
      note_key: "town-map-layer-worries-note",
      args: { tier: "local" },
      source: "worries",
      scale: "sequential",
      data: {
        kind: "shaded",
        param: "worry",
        param_label_key: "town-map-param-worry",
        options: [
          {
            id: "top",
            label_key: "town-map-worry-top",
            scale: "categorical",
            bins: [
              { label_key: "concern-jobs", party: null, count: 1 },
              { label_key: "concern-housing", party: null, count: 1 },
            ],
            cells: [
              { class: 0, label_key: "concern-jobs" },
              { class: 1, label_key: "concern-housing" },
              { class: null, label_key: null },
            ],
            unknown: 1,
          },
          {
            id: "concern.jobs",
            label_key: "concern-jobs",
            scale: "sequential",
            bins: [
              { label_key: "town-map-worry-rank-low", party: null, count: 0 },
              { label_key: "town-map-worry-rank-high", party: null, count: 1 },
              { label_key: "town-map-worry-rank-top", party: null, count: 1 },
            ],
            cells: [
              { class: 2, label_key: "town-map-worry-rank-top" },
              { class: 1, label_key: "town-map-worry-rank-high" },
              { class: null, label_key: null },
            ],
            unknown: 1,
          },
        ],
        knowledge: [known, known, unknown],
        details: [["concern-jobs", "concern-housing", "concern-taxes"], ["concern-housing", "concern-jobs", "concern-services"], []],
      },
    },
  ],
};
