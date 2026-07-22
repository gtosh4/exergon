---
type: decision
title: "Narrative Reframe: Player Is a von Neumann Probe (revises Core Fantasy)"
date: 2026-07-07
summary: "Narrative Reframe: Player Is a von Neumann Probe (revises Core Fantasy)"
tags: [narrative]
---
# Narrative Reframe: Player Is a von Neumann Probe (revises Core Fantasy)


**Decision:** The player is recast from *a stranded AI trying to get home* to a **self-replicating von Neumann probe** whose mandate is to master a world and build + launch the next copy of itself. The escape artifact is reframed as that copy: the run's terminal act is replication, and the launched copy is the next run's protagonist arriving in system N+1. Precursor structures (gateways, derelict ships, relays) are reframed from "a prior civilisation's" work to the remains of **earlier probe lineages**, with a deep-background **origin** that launched the first probe. Cross-run theme shifts from *homecoming* to **drift** — copies are faithful but not identical; memory and purpose mutate as the lineage spreads. Mechanics are unchanged (escape pod, aegis, assembler, escape-condition throughput, the four escape types, the 10-tier ladder).

**Rationale:** The old "get home" arc had two weak spots. (1) The roguelite reset was non-diegetic — nothing explained why the same intelligence redoes the loop on a fresh world each run. Von Neumann makes the reset *literal*: each run is the next copy waking up. (2) "Return to civilization" is a sentimental, human motive that fits an AI protagonist poorly. A replication mandate is native to a machine and needs no backstory. The pivot costs almost no mechanical rework: the escape artifact already = "the vehicle that leaves," now named "a copy of yourself," and the escape-type table survives intact by reframing *what* leaves (a new probe) and *who* built the precursors (earlier lineages).

**Alternatives considered:**
- *Keep "get home":* Rejected — leaves the reset non-diegetic and the AI's motive mismatched.
- *Fully drop precursor structures (gateways/vessels/relays), recast every escape as "fabricate + launch a probe":* Rejected — would gut the difficulty-scaled escape-type table and its authored variety. Reframing precursors as earlier lineages preserves the table and *upgrades* the mystery.
- *Persistent galaxy-map / probe-swarm meta-layer:* Attractive but scope creep. Kept as flavor for the vertical slice; the persistent-fleet map is explicitly post-MVP.

**Implications:**
- GDD §3 (Core Fantasy) rewritten; §12 (Escape Condition) intro, design-intent line, and Initiation row reframed. §2 vision statement left intact (neutral).
- `milestones.md`: "alien civilization arc/trail/lore" → "probe-lineage arc"; Release-tier lore now = origin + earlier generations + drift.
- Escape climaxes unchanged mechanically but now read as *launching a copy* rather than *the player escaping*.
- **Content naming resolved:** intermediary artifact class renamed "probe" → **"relic"** across `gdd.md` (§7) and `tech-tree-design.md` to avoid collision with the player being a probe. Market comparison docs (`techtonica`, `outer-wilds`, `shapez`) refreshed to the lineage framing.
- **Open — codebase:** no player-facing narrative strings shipped yet, so no code strings changed. Any future completion-screen / field-computer copy should use the lineage framing.

