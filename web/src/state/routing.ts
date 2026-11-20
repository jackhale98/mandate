// Auto-routing (DEC-079 rule 2): which of the engine's routes a command uses.
// The engine says where each action is done (PocketView.routes) and validates
// every command; this only maps a command to its action key.
import type { CommandDto } from "../engine/protocol/CommandDto";
import type { LocationDto } from "../engine/protocol/LocationDto";
import type { OfficeView } from "../engine/protocol/OfficeView";
import type { PocketView } from "../engine/protocol/PocketView";
import type { RouteView } from "../engine/protocol/RouteView";

export interface RouteKey {
  action: string;
  lateNight: boolean;
  neighborhood: number | null;
}

/** The action key of a command, or null for movement and time. */
export function routeKey(cmd: CommandDto, office: OfficeView | null): RouteKey | null {
  const k = (action: string, lateNight = false, neighborhood: number | null = null): RouteKey => ({ action, lateNight, neighborhood });
  switch (cmd.type) {
    case "cast_vote":
      return k("vote");
    case "file":
      return k("file");
    case "draft_policy":
      return k("draft");
    case "amend_policy": {
      // Drafts are revised at the clerk's desk, bills on the chamber floor.
      const draft = office?.bills.find((b) => b.id === cmd.policy)?.status === "draft";
      return k(draft ? "draft" : "amend");
    }
    case "introduce_policy":
      return k("introduce");
    case "withdraw_policy":
      return k("withdraw");
    case "act": {
      const a = cmd.action;
      switch (a.type) {
        case "lobby":
        case "trade_favor":
          return k(a.type, cmd.late_night);
        case "fundraise":
          return k("fundraise", cmd.late_night);
        case "canvass":
          return k("canvass", false, a.neighborhood);
        case "town_hall":
          return k("town_hall", false, a.neighborhood);
        case "press":
        case "advertise":
        case "rest":
        case "dip_funds":
        case "leak_file":
          return k(a.type);
        case "take_bribe":
          return k("take_bribe", true);
      }
      return null;
    }
    case "end_turn":
    case "advance_days":
    case "move_to":
    case "go":
    case "meet":
    case "answer_decision": // decisions are answered from anywhere (DEC-079)
    case "notebook_seen": // looking at the notebook, anywhere (K5)
    case "talk": // where the person is; the talk panel walks there first (DEC-093)
    case "set_directives": // time skips (DEC-012): no place, no slot
    case "skip":
    case "cancel_skip":
      return null;
  }
}

export function routeFor(pocket: PocketView | null, key: RouteKey): RouteView | undefined {
  return pocket?.routes.find((r) => r.action === key.action && r.late_night === key.lateNight && r.neighborhood === key.neighborhood);
}

export function findRoute(pocket: PocketView | null, office: OfficeView | null, cmd: CommandDto): RouteView | undefined {
  const key = routeKey(cmd, office);
  return key ? routeFor(pocket, key) : undefined;
}

export function sameLocation(a: LocationDto, b: LocationDto): boolean {
  if (a.type !== b.type) return false;
  if (a.type === "street" && b.type === "street") return a.neighborhood === b.neighborhood;
  if (a.type === "place" && b.type === "place") return a.place === b.place;
  return true;
}
