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

This game is a **factory-building roguelite** in which every run takes place on a procedurally generated world with alien physical laws. The player is a scientist-engineer whose job is to decode how this particular world works, design a factory that exploits those laws, and synthesize a single large escape artifact that proves they understood the system well enough to master it.

The primary inspiration is **GregTech: New Horizons (GTNH)** — specifically its depth of production graph complexity, its multi-tier processing chains, its demand that players understand systems rather than execute recipes from memory, and its culture of genuine accomplishment through genuine difficulty. The roguelite layer is not a cosmetic addition. It is the mechanism by which a GTNH-depth game becomes replayable: the graph itself is different every run.

The game is also meaningfully different from GTNH in one key respect: **the design phase is the game.** Watching machines run, fixing belt bottlenecks, and grinding execution time are minimized. The intellectual work of reading a run, planning a factory, and discovering alien science is maximized. A run that takes 20 hours of engaged thinking is more satisfying than a run that takes 200 hours of engaged thinking plus 800 hours of waiting and grinding.

### Closest existing references
- **GregTech: New Horizons** — depth, complexity, tier-gated progression, power systems
- **Factorio** — factory layout, belt logistics, the satisfaction of throughput
- **Slay the Spire** — roguelite meta-progression, run variance as the replayability engine
- **Zachtronics games (Opus Magnum, Infinifactory)** — puzzle-oriented, the solution is the reward
- **Outer Wilds** — physical presence in a world whose rules you are discovering

---

## 2. Design Pillars

These four pillars are the filter through which every design decision should pass. If a proposed mechanic conflicts with a pillar, the mechanic changes — not the pillar.

### Pillar 1 — Legible Chaos
Randomization must produce *solvable* problems, not arbitrary noise. Every procedural element must have a legible in-world explanation the player can reason about. A planet further from its star has weaker solar output. A world with unusual atmospheric chemistry has different combustion properties. The player should be able to look at a run's constraints and think *"this is a specific, interesting problem"* — not *"this is random."*

### Pillar 2 — The Design Phase Is the Game
The most interesting moment in a run is before the first building is placed. Planning, scouting, information negotiation, and graph analysis are the primary gameplay. Execution — placing machines, routing belts — is a satisfying consequence of good planning, not the challenge itself. Watch-and-fix time is minimized wherever possible.

### Pillar 3 — Difficulty Through Depth, Not Friction
Difficulty comes from the genuine complexity of the puzzle, not from systems friction, obscured UI, low drop rates, or artificial time sinks. Hard means the graph is deep and the decisions are consequential. It does not mean the interface is opaque, the execution is tedious, or progress requires grinding.

### Pillar 4 — Content Is Data, Engine Is Platform
The game is designed from the start as a moddable platform. All content — tech nodes, recipes, planet modifiers, power sources, biome definitions — is defined in data files, not code. The official game ships as the reference content pack. Modders extend the platform by writing data, not by modifying the engine.

---

## 3. Core Fantasy

> *"I landed on an alien world, figured out how its physics worked, and built something that shouldn't exist — and then I left."*

**The player is an AI — a small, self-directed intelligence running on a portable substrate.** Stranded on an alien world, the goal is to escape — ultimately leaving the solar system entirely — and find a way back to civilization. The AI is embodied in a compact flying unit: flight is the natural movement mode, not a late-game unlock. The route out depends on how far the player has come: early runs find and activate alien gateways or devices left by a prior civilisation; later runs build increasingly capable spacecraft (first intra-system, then inter-system) by mastering the world's science deeply enough to manufacture the technology from scratch.

The player is not a factory operator. They are a **scientist-explorer** who happens to build factories as the output of their scientific work. The factory is the proof of understanding, not the activity itself.

A successful run feels like solving a deep puzzle — the satisfaction of having read a complex system, found the critical path through it, and executed a plan that required genuine expertise. The escape isn't a grind reward. It's a thesis.

**The thematic arc across runs:** Each run is one leg of a journey through the galaxy — you escape system N and arrive stranded in system N+1. Early runs rely on discovered alien technology (the gateways imply someone traveled this route before you — who, and why?). Later runs transcend that dependency, building your own way out with increasing mastery. The destination is home, or something beyond it; the narrative unfolds across many runs.

Alien ruins and persistent sites are remnants of a prior civilisation that traveled the same route. Their technology appears across multiple systems — seeded differently each time, but recognisably theirs. The codex accumulates knowledge across every world you've passed through: part scientific journal, part map of a civilisation's vanishing trail.

**Two science tracks — complementary, not exclusive:**
- *Universal science* — real-world-inspired physics and engineering. Applies on any world. Base materials, fundamental processes. The foundation every run shares regardless of seed.
- *Alien science* — the prior civilisation's technology. Seeded per run; unique each time. Exotic materials and processes unlocked primarily through exploration, observation, and site interaction.

The two tracks feed the same recipe graph and tech tree. Most nodes are accessible through either track (or both). Some nodes offer genuine alternative paths: a human-engineering approach (production milestone or research spend) vs. an alien-science approach (exploration discovery). Explorer-first and factory-first playstyles find different routes to the same capabilities.

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

**Planet properties.** A set of physical characteristics that apply passive modifiers to the run. These are the most legible layer of variance — in-world explanations for why certain strategies are favored or penalized. Examples:
- Distance from star → solar efficiency modifier
- Atmospheric composition → combustion efficiency modifier
- Geological activity → geothermal availability and efficiency
- Temperature → affects machine cooling requirements, certain reaction efficiencies
- Atmospheric pressure → affects fluid dynamics, certain chemical processes

Planet properties are partially visible at run start (broad characteristics) and fully revealed through early scouting. An experienced player reads planet properties at landing and immediately forms a rough power strategy.

**Tech tree node selection.** The nodes that exist in this run's tech tree are drawn from a larger pool. Not every node exists in every run. This means the set of available machines, processing methods, and solutions is genuinely different run to run — not just reshuffled but different in kind. See Section 7.

**Tech tree unlock conditions.** Each node's unlock vector(s) are also seeded within defined constraints. A node that is researchable in one run might be exploration-discovered in another. See Section 7.

**Recipe parameters.** Within the constraints of the run's available nodes, specific recipe parameters — efficiency, byproduct rates, processing times — vary within bounded ranges. See Section 8.

**Resource geography.** Ore patch locations, fluid deposits, unique resource sites, and terrain features are procedurally placed. The map's geometry shapes factory layout decisions. See Section 11.

**World reactivity profile.** The rate and nature of the world's response to factory footprint and experimentation. Some worlds are resilient; others react quickly and severely. See Section 11.

### Seed legibility
Runs can be shared by seed string. Community discussion of specific runs ("seed 4729 has terrible solar but incredible geothermal and a near-surface rare ore deposit at coordinates X") is an intended and supported part of the game's culture, consistent with the GTNH tradition of community knowledge-building.

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

- Exploration is conducted via player-piloted drones. The AI body stays within a habitat bubble; the player's *attention* travels via drone. Piloting a drone to collect samples is an active time cost, not passive automation.
- Collecting samples requires the player to pilot a drone to the location and interact with it
- Some sample sites are dangerous, distant, or require a specific drone tier to reach — creating a progression gate on information as well as production
- Analysis stations are built in the world and consume samples + research currency to produce knowledge
- The act of experimentation has a mild world-reactivity cost — running reactions disturbs local ecosystems

### Information visibility model
At any point in a run, the player has three tiers of knowledge about any given recipe or tech node:

1. **Known to exist** — the node appears in the tech tree or recipe list, but parameters are hidden
2. **Partially revealed** — broad parameters visible (approximate inputs, rough output range), specifics still hidden
3. **Fully revealed** — complete recipe, all parameters, buildable

**Partially revealed** is earned through gameplay, not purchased — hitting a related production milestone, making an exploration discovery, or completing a relevant experiment. It is a reward for engagement.

**Fully revealed** is purchased with research currency of the appropriate type. Players can skip partial reveal entirely and go directly from known-to-exist → fully revealed at higher cost. The decision of *which* nodes to fully reveal before others is a core strategic choice, especially on runs where research is tight.

> **Post-MVP:** A "building blind" mechanic — committing to a partially-revealed recipe at some risk cost — is a candidate optional challenge mode, not a core MVP mechanic. Core loop assumes players reveal before committing.

---

## 7. The Tech Tree

The tech tree is the skeleton of the run. It is the one structure that is always partially visible — players can always see its shape, even when its contents are hidden.

### Tier structure
The tech tree is organized into **tiers** that follow a canonical 10-tier sequence. Each difficulty uses a prefix of this sequence, producing meaningfully different run lengths:

| # | Tier name | Terminal for | Gate condition |
|---|---|---|---|
| 1 | Landfall | — | Analyze first alien sample + deploy surface drone |
| 2 | Roots | — | Produce 100 units of any refined base material |
| 3 | Contact | **Initiation** | Activate alien structure (terminal: gateway; intermediary: ruin/cache unlocking alien material or machine) |
| 4 | Reach | — | Achieve first orbital flight |
| 5 | Salvage | **Standard** | Interact with alien vessel (terminal: repair + launch; intermediary: extract fabrication data) |
| 6 | Traverse | — | Reach outer-system zone |
| 7 | Interface | **Advanced** | Interact with alien megastructure (terminal: operate relay; intermediary: extract FTL theory fragments) |
| 8 | Revelation | — | Synthesize first exotic material |
| 9 | Forge | — | Produce all FTL drive component types + sustain FTL-grade power |
| 10 | Transcendence | **Pinnacle** | — (escape condition is the terminal) |

| Difficulty | Tiers | Unlocked by |
|---|---|---|
| Initiation | 1–3 | Available from start |
| Standard | 1–5 | Complete an Initiation run |
| Advanced | 1–7 | Complete a Standard run |
| Pinnacle | 1–10 | Complete an Advanced run |

**Tiers 3, 5, and 7 have two variants** — terminal and intermediary. When a tier is terminal for the current difficulty, the alien structure at that tier is the escape objective (gateway, derelict ship, relay node). When the same tier appears as an intermediary in a harder difficulty, a different artifact class is present: an automated probe, a knowledge archive. These are intact but not usable for escape — their value is what they teach or produce. This preserves immersion across runs: each run is a different world with a different precursor remnant.

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

**Special recipe nodes** unlock a specific recipe that doesn't arise from template expansion — a unique item recipe, a cross-material process, an alien reaction, or an unusually efficient shortcut. These always require explicit tech tree unlock.

Standard recipes (template instantiations) need no tech node — they are available automatically once both the material and the machine are known.

### Unlock vectors
Each node supports one or more unlock methods. The seed determines which are active in a given run. Supported vectors:

**Research unlock.** Spend research currency directly to unlock the node. The most reliable but most expensive vector.

**Prerequisite chain.** The node becomes available automatically once one or more other nodes are unlocked. Creates natural dependency structures.

**Production milestone.** Unlock by producing a certain quantity of a certain item. Rewards factory progress with knowledge.

**Exploration discovery.** The node only appears once the player finds a specific in-world location, artifact, or phenomenon. Cannot be researched — must be found. Creates runs where certain knowledge is gated behind exploration rather than investment.

**Observation unlock.** The node unlocks by observing a specific in-world event or process. Related to exploration discovery but more passive — the player must be in the right place at the right time, or create the right conditions.

A node can have multiple active vectors in the same run — unlocking via any one of them suffices. This creates alternative paths: a player who hasn't found the exploration trigger can still research their way to a node, but at higher cost.

### Cross-run expertise
Because nodes are drawn from a known pool, experienced players develop expertise about the pool itself — not about specific run configurations. They know a given node exists, roughly what it does, and roughly what tier it appears in. What they don't know is whether it exists *in this run*, what its specific parameters are, and how it's unlocked *this time*. This preserves the puzzle while rewarding meta-knowledge.

---

## 8. Production & The Recipe Graph

The recipe graph is the intellectual heart of the run. It defines what this world's alien science looks like — which materials exist, what items they can be formed into, and what the critical path to the escape artifact requires.

### Fictional science grounding
The recipe graph is grounded in a **consistent fictional science** — not real-world chemistry, but an internally logical system with its own rules and properties. Players cannot import real-world knowledge directly, but they can develop genuine expertise in the fictional system's structure across runs.

The system has partial real-world inspiration — materials behave in ways that feel physically motivated even if they don't match actual chemistry. This gives the world texture and makes the planet's physical properties feel connected to its production chains, without requiring chemistry knowledge to play.

### Materials, forms, and items

Three concepts form the production vocabulary:

**Materials** are abstract substance identities — *copper*, *tin*, *resonite* (alien). A material is not itself a recipe node; it is the identity that items inherit. Each material has a **kind**: base (real-world inspired, consistent across runs) or alien (seeded per run, unique to this run's science). The ratio shifts across tiers: early tiers are mostly base materials; the final tier and escape artifact are primarily alien.

**Form groups** are content-defined categories of physical states a material can take, declared per material. Example groups: `metal` (ore, crushed_ore, dust, ingot, plate, wire, rotor…), `combustible` (ore, chunk, dust), `exotic` (shard, crystal, lens…). A material may belong to multiple groups and gets the union of their forms.

**Items** are the actual recipe nodes — three kinds:
- **Derived items** — a (material, form) pair, generated automatically from a material's group membership. *copper_wire* = copper + wire form. No asset file required; exist whenever the material is present in the run.
- **Composite items** — defined in assets. May follow a template pattern (e.g. `[X]_cable = [X]_wire + rubber`, instantiated for every material with a wire form) or be fully unique (e.g. *resonite_circuit*).
- **Unique items** — one-off asset-defined items with no material-form derivation.

### Graph structure
The recipe graph is a directed acyclic graph (DAG) of **items** with the escape artifact as its terminal node. Every recipe transforms one set of items into another; every path through the graph leads toward the terminal.

**Recipe templates** define item transformations at the form level: `[M]_ingot → [M]_wire (wiremill)`. Templates apply automatically to every material whose groups include both involved forms. Concrete recipes are generated at run start by expanding templates over all present materials — adding a new material with the `metal` group automatically gives it all metal processing recipes.

Graph properties that vary by seed:
- **Which alien materials exist** — determines which derived items and concrete recipes appear this run
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

**Layer 1 — Planet physical modifiers.** The world's physical properties apply passive multipliers to specific generation types. These are legible and in-world:
- Distance from star → solar efficiency (e.g., 0.4× to 1.6× base)
- Atmospheric oxygen content → combustion efficiency
- Geological activity → geothermal availability and output
- Temperature → affects thermodynamic cycle efficiency
- Wind patterns → affects wind generation (if present as a node)

These modifiers are revealed through early scouting and are fixed for the run. They give experienced players an immediate read on which power strategies are favored.

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

The factory is the physical consequence of good planning. The world is fully three-dimensional — machines are building-scale prefab structures placed in the environment. Spatial optimization is not the primary challenge; the complexity comes from the logistics network, recipe graph, and machine configuration.

### Design intent
Factory layout should feel satisfying and consequential without being the game's central puzzle. A player who makes good planning decisions should be able to build a functional factory without needing spatial optimization expertise. A player who is also a strong spatial thinker gets additional satisfaction from elegant, impressive-looking layouts — and base aesthetics are a first-class feature, not an afterthought. Sharing well-built bases is an intended community behavior.

### Visual model
The game is fully 3D. Machines are building-scale prefab structures; players place them freely in three dimensions and verticality is a meaningful routing and layout tool. Late-game bases visually dwarf early-game ones as larger, more complex machines accumulate — progression is legible from screenshots.

### Logistics network
Item and fluid transport uses a logistics network model (ME-style) rather than physical belt routing. The network is physical infrastructure — cables and conduits are visible blocks that must be placed and routed — but the challenge is network design rather than belt-path puzzles. Network cables as visible infrastructure contribute to base aesthetics.

**Capacity — discrete channel limits.** Cables have a discrete channel capacity (like AE2). Exceeding capacity requires higher-tier cables, sub-network segmentation, or architectural redesign. This is the primary driver of network complexity and a revisitable design parameter. Channel limits are intentionally the pressure that encourages players to segment their network into logical sub-networks (e.g. a smelting network, a processing network) connected via defined interfaces — segmentation is a solution to be discovered, not a forced constraint.

**Auto-crafting.** The network handles on-demand crafting automatically on request. The design challenge is configuring the crafting graph correctly, not clicking through individual recipes. This directly serves Pillar 2 — the work is in the planning, not the execution.

The network **resolves recipe chains automatically**: given machines capable of `{A+B → C}` and `{C+D → E}`, a request for E spawns a two-job plan and the network presents the effective combined recipe `{A+B+D → E}` to the player. No manual chaining or intermediate requests required. Players see the full effective recipe for any planned output in the graph analyzer.

**Unified storage.** The network presents a unified item inventory across all connected storage nodes. Storage exists as a necessary system but is not intended to be a primary design constraint or puzzle — inventory management is friction, not depth.

### Machines and multiblock structures
All significant machines are multi-block structures: a fixed core footprint plus flexible modular attachments.

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

The AI exists in one body at a time. Within a connected habitat zone, the player flies and builds directly. Remote exploration beyond the habitat is conducted via drones (see §11). Switching to a new outpost means switching which body the AI currently inhabits — an explicit action, not free travel.

### Base management — Habitat bubbles and outpost islands

The alien atmosphere is lethal without life support infrastructure. The player begins with a core **Habitat Generator** (delivered with the escape pod) that projects a survivable bubble around the landing site. Within this bubble, the AI can fly, build, and interact directly. Outside it: drones only.

**Expanding into new biomes** is done through **Outpost Beacons** — support structures built at the main base, then shipped to a target location via drone. Once an Outpost Beacon is placed and connected to the logistics network, it projects its own life-support bubble. The player can then fabricate a new **body chassis** at the main base, ship it to the outpost via drone, and upload into it — becoming physically present there to build and interact directly.

Key properties:
- The player inhabits exactly one body at a time. Switching bodies is deliberate: select a connected outpost, transfer via the network. Instantaneous once connected, but requires an active network link (power + logistics).
- Each body is a manufactured item. Losing a body to environmental hazard is a real setback. Body fabrication costs scale with chassis tier.
- Outpost Beacons require power from the network. A power interruption collapses the life-support bubble — the AI must evacuate or risk body loss.
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
- **Biome regions** — areas with distinct environmental properties that affect machine operation and scouting conditions, distributed across all vertical layers

### Vertical layers
The world has distinct vertical layers, each with different environmental properties, biome types, and resource affinities:

- **Underground** — caves, deep deposits, geothermal phenomena; accessible via digger drones
- **Surface** — starting layer; most accessible biomes; land and water terrain
- **Sky/atmosphere** — upper atmospheric biomes; accessible via flying drones
- **Orbital/space** — extreme-tier layer; accessible via space-capable drones

Resources have affinity or hard restriction to specific layers and/or biomes. A resource that only forms in underground geothermal biomes requires both a digger drone and a world with geothermal activity to access. This makes layer+biome combinations meaningful variance axes, and gives experienced players immediate strategic reads from early scan data.

### Biomes
The world contains multiple biome regions distributed across layers. Biomes affect:
- Local machine efficiency (heat, cold, pressure)
- Sample types available for analysis
- World reactivity rate
- Which resources can generate there
- Visual character

Biome types are expanded through meta-progression — new biomes unlock across runs, adding variety to the world generation pool.

### Exploration model — drones
Exploration is done via deployable drones, not direct player travel. Drone types are tier-gated through the tech tree, making factory progress the key to unlocking new layers and biomes:

| Drone tier | Access |
|---|---|
| Land drone | Surface terrain |
| Amphibious drone | Surface water bodies and underwater biomes |
| Digger drone | Underground layer |
| Flying drone | Sky/atmosphere layer |
| Space drone | Orbital layer |

Drones are constructed from factory-produced components — the same progression that advances your production graph also advances your exploration reach. This creates a natural pacing gate without arbitrary locks.

### Map reveal and scanning
Fog of war is lifted by drone presence, but the reveal is intentionally imprecise at range. Scanners provide a general read on nearby areas: biome type and broad resource category presence (e.g. "mineral deposits," "fluid pockets") without exact quantities or positions. Precise data requires physical drone proximity or deployed sensor structures.

This means players can plan exploration routes based on scan data ("that region has geothermal activity — worth sending a digger drone") without the world being fully solved from a distance.

### Persistent sites
Points of interest are persistent structures that remain in the world across the run. Players may discover a sealed door, a ruin, or an anomaly they cannot yet interact with — a visible future goal. Returning to a site with the right tech or resources to unlock it is a concrete mid-run milestone.

Sites are sources of exploration discoveries, unlock triggers for tech tree nodes, and sometimes unique one-time resources. Their existence and placement is seeded.

### World reactivity
The world is not hostile in the traditional sense — there are no enemies that attack the factory. Instead, the world **responds** to the player's presence and factory footprint. This reactivity is:

**Caused by:**
- Factory pollution and emissions
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

The world's reactivity profile is seeded — some worlds react quickly and dramatically, others are resilient. This is a meaningful run modifier that affects pacing and strategy.

> ⚠️ **OPEN QUESTION:** Should world reactivity create new opportunities as well as problems — e.g., reactive events that open access to new resources or discovery sites? This would make reactivity feel more dynamic and less purely punishing.

---

## 12. The Escape Condition

Each run has a single escape objective — a multi-step construction and activation challenge that requires mastering the run's full production graph.

### Design intent
The escape condition is the run's thesis statement. Completing it means the player understood this alien world's science well enough to leave it. It is not a checklist — it is proof of mastery.

### Escape type by difficulty

The nature of the escape scales with difficulty tier, reflecting the player's growing self-sufficiency across the meta-progression arc:

| Difficulty | Escape type | Description |
|---|---|---|
| Initiation | Alien gateway activation | Discover an alien gateway left by a prior civilisation. Construct the activation key (a complex alien-spec artifact). Sustain sufficient power. Insert key and hold power to activate. |
| Standard | Alien derelict ship | Locate a derelict alien ship within the solar system. Construct several ship system components (hull section, navigation, engines, life support). Produce alien-spec fuel. Install all components and launch. |
| Advanced | Outer-system relay | Locate scattered relay fragments across the solar system (count fixed, locations seeded per run). Construct relay repair components. Collect all fragments, install repairs, sustain power to activate. |
| Pinnacle | Interstellar spacecraft | Construct four major ship systems from scratch — engines, FTL drive, reactor, shielding — each requiring deep production chains. Assemble and launch. |

Escape types are content-defined — modders can create new escape scenarios.

### Structure
The escape condition is the terminal tier of the run. Its construction prerequisites cascade through the entire production graph — no major chain can be skipped. The specific escape type is determined by the run's difficulty tier (see meta-progression).

Each escape has three phases:
1. **Construction** — produce all required components (multiple distinct items, each with its own production chain)
2. **Field requirement** — a non-production prerequisite: sustained power, fuel stockpile, or fragment collection via exploration
3. **Activation** — final trigger that ends the run once all conditions are met simultaneously

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

### Difficulty tiers (design targets)

**Initiation.** First 1–3 runs. Shallow graph, mild modifiers, generous research, most nodes researchable. Planet is relatively hospitable. Designed to teach the science discovery loop and produce a satisfying first completion. Target: 4–6 hours.

**Standard.** Core experience. Moderate graph depth, meaningful modifiers, some exploration-gated nodes. Power requires one transition. Target: 10–15 hours.

**Advanced.** For experienced players. Deep graph, strong modifiers, significant exploration requirements, tight research budget. Multiple power transitions. World reactivity is a genuine strategic consideration. Target: 20–30 hours.

**Pinnacle.** Maximum difficulty. Full graph variance, intense modifiers, many exploration-only unlocks, scarce research. Finishing a Pinnacle run is a genuine community achievement. Target: 30–50+ hours.

---

## 14. Meta-Progression

Meta-progression persists across runs and expands the game's possibility space without reducing the difficulty of individual runs. Unlocks make the game *broader*, not *easier*.

### Unlock categories

**Biomes.** New biome types added to the world generation pool. Each new biome brings new planet modifier combinations, new sample types, and new visual environments. Biomes expand the variety of runs, not their tractability.

**Run modifiers & scenarios.** New run modifier types unlock — additional axes of seeded variance, special scenario conditions, unique escape artifact types. These expand what a run can be, not how easy it is to complete one.

Run modifiers use a **point-buy system** at run start. Challenges (harder planet modifiers, tighter research budgets, disabled tools, higher reactivity rates, *increased power punishment severity*) award points. Boons (starting resource cache, pre-researched node, *earlier access to a QoL tool*) cost points. The net must be **zero or positive** — players can customize how the run is hard and can push harder than the tier baseline, but cannot soften a tier below its baseline. Boons require equal or greater challenge point backing.

The tool-access boons specifically shift the in-run Engineering research unlock window for a given tool to an earlier tier, not to run start. Boon cost scales with the depth of challenge the tool removes — early ratio calculator costs less than early auto-crafting network. No tool access boon should be an obvious always-buy.

**Narrative.** Completing runs at various difficulty tiers unlocks story content — lore about the world(s), the character's history, the reason for the escape conditions. Narrative is delivered through the field computer, discovered artifacts, and run completion screens. The story emerges across many runs, rewarding long-term play.

**Blueprint slots.** Additional blueprint save slots unlock across progression. Blueprints save sub-factory layouts as templates. Critically, **blueprints are templates, not solutions** — they require adaptation to each run's specific ratios and machine parameters. They save layout time, not thinking.

**Starting conditions pool.** Small starting boons become available — a modest extra resource cache, a single pre-researched node, a slightly upgraded starting tool. One boon is chosen at run start from a small pool. These ease the very earliest moments of a run without affecting its depth.

**Codex.** A persistent encyclopedia filled in through play. Encountering something for the first time — a biome, a node type, a planet modifier, a machine — creates or extends its codex entry. Entries record what the player has learned about that *type* across runs, not what is true of the current run.

Examples of what the codex surfaces:
- Biome entry: the resource pool that *can* spawn in this biome (which ones actually spawned is still seeded variance this run)
- Node type entry: observed tier range, category, known behavior patterns
- Planet modifier entry: which systems it affects and the effect direction
- Machine entry: function and module types (unlocked after first build)

The codex rewards thorough play and reduces the "I've never seen this before" friction on repeat encounters. It does not reduce run difficulty — experienced players read the map faster, not more easily. Codex content is a meta-progression unlock in the sense that it fills in over time, but it is not gated — it expands automatically through play.

---

## 15. The First Run & Tutorial

The first run is a real run — completable, satisfying, and representative of the game's core loop. It is not a separate tutorial mode.

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

> **Resolved:** Official mod tooling (content editor, run validator, balance checker) is post-MVP unless it proves useful during development of the official content pack. The run validator in particular is likely needed early for internal use (seed reachability guarantees, recipe bounds validation) and will be released to modders when ready. Tooling scope is driven by internal need, not modder convenience.

---

## 18. Open Questions Register

Collected here for easy tracking. Each item links back to the section where it first appears.

| # | Question | Section | Priority |
|---|---|---|---|
| 1 | ~~Should "building blind" be a supported mechanic?~~ **Resolved: post-MVP optional challenge mode** | §6 | ~~Medium~~ |
| 2 | ~~Exact tier count for tech trees across difficulty levels~~ **Resolved: canonical 10-tier sequence, difficulties use prefix 1–3/1–5/1–7/1–10** | §7 | ~~High~~ |
| 3 | ~~Visual perspective and movement model (2D, isometric, other)~~ **Resolved: 3D with building-scale prefab machines, heightmap terrain, graph-based underground tunnels** | §10 | ~~High~~ |
| 4 | ~~Should world reactivity also create opportunities?~~ **Resolved: yes, post-MVP — considered core to the game's reactivity design, not optional** | §11 | ~~Medium~~ |
| 5 | ~~Should power transitions have a dramatic in-world expression?~~ **Resolved: post-MVP enhancement, not core** | §9 | ~~Low~~ |
| 6 | ~~Exact failure conditions for permadeath runs~~ **Resolved: no forced failure conditions; runs always completable; permadeath modes post-MVP** | §16 | ~~High~~ |
| 7 | ~~Should a persistent "field notes" system exist?~~ **Resolved: Codex — persistent encyclopedia filled in through play, records type-level knowledge (biome resource pools, node tier ranges, etc.), no mechanical effect on run difficulty** | §14 | ~~Low~~ |
| 8 | ~~Should official mod tooling ship with the game?~~ **Resolved: post-MVP unless useful during official content development; tooling scope driven by internal need** | §17 | ~~Medium~~ |
| 9 | ~~Working title — game needs a name~~ **Resolved: Exergon** | — | ~~Medium~~ |
| 10 | Run length targets need playtesting validation | §4 | High |

---

*GDD v0.2 — All open questions resolved except Q#10 (run length targets, requires playtesting). See `design-decisions.md` for rationale behind key decisions. Next step: technical design document for core systems.*
