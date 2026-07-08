# Design Decisions Log

Rationale and context behind key decisions. The GDD contains the *what*; this document captures the *why* and records alternatives considered. Update when decisions are made or revisited.

---

## 2026-07-08 — GTNH Dev-Channel Lessons: Hint Every Gate; Integration & Community-Maintenance Principles

**Decision:** Three learnings from analysing GTNH's development Discord (~195k messages across beta-testing, github-discussion, quest-dev, wiki-dev — see `market/gtnh.md`) are encoded into the GDD:
1. **Discoverability rule (§7):** an unlock vector the player cannot infer the *existence* of is a bug, not a puzzle. Exploration/observation gates must leave a trail (tech-tree shadow, scan rumor, codex breadcrumb, visible sealed site). The challenge is finding/reaching the trigger, never guessing an undocumented action exists.
2. **Integration over volume (§17):** the dominant cost of a deep pack is making content cohere, not authoring it — reward tightly interlocked sciences over item count, and make coherence machine-checkable via the run validator rather than litigated by hand.
3. **Community maintenance as a supported direction (§17):** a community-maintained content pack is a viable long-term direction (post-Release), but the platform must *lower* its cost — content-as-data + versioned schema, the validator as the community's QA safety net, and the codex as an auto-filled curriculum layer that never depends on a human wiki team.

**Rationale:** GTNH is the clearest proof that (a) players will sustain a legendary-depth pack via an organized volunteer org, and (b) that org's cost is real and specific: a dedicated wiki lead (200+ pages), a quest-dev team, an experimental release train, and standing recipe-integration + QA effort. The dev channels show integration/tooling/QA vastly outweigh new-content creation, and the community's single most-cited anti-fun case is hidden, unhinted gates ("hold some flippers to unlock a quest with no hint it exists"). Encoding the hint rule closes a known failure mode Exergon's exploration/observation unlock vectors are directly exposed to; the platform principles turn GTNH's manual maintenance burden into machine-checked/auto-generated systems Exergon already plans (validator, codex).

**Alternatives considered:**
- *Leave "hint your gates" implicit in the observant-tutorial system (§15):* Rejected — the tutorial is a confusion backstop; discoverability must be guaranteed at the unlock-vector level regardless of tutorial state.
- *Commit to community maintenance as the plan:* Rejected — recorded as a supported *direction*, not a commitment; it stays post-Release and optional. Milestones already list a community content pipeline.

**Implications:**
- §7 gains the "Discoverability rule"; §17 gains "Integration over volume" and "Community maintenance as a supported direction."
- Reinforces run-validator scope (reachability + bounds + balance envelopes) as first-class engineering, and codex-as-curriculum completeness as a hard requirement — both already in the docs, now with an explicit rationale.
- Content-authoring guidance: exploration/observation-gated nodes must ship with an in-run breadcrumb; recipe graphs should favor interlock over count.

---

## 2026-07-07 — Byproduct Discipline Is the Primary World-Reactivity Lever (sharpens §11)

**Decision:** World reactivity (§11) is given a concrete primary driver: **vented byproducts and unconsumed side-streams**. A closed-loop factory that consumes/recycles its waste runs quietly; one that dumps waste drives reactivity faster. The coupling is **soft and bidirectional** — venting some seeded streams degrades the environment, venting others triggers *beneficial* reactions (harvestable atmospheric product, enriched deposit). It never hard-blocks progress. This makes "bad run = ugly escape" mechanically true: an ugly run is ugly *because* it dumped waste.

**Rationale:** §11 already resolved that reactivity is factory-footprint-driven and two-sided (Q#4), but "footprint" was vague. Nullius Discord analysis (~30k messages, 1 year — see `market/nullius.md`) shows **byproduct management is by far the most-engaged activity** (~1,570 messages, #1 theme), and Nullius's strongest single mechanic is that **vented gases are tracked against the terraforming win condition**. Routing waste back into reactivity turns Exergon's deepest planning activity into its pressure system, with one lever, and reinforces the existing "clean vs ugly escape" stance. The user explicitly asked that the coupling be **soft, not hard**, and allow beneficial reactions — hence bidirectional, not a penalty meter.

**Alternatives considered:**
- *Hard penalty (Nullius-style tracked voiding blocking the win):* Rejected per direction — too punishing, conflicts with the no-forced-failure model (§16).
- *Leave §11 "footprint" generic:* Rejected — the vagueness was the gap; a single legible lever (waste) is more designable and more teachable.

**Implications:**
- §11 "Caused by" first bullet sharpened to name vented byproducts/side-streams; new paragraph "Byproduct discipline is the primary reactivity lever" added.
- Content schema (§17) will need per-waste-stream reactivity tags (harmful / neutral / beneficial), seeded per run — an unlock/discovery vector in its own right.
- Reinforces that recipe graphs must *generate* meaningful side-streams (not just clean input→output chains) for this lever to have teeth. Flag for recipe-graph content design.
- Still post-MVP for the beneficial/two-sided half, consistent with §11 and Q#4.

---

## 2026-07-07 — Narrative Reframe: Player Is a von Neumann Probe (revises Core Fantasy)

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
