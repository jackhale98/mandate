// Tests only: a notebook as the engine might send it (a sitting councillor
// who has canvassed one more neighbourhood this turn and not looked yet).
import type { DateView } from "../../engine/protocol/DateView";
import type { LeadView } from "../../engine/protocol/LeadView";
import type { LocationView } from "../../engine/protocol/LocationView";
import type { NotebookView } from "../../engine/protocol/NotebookView";

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
