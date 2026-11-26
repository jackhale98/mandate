/// <reference types="node" />
// Rust ⇄ TS round trip for the constitution posts (DEC-098, R2): the posts
// the engine (its Node WASM build) sends in `CampaignView.nominations` go
// back in a `File` command as the client typed them, and come back as the
// filing; the player's posts and the office view's bodies agree (the
// deprecated aliases are checked natively, crates/mandate-wasm/tests/posts.rs). Needs target/wasm-node (scripts/build-wasm.sh); skipped without
// it. MANDATE_WASM_NODE overrides its directory.
import { createRequire } from "node:module";
import { existsSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import type { CampaignView } from "./protocol/CampaignView";
import type { CommandDto } from "./protocol/CommandDto";
import type { OfficeView } from "./protocol/OfficeView";
import type { PostDto } from "./protocol/PostDto";
import type { TownPlacesView } from "./protocol/TownPlacesView";
import type { RequestBody } from "./protocol/RequestBody";

const here = path.dirname(fileURLToPath(import.meta.url));
const dir = process.env.MANDATE_WASM_NODE ?? path.resolve(here, "../../../target/wasm-node");
const built = existsSync(path.join(dir, "mandate_wasm.js"));

describe.skipIf(!built)("posts round-trip through the engine (WASM build)", () => {
  it("files for the executive office by the post the engine sent, and reads it back", () => {
    const require = createRequire(import.meta.url);
    const { WasmSession } = require(path.join(dir, "mandate_wasm.js")) as { WasmSession: new () => { handle(s: string): string } };
    const s = new WasmSession();
    let id = 0;
    const send = (body: RequestBody) => {
      const res = JSON.parse(s.handle(JSON.stringify({ protocol_version: 1, request_id: ++id, body })));
      if (res.result.status !== "ok") throw new Error(JSON.stringify(res.result.error));
      return res.result.body.payload;
    };
    const command = (c: CommandDto) => send({ kind: "command", payload: c });
    send({ kind: "new_game", payload: { seed: "4040", size: "small", scenario: null, storyteller: null, local_knowledge: false, recent_openings: [] } });
    // Filing is done where the engine offers it (walked to, as the UI does).
    const town = send({ kind: "query", payload: { type: "town" } }) as TownPlacesView;
    const where = town.neighborhoods.flatMap((h) => h.spots).find((sp) => sp.actions.includes("file"))?.location.location;
    expect(where).toBeDefined();
    if (!where) return;
    const file = (post: PostDto | null, retire: boolean) => command({ type: "go", location: where, then: { type: "file", post, retire } });

    const office = send({ kind: "query", payload: { type: "office" } }) as OfficeView;
    const body = office.bodies.find((b) => b.legislature);
    expect(body?.members.length).toBeGreaterThan(0);
    expect(office.player.posts).toHaveLength(1);
    expect(office.player.role_key).toBe(office.player.posts[0]?.role_key);

    let c = send({ kind: "query", payload: { type: "campaign" } }) as CampaignView;
    const held = c.nominations.find((n) => n.held);
    const exec = c.nominations.find((n) => n.post.kind === "office");
    expect(held?.chosen).toBe(true);
    expect(held?.post).toEqual(office.player.posts[0]);
    expect(exec?.open).toBe(true);
    expect(c.filing).toBeNull();
    if (!exec) return;

    file(exec.post, false);
    c = send({ kind: "query", payload: { type: "campaign" } }) as CampaignView;
    expect(c.filing).toEqual({ type: "stand", post: exec.post });
    expect(c.nominations.find((n) => n.chosen)?.post).toEqual(exec.post);

    file(null, true);
    c = send({ kind: "query", payload: { type: "campaign" } }) as CampaignView;
    expect(c.filing).toEqual({ type: "retire" });
    expect(c.nominations.some((n) => n.chosen)).toBe(false);

    // Neither: stand again for the post held.
    file(null, false);
    c = send({ kind: "query", payload: { type: "campaign" } }) as CampaignView;
    expect(c.filing).toEqual({ type: "stand", post: held?.post });

    // A post the player may not stand for is refused.
    expect(() => file({ ...exec.post, id: "no_such_office" }, false)).toThrow(/no such post/);
  }, 120_000);
});
