## DEC-044 revision 2 — Save format compatibility mechanism

- **Status:** Accepted (agent technical decision during M0, 2026-11-18; within the owner's delegated technical authority).
- **Change:** revision 1 specified separate save DTO structs mirroring every entity. Duplicating every simulation type would double maintenance across all milestones. Revision 2 achieves the same goal, a stored format independent of internal refactors, through a different mechanism:
  - **The stored payload is a versioned JSON snapshot** of the serialized state, DEFLATE-compressed, with a BLAKE3 integrity hash. It has a binary header (`MNDT`, format version, metadata length, metadata JSON) so metadata can be read without inflating the payload.
  - **Compatibility comes from JSON-level migrations** (`vN → vN+1` functions on `serde_json::Value`) plus a fixture corpus.
  - **A shape-fingerprint test** (`mandate-save/tests/saves.rs`) fails whenever the serialized state shape changes without a `SAVE_FORMAT_VERSION` bump. Refactors can never silently break saves.
  - **Pre-release policy:** development builds may bump the version without a migration. Such saves are rejected with a clear message. From the first public release, every bump requires a migration and a fixture.
- **Protocol DTOs remain separate** from internal types (DEC-043). Only the save payload uses this mechanism.
- **Later:** DEC-074 replaced the command log with a full save after every action, and lets saves load under updated rules.

