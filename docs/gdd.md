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

This game is a **factory-building roguelike** in which every run takes place on a procedurally generated world with alien physical laws. The player is a scientist-engineer whose job is to decode how this particular world works, design a factory that exploits those laws, and synthesize a single large escape artifact that proves they understood the system well enough to master it.

The primary inspiration is **GregTech: New Horizons (GTNH)** — specifically its depth of production graph complexity, its multi-tier processing chains, its demand that players understand systems rather than execute recipes from memory, and its culture of genuine accomplishment through genuine difficulty. The roguelike layer is not a cosmetic addition. It is the mechanism by which a GTNH-depth game becomes replayable: the graph itself is different every run.

The game is also meaningfully different from GTNH in one key respect: **the design phase is the game.** Watching machines run, fixing belt bottlenecks, and grinding execution time are minimized. The intellectual work of reading a run, planning a factory, and discovering alien science is maximized. A run that takes 20 hours of engaged thinking is more satisfying than a run that takes 200 hours of engaged thinking plus 800 hours of waiting and grinding.

### Closest existing references
- **GregTech: New Horizons** — depth, complexity, tier-gated progression, power systems
- **Factorio** — factory layout, belt logistics, the satisfaction of throughput
- **Slay the Spire** — roguelike meta-progression, run variance as the replayability engine
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

> *"I landed on an alien world, figured out how its physics worked, and built something that shouldn't exist."*

The player is not a factory operator. They are a **scientist-explorer** who happens to build factories as the output of their scientific work. The factory is the proof of understanding, not the activity itself.

A successful run feels like solving a deep puzzle — the satisfaction of having read a complex system, found the critical path through it, and executed a plan that required genuine expertise. The escape artifact isn't a grind reward. It's a thesis.

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
A dedicated **Research** currency (name TBD) is earned through scientific activity and spent to formalize knowledge into actionable game content — unlocking recipe details, confirming tech node requirements, and revealing tier contents.

Research is earned by:
- Collecting and analyzing samples from the world (primary source, especially early)
- Running experiments at analysis stations
- Reaching certain production milestones (the factory itself generates research through operation)
- Exploration discoveries (found objects, ruins, anomalies in the world)

Research is spent to:
- Reveal a recipe's full parameters (inputs, outputs, ratios, processing requirements)
- Confirm a tech node's unlock conditions
- Partially reveal an adjacent tier's contents
- Upgrade analysis equipment to access higher-tier samples

Research is **scarce enough to force tradeoffs**, especially early. The player cannot reveal everything before building anything. They must commit to investigating certain paths before others, which makes their scouting decisions consequential.

### The physical discovery loop
The player's avatar is the research instrument. Science happens in the world, not in menus.

- The player physically moves through the world to find sample sites, anomalies, and resource deposits
- Collecting samples requires reaching the location and interacting with it
- Some sample sites are dangerous, distant, or require specific equipment to access — creating a progression gate on information as well as production
- Analysis stations are built in the world and consume samples + research currency to produce knowledge
- The act of experimentation has a mild world-reactivity cost — running reactions disturbs local ecosystems

### Information visibility model
At any point in a run, the player has three tiers of knowledge about any given recipe or tech node:

1. **Known to exist** — the node appears in the tech tree or recipe list, but parameters are hidden
2. **Partially revealed** — broad parameters visible (approximate inputs, rough output range), specifics still hidden
3. **Fully revealed** — complete recipe, all parameters, buildable

The player spends research to move nodes from tier 1 → 2 → 3. The decision of *which* nodes to fully reveal before others is a core strategic choice, especially on runs where research is tight.

> ⚠️ **OPEN QUESTION:** Should there be a mechanic for "building blind" — committing to a recipe before fully revealing it, at some risk cost? This would add tension but could be frustrating if it produces irreversible bad outcomes.

---

## 7. The Tech Tree

The tech tree is the skeleton of the run. It is the one structure that is always partially visible — players can always see its shape, even when its contents are hidden.

### Tier structure
The tech tree is organized into **tiers** (count TBD). Each tier has:
- A visible unlock condition (what production milestone, research threshold, or exploration achievement opens this tier)
- Hidden contents (the specific nodes inside are not visible until the tier is unlocked)
- A visible "shadow" — players can see *how many* nodes are in the tier and their rough categories, but not their specifics

This mirrors the GTNH quest book model: the journey's shape is legible, the specifics are discovered.

> ⚠️ **OPEN QUESTION:** Exact tier count needs to be determined in conjunction with target run lengths. Preliminary thinking is 4–6 tiers for a Standard run, with Pinnacle runs having deeper trees.

### Node seeding
Nodes are drawn from a content pool at run generation. Not every node exists in every run. Nodes have:
- A **category** (power generation, material processing, logistics, science, etc.)
- A **tier range** — the tiers in which this node can appear (a node won't appear in a tier wildly outside its intended power level)
- **Rarity** — how likely the node is to appear in any given run
- **Unlock vectors** — one or more ways the node can be unlocked (see below)

The seed selects which nodes appear and assigns each node's active unlock vector(s) for this run.

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

The recipe graph is the intellectual heart of the run. It defines what this world's alien science looks like — which materials exist, how they process into other materials, and what the critical path to the escape artifact requires.

### Fictional science grounding
The recipe graph is grounded in a **consistent fictional science** — not real-world chemistry, but an internally logical system with its own rules and properties. Players cannot import real-world knowledge directly, but they can develop genuine expertise in the fictional system's structure across runs.

The system has partial real-world inspiration — materials behave in ways that feel physically motivated even if they don't match actual chemistry. This gives the world texture and makes the planet's physical properties feel connected to its production chains, without requiring chemistry knowledge to play.

### Graph structure
The recipe graph is a directed acyclic graph (DAG) with the escape artifact as its terminal node. Every recipe in the graph is a path toward that terminal, either directly or as a prerequisite.

Graph properties that vary by seed:
- **Which nodes exist** (drawn from the node pool, as above)
- **Recipe parameters** — input quantities, output quantities, processing time, energy cost — vary within bounded ranges per node
- **Byproduct generation** — some recipes produce secondary outputs; which byproducts and at what rates is seeded
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
No single power source should be the correct answer across an entire run. Players are forced to transition their power strategy as they progress, making power a recurring strategic problem rather than a solved one. This mirrors GTNH's best power design while adding roguelike variance on top.

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

### Power transitions as factory events
Transitioning power tiers is potentially the most disruptive event in a run — it may require rebuilding significant infrastructure. Combined with the world's reactivity to factory footprint, a major power transition is a meaningful moment in the run's arc, not a background task.

> ⚠️ **OPEN QUESTION:** Should there be a mechanic that makes power transitions visible or dramatic — a moment where the factory goes dark briefly, or where the world reacts specifically to the energy signature change? This could add narrative texture to what is otherwise a logistics problem.

---

## 10. The Factory Layer

The factory is the physical consequence of good planning. It is moderately spatial — the map matters and creates real routing decisions, but spatial optimization is not the primary challenge.

### Design intent
Factory layout should feel satisfying and consequential without being the game's central puzzle. A player who makes good planning decisions should be able to build a functional factory without needing to be a Factorio expert. A player who is also a strong spatial thinker gets additional satisfaction from elegant layouts, but this is a bonus rather than a requirement.

### Core mechanics
Standard factory-game primitives apply: conveyor belts (or equivalent), pipes for fluids, machines that consume inputs and produce outputs, power distribution networks, storage. The specific implementation (2D top-down, 2.5D isometric, etc.) is a separate visual/technical decision.

> ⚠️ **OPEN QUESTION:** Visual perspective and core movement model (top-down 2D vs isometric vs other) not yet decided. This significantly affects development scope and feel.

### The avatar in the factory
The player's physical presence in the world means they move through their own factory. This creates an organic relationship between the scouting/science layer and the factory layer — the player is always in the same world, not switching between a map view and a factory view. The factory grows around them as they work.

### Minimizing watch-and-fix time
Several design decisions exist specifically to reduce passive observation and incremental fixing:

- **Ghost planning.** The player can lay out a factory plan in "ghost" mode before committing resources, allowing full design before execution
- **Blueprint system.** Sub-factory templates can be saved and placed, reducing repetitive placement on known patterns
- **Bottleneck visualization.** The game clearly surfaces throughput problems rather than requiring the player to stare at belts to find them
- **Automation of routine fixes.** Where possible, the game handles routine maintenance automatically; the player's attention is reserved for genuine decisions

---

## 11. The World & Environment

### Map generation
The world map is procedurally generated from the seed. Key elements:
- **Resource deposits** — ore patches, fluid pockets, unique material sites, placed with intention rather than pure randomness (no run should have a critical resource unreachably far from a viable starting location)
- **Terrain features** — cliffs, water bodies, elevation changes create physical routing constraints and shape factory orientation
- **Points of interest** — ruins, anomalies, and phenomena that are sources of exploration discoveries and unlock triggers
- **Biome regions** — areas with distinct environmental properties that affect machine operation and scouting conditions

### Biomes
The world may contain multiple biome regions, each with distinct properties. Biomes affect:
- Local machine efficiency (heat, cold, pressure)
- Sample types available for analysis
- World reactivity rate
- Visual character

Biome types are expanded through meta-progression — new biomes unlock across runs, adding variety to the world generation pool.

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

The world's reactivity profile is seeded — some worlds react quickly and dramatically, others are resilient. This is a meaningful run modifier that affects pacing and strategy.

> ⚠️ **OPEN QUESTION:** Should world reactivity create new opportunities as well as problems — e.g., reactive events that open access to new resources or discovery sites? This would make reactivity feel more dynamic and less purely punishing.

---

## 12. The Escape Condition

Each run has a single large escape artifact — a complex final item that requires the player to have mastered the run's full production graph to synthesize.

### Design intent
The escape artifact is the run's thesis statement. Completing it means the player understood this alien world's science well enough to produce something that could not exist without that understanding. It is not a checklist — it is proof of mastery.

### Structure
The escape artifact is a terminal node in the recipe graph. Its prerequisites cascade through the entire graph, meaning a player cannot shortcut to it — every major production chain must be solved. The specific artifact varies by run scenario (see meta-progression).

### Throughput requirement
The escape condition is not completed by producing the artifact once. It requires **sustained factory output** at a target throughput for a defined duration. This means:
- The factory must be genuinely functional, not a one-off craft
- Bottlenecks are punished even at the run's end
- The player must judge when their factory is ready to attempt the escape, not just when they have the ingredients

### Player-initiated
The player chooses when to attempt the escape. There is no forced end condition. A player who wants to optimize their factory further before attempting can do so. A player who is satisfied with a functional solution can attempt immediately. This respects the "variable length" run philosophy.

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

**Narrative.** Completing runs at various difficulty tiers unlocks story content — lore about the world(s), the character's history, the reason for the escape conditions. Narrative is delivered through the field computer, discovered artifacts, and run completion screens. The story emerges across many runs, rewarding long-term play.

**Blueprint slots.** Additional blueprint save slots unlock across progression. Blueprints save sub-factory layouts as templates. Critically, **blueprints are templates, not solutions** — they require adaptation to each run's specific ratios and machine parameters. They save layout time, not thinking.

**Starting conditions pool.** Small starting boons become available — a modest extra resource cache, a single pre-researched node, a slightly upgraded starting tool. One boon is chosen at run start from a small pool. These ease the very earliest moments of a run without affecting its depth.

> ⚠️ **OPEN QUESTION:** Should there be any persistent knowledge that carries between runs — e.g., a "field notes" system where the player can record observations about node types or planet modifier effects? This could reward careful players without making runs trivial.

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
Permadeath is **opt-in**. Players choose their relationship with failure at run start. This respects the tension between roguelike tradition and factory game investment — a player who has spent 20 hours on a run should be able to choose whether losing it is part of the game.

### Failure conditions
Failure (on permadeath runs) can occur when:
- World reactivity reaches a critical threshold that makes factory operation impossible
- The player's power situation collapses and cannot be recovered
- (Other failure conditions TBD — see open question below)

> ⚠️ **OPEN QUESTION:** Exact failure conditions need careful design. Failure should feel earned and legible — the player should be able to see it coming and have had real opportunities to prevent it. Sudden run-ending events with no warning are not appropriate for this game.

### Non-permadeath
Without permadeath, runs always complete — the only question is how elegantly. The player can always limp to the escape condition with an inefficient factory. The roguelike variance expresses itself as difficulty and elegance, not as binary success/failure.

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

> ⚠️ **OPEN QUESTION:** Should there be official mod support tooling (a content editor, a run validator, a balance checker) shipped with the game or as a separate tool? This significantly affects development scope but substantially lowers the barrier to quality mods.

---

## 18. Open Questions Register

Collected here for easy tracking. Each item links back to the section where it first appears.

| # | Question | Section | Priority |
|---|---|---|---|
| 1 | Should "building blind" be a supported mechanic — committing to a recipe before full reveal, at some risk? | §6 | Medium |
| 2 | Exact tier count for tech trees across difficulty levels | §7 | High |
| 3 | Visual perspective and movement model (2D, isometric, other) | §10 | High |
| 4 | Should world reactivity also create opportunities (new resources, discoveries) as well as problems? | §11 | Medium |
| 5 | Should power transitions have a dramatic in-world expression (factory goes dark, energy signature event)? | §9 | Low |
| 6 | Exact failure conditions for permadeath runs — what specifically ends a run? | §16 | High |
| 7 | Should a persistent "field notes" system exist for cross-run knowledge recording? | §14 | Low |
| 8 | Should official mod tooling ship with the game? | §17 | Medium |
| 9 | Working title — game needs a name | — | Medium |
| 10 | Run length targets need playtesting validation | §4 | High |

---

*GDD v0.1 — Generated from initial design exploration session. Next step: revisit high-priority open questions, then move to technical design document for core systems.*
