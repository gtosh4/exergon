# Design Decisions Log

Rationale and context behind key decisions. The GDD contains the *what*; this document captures the *why* and records alternatives considered. Update when decisions are made or revisited.

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
