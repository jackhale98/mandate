// The hard boundary (PIXEL_ART_TOOL.md §4.4, D6): the tool's core and runtime
// import nothing from the game (web/) or from Mandate's art (art/); nothing in
// the tool imports the game; the core uses no DOM and no randomness.
import assert from "node:assert/strict";
import { readFileSync, readdirSync, statSync } from "node:fs";
import path from "node:path";
import { describe, it } from "node:test";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

function files(dir: string): string[] {
  return readdirSync(dir).flatMap((f) => {
    const p = path.join(dir, f);
    return statSync(p).isDirectory() ? files(p) : p.endsWith(".ts") ? [p] : [];
  });
}

const imports = (file: string) => [...readFileSync(file, "utf8").matchAll(/(?:import|export)[^'"]*from\s+["']([^"']+)["']|import\(\s*["']([^"']+)["']\s*\)/g)].map((m) => (m[1] ?? m[2]) as string);

describe("boundary", () => {
  it("core and runtime import nothing from art/ or the game", () => {
    for (const f of [...files(path.join(root, "src/core")), ...files(path.join(root, "src/runtime"))]) {
      for (const spec of imports(f)) {
        const target = spec.startsWith(".") ? path.resolve(path.dirname(f), spec) : spec;
        assert.ok(!target.includes(`${path.sep}art${path.sep}`) && !target.includes(`${path.sep}web${path.sep}`) && !target.startsWith("node:"), `${path.relative(root, f)} imports ${spec}`);
      }
    }
  });
  it("nothing in the tool imports the game", () => {
    for (const f of files(root)) {
      if (f.includes("node_modules")) continue;
      for (const spec of imports(f)) assert.ok(!path.resolve(path.dirname(f), spec).includes(`${path.sep}web${path.sep}`), `${path.relative(root, f)} imports ${spec}`);
    }
  });
  it("the core and runtime use no DOM, clock or randomness", () => {
    for (const f of [...files(path.join(root, "src/core")), ...files(path.join(root, "src/runtime"))]) {
      const src = readFileSync(f, "utf8").replace(/\/\/.*$/gm, "");
      for (const banned of ["document.", "window.", "Math.random", "Date.now", "new Date", "performance.now"]) assert.ok(!src.includes(banned), `${path.relative(root, f)} uses ${banned}`);
    }
  });
});
