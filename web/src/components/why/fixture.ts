// Test fixtures: WhyViews as the engine sends them (`QueryDto::Why`), written
// to the generated types. Between them they carry every link type and every
// kind of start. The values every enum can take are listed here too, so the
// model's tests can ask for a line for each; the engine's own list is checked
// in Rust (`mandate-wasm/tests/why.rs`).
import type { CauseView } from "../../engine/protocol/CauseView";
import type { DateView } from "../../engine/protocol/DateView";
import type { LinkDto } from "../../engine/protocol/LinkDto";
import type { PersonRefView } from "../../engine/protocol/PersonRefView";
import type { WhyStepView } from "../../engine/protocol/WhyStepView";
import type { WhyView } from "../../engine/protocol/WhyView";

export const LINKS: readonly LinkDto[] = ["caused", "continues", "prompted", "revealed", "ended"];
export const PLAYER_ACTS = [
  "start", "vote", "file", "draft", "amend", "introduce", "withdraw", "move", "meet", "answer", "let_pass", "talk", "respond_ask", "promise",
  "lobby", "trade_favor", "town_hall", "press", "canvass", "fundraise", "rest", "advertise", "take_bribe", "dip_funds", "leak_file", "visit", "casework",
] as const;
export const PROCESSES = [
  "calendar", "markets", "central_bank", "budget", "lawmaking", "agenda", "elections", "parties", "director", "press", "justice", "asks", "society",
  "knowledge", "arcs", "update",
] as const;
export const PRESSURES = ["world_demand", "commodity_prices", "depletion", "competitiveness", "labour_costs", "track", "worry"] as const;
export const SHOCKS = ["world_economy", "commodities", "hazard", "race"] as const;

const date = (year: number, month: number, day: number): DateView => ({ day_number: (year - 2000) * 360 + (month - 1) * 30 + day, year, month, day });

export const person = (name: string): PersonRefView => ({ id: name ? 7 : null, ambient_slot: null, name, role_key: "role-councillor", tier: "named", level: name ? "heard" : "unknown", rumoured: false, can_talk: false, has_news: false });

function step(id: number, depth: number, kind: string, on: DateView, text_key: string, args: Record<string, string>, led_to: [number, LinkDto] | null, causes: CauseView[]): WhyStepView {
  return { id, kind, date: on, text_key, args, depth, led_to: led_to ? { event: led_to[0], link: led_to[1] } : null, causes };
}

const link = (event: number, l: LinkDto, in_chain = true): CauseView => ({ type: "event", event, link: l, in_chain });

/** A law the player introduced and voted for, passed: one line of causes (continues, prompted), the player's act behind it. */
export function lawFixture(): WhyView {
  const bill = { policy_key: "policy-transit", town: "Iverland" };
  return {
    event: 40,
    chain: [
      step(40, 0, "policy_enacted", date(2031, 3, 12), "event-policy-enacted", { ...bill, yes: "5", no: "2" }, null, [{ type: "process", process: "lawmaking" }, link(31, "continues")]),
      step(31, 1, "player_deed", date(2031, 3, 5), "event-player-deed", {}, [40, "continues"], [{ type: "player", act: "vote" }, link(22, "continues")]),
      step(22, 2, "policy_introduced", date(2031, 1, 20), "event-policy-introduced", bill, [31, "continues"], [{ type: "player", act: "introduce" }, link(9, "prompted")]),
      step(9, 3, "proposal_opened", date(2030, 11, 2), "event-proposal-opened", { family: "family.transit" }, [22, "prompted"], [{ type: "process", process: "agenda" }]),
    ],
    truncated: false,
    player: { act: "vote", event: 31, depth: 1 },
    depth: 6,
    max_depth: 12,
  };
}

/** A plant closing in a recession: a pressure and a link that caused it; the recession is an outside shock. */
export function closureFixture(): WhyView {
  return {
    event: 70,
    chain: [
      step(70, 0, "plant_closure", date(2032, 6, 1), "event-plant-closure", { town: "Iverland", sector_key: "sector-manufacturing", jobs: "120" }, null, [{ type: "pressure", pressure: "world_demand" }, link(55, "caused")]),
      step(55, 1, "global_recession_began", date(2032, 2, 1), "event-global-recession", { depth: "4" }, [70, "caused"], [{ type: "exogenous", shock: "world_economy" }]),
    ],
    truncated: false,
    player: null,
    depth: 6,
    max_depth: 12,
  };
}

/**
 * A promise kept, with two lines of causes behind it (the deed that kept it,
 * and the promise it settled, which a closure prompted), a story that brought
 * a deed to light, and every other kind of start; more lies further back.
 */
export function everythingFixture(): WhyView {
  const ask = { ask_key: "promise-jobs" };
  return {
    event: 90,
    chain: [
      step(90, 0, "ask_resolved", date(2033, 5, 3), "event-ask-resolved", { ...ask, outcome: "kept" }, null, [{ type: "process", process: "asks" }, link(88, "caused"), link(60, "ended")]),
      step(88, 1, "player_deed", date(2033, 5, 3), "event-player-deed", {}, [90, "caused"], [{ type: "player", act: "town_hall" }]),
      step(60, 1, "ask_accepted", date(2033, 2, 1), "event-ask-accepted", ask, [90, "ended"], [{ type: "player", act: "promise" }, link(50, "continues")]),
      step(50, 2, "ask_offered", date(2033, 1, 20), "event-ask-offered", ask, [60, "continues"], [{ type: "actor", person: person("Vera Holt") }, { type: "actor", person: null }, link(45, "prompted")]),
      step(45, 3, "scandal_broke", date(2033, 1, 10), "event-scandal-broke", { name: "", town: "Iverland", fact_key: "fact-bribe" }, [50, "prompted"], [
        { type: "process", process: "press" },
        { type: "actor", person: person("") },
        link(41, "revealed"),
      ]),
      step(41, 4, "disaster", date(2032, 12, 1), "event-disaster", { town: "Iverland", hazard_key: "hazard-flood", severity: "40" }, [45, "revealed"], [
        { type: "exogenous", shock: "hazard" },
        { type: "history" },
        { type: "forgotten" },
        { type: "unknown" },
        link(12, "caused", false),
      ]),
    ],
    truncated: true,
    player: { act: "town_hall", event: 88, depth: 1 },
    depth: 6,
    max_depth: 12,
  };
}
