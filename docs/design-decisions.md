# Design Decisions Log

Rationale and context behind key decisions. The GDD contains the *what*; this document captures the *why* and records alternatives considered. Update when decisions are made or revisited.

---

## 2026-07-04 — Lander Drops a Placeable Starting Kit; No Hand Scanner (revises 2026-05-23)

**Decision:** The escape pod no longer contains a built-in assembler or a pre-stocked cache of raw materials. On a fresh run it drops a **starting kit** of placeable machines — a miner, a solar generator, an assembler, and an analysis station — plus 100 each of logistics and power cables, and **no raw materials** (`pod::starting_kit`). The player places and wires these to stand up the first base. The **built-in hand scanner is removed entirely** (component, system, event, interaction, and docs).

**Rationale:** The hand scanner existed only to break the "need research to build the research station / need the station to get research" chicken-and-egg. With `science_basics` auto-unlocking `basic_analysis` and the analysis station handed to the player directly, that knot never forms — the lander *is* the bootstrap. The hand scanner was a single-use verb (manual click-to-scan exists nowhere else, abandoned within minutes) — friction against Pillar 3 for no planning interest. Giving concrete machines instead teaches the real production loop from turn 0.

Crucially, this closes the brick risk the previous model carried: because the machines are given (not crafted from a finite starting cache that a player could spend wrong), and because the origin deposit is guaranteed stone-bearing and within the Aegis radius, no run can lock itself out of progression.

**What this revises from 2026-05-23:** The pod is no longer an all-in-one assembler/storage structure. It still projects the Aegis field and is a permanent fixture, but "machine-zero" is now a placeable assembler in the kit rather than built into the hull. The "starting storage pre-stock" is replaced by the placeable kit (machines + cables, no ore).

**Alternatives considered:**
- *Keep the hand scanner as a safety-net fallback:* Rejected — the guaranteed origin deposit + non-consumable kit already make the loop un-brickable, so the fallback is redundant weight.
- *Give raw materials instead of machines:* Rejected — a finite material cache can be spent on the wrong things and brick the run; that risk is exactly what prompted this change.
- *Pre-place and pre-wire the kit:* Considered; deferred in favor of handing the kit as inventory so the player learns placement/wiring immediately (a guaranteed deposit still prevents a bad-seed brick).

**Implications:**
- Stone must be mineable: the starter deposit (`iron_copper`) now yields `stone`, and `DepositRegistry::ore_at` forces a stone-bearing deposit in the origin cell `(0,0)`.
- Miners are now placeable (`assets/machines/miner.ron`); `place_machine_system` latches a placed miner onto the nearest deposit in range, and `miner_tick_system` resolves its network via its logistics ports.
- Building *more* of each kit machine is now supported: `make_assembler` (via `basic_processing`), `make_analysis_station` (`science_basics`), `make_miner` (`ore_extraction`, retargeted from the never-implemented `drill`/`extract_*`), and `logistics_cable_craft` / `power_cable_craft` (`logistics_basics` / `power_basics`).
- The `iron_copper` deposit now yields `iron_ore`/`copper_ore` (the ids the recipe graph generates from the `iron`/`copper` materials × `ore` form), fixing the mined-ore-vs-recipe-input mismatch. The `xalite` deposit (`xalite`/`resonite` vs `xalite_shard`) has the same class of mismatch and is still open.
- End-to-end coverage: `tests/landing_to_first_research.rs` drives the real placement + cable + mining + recipe + research systems from placing machines through unlocking the first node.

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
