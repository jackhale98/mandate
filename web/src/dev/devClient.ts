// The content workbench's engine calls (dev builds only). The dev queries
// exist only in an engine built with the `dev-tools` feature; the release
// engine refuses them as unknown requests, which `isDevless` recognises.
// Everything here goes through the one engine client: the page never
// evaluates a condition itself (the engine's evaluators decide).
import { EngineError, expectKind, type EngineClient } from "../engine/client";
import type { RequestBody } from "../engine/protocol/RequestBody";
import type { ResponseBody } from "../engine/protocol/ResponseBody";
import type { DevCatalogView } from "./protocol/DevCatalogView";
import type { DevExplainView } from "./protocol/DevExplainView";

export type DevKind = "dilemma" | "situational" | "storyline" | "overheard" | "topic" | "line" | "voice" | "scene";

export const DEV_KINDS: DevKind[] = ["dilemma", "situational", "storyline", "overheard", "topic", "line", "voice", "scene"];

/** Kinds explained for one speaker (the page offers a speaker picker). */
export const SPEAKER_KINDS: DevKind[] = ["topic", "line", "voice"];

/** The dev request bodies (not in the generated `RequestBody`). */
type DevQuery =
  | { type: "dev_catalog" }
  | { type: "dev_explain"; kind: string; id: string; speaker: number | null; place: number | null };

type DevResponse =
  | { kind: "dev_catalog"; payload: DevCatalogView }
  | { kind: "dev_explain"; payload: DevExplainView };

async function devQuery(client: EngineClient, q: DevQuery): Promise<DevResponse> {
  const body = { kind: "query", payload: q } as unknown as RequestBody;
  const res: ResponseBody = await client.request(body);
  return res as unknown as DevResponse;
}

export async function devCatalog(client: EngineClient): Promise<DevCatalogView> {
  const r = await devQuery(client, { type: "dev_catalog" });
  if (r.kind !== "dev_catalog") throw new Error(`expected dev_catalog, got ${r.kind}`);
  return r.payload;
}

export async function devExplain(
  client: EngineClient,
  kind: string,
  id: string,
  speaker: number | null,
  place: number | null,
): Promise<DevExplainView> {
  const r = await devQuery(client, { type: "dev_explain", kind, id, speaker, place });
  if (r.kind !== "dev_explain") throw new Error(`expected dev_explain, got ${r.kind}`);
  return r.payload;
}

/** The engine was built without `dev-tools` (it does not know the dev queries). */
export function isDevless(e: unknown): boolean {
  return e instanceof EngineError && e.code === "bad_request" && /unknown variant `dev_catalog`/.test(e.message);
}

/** A new game for the workbench (its own engine; nothing is saved). */
export async function newGame(client: EngineClient, seed: string): Promise<void> {
  const body: RequestBody = {
    kind: "new_game",
    payload: { seed, size: "small", scenario: null, storyteller: null, local_knowledge: false },
  };
  await client.request(body);
}

/** Pass time on the workbench's own engine (end turns). */
export async function endTurns(client: EngineClient, n: number): Promise<void> {
  for (let i = 0; i < n; i++) await client.request({ kind: "command", payload: { type: "end_turn" } });
}

/** The English Fluent source, as the engine serves it. */
export async function englishFtl(client: EngineClient): Promise<string> {
  const r = await client.request({ kind: "query", payload: { type: "locale", lang: "en" } });
  return expectKind(r, "locale").ftl;
}
