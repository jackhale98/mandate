// Tests only: a notebook as the engine might send it (a sitting member
// who has canvassed one more neighbourhood this turn and not looked yet).
import type { DateView } from "../../engine/protocol/DateView";
import type { LeadView } from "../../engine/protocol/LeadView";
import type { LocationView } from "../../engine/protocol/LocationView";
import type { NotebookView } from "../../engine/protocol/NotebookView";
import type { CharacterView } from "../../engine/protocol/CharacterView";
import type { MemberView } from "../../engine/protocol/MemberView";
import type { OfficeView } from "../../engine/protocol/OfficeView";
import type { PersonRefView } from "../../engine/protocol/PersonRefView";
import type { SeenView } from "../../engine/protocol/SeenView";
import type { TownCharacterView } from "../../engine/protocol/TownCharacterView";

export const d = (day: number, month = 3, year = 2026): DateView => ({ day_number: year * 372 + month * 31 + day, year, month, day });

const streets = (hood: number, name: string): LocationView => ({ location: { type: "street", neighborhood: hood }, kind_key: "streets", name: "", neighborhood: hood, neighborhood_name: name });
const diner: LocationView = { location: { type: "place", place: 7 }, kind_key: "diner", name: "Rosa's", neighborhood: 1, neighborhood_name: "Millbrook" };

export const canvassLead: LeadView = {
  subject: { type: "hood_concerns", hood: 3 },
  key: "lead-notebook-canvass",
  args: { hood: "Eastgate", to_hood: "Eastgate" },
  level: "known",
  to: streets(3, "Eastgate"),
  route: { action: "go", late_night: false, neighborhood: 3, to: streets(3, "Eastgate"), here: false },
  since: d(1),
};

export const brokerLead: LeadView = {
  subject: { type: "broker", town: 0 },
  key: "lead-ask-diner",
  args: { town: "Ashford", to_kind_key: "place-diner" },
  level: "heard",
  to: diner,
  route: { action: "go", late_night: false, neighborhood: 1, to: diner, here: false },
  since: d(1),
};

/** An open lead the player picked up in conversation (the same subject as a standing one). */
export const talkLead: LeadView = { ...brokerLead, key: "lead-ask-market", args: { town: "Ashford", to_kind_key: "place-market" }, since: d(2) };

export function notebookFixture(): NotebookView {
  const start = { level: "known" as const, on: d(1), key: "learned-start", args: { profile: "start.sitting_councillor" }, exchange: null };
  return {
    zone: "Ashford",
    origin: "profile",
    sections: [
      {
        key: "people",
        entries: [
          {
            subject: { type: "person", person: 12 },
            level: "known",
            title_key: "notebook-person",
            args: { name: "Vera Holt", role_key: "role-councillor", cares_key: "issue-environment" },
            history: [
              { level: "heard", on: d(1), key: "learned-start", args: { profile: "start.sitting_councillor" }, exchange: null },
              { level: "known", on: d(1), key: "learned-start", args: { profile: "start.sitting_councillor" }, exchange: null },
            ],
            as_of: null,
            stale: false,
            new_this_turn: false,
            lead: null,
          },
          {
            subject: { type: "person", person: 4 },
            level: "heard",
            title_key: "notebook-person",
            args: { name: "Albert Crane", role_key: "role-councillor" },
            history: [{ level: "heard", on: d(1), key: "learned-start", args: { profile: "start.sitting_councillor" }, exchange: null }],
            as_of: null,
            stale: false,
            new_this_turn: false,
            lead: null,
          },
        ],
      },
      {
        key: "neighbourhoods",
        entries: [
          {
            subject: { type: "town_concerns", town: 0 },
            level: "known",
            title_key: "notebook-town-concerns",
            args: { town: "Ashford", concern_key: "concern-jobs", concern2_key: "concern-housing", concern3_key: "concern-services" },
            history: [{ level: "heard", on: d(1), key: "learned-start", args: {}, exchange: null }, start],
            as_of: null,
            stale: false,
            new_this_turn: false,
            lead: null,
          },
          {
            subject: { type: "hood_concerns", hood: 2 },
            level: "known",
            title_key: "notebook-hood-concerns",
            args: { hood: "Westfield", concern_key: "concern-taxes" },
            history: [
              { level: "heard", on: d(8), key: "learned-action", args: { action_key: "action-canvass" }, exchange: null },
              { level: "known", on: d(8), key: "learned-action", args: { action_key: "action-canvass" }, exchange: null },
            ],
            as_of: null,
            stale: false,
            new_this_turn: true,
            lead: null,
          },
        ],
      },
      {
        key: "support",
        entries: [
          {
            subject: { type: "policy_support", town: 0, policy: { type: "template", template: "policy.transit_frequency" } },
            level: "known",
            title_key: "notebook-policy-support",
            args: { policy_key: "policy-transit", band_key: "support-band-popular", support: "20" },
            history: [start],
            as_of: null,
            stale: false,
            new_this_turn: false,
            lead: null,
          },
        ],
      },
    ],
    leads: [talkLead],
    gaps: [
      { subject: { type: "hood_concerns", hood: 3 }, section: "neighbourhoods", title_key: "notebook-hood-concerns", args: { hood: "Eastgate" }, lead: canvassLead },
      { subject: { type: "broker", town: 0 }, section: "power", title_key: "notebook-broker", args: { town: "Ashford" }, lead: talkLead },
      { subject: { type: "rival_plans", town: 0 }, section: "rival", title_key: "notebook-rival-plans", args: { town: "Ashford", rival: "Mara Quill" }, lead: null },
    ],
  };
}

// --- People: the key figures' views (the office view and the town's character) and a sheet. ---

export function member(over: Partial<MemberView> = {}): MemberView {
  return { id: 1, name: "Vera Holt", party: 0, role_key: "role-councillor", ward_number: 2, is_player: false, character: 12, opinion_of_player: 22, favors: 1, convictions: [], pragmatism: null, level: "known", ...over };
}

export function personRef(over: Partial<PersonRefView> = {}): PersonRefView {
  return { id: 30, ambient_slot: null, name: "Otto Brandt", role_key: "role-private", tier: "principal", level: "heard", rumoured: true, can_talk: false, has_news: false, ...over };
}

/** A sitting member's office view: the executive post and the body, with the player in it (the base constitution's keys). */
export function officeFixture(): OfficeView {
  const executive = member({ id: 9, name: "Ada Mills", role_key: "role-mayor", ward_number: null, character: 21, opinion_of_player: -20 });
  const members = [member(), member({ id: 2, name: "", character: 22, level: "unknown", ward_number: 4 }), member({ id: 3, name: "You", is_player: true, character: 1 }), member({ id: 4, name: "Mara Quill", character: 31, ward_number: 1 })];
  const at = { kind: "municipality" as const, tier: "local", id: 1, name: "Millbrook", name_key: "jurisdiction-municipality" };
  const body = { id: "body", label_key: "body-council", role_key: "role-councillor", at, legislature: true, members };
  return { executive, bodies: [body] } as unknown as OfficeView;
}

/** The broker (a rumour) and the rival, who also sits in the body. */
export function townCharacterFixture(): TownCharacterView {
  return { broker: personRef(), rival: personRef({ id: 31, name: "Mara Quill", role_key: "role-councillor", level: "known", rumoured: false }) } as unknown as TownCharacterView;
}

export function sheetFixture(id: number, over: Partial<CharacterView> = {}): CharacterView {
  return {
    id,
    name: "",
    role_key: "",
    ward_number: null,
    party: null,
    tier: "principal",
    age: 50,
    home_neighborhood: "Millbrook",
    place: null,
    personality: [],
    traits: [],
    expertise_key: null,
    opinion_band: "warm",
    trust_band: "some",
    bonds: [],
    owes_you: 0,
    you_owe: 0,
    favours: [],
    memories: [],
    is_player: false,
    is_rival: false,
    is_broker: false,
    goals: null,
    favours_with_others: null,
    knowledge: { level: "known", as_of: null, stale: false, lead: null },
    ...over,
  };
}

export const seenAtDiner: SeenView = { key: "seen-usually", args: { place: "Rosa's", kind_key: "place-diner", part_key: "day-part-afternoon" }, at: diner, part: "afternoon", part_key: "day-part-afternoon", on: d(3) };
