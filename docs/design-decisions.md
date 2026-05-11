# Design Decisions Log

Rationale and context behind key decisions. The GDD contains the *what*; this document captures the *why* and records alternatives considered. Update when decisions are made or revisited.

---

## Visual perspective — 3D (resolves GDD Q#3)

**Decision:** Fully 3D world.

**Why:** Three reasons: (1) adds meaningful depth to exploration via vertical layers; (2) enables impressive aesthetic builds that 2D space constraints would prevent; (3) base-sharing is a core intended community loop — sharing screenshots of impressive builds is how the GTNH community sustains itself, and Exergon targets the same behavior.

**Alternatives considered:** 2D top-down (Factorio-style, space as primary constraint), 2D isometric. Both were rejected because spatial layout optimization is not the intended primary challenge.


---

## Multiblock machines — fixed core + flexible modules

**Decision:** Each machine type has a canonical fixed-shape core (recognizable silhouette) with flexible modular attachments.

**Why:** Fixed core = recognizability from screenshots (viewer can identify machine type at a glance). Flexible modules = aesthetic latitude and optimization decisions. This is the best combination of iconic identity and creative freedom.

**Module slot count:** Determined by core tier — upgrading the core earns more slots. Tier is expressed through visually distinct model variants — higher-tier machines look more impressive and complex; size may increase to accommodate more module slots but is not required. Late-game machines visually dwarf early-game ones.

**Tier-up model:** Replace-in-place — upgrading a machine despawns the current tier prefab and spawns the tier+1 prefab at the same position and orientation; machine state (current recipe, inventory, module assignments, IO configuration) transfers where compatible. The original additive/in-place concept (smaller structure as valid sub-structure of the larger, expanding outward) was considered and superseded by this simpler model.

**Modules carry real tradeoffs:** speed vs. efficiency, parallel processing slots, buffer capacity. Not cosmetic. Which modules exist in a given run is a valid seed variance axis.

---

## Logistics network — ME-style, discrete channels

**Decision:** ME-style logistics network (not belt routing). Discrete channel capacity limits. Unified network storage. On-demand auto-crafting.

**Why — no belts:** Belt routing in 3D is untenable as a design challenge. The intended complexity is the recipe graph and network architecture, not path-finding cables.

**Why — discrete channels:** Simplest capacity model, analogous to AE2. Creates natural pressure to segment networks into logical sub-networks (smelting zone, processing zone, etc.) connected via interfaces. Segmentation is a depth mechanic to discover, not a forced constraint.

**Why — auto-crafting:** Players define the crafting graph; the network executes it on demand. Design challenge is configuration, not clicking. Directly serves Pillar 2 — work is in the planning.

**Why — unified storage:** Storage exists as a system but is not intended as a primary design constraint. Inventory management is friction, not depth.

**Revisit:** Channel model (discrete vs. bandwidth) is explicitly tentative and can be revisited.

---

## Exploration — drone-based, tier-gated

**Decision:** Exploration via deployable drones, not direct player travel. Drone types are tier-gated through the tech tree.

| Drone | Access |
|---|---|
| Land | Surface |
| Amphibious | Water / underwater |
| Digger | Underground |
| Flying | Sky / atmosphere |
| Space | Orbital |

**Why:** Drones naturally gate layer access by factory progression — you need the tech and factory output to build better drones. This creates organic pacing without arbitrary locks. The player always has a visible next step (build a digger drone → access underground resources).

---

## World layers — vertical with biome+layer resource affinity

**Decision:** Distinct vertical layers (underground, surface, sky, orbital). Resources have affinity or hard restriction to layer+biome combinations.

**Why:** Gives resource geography meaningful depth. Experienced players can read scan data to immediately identify which resource strategies are viable this run. Layer+biome combinations are a variance axis.

---

## Map reveal — imprecise scanning

**Decision:** Fog of war lifted by drone presence. Range scanning provides biome type and broad resource category (not exact quantities or positions). Precise data requires physical proximity or deployed sensors.

**Why:** Preserves the discovery feeling and information scarcity of scouting while giving players enough data to make exploration decisions. Fits "Legible Chaos" — you know enough to plan, not so much that the world is pre-solved.

---

## Persistent sites

**Decision:** Points of interest (ruins, sealed doors, anomalies) are permanent structures visible before they are accessible. Seeing something you can't interact with yet is a visible future goal.

**Why:** Creates concrete mid-run milestones. "I saw a sealed vault in tier 1 — now I have the tech to open it" is a satisfying progression beat. Sites are sources of exploration discoveries and tech tree unlock triggers.

---

## Theme and narrative — escape the solar system

**Decision:** The player is stranded; the goal across all runs is to leave the solar system. Escape type scales with difficulty: Initiation = activate alien gateway, Standard = gateway or intra-system ship, Advanced = intra-system ship, Pinnacle = inter-system ship.

**Why:** Gives the factory-building a concrete narrative purpose and a thematic arc across meta-progression. Early runs rely on discovered alien technology (lower barrier to entry); later runs require full mastery to build your own way out. The arc from "use their tech" to "build your own" maps cleanly onto difficulty progression and rewards long-term play.

**Implications:** Each run = one leg of a galactic journey. Escape system N → arrive stranded in system N+1. The meta-progression narrative is the story of that journey. Alien ruins across multiple systems = same prior civilisation, traveled this route before you. Codex = accumulated knowledge across all systems visited, part scientific journal part trail map. Orbital layer = launch point. Meta-progression narrative unlocks explore who the prior civilisation was and why they traveled this route.

**Two science tracks:** Universal science (real-world-inspired, applies any run) and alien science (seeded per run, prior civilisation's tech) are complementary, not exclusive. Both feed the same recipe graph. Some tech nodes offer alternative paths: human-engineering approach (production milestone/research) vs. alien-science approach (exploration/observation). Rewards both explorer-first and factory-first playstyles.

---

## Failure conditions — none forced (resolves GDD Q#6)

**Decision:** No forced failure conditions. Runs always complete. World reactivity, power collapse, resource pressure = strategic costs, not run-enders. Permadeath modes post-MVP.

**Why:** Run lengths are 10–30+ hours. Forced failure at that scale is devastating, not interesting. Difficulty expresses as elegance of solution — bad run = slow ugly escape, good run = clean optimized one.

**Permadeath post-MVP:** Will likely involve save constraints or meta-progression penalties rather than forced run termination. Multiple variants expected.

---

## Meta-progression — Codex (resolves GDD Q#7)

**Decision:** Persistent codex fills in through play. First encounter with a biome/node type/modifier/machine creates its entry. Entries record type-level knowledge, not run-specific values.

**Why:** Rewards thorough play across runs without undermining per-run discovery. Biome entry shows possible resource pool — which resources spawned is still seeded variance. Experienced players read the map faster, not more easily. Fits "meta-progression expands possibility space without making runs easier."

**Scope:** Biomes → resource pools; node types → tier range + behavior; planet modifiers → affected systems; machines → function + module types (after first build). Expands automatically through play, not gated.

---

## Science discovery — drone clarification and building blind

**Drones are player-piloted, not autonomous.** The player's character stays at the base; the player's *attention* travels via drone control (**Remote mode**). Returning to the body is **Local mode**. Scouting and sample collection are active time costs. Drones are mobility tools, not automation.

**Building blind** (committing to a partially-revealed recipe at risk) is a post-MVP optional challenge mode, not core. MVP assumes players reveal before committing.

---

## Tech tree — goal-oriented tiers aligned with escape conditions (supersedes prior tier count decision, resolves GDD Q#2)

**Decision:** Canonical 10-tier sequence. Each difficulty uses a prefix: Initiation=1–3, Standard=1–5, Advanced=1–7, Pinnacle=1–10. Tier names and gate conditions are goal-oriented — each tier is a narrative step toward its difficulty's escape condition, not a generic depth label.

**Tier names:** Landfall → Roots → Contact → Reach → Salvage → Traverse → Interface → Revelation → Forge → Transcendence.

**Tiers 3, 5, 7 have two variants:**
- *Terminal*: the escape objective for that difficulty (gateway, derelict ship, relay)
- *Intermediary*: a different alien artifact class that yields materials/knowledge but cannot itself be used to escape — T3: ruin/cache (unlocks alien material or machine type); T5: alien fabrication probe (extract fabrication data); T7: alien archive (extract FTL theory fragments)
- Immersion preserved by roguelite logic: each run is a different world with a different precursor remnant. Same civilization, different artifact.

**Why goal-oriented tiers:** Generic tier names (Foundation/Expansion/Mastery) don't communicate the run's narrative arc. Goal-oriented tiers make the escape feel like a conclusion the entire run was building toward, not a bolt-on win condition. Each tier gate is a milestone in the specific escape arc.

**Why larger tier gaps:** Prior design used 3/4/5/6 tiers — a 1-tier increment per difficulty, producing minimal run-length differentiation. New design (3/5/7/10) creates meaningful depth gaps and adds two entirely new tier phases (Revelation, Forge) that only exist in Pinnacle, making the hardest difficulty genuinely distinct rather than just "more of the same."

**Escape condition redesign:** Replaced sustained throughput requirement with multi-step construction + activation per escape type:
- T3 (Initiation): craft activation key + sustain gateway power + activate
- T5 (Standard): construct ship systems (hull, nav, engines, life support) + produce alien fuel + launch
- T7 (Advanced): collect seeded relay fragments + construct repair components + sustain power + activate; fragment count fixed, locations seeded per run
- T10 (Pinnacle): construct four major systems (engines, FTL drive, reactor, shielding) + assemble + launch

**Why multi-step escape:** Sustained throughput was too mechanically thin — proved factory works but didn't create a dramatic final act. Multi-step construction gives the escape a shape: parallel build tracks, a field-collection phase (T7 fragments), and a moment of activation. Scales appropriately with difficulty.

**Node visibility in shadow:** Locked nodes show category + rarity (e.g. "Power — Tier 2, Rare"), not blank slots. Gives enough information to plan without removing discovery reward.

**Unlock vectors:** All five (research spend, prerequisite chain, production milestone, exploration discovery, observation) are MVP. A node can have multiple active vectors per run — any one suffices.

**Reachability guarantee:** If a node is present in a run's tree, it must be reachable. Seed generation must validate this before finalizing the tree.

---

## Power system model — V×A with non-punishing failures

**Decision:** Power has two independent dimensions: **Voltage tier** (qualitative delivery level — LV / MV / HV / …) and **Amperage** (simultaneous throughput). Their product is the instantaneous watt draw. Cables carry a `(voltage_tier, max_amps)` pair; recipes carry a `min_voltage_tier`. All failure modes are **non-destructive** — machines pause or block, cables never burn, machines never explode.

Specific failure behaviors:
- Voltage mismatch (network tier < recipe requirement): machine blocked, does not start, reason displayed
- Amp cap reached: machine blocked, waits until headroom frees
- Generator buffers empty mid-recipe: recipe pauses, amps held, resumes when buffers refill
- Cable removal causes amp overload: affected machines pause and release amps, resume when headroom restores

Generators have fixed output; machines draw recipe-based wattage when active, zero when idle. Upgrade pressure comes from demand growth outpacing fixed supply — not from degradation. Planet modifiers apply efficiency multipliers to generator output. Power cables are physically separate from logistics cables.

---

**Design space investigation:**

We audited power systems across the factory/automation genre before committing:

| Game | System | Shortage behavior | Destruction risk |
|---|---|---|---|
| Factorio | Single scalar (watts) | Proportional slowdown, all machines equally | None |
| Satisfactory | Single scalar (MW) | Hard grid cutoff, manual fuse reset | None |
| Dyson Sphere Program | Single scalar (MW) | Proportional slowdown, cascade collapse at ~10% | None |
| Mindustry | Single scalar (power/tick) | Proportional slowdown | None |
| Oxygen Not Included | Watts + wire tier capacity | Wire tile takes damage (random tile), breaks | None (tiles repaired) |
| GregTech (GTNH) | Voltage tier × Amperage | Machine blocked / cable fire / machine explosion | Yes — permanent |

**GregTech is the only mainstream factory game where power misconfiguration permanently destroys infrastructure.**

---

**What GregTech gets right:**

The V×A model creates genuine infrastructure design decisions absent from all single-scalar systems:
- Cable tier selection (voltage + amp rating per segment)
- Transformer placement at tier boundaries — inter-tier power routing is an explicit design choice, not a free connection
- Amp routing and per-zone amperage accounting
- "Power epochs" — each machine tier upgrade requires retrofitting cables, machines, and transformers, creating real architectural evolution across the run

Veterans consistently praise these decisions. The system is widely considered the most intellectually interesting power model in the genre.

**What GregTech gets wrong:**

The failure mode is explosive and permanent. Community research surfaced consistent patterns:
1. **Opaque failure feedback** — the game doesn't tell you *why* the explosion happened. Players lose progress to rules they couldn't observe.
2. **Culture shock from Factorio/Satisfactory** — every other popular factory game uses a single scalar where "more power = more better" and failure is graceful. GT's "wrong voltage = base explodes" violates deeply conditioned player expectations.
3. **Three specific mental model collisions** for players coming from simpler systems: (a) power is one number, not two dimensions; (b) more generation never hurts; (c) shortage = slow, not catastrophic.
4. **Supplementary guide requirement** — GTNH has a player-written "Snagger's Electricity Guide for New Players" separate from the main electricity article, plus 4+ dedicated YouTube tutorial series, specifically because in-game documentation is insufficient to navigate the system safely.
5. **~35% of experienced modpack players actively avoid GT** (FTB community poll). Simplified GT-adjacent packs (Omnifactory, Nomifactory) exist specifically to preserve the progression depth while eliminating the "mistakes destroy your base" penalty — market evidence that demand for the mechanic without the punishment is real.

**Factorio's power system** is the genre benchmark for legibility: the Electric Network Info screen gives ranked consumption, adjustable history graph, and satisfaction bar. Brownouts are puzzles because you can see exactly what's wrong. This is separate from the single-scalar model — it's a UI/feedback design lesson applicable to any model.

**ONI's wire tier system** demonstrates that capacity tiers + failure consequences can coexist without catastrophic loss: exceeding wire capacity damages a tile (a Dupe fixes it), not the machine.

---

**Why V×A over single scalar:**

We could match Factorio's single-scalar model. We chose not to because:
- Single-scalar power is invisible as a design domain. Factorio's power system disappears once you've overbuilt. There is no ongoing infrastructure design challenge.
- V×A creates decisions that recur at every tier upgrade: what cable grade, where to place transformers, how to segment amp zones. These decisions parallel the recipe graph decisions that are the intellectual core of the game — they're the same kind of thinking.
- Planet modifiers (solar efficiency, combustion yield, geothermal availability) are more interesting when applied to a power system that already has structural depth. A modifier that halves solar output is a minor inconvenience in Factorio; in a V×A system it affects which tier strategy is viable this run.

**Why non-punishing failures:**

The GT failure model conflates two separable things: *system depth* (the V×A model) and *punishment severity* (explosions). Every other factory game proves these are independent axes. You can have the depth without the punishment.

Non-punishing failures are also consistent with the broader game design principle (see "Failure conditions — none forced"): difficulty expresses as elegance of solution, not run termination. A misconfigured power network that pauses the factory and displays the reason is a puzzle. A misconfigured network that destroys machines and provides no explanation is friction.

**The specific differentiator:** GT's primary flaw is failure *communication*, not the failure itself. "Your cable burned" is not the same as "Your cable burned because its amp rating (2A) was exceeded — 3A were in use when you removed the bypass cable." Exergon can deliver the first form of power system with the second form of feedback, which no other game in the genre has done.

**Higher punishment as opt-in challenge modifiers:**

Non-punishing is the *default*, not the only option. GT-style consequences are a natural fit for the point-buy challenge modifier system (GDD §14) — players who want the harder failure model can opt in and earn challenge points for doing so. Candidates: cable tier degradation on sustained overload, machine damage on voltage mismatch, cascading segment shutdown on amp overload, hidden power diagnostics. Veterans who cleared runs at baseline and want a GT-authentic experience can reconstruct it through modifiers; new players never encounter it unless they choose to.

---

## QoL tool progression — knowledge gates, not friction gates

**Decision:** Factory QoL tools (ratio calculator, auto-crafting network, recipe-chain auto-resolution, blueprint deployment) are in-run Engineering research unlocks, not available from run start. Default unlock windows are calibrated so each tool arrives after the player has encountered and worked through the problem it solves. Earlier access is purchasable as a boon in the run modifier point-buy system.

**Why — knowledge gates:** Tools arriving before players understand what they solve remove learning, not friction. A ratio calculator on day one prevents players from building recipe-graph intuition. Auto-crafting before tracing multi-step chains manually produces a factory the player doesn't understand. Each tool should feel like relief — "finally I don't have to do this by hand" — not like a feature on a checklist. That relief requires having done it by hand first.

**Why — boons for earlier access:** Veterans who have learned the systems in prior runs don't need to re-learn them. Earlier access boons let experienced players trade challenge-point cost (earned only through completed runs) for reduced rote re-tread. By definition, a player spending challenge points has already done the runs — they understand what they're bypassing.

**Boon cost principle:** Cost must reflect depth of challenge removed. Early ratio calculator = minor convenience, low cost. Early auto-crafting = skips a significant architectural learning phase, higher cost. Early full recipe-chain resolution = highest cost. No tool access boon should be an obvious always-buy for experienced players.

**Point-buy net — minimum-zero:** Net must be ≥ 0. Players can run at exactly net-zero (pure difficulty customization — different challenges offset by matching boons) or net-positive (harder than tier baseline). Net-negative is not allowed — no run can be softer than its difficulty tier's baseline.

**Why minimum-zero over fixed-zero:** Fixed-zero prevents voluntary self-challenge within a tier. A veteran who has mastered Standard but isn't ready for Advanced has no headroom to push without jumping tiers. Minimum-zero preserves that axis.

**Why not scaling minimum per tier:** The difficulty tier already sets a harder baseline. Requiring Advanced players to also net-positive on top of harder base content double-counts difficulty and reads as punishing. Plain Advanced (no modifiers) should be a valid and clean run.

**Community comparability:** Within a difficulty tier, every player is at baseline or above. Leaderboards and seed sharing categorize by tier first, net modifier value second — same pattern as Hades Heat or Slay the Spire Ascension level.

**What tools don't do:** Earlier access shifts *when* tools arrive, not *what* they do. Ratio calculator shows numbers; player still decides what to build. Auto-crafting handles execution; player still designs the crafting graph. Tools surface the problem more clearly — they don't eliminate it.

---

## Player body constraint — hardware-hostile environment, per-run hazard type

**Decision:** The open environment is hostile to the AI's hardware. The AI body can only operate in Local mode inside aegis fields projected by infrastructure (Aegis Emitter, Outpost Beacons). Outside, hardware damage accumulates and leads to body loss. The hazard type is a planet property seeded per run (EM interference, corrosive particulates, exotic radiation, etc.) — cosmetically distinct but mechanically identical across all types. Drones are ruggedized expendable hardware, unaffected.

**Why — hardware-hostile over "lethal atmosphere":** The player is an AI, not an organism. "Lethal atmosphere" implies biological life support, which is inconsistent with the character identity. A hardware-hostile environment explains why sophisticated AI cognition can't operate freely outside aegis fields while simple drone firmware can — smarter hardware is more vulnerable.

**Why — per-run hazard type:** Identical hazard on every planet reads as contrivance. Varying the flavor per run fits the planet modifier system (§5) and the Legible Chaos pillar — players read scan data at landing and learn what threat they're dealing with this run. Mechanically identical; the distinction is narrative texture and feedback framing only.

**Why — drones unaffected:** Drones use simpler hardened hardware designed for open-environment operation. This is the diegetic explanation for why Remote mode exploration is possible while the AI body must stay shielded. Higher drone tiers tolerate more extreme conditions (space, deep underground), consistent with the tier system.

---

## Power transition drama — post-MVP

**Decision:** Dramatic in-world power transition events (factory going dark, world reacting to energy signature change) are a post-MVP enhancement, not core.

**Why:** Power transitions are already meaningful as economic decisions (when is the pain of staying on this source worse than rebuilding?). The drama layer adds narrative texture but is not required for the core gameplay loop to function.
