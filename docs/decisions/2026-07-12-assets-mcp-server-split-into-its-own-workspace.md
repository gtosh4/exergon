---
type: decision
title: "`assets` MCP server split into its own workspace crate; `query_assets` jq tool added"
date: 2026-07-12
summary: "assets MCP server split into its own workspace crate; queryassets jq tool added"
tags: [content-pipeline]
---
# `assets` MCP server split into its own workspace crate; `query_assets` jq tool added


**Decision:** The repo becomes a Cargo **workspace**: the root package `exergon` (the game) plus a member crate `exergon-assets` (`crates/assets/`, binary still named `assets`). The MCP server source moves `src/bin/assets.rs` → `crates/assets/src/main.rs`, and the launch command changes `cargo run -q --bin assets` → `cargo run -q -p exergon-assets --bin assets` (still from the repo root so `assets/` is reachable; `.mcp.json` updated). A new tool **`query_assets(kind, jq)`** is added: it runs a jq program (via the pure-Rust `jaq-core`/`jaq-std`/`jaq-json`) over the JSON array of every entity of `kind` and returns the program's outputs — a single output bare, a multi-value stream as an array. The generic tool surface is now `list_kinds`, `describe_kind`, `list_assets`, `get_asset`, `create_asset`, `update_asset`, `delete_asset`, `query_assets`, plus the texture-manifest pair and resolved-graph queries (`resolve_recipe`, `list_all_recipes`, `tech_path`, `item_uses`).

**Rationale:** Dependency separation is the point — the game crate `exergon` no longer pulls the server-only deps (`rmcp`, `tokio`, `anyhow`, and the new jq deps `jaq-core`/`jaq-std`/`jaq-json`), which now live in `crates/assets/Cargo.toml`. `schemars` deliberately **stays** in the game crate: the content def types (`ItemDef`, `NodeDef`, `MachineFileDef`, …) derive `schemars::JsonSchema`, so the derive has to live with the types; the assets crate re-lists `schemars` for the schema tools. `src/asset_store` stays in the game crate. `query_assets` gives agents a general-purpose search/filter surface over any kind (top-N by cost, id-pattern selects, counts) without a bespoke tool per query shape.

**Implications:**
- New workspace layout: `crates/assets/` member crate; server-only deps confined there.
- Docs updated: testing.md §4, technical/README.md, this log; agent defs re-pointed at the MCP tools and the new launch command.

