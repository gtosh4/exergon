---
type: decision
title: "Pod Delivers Three Starting Structures; No Hand-Crafting Phase"
date: 2026-05-23
summary: "Pod Delivers Three Starting Structures; No Hand-Crafting Phase"
tags: [building]
---
# Pod Delivers Three Starting Structures; No Hand-Crafting Phase


**Decision:** The escape pod is a single all-in-one structure: it projects the aegis field, houses starting storage (small pre-stocked resource cache), and contains a built-in assembler (machine-zero for crafting all other machines). The pod is self-powered — it runs its own Aegis Emitter and assembler, but cannot supply power to externally placed machines. The player has no hand-crafting ability. The first thing the player must independently build is a power source.

**Rationale:** Pillar 2 ("The Design Phase Is the Game") protects the planning moment before each production line, not the bootstrapping of tools. A hand-gathering/hand-crafting phase before the first machine is systems friction (against Pillar 3), not interesting planning. Putting three clear structures on the ground immediately orients the player and gets them to the first interesting decision — what to research, what to build first — without a grind gate.

**Alternatives considered:**
- *Hand-crafting phase (Factorio model):* Player manually gathers raw materials and crafts first machines by hand. Rejected — friction without planning interest.
- *Pod auto-deploys full starter factory:* Too much given up front; reduces first placement decisions.
- *Pod deploys only Aegis Emitter:* No path to build anything; defers the bootstrap problem without solving it.
- *Separate pod generator as 4th structure:* Rejected in favor of pod being self-contained; cleaner first-impression, avoids orphaned generator entity.

**Implications:**
- Starting storage pre-stock must be sufficient to build at least one power source and one drone from the assembler — balance TBD.
- Power isolation (pod can't power external machines) makes "build a power source" the first mandatory decision — a natural tutorial moment.
- Pod is a permanent fixture; it cannot be picked up or moved. Players build around it.
- Tutorial system should call out the pod's power limitation explicitly on first run.

