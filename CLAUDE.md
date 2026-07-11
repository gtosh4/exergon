# CLAUDE.md

## What Is Being Built

**Exergon** = 3D factory-building roguelite. Each run: procedurally generated alien world with unique physical laws. Core loop: decode world science → design factory exploiting those laws → build escape vehicle. Inspiration: GregTech: New Horizons depth + Factorio mechanics + Slay the Spire meta-progression.
Built using `bevy` game engine and `clap` for CLI argument parsing.

## Before Implementing Any Feature

1. Read relevant section of [gdd](docs/gdd.md) for design intent
2. Read relevant section of [tech-design](docs/technical/technical-design.md) for architecture
3. Check [milestones](docs/milestones.md) — is this feature in scope for current milestone?
4. Did we make a design decision? Record in [log](docs/design-decisions.md)

## Keeping Docs Current
**Docs = source of truth. Keep in sync with decisions and code.**
See [docs](docs/README.md) for full doc index.

Docs and code diverge → reconcile explicitly. Never leave silent.

## Instructions
- Use TDD: write tests first based on requirements. Minimal tests to cover requirements, not comprehensive
  - Test non-ecs methods (ie, systems) directly, `World`, no `App`. The fastest tests you can write
  - See [ecs.md](.claude/skills/bevy/ecs.md) for how to test systems
  - Don't test: what the compiler already proves. trivial getters, external/dependency crates (eg bevy)
- Always add non-test code above `mod tests`
- When writing code, make sure to run `cargo fmt`, `cargo clippy` and `cargo test`; make sure lints and tests pass

## Exploring & editing RON content
Query and edit the game's RON assets with the `assets` **MCP server** (registered in `.mcp.json` as `exergon-assets`) instead of reading/editing `assets/**.ron` by hand — it loads and writes through the real (de)serializers, so what you see/write = what the game sees. Generic over a `kind` argument (recipe, tech, item, material, machine, placeable, vein, …): `list_assets`, `get_asset`, `create_asset`, `update_asset` (JSON merge-patch), `delete_asset`; plus `list_kinds` / `describe_kind` (schema) for discovery and graph queries `resolve_recipe` / `list_all_recipes` / `tech_path` / `item_uses`. See [testing.md](docs/technical/testing.md) §4.

## Verifying gameplay in tests
The landing→victory path is regression-tested end-to-end in `tests/standard_full_run.rs`, which fast-forwards simulated time (no manual play). **When you implement a new stage on that path (research tier, crafting step, exploration unlock, escape), add a matching stage to that test.** See [testing.md](docs/technical/testing.md) §3 for the step-by-step recipe. This keeps the dev loop at `cargo test`.

## Development
Exergon is in very early development and is a solo-developer effort for now. Thus, don't use remote branches / PRs. Local branches are okay if necessary, but development is mostly single-threaded so they aren't needed by default.
