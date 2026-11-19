#!/usr/bin/env node
// The pixelkit CLI (see src/cli/main.ts). Needs Node ≥ 22.18 (TypeScript type stripping).
import "./hooks.mjs";

await import("../src/cli/main.ts");
