# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Status

**Early implementation phase.** Design is complete; initial code scaffold exists. The `src/` directory contains a Bevy 0.18 project with module stubs for all core systems. No system is fully implemented yet.

See [`docs/README.md`](docs/README.md) for a full index of all documentation.

## What Is Being Built

**Exergon** is a 3D voxel factory-building roguelike where every run is a procedurally generated alien world with unique physical laws. The player is stranded and must escape — ultimately leaving the solar system. Core loop: decode the world's science → design a factory exploiting those laws → build the escape vehicle. Primary inspiration: GregTech: New Horizons depth + Factorio factory mechanics + Slay the Spire roguelike meta-progression.

## Design Pillars (filter all decisions through these)

1. **Legible Chaos** — Procedural variance must produce solvable, in-world-explicable problems, not arbitrary noise.
2. **Design Phase Is the Game** — Planning and graph analysis are the primary gameplay; execution is secondary. Minimize watch-and-fix time.
3. **Difficulty Through Depth, Not Friction** — Hard means genuinely complex graph, not obscure UI or tedious grinding.
4. **Content Is Data, Engine Is Platform** — All game content (nodes, recipes, planet modifiers, biomes) in RON data files, not code. The official game ships as the reference content pack. Modders write data, not engine code.

## Key Resolved Decisions

Full rationale in [`docs/design-decisions.md`](docs/design-decisions.md). Key facts:

- **3D voxel world**, free-form block placement. 1 block = 1 meter.
- **Multiblock machines**: fixed recognisable core + flexible modules. Tier = physical size. 8 orientations (4 rotations × 2 mirror states).
- **Logistics**: ME-style network, discrete channel limits, unified storage, auto-crafting job dispatch (machines auto-register capable recipes — no manual patterns).
- **Power**: flow-based (watts), separate power cables, recipe-based demand, proportional brownout throttling.
- **Exploration**: player-piloted drones (not autonomous). Land + digger drones for MVP.
- **Science**: multiple research types, specialised analysis stations, crafting-style experiments. Partial reveal earned through gameplay, not purchased.
- **No forced failure conditions.** Runs always completable. Permadeath modes post-MVP.
- **Escape**: leave the solar system. By difficulty: alien gateway → intra-system ship → inter-system ship.
- **Two science tracks**: universal (real-world-inspired) + alien (seeded per run, prior civilisation's tech).
- **Meta-narrative**: each run = one leg of a galactic journey. System N → system N+1.

## Tech Stack

- Language: Rust, edition 2024
- Engine: Bevy 0.18 (ECS-native throughout)
- Voxel: `bevy_voxel_world` 0.15
- Save format: SQLite via `rusqlite` (bundled)
- Content format: RON via `ron` crate
- RNG: `rand` 0.8 + `rand_pcg` 0.3 (stable deterministic)
- Sub-seed derivation: `xxhash-rust` (xxh64)
- Noise: `noise` 0.9

## Toolchain

- Build: `cargo build`
- Test: `cargo test`
- Lint: `cargo clippy`
- Format: `cargo fmt`
- Mutation testing: `cargo mutants` (gitignored output: `mutants.out*/`)
- IDE: Zed

## Before Implementing Any System

1. Read the relevant section of [`docs/gdd.md`](docs/gdd.md) for design intent
2. Read the relevant section of [`docs/technical-design.md`](docs/technical-design.md) for architecture
3. Check [`docs/milestones.md`](docs/milestones.md) — is this system in scope for the current milestone?

## Keeping Docs Current

**Docs are the source of truth. Keep them in sync with decisions and code.**

| When | What to update |
|---|---|
| Design decision made | `docs/gdd.md` (the what) + `docs/design-decisions.md` (the why) |
| Architecture decision made | `docs/technical-design.md` |
| Open question resolved | Mark resolved in `docs/gdd.md` §18 + relevant section |
| Scope changes | `docs/milestones.md` |
| New open question arises | Add to `docs/gdd.md` §18 and the relevant section |

When docs and code diverge, reconcile explicitly — never leave them silently out of sync.
