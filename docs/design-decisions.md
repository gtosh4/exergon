# Design Decisions Log

Rationale and context behind key decisions. The GDD contains the *what*; this document captures the *why* and records alternatives considered. Update when decisions are made or revisited.

---

## 2026-05-23 — Pod Delivers Three Starting Structures; No Hand-Crafting Phase

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

---

## 2026-05-15 — Exploration Domains Replace Universal Vertical Layers

**Decision:** The world is surface-first. Underground, atmospheric, and orbital content are no longer treated as always-present full vertical layers. They are **exploration/resource domains**: scoped destination types introduced only when a run's tier, recipe graph, or escape objective needs them.

**Rationale:** The current run structure is tier/objective driven: Initiation targets 4-6 hours, Standard 10-15 hours, Advanced 20-30 hours, and Pinnacle 30-50+ hours. A universal multi-layer world adds content, navigation, generation, and UI burden that competes with the real progression spine: tech tiers, recipe graph discovery, planet identity, power transitions, and escape objectives.

**Implications:**
- Surface remains the main factory substrate and default exploration space.
- Initiation should be surface-only except for authored POIs.
- Standard should introduce at most one significant off-surface dependency, and only when the escape objective or resource graph benefits from it.
- Advanced and Pinnacle may use multiple domains, but each domain must justify itself through progression, production, or escape requirements.
- Drone types are access capabilities, not proof that a matching full world layer exists in every run.

**Rejected alternative:** Keep the Minecraft-inherited stack of underground/surface/sky/orbit as a default world model. This was rejected because it implies four complete content spaces before the tier pacing has proven it can support them.
