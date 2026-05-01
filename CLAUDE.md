# CLAUDE.md

## What Is Being Built

**Exergon** = 3D voxel factory-building roguelike. Each run: procedurally generated alien world with unique physical laws. Core loop: decode world science → design factory exploiting those laws → build escape vehicle. Inspiration: GregTech: New Horizons depth + Factorio mechanics + Slay the Spire meta-progression.
Built using `bevy` game engine.

## Before Implementing Any System

1. Read relevant section of `docs/gdd.md` for design intent
2. Read relevant section of `docs/technical-design.md` for architecture
3. Check `docs/milestones.md` — is this system in scope for current milestone?

## Keeping Docs Current
**Docs = source of truth. Keep in sync with decisions and code.**
See `docs/README.md` for full doc index.

Docs and code diverge → reconcile explicitly. Never leave silent.

## Instructions
- Always use `Component`s for data that should go in save files over `Resource`s
