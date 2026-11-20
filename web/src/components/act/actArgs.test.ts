import { describe, expect, it } from "vitest";
import { EMPTY_TRANSLATOR } from "../../i18n/fluent";
import { actArgs } from "./actArgs";

describe("actArgs", () => {
  it("passes numbers as numbers and resolves `_key` arguments to their text", () => {
    const a = actArgs(EMPTY_TRANSLATOR, { years: "8", value: "-3", name: "Ada Park", target_key: "skip-target-campaign" });
    expect(a.years).toBe(8);
    expect(a.value).toBe(-3);
    expect(a.name).toBe("Ada Park");
    // No such message in the empty bundle: the gap shows (the key is resolved, not passed raw).
    expect(a.target).toBe("⟦skip-target-campaign⟧");
    expect("target_key" in a).toBe(false);
  });
});
