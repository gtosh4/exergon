# CLAUDE.md

## Project Status

**Early implementation phase.** Design complete; initial scaffold exists. `src/` = Bevy 0.18 project with module stubs for all core systems. No system fully implemented yet.

See [`docs/README.md`](docs/README.md) for full doc index.

## What Is Being Built

**Exergon** = 3D voxel factory-building roguelike. Each run: procedurally generated alien world with unique physical laws. Player stranded, must escape — leave solar system. Core loop: decode world science → design factory exploiting those laws → build escape vehicle. Inspiration: GregTech: New Horizons depth + Factorio mechanics + Slay the Spire meta-progression.

## Design Pillars (filter all decisions through these)

1. **Legible Chaos** — Procedural variance must produce solvable, in-world-explicable problems, not arbitrary noise.
2. **Design Phase Is the Game** — Planning and graph analysis = primary gameplay; execution secondary. Minimize watch-and-fix time.
3. **Difficulty Through Depth, Not Friction** — Hard means genuinely complex graph, not obscure UI or tedious grinding.
4. **Content Is Data, Engine Is Platform** — All game content (nodes, recipes, planet modifiers, biomes) in RON data files, not code. Official game ships as reference content pack. Modders write data, not engine code.

## Key Resolved Decisions

Full rationale in [`docs/design-decisions.md`](docs/design-decisions.md). Key facts:

- **3D voxel world**, free-form block placement. 1 block = 1 meter.
- **Multiblock machines**: fixed recognisable core + flexible modules. Tier = physical size. 8 orientations (4 rotations × 2 mirror states).
- **Logistics**: ME-style network, discrete channel limits, unified storage, auto-crafting job dispatch (machines auto-register capable recipes — no manual patterns).
- **Power**: flow-based (watts), separate power cables, recipe-based demand, proportional brownout throttling.
- **Exploration**: player-piloted drones (not autonomous). Land + digger drones for MVP.
- **Science**: multiple research types, specialised analysis stations, crafting-style experiments. Partial reveal earned through gameplay, not purchased.
- **No forced failure conditions.** Runs always completable. Permadeath modes post-MVP.
- **Escape**: leave solar system. By difficulty: alien gateway → intra-system ship → inter-system ship.
- **Two science tracks**: universal (real-world-inspired) + alien (seeded per run, prior civilisation's tech).
- **Meta-narrative**: each run = one leg of galactic journey. System N → system N+1.

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

1. Read relevant section of [`docs/gdd.md`](docs/gdd.md) for design intent
2. Read relevant section of [`docs/technical-design.md`](docs/technical-design.md) for architecture
3. Check [`docs/milestones.md`](docs/milestones.md) — is this system in scope for current milestone?

## Keeping Docs Current

**Docs = source of truth. Keep in sync with decisions and code.**

| When | What to update |
|---|---|
| Design decision made | `docs/gdd.md` (the what) + `docs/design-decisions.md` (the why) |
| Architecture decision made | `docs/technical-design.md` |
| Open question resolved | Mark resolved in `docs/gdd.md` §18 + relevant section |
| Scope changes | `docs/milestones.md` |
| New open question arises | Add to `docs/gdd.md` §18 and relevant section |

Docs and code diverge → reconcile explicitly. Never leave silent.
