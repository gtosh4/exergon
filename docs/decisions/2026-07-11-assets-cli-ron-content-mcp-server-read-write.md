---
type: decision
title: "`assets` CLI → RON content MCP server (read/write)"
date: 2026-07-11
summary: "assets CLI → RON content MCP server (read/write)"
tags: [tech-tree, progression, content-pipeline]
---
# `assets` CLI → RON content MCP server (read/write)


**Decision:** The `assets` binary is no longer a read-only query CLI; it is an **MCP stdio server** (`crates/assets/src/main.rs`, built on the official `rmcp` SDK) exposing full **CRUD** over every RON asset kind. The tools are **generic over a `kind` argument** — `list_assets` / `get_asset` / `create_asset` / `update_asset` / `delete_asset` / `query_assets` (jq query over a kind's JSON array), plus `list_kinds` and `describe_kind` (returns a kind's JSON schema so clients still get the exact entity shape) — rather than one named tool per type (which was ~76 tools; the generic surface is ~13). Also keeps the resolved-graph read queries (`resolve_recipe`, `list_all_recipes`, `tech_path`, `item_uses`) and a texture-manifest pair. It is registered for this repo in `.mcp.json` as `exergon-assets`. All read/write goes through the game's real (de)serializers via a new engine-free `src/asset_store` module, so tools operate on exactly what the game loads. Three sub-decisions:
- **Structured, field-level updates via JSON merge-patch:** `update_<kind>` takes `{ "field": value }`; nested objects merge, arrays/scalars replace wholesale. Implemented generically (serialize → merge → re-deserialize/validate → write) rather than per-field tools.
- **Canonical RON on write:** every field is re-emitted, so `#[serde(default)]` fields (`energy_output`, `template_id`, `max_reach`, …) become explicit in any rewritten file. Accepted so the write path is uniform across all kinds.
- **Lossless via the *complete* def types:** required adding `Serialize` + `schemars::JsonSchema` to every asset def and nested enum, and using the complete `MachineFileDef` (not the partial `MachineDef`/`MachineItemEntry`) so machine writes don't silently drop fields. Guarded by a round-trip regression test over every asset dir (`tests/asset_roundtrip.rs`).

**Rationale:** The CLI already loaded content through the real loaders, but only for inspection; content authoring still meant hand-editing `assets/**.ron`, which drifts from the schema and can't be driven by an agent. An MCP server turns content authoring into a first-class, validated, agent-drivable surface while keeping the "what you see is what the game loads" guarantee. Merge-patch keeps updates ergonomic without a combinatorial explosion of per-field tools. The stdout channel belongs to JSON-RPC, so logging is routed to stderr.

**Alternatives considered:**
- *Hand-rolled JSON-RPC over stdio (no new deps):* Rejected — `rmcp` is spec-compliant and future-proof; the added `tokio`/`schemars` weight is confined to this binary.
- *Whole-file RON writes (validated blob) instead of structured edits:* Rejected — structured merge-patch is safer and more agent-friendly; the generic implementation made it cheap across all kinds.
- *Keep the CLI subcommands alongside MCP:* Rejected — the binary is MCP-only now; the manual-poke path is newline-delimited JSON-RPC (documented in testing.md §4).

**Implications:**
- New deps: `rmcp` (server/transport-io/macros/schemars), `tokio`. New module `src/asset_store`; `MachineFileDef` re-exported from `src/machine`. (Server-only deps later moved into the `exergon-assets` workspace crate — see the 2026-07-12 entry; `schemars` stays in the game crate for the def-type derives.)
- Docs updated: testing.md §4 rewritten (tool reference + manual poke), §3 and CLAUDE.md now point at the MCP tools.
- Round-tripped files normalise to canonical RON (defaults explicit) the first time they're rewritten.

