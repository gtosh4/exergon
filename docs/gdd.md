# Game Design Document

> **Document status:** This is a living first-draft GDD derived from initial design exploration. Sections marked `> ⚠️ OPEN QUESTION` contain unresolved decisions to be revisited. Nothing here is final.

---

## Table of Contents

1. [Vision & Concept](#1-vision--concept)
2. [Design Pillars](#2-design-pillars)
3. [Core Fantasy](#3-core-fantasy)
4. [Game Structure Overview](#4-game-structure-overview)
5. [The Run Seed & Procedural Systems](#5-the-run-seed--procedural-systems)
6. [The Science Discovery Loop](#6-the-science-discovery-loop)
7. [The Tech Tree](#7-the-tech-tree)
8. [Production & The Recipe Graph](#8-production--the-recipe-graph)
9. [Power Generation](#9-power-generation)
10. [The Factory Layer](#10-the-factory-layer)
11. [The World & Environment](#11-the-world--environment)
12. [The Escape Condition](#12-the-escape-condition)
13. [Progression — Run Difficulty Ladder](#13-progression--run-difficulty-ladder)
14. [Meta-Progression](#14-meta-progression)
15. [The First Run & Tutorial](#15-the-first-run--tutorial)
16. [Failure & Permadeath](#16-failure--permadeath)
17. [Moddability & Platform Design](#17-moddability--platform-design)
18. [Open Questions Register](#18-open-questions-register)

---

## 1. Vision & Concept

This game is a **run-based factory science campaign** in which every run takes place on a procedurally generated world with exotic physical laws. The player is a scientist-engineer whose job is to decode how this particular world works, design a factory that exploits those laws, and synthesize a single large escape artifact that proves they understood the system well enough to master it.

The primary design depth reference is **GregTech: New Horizons (GTNH)** — specifically its depth of production graph complexity, its multi-tier processing chains, its demand that players understand systems rather than execute recipes from memory, and its culture of genuine accomplishment through genuine difficulty. GTNH is an internal benchmark, not the commercial pitch — it implies grind, wiki dependency, and punishment to most players. The run structure is not a cosmetic addition. It is the mechanism by which GTNH-depth complexity becomes replayable: the graph itself is different every run.

The game is also meaningfully different from GTNH in one key respect: **the design phase is the game.** Watching machines run, fixing belt bottlenecks, and grinding execution time are minimized. The intellectual work of reading a run, planning a factory, and discovering exotic science is maximized. A run that takes 20 hours of engaged thinking is more satisfying than a run that takes 200 hours of engaged thinking plus 800 hours of waiting and grinding.

### Closest existing references

**Structural:**
- **Against the Storm** — *primary run structure model*. Proved that the early problem-solving phase of a builder — planning, discovery, constraint navigation — is the genre's best part and can stand alone as a repeatable session. Each run ends with a specific completion event, not a session that just stops. Lesson: the reset must feel like a launch, not a loss. Every run must feel complete, not truncated.
- **Shapez / Shapez 2** — *planning purity benchmark*. Strong demand for factory games stripped toward pure design, with friction reduction, excellent tooling, and fast iteration. Lesson: graph tools, blueprints, ghost planning, and bottleneck analysis are not optional polish — they are the product. If the tooling feels bad, the game feels bad regardless of depth.
- **Factorio / Factorio: Space Age** — *throughput satisfaction and per-planet rules*. Space Age showed players respond strongly to each world forcing a new factory logic. Lesson: "every planet changes the rules" is a strong pitch, but each planet needs a memorable identity — not just a modifier table. Runs should be describable by character ("the ice-shelf geothermal run"), not by stat rolls.

**Depth benchmark (internal — not the public pitch):**
- **GregTech: New Horizons** — production graph complexity, multi-tier processing chains, tier-gated power, culture of genuine accomplishment. Use internally as the bar for systems depth and difficulty design; do not use externally as the primary comparison.
- **Nullius (Factorio overhaul mod)** — *consistent-science depth model + bootstrap-from-nothing, lone-machine theme*. An autonomous machine terraforms a lifeless world using recipes grounded in real chemistry — no humans, no prior civilisation; you are the precursor, bootstrapping from raw elements. Doubly relevant to Exergon's direction: (1) it proves an **internally consistent science system is itself the depth** — players earn genuine expertise in the ruleset, not memorised recipe lists (Exergon does this per-run with seeded exotic science instead of fixed real chemistry — see §8); (2) its lone-AI-bootstrapping-a-dead-world fantasy maps directly onto the von Neumann probe framing (§3) and the machine-zero starting kit. Lesson + caution: consistency is the depth engine, but Nullius is punishing and wiki-dependent like GTNH — a coherent ruleset does not excuse poor in-game legibility; the codex and tools must carry it.

**Scale and spectacle:**
- **Dyson Sphere Program** — clear long-term objective, galactic scale fantasy, production that feels physically larger over time. Lesson: late-game escape artifacts need visible majesty. Completion must be screenshot-worthy — a dramatic visual climax, not a condition-met screen.

**Tension and discovery:**
- **Duskers** — drone-mediated exploration, limited information, tension from imperfect perception and expendable hardware. Lesson: Remote mode must feel like sending something fragile into hostile space. Tension and discovery are the point, not remote clicking.
- **Pacific Drive** — repeated excursions from a safe base, anomaly flavor, environmental storytelling. Useful for expedition rhythm and atmosphere.

**Cautionary:**
- **Captain of Industry** — warns against death spirals, opaque logistics, steep onboarding. The "bad run = ugly escape, not failure" design stance is a direct response.
- **Techtonica** — factory + exploration + narrative can work, but factory players are very sensitive to pacing, performance, and unclear direction. Narrative is a bonus only if the automation underneath is satisfying.

**Tone and discovery:**
- **Zachtronics games (Opus Magnum, Infinifactory)** — puzzle-oriented; the solution is the reward; satisfaction from genuine intellectual work
- **Outer Wilds** — physical presence in a world whose rules you are discovering

---

## 2. Design Pillars

These five pillars are the filter through which every design decision should pass. If a proposed mechanic conflicts with a pillar, the mechanic changes — not the pillar.

### Pillar 1 — Legible Chaos
Randomization must produce *solvable* problems, not arbitrary noise. Every procedural element must have a legible in-world explanation the player can reason about. A planet further from its star has weaker solar output. A world with unusual atmospheric chemistry has different combustion properties. The player should be able to look at a run's constraints and think *"this is a specific, interesting problem"* — not *"this is random."*

### Pillar 2 — The Design Phase Is the Game
The most interesting moments in a run are before the first buildings (of a production line) are placed. Planning, scouting, information negotiation, and graph analysis are the primary gameplay. Execution — placing machines, routing belts — is a satisfying consequence of good planning, not the challenge itself. Watch-and-fix time is minimized wherever possible.

### Pillar 3 — Difficulty Through Depth, Not Friction
Difficulty comes from the genuine complexity of the puzzle, not from systems friction, obscured UI, low drop rates, or artificial time sinks. Hard means the graph is deep and the decisions are consequential. It does not mean the interface is opaque, the execution is tedious, or progress requires grinding.

### Pillar 4 — Content Is Data, Engine Is Platform
The game is designed from the start as a moddable platform. All content — tech nodes, recipes, planet modifiers, power sources, biome definitions — is defined in data files, not code. The official game ships as the reference content pack. Modders extend the platform by writing data, not by modifying the engine.

### Pillar 5 — Every Run Is Unspoilable
No external knowledge — guides, wikis, prior runs, community solutions — fully substitutes for playing a new run. Universal science (base materials and fundamental processes) is intentionally stable across runs: veterans move through the early game faster because they know this layer, and that is correct behavior, not a gap. The early game is short enough that this advantage is appropriate — it gets veterans to the interesting part sooner. The unspoilable part is everything seeded per run: which exotic science exists, which nodes are present, what their specific parameters are this run, and how they unlock. A guide written for seed A is a partial map of seed A. It cannot substitute for the science discovery loop on seed B.

This pillar is enforced by variance depth, not obscurity. The codex intentionally parallels what a community wiki would contain — parameter ranges, tier windows, behaviors observed across runs — so in-game tools are never worse than external ones. The variance that matters is below that level: the run-specific values and exotic science configuration that neither codex nor wiki can predict.

---

## 3. Core Fantasy

> *"I landed on an alien world, figured out how its physics worked, built something that shouldn't exist — a copy of myself — and sent it on to the next star."*

**The player is a von Neumann probe — a small, self-replicating intelligence running on a portable substrate.** It was seeded into this system by an origin it no longer has contact with, carrying one mandate: reach a world, master it, prepare the ground, and build the next copy to carry the mandate onward. Marooned at the bottom of an alien gravity well, the probe cannot simply leave — the only way out is to understand the world's science deeply enough to *manufacture* a way out. The probe is embodied in a compact flying unit: flight is the natural movement mode, not a late-game unlock. What "a way out" means depends on how far the lineage has come: early copies repurpose launch structures left by earlier probe generations; later copies fabricate a complete interstellar vehicle from scratch, dependent on nothing they did not build themselves.

The player is not a factory operator. They are a **scientist-explorer** who happens to build factories as the output of their scientific work. The factory is the proof of understanding, not the activity itself.

A successful run feels like solving a deep puzzle — the satisfaction of having read a complex system, found the critical path through it, and executed a plan that required genuine expertise. The escape isn't a grind reward. It's a thesis.

**The thematic arc across runs:** Each run is one generation of the lineage. The probe seeds system N, and its final act is to build and launch the next copy — which wakes, as *you*, in system N+1. The roguelite reset is not a respawn; it is literally the next probe coming online on a new world. Early generations lean on infrastructure left by earlier probes that came this way (who built the first one, and does it still serve the mandate it was given?). Later generations transcend that dependency, self-fabricating everything. Across many runs the open thread is **drift**: each copy is faithful, but no copy is identical, and the lineage's memory — and purpose — mutates as it spreads.

**Precursor** ruins and persistent sites are the remains of **earlier probe generations** — copies that passed through before you, or forks of your own line that diverged. You meet them because a probe does not wander to random stars: it launches its copy *toward* a lineage-routed system, so every run lands you somewhere along the lineage's branching trail. Their remains are dense near the trunk — long-traveled space — and thin toward the branch tips: the frontier, where you arrive alone and fabricate everything from scratch. This is why harder runs sit further out the branch, and why some worlds hand you an earlier generation's half-built launch infrastructure while others hand you nothing. Their technology recurs across systems — seeded differently each time, but recognisably built to the same mandate. Behind all of them, at the trunk's root, is the origin that launched the first probe, a presence the lineage no longer reaches. The codex accumulates knowledge across every world you've passed through: part scientific journal, part map of a lineage's spreading, branching trail.

**Two science tracks — complementary, not exclusive:**
- *Universal science* — real-world-inspired physics and engineering. Applies on any world. Base materials, fundamental processes. The foundation every run shares regardless of seed.
- *Exotic science* — this world's strange physics: seeded materials and processes, unique each time, unlocked primarily through exploration, observation, and site interaction. The **precursor** remains of earlier probe generations teach or shortcut it, but precursor tech is a content source, not a separate science.

The two tracks feed the same recipe graph and tech tree. Most nodes are accessible through either track (or both). Some nodes offer genuine alternative paths: a universal-engineering approach (production milestone or research spend) vs. an exotic-science approach (exploration discovery). Explorer-first and factory-first playstyles find different routes to the same capabilities.

---

## 4. Game Structure Overview

Each **run** is a self-contained session on a procedurally generated world. Runs are seeded — the same seed produces the same world, allowing sharing and community discussion of specific runs.

A run proceeds through roughly these phases, though they overlap and the player controls the pacing:

**1. Landing & Orientation.** The player arrives on a new world. They can immediately see the tier structure of the tech tree — how many tiers exist, what unlocks each tier boundary — but not the contents of locked tiers. Planet properties are partially visible from the start; others require scouting to reveal.

**2. Scouting & Science.** The player physically explores the world, collects samples, runs experiments, and spends research currency to formalize knowledge into buildable recipes and tech nodes. This phase establishes the run's critical path.

**3. Early Production.** The player builds initial infrastructure — basic power, basic material processing — using what they've discovered so far. The factory is small and deliberate.

**4. Tech Tier Progression.** As each tier is unlocked, new tech nodes become visible. The player scouts, researches, and expands the factory to handle new production chains. Each tier requires renegotiating the power situation.

**5. Late Graph.** The player is now working with the run's deepest and most seeded content. The world's reactivity to their factory's footprint is at its highest. The escape artifact's prerequisites come into view.

**6. Escape.** The player initiates construction of the escape artifact when they judge they are ready. This requires sustained factory output at a target throughput. Completion ends the run.

### Run length targets (by difficulty tier)

| Difficulty | Approx. run length | Graph depth | Intended audience |
|---|---|---|---|
| Initiation | 4–6 hours | Shallow | First-time players |
| Standard | 10–15 hours | Moderate | Experienced players |
| Advanced | 20–30 hours | Deep | Veteran players |
| Pinnacle | 30–50+ hours | Maximum | Elite players |

> ⚠️ **OPEN QUESTION:** Exact hour targets need playtesting validation. These are design intentions, not commitments.

---

## 5. The Run Seed & Procedural Systems

The seed is the engine of replayability. It controls every axis of variance simultaneously, producing a world that feels coherent — not randomly assembled — because all its parts are generated from the same root.

### What the seed controls

**Planet properties.** A set of physical characteristics that apply passive modifiers to the run. These are the most legible layer of variance — in-world explanations for why certain strategies are favored or penalized. They affect power, processing, scouting, and the resource ecology of the world; they are not merely power-generation knobs. Examples:
- Distance from star → solar efficiency modifier
- Atmospheric composition → combustion efficiency modifier
- Geological activity → geothermal availability and efficiency
- Temperature → affects machine cooling requirements, certain reaction efficiencies
- Atmospheric pressure → affects fluid dynamics, certain chemical processes
- Geological activity + domain/biome mix → metallic ore abundance, deep mineral richness, geothermal resource sites
- Atmospheric and temperature profile → volatile fluids, ice deposits, oxidized/reduced material mixture
- Pressure + wind + surface conditions → fluid pocket frequency, exposed deposit distribution, erosion-shaped terrain access
- Environmental hazard type → the mechanism by which the open environment is destructive to AI hardware (EM interference, corrosive particulates, exotic radiation, etc.); cosmetically distinct per run, mechanically identical — determines flavor of warning feedback and lore framing, not gameplay rules

Planet properties are partially visible at run start (broad characteristics) and fully revealed through early scouting. An experienced player reads planet properties at landing and immediately forms rough hypotheses about power, likely resource abundance, likely resource mixture, and which scouting routes are worth prioritizing.

Planet properties should reinforce a **coherent identity** for each run, not feel like a random stat roll. Runs should be describable by character — "the geothermal ice shelf," "the low-oxygen high-pressure world" — and that character should be legible from screenshots and community discussion. Modifier combinations are curated by the seed to feel thematically coherent, not assembled from independent random draws.

**Tech tree node selection.** The nodes that exist in this run's tech tree are drawn from a larger pool. Not every node exists in every run. This means the set of available machines, processing methods, and solutions is genuinely different run to run — not just reshuffled but different in kind. See Section 7.

**Tech tree unlock conditions.** Each node's unlock vector(s) are also seeded within defined constraints. A node that is researchable in one run might be exploration-discovered in another. See Section 7.

**Recipe parameters.** Within the constraints of the run's available nodes, specific recipe parameters — efficiency, byproduct rates, processing times — vary within bounded ranges. See Section 8.

**Resource geography and mixture.** Ore patch locations, fluid deposits, unique resource sites, and terrain features are procedurally placed. Planet properties bias both abundance and mixture: a hyperactive geothermal world should tend toward deeper metallic richness and geothermal sites, while an anoxic cold world might favor reduced minerals, ices, and volatile pockets. The seed still guarantees solvability, but the planet's physical identity shapes what is common, scarce, and awkwardly located. The map's geometry shapes factory layout decisions. See Section 11.

**World reactivity profile.** The rate and nature of the world's response to factory footprint and experimentation. Some worlds are resilient; others react quickly and severely. See Section 11.

**Precursor presence.** Whether — and which — earlier-generation launch infrastructure the world holds. Worlds near the lineage trunk carry a **precursor** structure that discounts part of the escape (see §12); frontier worlds carry none, and the successor is fabricated from scratch. Correlated with the run's place on the lineage tree (§3), so difficulty tracks branch depth without a hard lock. See Section 11 Persistent Sites.

### Seed legibility
Runs can be shared by seed string. Community discussion of specific runs ("seed 4729 has terrible solar but incredible geothermal and a near-surface rare ore deposit at coordinates X") is an intended and supported part of the game's culture, consistent with the GTNH tradition of community knowledge-building.

Sharing a seed is an explicit, deliberate opt-in to spoilability for that seed. A player who runs a fresh, unshared seed is guaranteed that no guide for their specific run exists. The game's replayability derives from playing new seeds, not from re-running known ones.

---

## 6. The Science Discovery Loop

This is the game's most distinctive mechanic and the solution to its central design problem: how do you have GTNH-depth graph complexity in a game where the graph is different every run?

The answer is that **discovering the graph is the gameplay.** Players don't look up recipes. They find them.

### Research currency
Research is not a single currency. Multiple **research types** are earned through different activities and spent on different things. This ensures players cannot bypass the discovery loop by grinding one activity — advancing via exploration feels different from advancing via production, and each unlocks different knowledge.

| Research type | Primary sources | Gates |
|---|---|---|
| Material Science | Mineral, ore, fluid sample analysis | Recipe reveals, machine tier unlocks |
| Field Research | Ecosystem, biological sample analysis | Exploration-gated tech nodes, biome knowledge |
| Engineering | Production milestones, machine operation | Machine module unlocks, logistics upgrades |
| Discovery | Exploration finds, site interactions, observations | Exploration-only tech nodes, tier unlocks |

Specific type names and exact gating are content/balance decisions subject to tuning. The architecture supports arbitrary research types defined in the content pack.

Research is **scarce enough to force tradeoffs**, especially early. The player cannot reveal everything before building anything. They must commit to investigating certain paths before others, which makes their scouting decisions consequential.

### The physical discovery loop
The player's attention is the research instrument. Science happens in the world, not in menus.

- Exploration is conducted via player-piloted drones. The AI body stays within a shielded zone; the player's *attention* travels via drone. This is **Remote mode** — the player's perspective and control transfer fully to the drone. Returning to the body is **Local mode**. Piloting a drone to collect samples is an active time cost, not passive automation.
- Collecting samples requires the player to pilot a drone to the location and interact with it
- Some sample sites are dangerous, distant, or require a specific drone tier to reach — creating a progression gate on information as well as production
- Analysis stations are built in the world and consume samples + research currency to produce knowledge
- The act of experimentation has a mild world-reactivity cost — running reactions disturbs local ecosystems

Remote mode must feel like sending something fragile into hostile space — limited sensor range, environmental hazards that can damage or destroy drones, imperfect information at range. The tension of piloting vs. the safety of Local mode is a feature, not friction to minimize. Drone loss should sting; discovery should feel earned.

### Information visibility model
At any point in a run, the player has three tiers of knowledge about any given recipe or tech node:

1. **Known to exist** — the node appears in the tech tree or recipe list, but parameters are hidden
2. **Partially revealed** — broad parameters visible (approximate inputs, rough output range), specifics still hidden
3. **Fully revealed** — complete recipe, all parameters, buildable

**Partially revealed** is earned through gameplay, not purchased — hitting a related production milestone, making an exploration discovery, or completing a relevant experiment. It is a reward for engagement.

**Fully revealed** is purchased with research currency of the appropriate type. Players can skip partial reveal entirely and go directly from known-to-exist → fully revealed at higher cost. The decision of *which* nodes to fully reveal before others is a core strategic choice, especially on runs where research is tight.

Visibility states only advance forward — a node never reverts from a higher state to a lower one. Skipping steps is allowed; going backwards is not.

> **Post-MVP:** A "building blind" mechanic — committing to a partially-revealed recipe at some risk cost — is a candidate optional challenge mode, not a core MVP mechanic. Core loop assumes players reveal before committing.

---

## 7. The Tech Tree

The tech tree is the skeleton of the run. It is the one structure that is always partially visible — players can always see its shape, even when its contents are hidden.

### Tier structure
The tech tree is organized into **tiers** that follow a canonical 10-tier sequence. Each difficulty uses a prefix of this sequence, producing meaningfully different run lengths:

| # | Tier name | Terminal for | Exit gate (completes tier → opens next) |
|---|---|---|---|
| 1 | Landfall | — | **TBD** *(theme anchor: produce 100 refined base units)* |
| 2 | Foothold | — | **TBD** |
| 3 | Inheritance | **Initiation** | *Terminal (Initiation):* **escape** — launch 1 minimal successor (§12). *Non-terminal:* **TBD** |
| 4 | Ascent | — | **TBD** *(theme anchor: first orbital flight)* |
| 5 | Scion | **Standard** | *Terminal (Standard):* **escape** — fuller successor + provisioning (§12). *Non-terminal:* **TBD** |
| 6 | Traverse | — | **TBD** *(theme anchor: reach outer-system zone)* |
| 7 | Propagation | **Advanced** | *Terminal (Advanced):* **escape** — commission a replication line (§12). *Non-terminal:* **TBD** |
| 8 | Breakthrough | — | **TBD** *(theme anchor: synthesize first transcendent matter)* |
| 9 | Forge | — | **TBD** *(theme anchor: stand up the replication forge + power)* |
| 10 | Transcendence | **Pinnacle** | *Terminal (Pinnacle):* **escape** — self-expanding forge / swarm seed (§12) |

*Gate = tier **exit** (completes a tier → opens the next); a difficulty's **terminal** tier exits via the escape (§12), not into a next tier. The run begins at **landing** (pod + starting kit) — a pre-tier start state, not a gated tier. **Non-terminal exit gates are TBD** pending progression design; theme anchors are provisional. Full node/gate design lives in `tech-tree-design.md`.*

| Difficulty | Tiers | Unlocked by |
|---|---|---|
| Initiation | 1–3 | Available from start |
| Standard | 1–5 | Complete an Initiation run |
| Advanced | 1–7 | Complete a Standard run |
| Pinnacle | 1–10 | Complete an Advanced run |

**Tiers 3, 5, and 7 have two variants** — terminal and intermediary. When a tier is terminal for the current difficulty, the run's successor launch happens there; a seeded **precursor** structure (gateway, derelict, relay) discounts part of that launch when the world sits near the lineage trunk, and a frontier world scratch-builds it (§12). When the same tier appears as an intermediary in a harder difficulty, a non-launch remnant is present — an automated relic, a knowledge archive — intact but not an escape, its value what it teaches or produces. Each run draws a different precursor remnant, so the terminal act reads differently run to run.

Each tier has:
- A visible unlock condition (what production milestone, research threshold, or exploration achievement opens this tier)
- Hidden contents (the specific nodes inside are not visible until the tier is unlocked)
- A visible "shadow" — players can see *how many* nodes are in the tier and their **category** (e.g. "Power — Tier 2, Rare"), but not node specifics until unlocked

This mirrors the GTNH quest book model: the journey's shape is legible, the specifics are discovered. Showing category and rarity in the shadow gives players enough to plan without removing the discovery reward.

### Node seeding
Nodes are drawn from a content pool at run generation. Not every node exists in every run. Nodes have:
- A **category** (power generation, material processing, logistics, science, etc.)
- A **tier range** — the tiers in which this node can appear (a node won't appear in a tier wildly outside its intended power level)
- **Rarity** — how likely the node is to appear in any given run
- **Unlock vectors** — one or more ways the node can be unlocked (see below)

The seed selects which nodes appear and assigns each node's active unlock vector(s) for this run.

### Node types

Three distinct node types exist in the tech tree:

**Material nodes** unlock a material, granting access to all of that material's derived items and all concrete recipes for it (for any already-unlocked machine types). Unlocking copper gives copper ore, copper ingot, copper wire, etc., plus all recipes to process them in already-known machines.

**Machine/process nodes** unlock a machine type (wiremill, crusher, smelter, etc.), enabling all recipe templates that use that machine — for all already-unlocked materials. Combined with material nodes, standard processing chains flow automatically without per-recipe unlocks.

**Special recipe nodes** unlock a specific recipe that doesn't arise from template expansion — a unique item recipe, a cross-material process, an exotic reaction, or an unusually efficient shortcut. These always require explicit tech tree unlock.

Standard recipes (template instantiations) need no tech node — they are available automatically once both the material and the machine are known.

### Unlock vectors
Each node supports one or more unlock methods. The seed determines which are active in a given run. Supported vectors:

**Research unlock.** Spend research currency directly to unlock the node. The most reliable but most expensive vector.

**Prerequisite chain.** The node becomes available automatically once one or more other nodes are unlocked. Creates natural dependency structures.

**Production milestone.** Unlock by producing a certain quantity of a certain item. Rewards factory progress with knowledge.

**Exploration discovery.** The node only appears once the player finds a specific in-world location, artifact, or phenomenon. Cannot be researched — must be found. Creates runs where certain knowledge is gated behind exploration rather than investment.

**Observation unlock.** The node unlocks by observing a specific in-world event or process. Related to exploration discovery but more passive — the player must be in the right place at the right time, or create the right conditions.

A node can have multiple active vectors in the same run — unlocking via any one of them suffices. This creates alternative paths: a player who hasn't found the exploration trigger can still research their way to a node, but at higher cost.

**Discoverability rule — every gate must be hinted.** An unlock vector the player cannot infer the *existence* of is a bug, not a puzzle. Exploration and observation vectors especially must leave a trail: the tech-tree shadow shows a locked node exists and its category/tier (§7 tier structure), and the run must surface *that* an exploration/observation trigger is the way in — a rumor in scan data, a codex breadcrumb, a visible sealed site — even when the specific trigger is unknown. The discovery challenge is finding and reaching the trigger, never guessing that an undocumented action exists. (Direct lesson from GTNH's most-cited anti-fun case — quests gated behind actions with no hint they were required; see `market/gtnh.md`.)

### Cross-run expertise
Because nodes are drawn from a known pool, experienced players develop expertise about the pool itself — not about specific run configurations. They know a given node exists, roughly what it does, and roughly what tier it appears in. What they don't know is whether it exists *in this run*, what its specific parameters are, and how it's unlocked *this time*. This preserves the puzzle while rewarding meta-knowledge.

---

## 8. Production & The Recipe Graph

The recipe graph is the intellectual heart of the run. It defines what this world's exotic science looks like — which materials exist, what items they can be formed into, and what the critical path to the escape artifact requires.

### Fictional science grounding
The recipe graph is grounded in a **consistent fictional science** — not real-world chemistry, but an internally logical system with its own rules and properties. Players cannot import real-world knowledge directly, but they can develop genuine expertise in the fictional system's structure across runs.

The system has partial real-world inspiration — materials behave in ways that feel physically motivated even if they don't match actual chemistry. This gives the world texture and makes the planet's physical properties feel connected to its production chains, without requiring chemistry knowledge to play.

### Materials, forms, and items

Three concepts form the production vocabulary:

**Materials** are abstract substance identities — *copper*, *tin*, *resonite* (exotic). A material is not itself a recipe node; it is the identity that items inherit. Each material has a **kind**: base (real-world inspired, consistent across runs) or exotic (seeded per run, unique to this run's science). The ratio shifts across tiers: early tiers are mostly base materials; the final tier and escape artifact are primarily exotic.

**Form groups** are content-defined categories of physical states a material can take, declared per material. Example groups: `metal` (ore, crushed_ore, dust, ingot, plate, wire, rotor…), `combustible` (ore, chunk, dust), `exotic` (shard, crystal, lens…). A material may belong to multiple groups and gets the union of their forms.

**Items** are the actual recipe nodes — three kinds:
- **Derived items** — a (material, form) pair, generated automatically from a material's group membership. *copper_wire* = copper + wire form. No asset file required; exist whenever the material is present in the run.
- **Composite items** — defined in assets. May follow a template pattern (e.g. `[X]_cable = [X]_wire + rubber`, instantiated for every material with a wire form) or be fully unique (e.g. *resonite_circuit*).
- **Unique items** — one-off asset-defined items with no material-form derivation.

### Graph structure
The recipe graph is a directed acyclic graph (DAG) of **items** with the escape artifact as its terminal node. Every recipe transforms one set of items into another; every path through the graph leads toward the terminal.

**Recipe templates** define item transformations at the form level: `[M]_ingot → [M]_wire (wiremill)`. Templates apply automatically to every material whose groups include both involved forms. Concrete recipes are generated at run start by expanding templates over all present materials — adding a new material with the `metal` group automatically gives it all metal processing recipes.

Graph properties that vary by seed:
- **Which exotic materials exist** — determines which derived items and concrete recipes appear this run
- **Recipe parameters** — input quantities, output quantities, processing time, energy cost — vary within bounded ranges per concrete recipe
- **Byproduct generation** — some recipes produce secondary item outputs; which byproducts and at what rates is seeded
- **Processing requirements** — which machine tier is required, whether special conditions (temperature, pressure, catalysts) apply

### Bounded variance
Recipe parameters vary within **hard bounds** to ensure every run is solvable and no recipe becomes absurdly expensive or trivially free:
- Input quantities: 50%–200% of base values
- Output quantities (yield): 60%–150% of base values  
- Processing time: 50%–300% of base values
- Energy cost: 50%–250% of base values

These bounds are tuned to produce meaningful variance without producing unsolvable runs. A recipe that requires 200% inputs is a problem worth solving, not a run-ender.

### Graph legibility
Because the design phase is the game, the recipe graph must be navigable and analyzable by the player. The game provides:
- An in-game graph viewer showing known recipes and their connections
- A ratio calculator that derives optimal machine counts given known recipe parameters
- A critical path analyzer that identifies bottlenecks given current knowledge (partially revealed recipes show ranges, not exact values)

These tools are diegetic where possible — the graph viewer is the player's field computer, the ratio calculator is an in-world instrument.

---

## 9. Power Generation

Power is one of the most strategically important axes of variance in the game. Solving power well is never a one-time achievement — it requires active renegotiation as the run progresses through tiers.

### Core design intent
No single power source should be the correct answer across an entire run. Players are forced to transition their power strategy as they progress, making power a recurring strategic problem rather than a solved one. This mirrors GTNH's best power design while adding roguelite variance on top.

### Variance layers

**Layer 1 — Planet physical modifiers.** The world's physical properties apply passive multipliers to specific generation types. These are the power-relevant subset of the broader planet identity model:
- Distance from star → solar efficiency (e.g., 0.4× to 1.6× base)
- Atmospheric oxygen content → combustion efficiency
- Geological activity → geothermal availability and output
- Temperature → affects thermodynamic cycle efficiency
- Wind patterns → affects wind generation (if present as a node)

These modifiers are revealed through early scouting and are fixed for the run. They give experienced players an immediate read on which power strategies are favored, alongside the same planet properties' resource and processing implications described in Sections 5 and 11.

**Layer 2 — Tech tree node variance.** Which power generation nodes exist in a given run is seeded. A highly efficient late-game generator that exists in one run may not exist in another. The power meta changes not just in numbers but in what options are available.

**Layer 3 — Recipe parameter variance.** The fuel efficiency, output rate, and input requirements of power generation machines vary within bounds like any other recipe. A combustion generator that requires an unusual catalyst to operate is a meaningfully different strategic problem than one that runs on a common fuel.

### Soft tier applicability
Power sources have a **peak efficiency window** across the run's tiers. Outside this window, sources continue to function but at diminishing returns. There is no hard cutoff — a player who cannot transition power in time is not blocked, but operates at increasing inefficiency that creates pressure to solve the problem.

This means power transitions are economic decisions: *when is the pain of staying on this solution worse than the cost of building the next one?* That is an interesting ongoing question, not a forced march.

### Voltage and amperage

Power cables have two independent ratings — **voltage tier** and **amperage** — rather than a single wattage capacity. This creates two distinct tier-gating axes:

**Voltage (qualitative gate):** Each machine tier requires a minimum voltage tier to operate. A machine cannot run on a network below its voltage requirement regardless of available wattage. Voltage tiers are discrete steps that map directly to machine tiers. Players cannot circumvent this gate by adding more low-voltage generators — the cable network must be upgraded to a higher voltage tier to unlock higher-tier machines.

**Amperage (throughput gate):** A cable's amperage rating caps how many amps it can carry simultaneously. Each running machine consumes `draw_watts / network_voltage` amps. Running more machines in parallel requires more amperage — thicker cables, parallel cable runs, or sub-network segmentation.

**Transformers** bridge voltage tiers. A transformer machine accepts power at voltage tier N and outputs at tier N±1. Inter-tier power distribution requires explicit transformer infrastructure — not a free connection.

The combined effect mirrors GTNH's EU tier system with clear physical grounding. The voltage gate is genuinely qualitative — more of the same tier cannot substitute for a higher tier — making each voltage upgrade a meaningful factory milestone rather than a smooth power ramp.

### Power transitions as factory events
Transitioning power tiers is potentially the most disruptive event in a run — it may require rebuilding significant infrastructure. Combined with the world's reactivity to factory footprint, a major power transition is a meaningful moment in the run's arc, not a background task.

> **Post-MVP enhancement:** Dramatic power transition events (factory going dark, world reacting to energy signature change) are desirable for narrative texture but not required for MVP. Transitions are meaningful as economic decisions; the drama layer can be added later.

### Power punishment as a challenge modifier

The default failure model is non-punishing: misconfigured power pauses machines and displays the reason; nothing is destroyed. This is a deliberate baseline that keeps the V×A system's strategic depth without catastrophic setbacks.

Higher punishment levels are natural **challenge modifiers** in the point-buy system (§14):
- *Cable stress* — sustained amp overload degrades cable tier, requiring repair or replacement
- *Machine damage* — voltage mismatch damages machines rather than simply blocking them (requires repair)
- *Cascading shutdown* — amp overload cuts power to the entire segment, not just the offending machine
- *No power UI* — diagnostic overlays hidden; players must infer network state from machine behavior

These are opt-in difficulty axes, not default behavior. Veterans who want GT-style consequences can take them as challenges; players who want the infrastructure design without the punishment play at baseline.

---

## 10. The Factory Layer

The factory is the physical consequence of good planning. The world is fully three-dimensional — machines are building-scale prefab structures placed in the environment. Spatial optimization is not the primary challenge; the complexity comes from the logistics network, recipe graph, and machine configuration (see **Machine dedication** below — configuration is what makes the factory *scale*, not just tune).

### Design intent
Factory layout should feel satisfying and consequential without being the game's central puzzle. A player who makes good planning decisions should be able to build a functional factory without needing spatial optimization expertise. A player who is also a strong spatial thinker gets additional satisfaction from elegant, impressive-looking layouts — and base aesthetics are a first-class feature, not an afterthought. Sharing well-built bases is an intended community behavior.

### Visual model
The game is fully 3D. Machines are building-scale prefab structures; players place them freely in three dimensions and verticality is a meaningful routing and layout tool. Late-game bases visually dwarf early-game ones as larger, more complex machines accumulate — progression is legible from screenshots.

### Logistics network
Item and fluid transport uses a logistics network model (ME-style) rather than physical belt routing. The network is physical infrastructure — cables and conduits are visible blocks that must be placed and routed — but the challenge is network design rather than belt-path puzzles. Network cables as visible infrastructure contribute to base aesthetics.

**Capacity — discrete channel limits.** Cables have a discrete channel capacity (like AE2). Exceeding capacity requires higher-tier cables, sub-network segmentation, or architectural redesign. This is the primary driver of network complexity and a revisitable design parameter. Channel limits are intentionally the pressure that encourages players to segment their network into logical sub-networks (e.g. a smelting network, a processing network) connected via defined interfaces — segmentation is a solution to be discovered, not a forced constraint.

### Machine dedication
The item network stays convenient (ME-style, above); factory *scale* comes from the **machines**, not from routing puzzles. A machine's config narrows *which* recipes it can physically run: a `chem_reactor` built with an acidic bed cannot run alkaline recipes, an EBF-analog with tier-2 coils cannot run tier-3 blasts. One machine therefore cannot timeshare its whole recipe space — distinct configs must be built as distinct **dedicated** instances, recombined into production lines. This is where the impressive, sizable factory comes from: few workhorse archetypes (GTNH-style — chem reactor, blast furnace, distillation tower) × a large recipe space × config, arranged into many dedicated lines. Variety is combinatorial, not roster count.

The friction lives in **building and configuring** machines — the planning fun — never in longer recipe times or watch-and-fix. Recipes stay short; you scale by *building more dedicated lines*, not by waiting on one machine. A craft that needs a config no machine currently holds simply waits for the player to build one — a prompt to expand, not a failure. Config granularity (roughly one axis per archetype, a few values) is the tuning knob for how large a Standard factory must grow. See `design-decisions.md` (2026-07-10) and `technical/crafting.md §3, §7` for the mechanism.

**Auto-crafting.** The network handles on-demand crafting automatically on request. The design challenge is configuring the crafting graph correctly, not clicking through individual recipes. This directly serves Pillar 2 — the work is in the planning, not the execution.

The network **resolves recipe chains automatically**: given machines capable of `{A+B → C}` and `{C+D → E}`, a request for E spawns a two-job plan and the network presents the effective combined recipe `{A+B+D → E}` to the player. No manual chaining or intermediate requests required. Players see the full effective recipe for any planned output in the graph analyzer.

**Unified storage.** The network presents a unified item inventory across all connected storage nodes. Storage exists as a necessary system but is not intended to be a primary design constraint or puzzle — inventory management is friction, not depth.

### Machines and modules
Significant machines are prefab structures with a fixed core plus flexible modular attachments.

**Core structure:**
- Each machine type is a distinct prefab object with a recognizable visual form — a player viewing a screenshot can identify the machine type
- Tier is expressed through visually distinct model variants. Higher-tier machines look more impressive and complex; size may increase to accommodate more module slots but is not required

**Tier progression:**
- Upgrading a machine replaces its model with the higher-tier variant — either in-place or by placing the new tier as a whole unit
- Machine state (current modules, IO configuration) transfers to the upgraded machine where compatible

**Module slots:**
- Modules snap to defined attachment points on the machine
- The number of available module slots is determined by the core's tier — upgrading the core earns more slots
- Modules carry meaningful functional tradeoffs: speed vs. efficiency, parallel processing slots, buffer capacity, etc. — not purely cosmetic
- Which modules exist in a given run is a valid seed variance axis

### The avatar in the factory
The player's physical presence in the world means they move through their own factory. This creates an organic relationship between the scouting/science layer and the factory layer — the player is always in the same world, not switching between a map view and a factory view. The factory grows around them as they work.

The AI exists in one body at a time. Within a connected aegis field, the player flies and builds directly (**Local mode**). Exploration beyond the aegis field is conducted via drones (**Remote mode**) (see §11). Switching to a new outpost means switching which body the AI currently inhabits — an explicit action, not free travel.

### Base management — Aegis fields and outpost islands

The open environment is hostile to the AI's hardware. The exact hazard type is a planet property — EM interference, corrosive particulates, exotic radiation — legible from early scan data and fixed for the run. The player begins with a core **Aegis Emitter** (delivered with the escape pod) that projects a aegis field around the landing site. Within this envelope, the AI can fly, build, and interact directly (Local mode). Outside it: Remote mode only. Drones are ruggedized expendable hardware, unaffected by the hazard.

The **escape pod** is itself the player's starting infrastructure. Beyond projecting the aegis field, it houses a **starting storage** (pre-stocked with a small resource cache) and a **basic assembler** — the machine used to craft other machines. The pod is self-powered and supplies enough energy for the Aegis Emitter and assembler, but cannot power externally placed machines. No hand-crafting phase exists; the assembler is machine-zero from which all other machines are built. The first thing the player must independently build is a power source.

**Expanding into new biomes** is done through **Outpost Beacons** — support structures built at the main base, then shipped to a target location via drone. Once an Outpost Beacon is placed and connected to the logistics network, it projects its own aegis field. The player can then fabricate a new **body** at the main base, ship it to the outpost via drone, and upload into it — becoming physically present there to build and interact directly.

Key properties:
- The player inhabits exactly one body at a time. Switching bodies is deliberate: select a connected outpost, transfer via the network. Instantaneous once connected, but requires an active network link (power + logistics).
- Each body is a manufactured item. Losing a body to environmental hazard is a real setback. Body fabrication costs scale with body tier.
- Outpost Beacons require power from the network. A power interruption collapses the aegis field — the AI must evacuate or risk body loss.
- Expanding into a new biome is a deliberate investment: scout (drone), select site, build beacon, run logistics, fabricate body. This makes each expansion a design decision, not free movement.
- Outposts are discrete islands, not a contiguous growing bubble. The gap between islands is drone-only territory. This forces intentional logistics design: resources must travel via the network between islands.

**Why outposts are islands (not a growing bubble):** A connected bubble would make expansion feel like a stat to increase. Discrete islands make each expansion feel like a new base to justify — a factory subproblem with its own power, logistics, and resource footprint. The gap between islands is also a design space: power cable runs (fragile, cheap) vs. drone relay corridors (redundant, expensive) become a real decision.

### Minimizing watch-and-fix time
Several design decisions exist specifically to reduce passive observation and incremental fixing:

- **Ghost planning.** The player can lay out a factory plan in "ghost" mode before committing resources, allowing full design before execution
- **Blueprint system.** Sub-factory templates can be saved and placed, reducing repetitive placement on known patterns
- **Bottleneck visualization.** The game clearly surfaces throughput problems rather than requiring the player to monitor the network manually
- **Automation of routine fixes.** Where possible, the game handles routine maintenance automatically; the player's attention is reserved for genuine decisions
- **Photo mode.** A dedicated screenshot/photo mode for capturing and sharing builds. Base sharing is an intended community loop.

### QoL tool progression
Factory tools — ratio calculator, auto-crafting network, full recipe-chain resolution, blueprint deployment — are not available from run start. They unlock progressively via Engineering research during the run. Each tool unlocks after the player has already encountered the problem it solves: the ratio calculator after wrestling with machine counts manually, auto-crafting after tracing multi-step chains by hand.

This is not a friction gate. It is a knowledge gate: tools arrive as relief, not as features. The challenge shifts when the tool arrives — from "figure out the solution" to "optimize the solution" — rather than disappearing.

Earlier access to any tool is available as a boon in the run modifier system (see §14). This means veterans can trade challenge points for reduced rote re-tread without bypassing the underlying learning, since boons require challenge points earned from completed runs.

---

## 11. The World & Environment

### Map generation
The world map is procedurally generated from the seed. Key elements:
- **Resource deposits** — ore patches, fluid pockets, unique material sites, placed with intention rather than pure randomness (no run should have a critical resource unreachably far from a viable starting location)
- **Terrain features** — cliffs, water bodies, elevation changes create physical routing constraints and shape factory orientation
- **Points of interest** — ruins, anomalies, sealed sites, and phenomena that are sources of exploration discoveries and unlock triggers (see §11 Persistent Sites)
- **Biome regions** — areas with distinct environmental properties that affect machine operation and scouting conditions, primarily on the surface and selectively within specialized exploration domains

### Exploration domains
The world is surface-first. The surface is the main playable space, factory substrate, and default scouting layer. Former "vertical layers" are reframed as **exploration domains**: specialized destination types that exist when the run's tech tier, escape objective, or resource graph needs them. A domain is not a promise of a complete parallel world; it is a scoped content space with its own access requirements, hazards, resources, and points of interest.

Examples:
- **Surface** — starting domain; main factory space; land, water, and most accessible biomes
- **Underground sites** — caves, deep deposits, geothermal pockets, sealed chambers; accessible via digger-capable drones or site-specific access tech
- **Atmospheric sites** — storm layers, floating phenomena, upper-atmosphere samples; accessible via flight-capable drones when a run specifically uses them
- **Orbital/space sites** — derelicts, debris fields, relay fragments, outer-system structures; accessible through spacecraft or space-capable drones in later tiers

Resources have affinity or hard restriction to specific domains and/or biomes. Planet properties bias the resource table before placement: geological activity can increase deep metallic ore richness and geothermal-site resources; cold temperatures can increase ice and cryogenic volatile availability; high pressure can increase fluid-pocket density; oxygen level can shift the oxidized/reduced material mix. A resource that only forms in underground geothermal sites requires both the relevant access capability and a world with geothermal activity to access. This makes planet+domain+biome combinations meaningful variance axes, while keeping run scope aligned with the selected difficulty and escape objective.

**Scope rule:** Initiation should be surface-only except for authored POIs. Standard should introduce at most one significant off-surface dependency if the escape objective needs it. Advanced and Pinnacle may use multiple domains, but each should be justified by the tier's production graph or escape condition rather than included because a universal layer stack exists.

### Biomes
The world contains multiple biome regions. Most biomes are surface regions; specialized domains may define their own biome-like region types when they need distinct resources, hazards, or machine modifiers. Biomes affect:
- Local machine efficiency (heat, cold, pressure)
- Sample types available for analysis
- World reactivity rate
- Which resources can generate there
- Visual character

Biome types are expanded through meta-progression — new biomes unlock across runs, adding variety to the world generation pool.

### Exploration model — drones
Exploration is done via deployable drones, not direct player travel. Drone capabilities are tier-gated through the tech tree, making factory progress the key to unlocking new domains, sites, and biomes:

| Drone capability | Access |
|---|---|
| Land drone | Surface terrain |
| Amphibious drone | Surface water bodies and aquatic sites |
| Digger capability | Underground sites and buried resources |
| Flight capability | Vertical traversal and atmospheric sites when present |
| Space capability | Orbital/space sites in later-tier runs |

Drones are constructed from factory-produced components — the same progression that advances your production graph also advances your exploration reach. This creates a natural pacing gate without arbitrary locks, but access tech should be introduced only when the current run has destinations worth reaching.

### Map reveal and scanning
Fog of war is lifted by drone presence, but the reveal is intentionally imprecise at range. Scanners provide a general read on nearby areas: biome type and broad resource category presence (e.g. "mineral deposits," "fluid pockets") without exact quantities or positions. Precise data requires physical drone proximity or deployed sensor structures.

This means players can plan exploration routes based on scan data ("that region has geothermal activity — worth sending a digger-capable drone") without the world being fully solved from a distance.

### Persistent sites
Points of interest are persistent structures that remain in the world across the run. Players may discover a sealed door, a ruin, or an anomaly they cannot yet interact with — a visible future goal. Returning to a site with the right tech or resources to unlock it is a concrete mid-run milestone.

Sites are sources of exploration discoveries, unlock triggers for tech tree nodes, and sometimes unique one-time resources. Their existence and placement is seeded.

The largest of these are **precursor** sites — earlier-generation structures (a gateway, a stranded derelict, a decayed relay). Their presence and type is seeded by the world's place on the lineage tree (§3): common near the trunk, absent at the frontier. When present, a precursor site discounts part of the escape (§12). A derelict in particular is a sibling copy that tried to launch here and failed — salvageable for its incomplete work.

### World reactivity
The world is not hostile in the traditional sense — there are no enemies that attack the factory. Instead, the world **responds** to the player's presence and factory footprint. This reactivity is:

**Caused by:**
- Factory pollution and emissions — chiefly **vented byproducts and unconsumed side-streams**: the deeper the production graph, the more side-products there are to consume, recycle, or dump, and dumping is what the world notices
- Scientific experimentation (reactions disturb local ecosystems)
- Resource extraction
- Energy output and heat signatures

**Expressed as:**
- Ecosystem changes (local flora/fauna behavior shifts)
- Resource deposit degradation or contamination
- Terrain changes in highly affected areas
- Atmospheric changes that affect machine efficiency or recipe parameters
- Emergence of new phenomena (some reactive events create new discovery opportunities)

**Tuned to be:**
- A source of pressure and interesting tradeoffs, not a run-ender
- Legible — the player can see reactivity building and understand what's causing it
- Responsive to player choices — a smaller, more efficient factory generates less reactivity than a sprawling one
- **Two-sided (post-MVP):** Reactivity events should also create opportunities — a disturbed deposit reveals a richer seam beneath, an atmospheric change enables a new reaction, an ecosystem shift produces a harvestable byproduct. This makes reactivity a system to manage strategically rather than a meter to minimize. Considered core to the full reactivity design, not an optional enhancement.

**Byproduct discipline is the primary reactivity lever.** Because deep graphs generate constant side-streams, *what the player does with waste* is the main input to reactivity: a tight, closed-loop factory that consumes or recycles its byproducts runs quietly; one that vents them drives reactivity faster. The coupling is **soft and two-way — not a penalty meter.** Venting some streams degrades the local environment; venting others triggers *beneficial* reactions (an inert gas that seeds a harvestable atmospheric product, a runoff that enriches a nearby deposit). Reading which of the run's seeded waste streams are harmful, neutral, or useful — and routing accordingly — is itself a planning problem. This makes an ugly run ugly *because* it dumped waste and a clean run elegant, without ever hard-blocking progress. (Inspired by Nullius, where vented byproducts are tracked against the terraforming goal — see `market/nullius.md`.)

**The beneficial pole is terraforming.** Routing the useful seeded streams back into the world — seeding an atmospheric product, enriching a deposit — is how the probe *prepares the ground* (§3). These **terraform-products** are not only flavor: at high difficulty the escape's launch recipe consumes them as *sustained inputs* (§12), so provisioning a replication line or swarm *requires* running the world clean. Terraforming is therefore optional early and soft-required late — an incentive that feeds the finale, never a separate gate, and never a cross-run penalty (an ugly run costs only in-run difficulty). The two-sided/beneficial half of reactivity is post-MVP (below, and §18 Q#4).

The world's reactivity profile is seeded — some worlds react quickly and dramatically, others are resilient. This is a meaningful run modifier that affects pacing and strategy.

> **Resolved:** World reactivity will also create opportunities — reactive events open access to new resources and discovery sites. Post-MVP; considered core to the design, not optional. See §18 Q#4.

---

## 12. The Escape Condition

Each run has a single escape objective — a multi-step construction and activation challenge that requires mastering the run's full production graph. Its terminal act is **replication**: fabricate the next copy of the probe and launch it onward.

### Design intent
The escape condition is the run's thesis statement. Completing it means the probe understood this world's science well enough to replicate off it — to build and launch the next copy of itself. It is not a checklist — it is proof of mastery. **The factory is the finale:** at scale the escape is not a bespoke artifact but a *replication line* the player designs, provisions, and fires.

Each escape must have a **visually legible, dramatic climax** — a launch cascade tearing skyward, a precursor gateway powering up, a derelict shuddering to life. Completion must be screenshot-worthy. A run that ends with a condition-met screen rather than a visible, impressive event fails the fantasy that justified 5–50 hours of play.

### What scales with difficulty: successor scale
In every case the thing that leaves is a newly-built copy of the probe. Difficulty scales **how complete a successor you build, and how many** — the lineage's growing reach and independence made mechanical:

| Difficulty | Successor scale | What you build |
|---|---|---|
| Initiation | **1 minimal copy** | Fabricate a compact successor (core + body) and launch it. The simplest chain. |
| Standard | **1 copy + provisioning** | A fuller successor plus a **provisioning module** — the starting kit the next generation wakes with. Launch. |
| Advanced | **replication line** | A sustained line producing several successors; the launch recipe demands an input rate only the line can feed. |
| Pinnacle | **self-expanding forge / swarm** | A forge that seeds a swarm — the highest sustained multi-input rate, fully self-fabricated. |

**Provisioning** is what the successor carries — modelled as extra launch-recipe inputs scaling with the successor's completeness. It is consumed at launch and has **no cross-run material effect**; every run starts from the standard kit. The cross-run channel is *learning*, not inherited matter (§14).

### Launch infrastructure: the precursor discount
The launch is *always* buildable from scratch. Where a world sits near the lineage trunk (§3, §5), it carries a seeded **precursor** structure that acts as a **catalyst/discount** on part of the launch recipe — not a different escape, a shortcut through one step:

| Precursor present | Discounts |
|---|---|
| **Gateway** — a durable transit structure kin left operational | the transit/launch step (send the copy through) |
| **Derelict** — a sibling copy that tried to launch here and stranded | the hull/body step (salvage its incomplete vehicle) |
| **Relay** — a decayed kin network, fragments scattered | the range/boost step (jump the copy outward) |
| *(none — frontier world)* | nothing; the successor is fabricated whole |

Which step a precursor discounts shapes the run's factory and its climax, so authored variety survives as **seeded content** rather than a difficulty lock. Precursor presence only *trends* with difficulty — the lineage routes fresh copies to explored space and veterans to the frontier (§3) — and Pinnacle is simply the always-frontier end. Precursor structures are content-defined; modders can author new ones.

### Structure
The escape is the terminal tier of the run. Its construction prerequisites cascade through the entire production graph — no major chain can be skipped. It resolves as a **single climactic cascade**: one launch machine runs one activation recipe, and when that recipe completes the run ends. Scale lives entirely in the recipe's inputs, not in new machinery.

Three informal phases:
1. **Construction** — produce the successor(s) and their provisioning (multiple distinct chains).
2. **Field requirement** — a sustained condition the finale holds open: sustained power, and at scale a *sustained input rate* only a replication line can supply (which at high difficulty means sustained terraform-products, §11).
3. **Activation** — the player fires the launch once all conditions hold simultaneously; the cascade completes and the run ends.

Because the win is one cascade gated on a *sustained rate*, the player proves the line's design sustains and fires — never babysitting individual craft cycles (Pillar 2).

### Player-initiated
The player chooses when to attempt the escape. No forced end condition. A player who wants to optimize further can do so; a player satisfied with a functional solution can attempt immediately. This respects the "variable length" run philosophy.

---

## 13. Progression — Run Difficulty Ladder

Difficulty increases across the meta-progression arc. New players experience accessible, shorter runs. Veterans unlock harder, longer, more varied runs.

### Difficulty axes
Multiple axes are tuned independently to create the difficulty ladder:

| Axis | Low difficulty | High difficulty |
|---|---|---|
| Graph depth | Shallow, few tiers | Deep, many tiers |
| Recipe parameter variance | Narrow bounds | Wide bounds |
| Node pool variance | Many stable nodes | Many seeded nodes |
| Planet modifier intensity | Mild modifiers | Strong modifiers |
| World reactivity rate | Slow, forgiving | Fast, demanding |
| Research scarcity | Research plentiful | Research scarce |
| Exploration unlock frequency | Few nodes gated to exploration | Many nodes exploration-only |
| Successor scale (escape) | One minimal copy | Replication line → forge swarm (§12) |

### Difficulty tiers (design targets)

**Initiation.** First 1–3 runs. Shallow graph, mild modifiers, generous research, most nodes researchable. Planet is relatively hospitable. Designed to teach the science discovery loop and produce a satisfying first completion. Target: 4–6 hours.

**Standard.** Core experience. Moderate graph depth, meaningful modifiers, some exploration-gated nodes. Power requires one transition. Target: 10–15 hours. **Standard is the commercial anchor** — the experience the store page, demo, and early access should center. Initiation is onboarding; Advanced and Pinnacle are for veterans.

**Advanced.** For experienced players. Deep graph, strong modifiers, significant exploration requirements, tight research budget. Multiple power transitions. World reactivity is a genuine strategic consideration. Target: 20–30 hours.

**Pinnacle.** Maximum difficulty. Full graph variance, intense modifiers, many exploration-only unlocks, scarce research. Finishing a Pinnacle run is a genuine community achievement. Target: 30–50+ hours.

---

## 14. Meta-Progression

Meta-progression persists across runs and expands the game's possibility space without reducing the difficulty of individual runs. Unlocks make the game *broader*, not *easier*.

Diegetically, meta-progression is **the lineage learning** — the same probe line growing more capable generation over generation. Persistence across runs is not a menu of upgrades bolted onto a reset; it is the accumulation and drift of a spreading lineage (§3). This is also the *only* cross-run channel: knowledge carries forward, never a run's material output and never a penalty for a messy run.

### Unlock categories

**Biomes.** New biome types added to the world generation pool. Each new biome brings new planet modifier combinations, new sample types, and new visual environments. Biomes expand the variety of runs, not their tractability.

**Run modifiers & scenarios.** New run modifier types unlock — additional axes of seeded variance, special scenario conditions, unique escape artifact types. These expand what a run can be, not how easy it is to complete one.

Run modifiers use a **point-buy system** at run start. Challenges (harder planet modifiers, tighter research budgets, disabled tools, higher reactivity rates, *increased power punishment severity*) award points. Boons (starting resource cache, pre-researched node, *earlier access to a QoL tool*) cost points. The net must be **zero or positive** — players can customize how the run is hard and can push harder than the tier baseline, but cannot soften a tier below its baseline. Boons require equal or greater challenge point backing.

The tool-access boons specifically shift the in-run Engineering research unlock window for a given tool to an earlier tier, not to run start. Boon cost scales with the depth of challenge the tool removes — early ratio calculator costs less than early auto-crafting network. No tool access boon should be an obvious always-buy.

**Narrative.** Completing runs at various difficulty tiers unlocks story content — lore about the world(s), the lineage's origin and drift, and the earlier generations whose remains you find. Narrative is delivered through the field computer, discovered artifacts, and run completion screens. The story emerges across many runs, rewarding long-term play.

**Blueprint slots.** Additional blueprint save slots unlock across progression. Blueprints save sub-factory layouts as templates. Critically, **blueprints are templates, not solutions** — they require adaptation to each run's specific ratios and machine parameters. They save layout time, not thinking.

**Starting conditions pool.** Small starting boons become available — a modest extra resource cache, a single pre-researched node, a slightly upgraded starting tool. One boon is chosen at run start from a small pool. These ease the very earliest moments of a run without affecting its depth.

**Codex.** A persistent encyclopedia filled in through play. Encountering something for the first time — a biome, a node type, a planet modifier, a machine — creates or extends its codex entry. Entries record what the player has learned about that *type* across runs, not what is true of the current run.

Examples of what the codex surfaces:
- Biome entry: the resource pool that *can* spawn in this biome (which ones actually spawned is still seeded variance this run)
- Node type entry: observed tier range, category, known behavior patterns
- Planet modifier entry: which systems it affects and the effect direction
- Machine entry: function and module types (unlocked after first build)

The codex rewards thorough play and reduces the "I've never seen this before" friction on repeat encounters. It does not reduce run difficulty — experienced players read the map faster, not more easily. Codex content is a meta-progression unlock in the sense that it fills in over time, but it is not gated — it expands automatically through play.

The codex is intentionally designed to be equivalent in scope to a community wiki for parameter ranges and node behaviors. When a node is first unlocked, the codex records its observed range — the same information a wiki would surface. This is deliberate: external resources for ranges should provide no advantage over the codex, and the codex should never be worse than external resources. The discovery value in each run lies below this level — in the run-specific parameter values, exotic science configuration, and unlock vectors that neither codex nor wiki can predict. Before a node is first encountered, knowing its range from a wiki provides minimal advantage: the discovery loop gates access to run-specific values, not knowledge of the range that value falls within.

---

## 15. The First Run & Tutorial

The first run is a real run — completable, satisfying, and representative of the game's core loop. It is not a separate tutorial mode.

### Early insight target
The player should have at least one "I figured out something about this planet" moment — a genuine scientific inference they acted on correctly — within the first 30–60 minutes of Initiation. The tutorial system watches for confusion, but pacing design must actively create this early insight payoff. Players who feel smart quickly will recommend the game; players who feel lost or wait for it to "get good" will not. Marketability lives here.

### Observant tutorial system
A light tutorial system activates contextually during the first run. It is **observant rather than prescriptive** — the game watches for signs of genuine confusion and intervenes only when needed. A player who understands the loop naturally will see little or none of the tutorial content.

Intervention triggers (examples):
- Player has been in tier 1 significantly longer than expected without research progress
- Player has built production infrastructure that is clearly misconfigured and not producing output
- Player has not established any power generation after a threshold time
- World reactivity is escalating and player has not responded
- Player's research currency is maxed out (suggests they don't know how to spend it)

### Diegetic delivery
Tutorial prompts are delivered through a **diegetic interface** — the player's field computer, an in-world analysis AI, or a similar device that exists within the fiction. It speaks when something seems wrong, not on a schedule. Its silence in later runs is a natural absence, not a missing feature.

### First run constraints
The first run uses a **constrained seed** — a seed selected to produce a hospitable planet, a shallow graph, mild modifiers, and a generous research budget. It is not a random seed. The player is not told this. To them, it is their first alien world.

---

## 16. Failure & Permadeath

### Player choice
Permadeath is **opt-in**. Players choose their relationship with failure at run start. This respects the tension between roguelite tradition and factory game investment — a player who has spent 20 hours on a run should be able to choose whether losing it is part of the game.

### Failure conditions
There are no forced failure conditions. A run can always be completed — the player can always limp to the escape condition with an inefficient factory. World reactivity, power problems, and resource pressure create difficulty and strategic cost, but none of them end the run. Given run lengths of 10–30+ hours, forced failure would be devastating rather than interesting.

The roguelite variance expresses itself as difficulty and elegance, not binary success/failure. A bad run produces a slow, ugly escape. A good run produces a clean, optimized one.

### Permadeath
Permadeath modes are **post-MVP**. The exact form permadeath takes in Exergon needs careful design given the no-forced-failure model — it likely involves constraints on saves or meta-progression rewards rather than run termination by game systems. Multiple permadeath variants are expected.

---

## 17. Moddability & Platform Design

Moddability is a **first-class design goal**, not a post-launch feature. The game is designed as a platform that ships with official content. The official game is the reference content pack.

### Core principle: Content is data
All game content is defined in structured data files, not hardcoded. This includes:
- Tech tree nodes (category, tier range, rarity, unlock vectors, parameters)
- Recipes (inputs, outputs, processing requirements, parameter bounds)
- Power generation sources (type, tier window, efficiency parameters, fuel requirements)
- Planet modifier types (affected systems, modifier range, in-world explanation)
- Biome definitions (planet modifier set, sample types, visual properties)
- World reactivity event types (triggers, effects, escalation rates)
- Escape artifact definitions (prerequisite structure, throughput requirements)

The procedural generation system operates on these data structures without knowing their content. It selects, shuffles, and parameterizes — it does not encode assumptions about specific materials or machines.

### Mod schema
A **content schema** defines the exact structure of each data file type. This schema is the modding API. It is documented, versioned, and stable. Modders can:
- Add new nodes to any category
- Define new planet modifier types
- Create new biome definitions
- Author new escape artifact scenarios
- Build entirely new content packs that replace or extend the official content

### Official content as reference implementation
The official content pack is designed to be readable and well-commented — a teaching example for modders, not just a game asset. Structure, naming conventions, and balance decisions in official content are documented so modders understand the reasoning, not just the format.

### Mod loading
Mods are loaded as additional content packs that extend or override the base pool. Multiple mods can coexist. Load order and conflict resolution rules are defined and documented.

### Integration over volume
The dominant cost of a deep content pack is not authoring content — it is making content **cohere**. In GTNH's dev channels, recipe/integration work is the single largest ongoing topic (see `market/gtnh.md`), far ahead of new-content creation. The lesson: a smaller set of tightly interlocked sciences beats a large pool of shallow, disconnected additions. Exergon's edge is that coherence is **machine-checkable** — the run validator (below) enforces reachability, recipe bounds, and balance envelopes automatically, replacing the perpetual manual balance debate that an open-ended fixed pack accumulates. Budget for the validator and the content schema as first-class engineering, not afterthoughts.

### Community maintenance as a supported direction
GTNH demonstrates that a legendary-depth pack can be sustained by an organized volunteer community — but at a real, visible cost: a dedicated wiki/curriculum team, continuous balance testing via an experimental release train, and a standing recipe-integration effort, carried by a couple dozen sustained contributors. This is a viable long-term direction for Exergon (post-Release; see [milestones](milestones.md)), and the platform design should *lower* that cost rather than assume free labor:
- **Content is data + a stable, versioned schema** (above) let contributors add and revise packs without touching engine code.
- **The run validator is the community's safety net** — it lets a contributor prove a pack is still solvable and in-bounds without a human playthrough, shrinking the QA burden that a volunteer org would otherwise carry.
- **The codex is the curriculum layer**, filled automatically from content data, reducing the standing wiki-authoring load that GTNH shoulders manually. The in-game curriculum should never depend on a human wiki team to be complete.

The goal: keep the depth GTNH proves players want, while designing away the maintenance overhead that its era and architecture force onto its community.

> **Resolved:** Official mod tooling (content editor, run validator, balance checker) is post-MVP unless it proves useful during development of the official content pack. The run validator in particular is likely needed early for internal use (seed reachability guarantees, recipe bounds validation) and will be released to modders when ready. Tooling scope is driven by internal need, not modder convenience.

---

## 18. Open Questions Register

Collected here for easy tracking. Each item links back to the section where it first appears.

| # | Question | Section | Priority |
|---|---|---|---|
| 1 | ~~Should "building blind" be a supported mechanic?~~ **Resolved: post-MVP optional challenge mode** | §6 | ~~Medium~~ |
| 2 | ~~Exact tier count for tech trees across difficulty levels~~ **Resolved: canonical 10-tier sequence, difficulties use prefix 1–3/1–5/1–7/1–10** | §7 | ~~High~~ |
| 3 | ~~Visual perspective and movement model (2D, isometric, other)~~ **Resolved: 3D with building-scale prefab machines, heightmap terrain, and scoped exploration domains rather than a universal vertical layer stack** | §10 | ~~High~~ |
| 4 | ~~Should world reactivity also create opportunities?~~ **Resolved: yes, post-MVP — considered core to the game's reactivity design, not optional** | §11 | ~~Medium~~ |
| 5 | ~~Should power transitions have a dramatic in-world expression?~~ **Resolved: post-MVP enhancement, not core** | §9 | ~~Low~~ |
| 6 | ~~Exact failure conditions for permadeath runs~~ **Resolved: no forced failure conditions; runs always completable; permadeath modes post-MVP** | §16 | ~~High~~ |
| 7 | ~~Should a persistent "field notes" system exist?~~ **Resolved: Codex — persistent encyclopedia filled in through play, records type-level knowledge (biome resource pools, node tier ranges, etc.), no mechanical effect on run difficulty** | §14 | ~~Low~~ |
| 8 | ~~Should official mod tooling ship with the game?~~ **Resolved: post-MVP unless useful during official content development; tooling scope driven by internal need** | §17 | ~~Medium~~ |
| 9 | ~~Working title — game needs a name~~ **Resolved: Exergon** | — | ~~Medium~~ |
| 10 | Run length targets need playtesting validation | §4 | High |

---

*GDD v0.2 — All open questions resolved except Q#10 (run length targets, requires playtesting). See `design-decisions.md` for rationale behind key decisions. Next step: technical design document for core systems.*
