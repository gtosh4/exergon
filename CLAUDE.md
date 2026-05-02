# CLAUDE.md

## What Is Being Built

**Exergon** = 3D voxel factory-building roguelite. Each run: procedurally generated alien world with unique physical laws. Core loop: decode world science → design factory exploiting those laws → build escape vehicle. Inspiration: GregTech: New Horizons depth + Factorio mechanics + Slay the Spire meta-progression.
Built using `bevy` game engine and `clap` for CLI argument parsing.

## Before Implementing Any Feature

1. Read relevant section of `docs/gdd.md` for design intent
2. Read relevant section of `docs/technical-design.md` for architecture
3. Check `docs/milestones.md` — is this feature in scope for current milestone?

## Keeping Docs Current
**Docs = source of truth. Keep in sync with decisions and code.**
See `docs/README.md` for full doc index.

Docs and code diverge → reconcile explicitly. Never leave silent.

## Instructions
- Use TDD: write tests first based on requirements. Minimal tests to cover requirements, not comprehensive
  - Test non-ecs methods (ie, systems) directly, `World`, no `App`. The fastest tests you can write
  - See [ecs.md](.claude/skills/bevy/ecs.md) for how to test systems
  - Don't test: what the compiler already proves. trivial getters, external/dependency crates (eg bevy)
- Always add non-test code above `mod tests`
