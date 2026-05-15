# Vertical Slice Signal Spec

> **Status:** Medium-level design specification. This document defines what the vertical slice must prove and which features, systems, and interfaces must exist to get a useful signal. It is not an implementation plan. Deeper technical designs and sprint plans should be derived from this before code is written.

---


## 1. Target Slice Shape

The slice should support two run profiles:

### 1.1 Insight Run

A short Initiation-style run, target **60-120 minutes**, used to test onboarding, first-hour insight, basic factory setup, first drone discovery, and escape.

Purpose:
- Validate the first 30-60 minutes.
- Validate the minimum viable science-discovery loop.
- Validate Remote mode in a low-complexity environment.
- Validate that a player can complete a full run without external guidance.

### 1.2 Standard Probe Run

A longer Standard-like probe, target **3-5 hours**, not a full 10-15 hour Standard run. It should contain enough mid-run structure to test whether the experience remains active after the opening.

Purpose:
- Test whether the "design phase is the game" pillar survives beyond the first setup phase.
- Test one meaningful power transition.
- Test one meaningful expansion or second-site logistics decision.
- Test repeat-run variation across several seeds.

The Standard Probe should be long enough to expose pacing problems, but not so long that iteration becomes impossible.

---

## 2. Core Gameplay Scope

### 2.1 Required Loop

The vertical slice must include the following complete loop:

1. Land on a seeded planet.
2. Read initial planet properties.
3. Make an early strategic inference from those properties.
4. Build basic production and power.
5. Use research/science to reveal actionable knowledge.
6. Pilot a drone beyond the local safe area.
7. Discover at least one remote site or material.
8. Use that discovery to unlock or reveal a production path.
9. Produce an escape-enabling item.
10. Connect or deliver it to an escape structure.
11. Trigger a visible run completion event.

Any feature that does not support this loop should be questioned for vertical-slice inclusion.

### 2.2 Required Player Decisions

The slice should force, or strongly invite, these decisions:

- Which early power source fits this planet?
- Which visible tech/research node should be revealed first?
- Which drone destination is worth the trip?
- Whether to spend limited research now or scout for a better reveal.
- How much production capacity is enough for the escape item.
- In the Standard Probe, whether to retrofit power/logistics or limp forward inefficiently.

The player should not be able to complete the slice by following a single obvious checklist with no meaningful interpretation.

---

## 3. Features Required For Good Signal

### 3.1 Seeded Planet Identity

Required:
- Seeded planet properties visible at run start.
- At least two meaningful planet-property axes that affect play.
- Visual or UI presentation that makes the planet feel like a specific world, not a parameter table.
- At least one early property-to-decision connection.

Minimum viable examples:
- Low solar efficiency makes solar weak.
- High atmospheric oxygen makes combustion strong.
- High geological activity marks a region as worth scouting.
- Cold/heat changes one recipe or machine efficiency.

Required interface:
- Landing/orientation panel with planet properties.
- Short explanation of what each visible property affects.
- Later confirmation when the player's action validates the inference.

Good signal:
- Players can explain after the run: "I chose X because the planet had Y."

Bad signal:
- Players describe the seed only as numbers.
- Players ignore planet properties.
- The correct choice is obvious without reading the planet.

### 3.2 First-Hour Insight Beat

Required:
- One designed insight beat that should occur within 30-60 minutes.
- The insight must connect observation to action to payoff.
- The payoff must be concrete: faster power, unlocked material, better route, or avoided bottleneck.

Example pattern:
- Player sees low solar and oxygen-rich atmosphere.
- Player chooses combustion over solar.
- The factory reaches stable power sooner.
- UI or world feedback reinforces that this was the right inference.

Required interfaces:
- Planet property display.
- Tech tree or recipe UI showing the relevant option.
- Feedback showing the result of the choice.

Good signal:
- A first-time player says some version of "I figured out that this planet wanted me to do X."

Bad signal:
- The player only says "I researched the next thing."
- The player succeeds without noticing the planet.
- The player notices the planet but cannot act on it.

### 3.3 Minimal Tech Tree

Required:
- Tiered tech tree with visible shadow structure.
- Enough nodes to create ordering choice.
- At least two unlock vectors:
  - Research spend
  - Exploration discovery
- Optional but valuable:
  - Production milestone

Minimum node set:
- Basic smelting/forming
- Basic extraction
- Two early power options
- Basic logistics/storage
- Basic analysis/research
- Land drone
- Alien material or site theory
- Escape synthesis

Required interfaces:
- Tech tree page.
- Node reveal panel.
- Clear locked, revealable, revealed, and unlocked states.
- Display of why a node is blocked.

Good signal:
- Players understand what options exist, even when specifics are hidden.
- Players make different early unlock choices across runs.

Bad signal:
- Players click every available node in order.
- Hidden nodes are too vague to plan around.
- Research scarcity causes confusion rather than tradeoff.

### 3.4 Research And Knowledge Visibility

Required:
- At least one research currency for the Insight Run.
- Standard Probe may add a second currency only if it produces a real decision.
- Known-to-exist and fully revealed states are required.
- Partial reveal should be included only if it enables a real decision.

For the slice, partial reveal must answer one of:
- Should I spend research here or elsewhere?
- Should I scout for more context before spending?
- Is this path probably compatible with my current factory?

If partial reveal cannot answer one of those, omit it from the slice and test it later.

**Research type scarcity is a mid-run lock-out risk.** If the slice uses multiple research types, a player who explores aggressively and neglects production milestones may find themselves unable to unlock QoL tools at the intended time, after 5+ hours with no recovery path. The slice should either use one research type (simplest), or explicitly test whether players can identify which type they are short on and what to do about it. If players discover the imbalance mid-run and cannot read the cause, the multi-type model is not ready.

Required interfaces:
- Research balance display.
- Reveal cost display.
- Research source display: where this currency comes from.
- Blocked-reason display for unaffordable reveals.
- If multiple research types exist: per-type balance and per-type source visible from one surface.

Good signal:
- Players can explain why they revealed one node before another.
- Players with an imbalanced research mix can identify the cause and correct it.

Bad signal:
- Players always skip partial reveal.
- Players hoard research because spend consequences are unclear.
- Players feel locked out by the wrong research type with no clear corrective action.

### 3.5 Recipe Graph And Planner UX

Required:
- Recipe browser for known/revealed recipes.
- Basic graph or dependency view for escape item.
- Basic ratio or throughput estimate earlier than full automation.
- Machine count estimate for at least the escape chain.
- Bottleneck or blocked-production alert.

The slice does not need full MVP planner depth. It does need enough tooling that players solve the factory instead of fighting the UI.

Required interfaces:
- Item/recipe browser.
- Escape item dependency view.
- Machine UI showing current recipe, progress, inputs, outputs, power state, and blocked reason.
- Alerts panel for blocked machines.

Good signal:
- Players use in-game tools to plan.
- Players do not need external spreadsheets for the slice.

Bad signal:
- Players guess machine counts.
- Players wait passively because they cannot identify the bottleneck.
- Players rely on trial-and-error instead of understanding the graph.

### 3.6 Basic Factory And Logistics

Required:
- Placeable machines.
- Basic machine recipes.
- Basic storage/logistics network.
- Enough cable/network setup to test readability.
- Clear machine blocked states.

Not required:
- Full module system.
- Full auto-crafting.
- Complex sub-network interfaces.
- Full blueprint system.
- Full 3D vertical factory optimization.

For the Standard Probe:
- Include one network capacity or routing pressure point only if it is legible.
- Avoid stacking logistics complexity on top of unproven recipe complexity.

Required interfaces:
- Placement preview.
- Machine interaction panel.
- Storage/terminal view.
- Network connection state.
- Machine and network alerts.
- Network overlay or topology view that shows connections without requiring the player to physically walk the factory.

**3D readability is a first-class signal here.** Every successful deep factory game (Factorio, DSP, GTNH) uses top-down or isometric because item flow and network topology are readable at a glance. In 3D, players walk through their factory. Cable routing, machine adjacency, and logistics flow must remain comprehensible from inside the space. If they are not, the recipe graph complexity cannot be engaged — players will be debugging the view instead of the design.

Good signal:
- Players understand why machines are or are not running.
- Players can recover from a misconfigured machine without external help.
- Players can identify a bottleneck or disconnected machine without walking the entire factory.

Bad signal:
- Players spend most of their time debugging connections.
- The factory works or fails mysteriously.
- 3D placement obscures network topology.
- Players need to physically walk to each machine to understand the factory state.

### 3.7 Power System

Required:
- At least two early power sources with planet-dependent viability.
- Basic power network.
- Non-destructive failures.
- Clear diagnostics.

Insight Run:
- One power tier is enough.
- The core test is planet-property inference and diagnostic clarity.

Standard Probe:
- Include one power transition or retrofit.
- Include voltage/amperage only to the degree needed to test whether the model is understandable.

Required interfaces:
- Power source output display.
- Network demand/supply display.
- Per-machine power blocked reason.
- If VxA is present, show voltage requirement and amp usage in plain terms.

Good signal:
- Players understand power failures and can fix them.
- Players make a different power choice on different planet seeds.

Bad signal:
- Players overbuild one source every run.
- Power failures feel arbitrary.
- VxA terminology becomes a tutorial wall.

### 3.8 Remote Mode And Drone Exploration

Required:
- Local mode / Remote mode switch.
- One land drone.
- Drone camera and controls.
- Drone inventory or sample buffer.
- At least two scouted destinations with different value/risk profiles.
- Fog or imperfect scan information.
- Manual sample or item collection.
- Explicit return/deposit flow.
- Drone discovery event that unlocks or reveals a tech path.

Not required:
- Multiple drone tiers.
- Digger, flying, or space drones.
- Full off-surface exploration domains.
- Autonomous drone commands.
- Full drone loss economy.

Risk should exist, but should be tuned carefully:
- The drone can be damaged, delayed, or forced to return.
- Permanent drone loss is optional for the slice and should only be included if recovery is fast.

Required interfaces:
- Mode indicator: Local vs Remote.
- Drone status display.
- Drone cargo/sample display.
- Scan result display.
- Return/deposit prompt.
- Map marker or compass for discovered sites.

Good signal:
- Players describe drone outings as tense or interesting.
- Players choose between destinations based on imperfect information.
- Returning from Remote mode does not disorient them.

Bad signal:
- Players see drones as slow errands.
- Players avoid risk and miss required content.
- Players forget what they were doing at the factory after returning.

### 3.9 Escape Objective

Required:
- One visible escape structure or site.
- Escape item production chain.
- A field requirement: power, key insertion, or site interaction.
- Player-initiated activation.
- Clear run completion screen.
- Visible in-world completion moment, even if simple.

The escape objective should feel like a culmination of the slice, not just a checked condition.

Required interfaces:
- Escape progress display.
- Site interaction prompt.
- Missing requirement display.
- Completion screen with seed and elapsed time.

Good signal:
- Players understand what remains before escape.
- Completion feels earned.

Bad signal:
- Escape is discovered only by accident.
- The final action is a UI-only state transition.

---

## 4. Standard Probe Additions

The Standard Probe extends the Insight Run with just enough mid-run complexity to test pacing.

Required additions:
- One additional tier or tier-like phase beyond the first escape chain.
- One deeper production chain with byproduct or catalyst pressure.
- One meaningful power transition or retrofit.
- One second-site or remote-site dependency.
- One stronger research tradeoff.
- One longer factory-running interval where bottleneck tools are needed.

Not required:
- Full Standard escape type.
- Full 10-15 hour length.
- Full procedural tech tree.
- Full multi-domain world.
- Full reactivity system.

The Standard Probe should answer:
- Does the player keep making decisions after the first factory works?
- Does the middle of the run generate new plans, or just waiting?
- Does the player understand what to improve next?
- Does the player want to start another seed?

---

## 5. Repeat-Run Discovery Test

The slice must support repeat testing across multiple seeds.

Required:
- At least 5 curated or generated seeds.
- Each seed should change one or more meaningful axes:
  - Power viability
  - Resource geography
  - Discovery-site location
  - Alien material chain
  - Research pressure
- Seeds must be valid and comparable.

For the vertical slice, curated seeds are acceptable and probably correct. They keep the slice focused on player signal instead of generator debugging. They also bypass the core procedural-risk question: whether the game can produce many valid, balanced, non-pathological recipe graphs. Passing this section with curated seeds does not validate procedural graph generation.

Minimum repeat-run test:
- Have the same player complete or substantially play 3 runs.
- Observe whether they are still investigating, or simply recognizing the same path.

**The threshold that matters is run 5–10, not run 3.** Run 3 may still feel like discovery simply because the player has not seen enough to have internalized the full node pool. If discovery collapses into parameter-reading by run 5 — "is resonite here, and what tier?" instead of genuine investigation — the replayability premise fails. Three-run tests are the minimum; 5-run and 10-run tests should be planned before shipping past vertical slice. This is the core unproven claim of the design.

Before expanding beyond curated seeds, a standalone generator validator is required — one that can confirm a generated run is not just technically solvable but within acceptable difficulty bounds. Curated seeds do not prove the procedural premise.

Good signal:
- Run 2 and Run 3 produce different plans.
- Players use prior knowledge without feeling the run is solved.
- Players discuss worlds by character, not just efficiency.

Bad signal:
- Players say "I know this, I just need to find the xalite equivalent."
- Different seeds produce the same build order.
- Variation only changes quantities or travel time.
- Discovery collapses by run 5 into reading known parameters off a known map.

---

## 6. Instrumentation

The vertical slice should record lightweight telemetry in development builds.

Required events:
- Run started: seed, profile, timestamp.
- First planet property viewed.
- First tech node revealed.
- First research spent.
- First machine placed.
- First stable production loop.
- First power failure.
- First power failure resolved.
- First drone deployed.
- First Remote mode entry and exit.
- First discovery event.
- First escape item produced.
- Escape completed.
- Time spent idle or with all machines blocked.
- Tutorial/intervention triggers, if present.

Required derived metrics:
- Time to first insight candidate.
- Time to first research unlock.
- Time to first stable production.
- Time to first drone discovery.
- Number and duration of blocked states.
- Number of Remote mode trips.
- Time to re-engage factory after Remote mode.
- Total run time.

Manual observation remains required. Telemetry should support playtest notes, not replace them.

---

## 7. Playtest Protocol

### 7.1 First-Time Player Test

Goal:
- Validate first-hour insight and onboarding.

Session:
- One Insight Run.
- 90-120 minutes.
- Minimal facilitator intervention.

Observe:
- What the player thinks the goal is.
- Whether they read planet properties.
- Whether they form a correct inference.
- Where they get stuck.
- Whether they can read the starter factory layout, machine state, and network connections in 3D.
- Whether Remote mode feels understandable.

Post-session questions:
- What did you figure out about the planet?
- Why did you choose your power setup?
- What was your next goal when the session ended?
- What was confusing?
- Did the drone feel useful, risky, slow, or interesting?

### 7.2 Repeat-Run Player Test

Goal:
- Validate discovery-to-recognition threshold.

Session:
- Same player plays 3 Insight Runs across different seeds.
- Runs may be shortened after first completion if the pattern is clear.

Observe:
- Whether later runs produce different plans.
- Whether player anticipation feels satisfying or rote.
- Whether discovery becomes lookup.

Post-session questions:
- Did this run feel meaningfully different?
- What did you know from prior runs?
- What did you still need to discover?
- Would you start another seed?

### 7.3 Standard Probe Test

Goal:
- Validate mid-run pacing.

Session:
- One 3-5 hour Standard Probe.
- Save/resume should be tested if the session is split.

Observe:
- Whether the player keeps making meaningful decisions.
- Whether bottleneck tools are used.
- Whether factory topology remains readable as the base grows.
- Whether power transition is understood.
- Whether factory operation becomes passive.

Post-session questions:
- When did the run feel most interesting?
- When did it feel slow?
- What were you planning during the middle hour?
- Did you know how to improve your factory?
- Did the longer structure make you want more, or feel stretched?

---

## 8. What To Stub Or Defer

The vertical slice should aggressively defer systems that do not answer the four core questions.

Defer:
- Full procedural recipe generation.
- Full procedural tech tree generation.
- Full 10-tier ladder.
- Full module system.
- Full auto-crafting.
- Full blueprint system.
- Full world reactivity.
- Full codex and meta-progression.
- Multiple drone tiers.
- Underground, atmospheric, and orbital domains unless explicitly needed for the Standard Probe.
- Mod loading and mod tooling.
- Full narrative content.
- Permadeath and challenge modifiers.

Stub where useful:
- Use curated seed templates instead of fully procedural generation.
- Use a small hand-authored node pool.
- Use bounded parameter variation on a small recipe set.
- Use simple visual placeholders for machines and sites.
- Use simple completion effects for escape.
- Use basic telemetry logs instead of a full analytics pipeline.

Do not stub:
- Machine blocked reasons.
- Research/reveal state clarity.
- Drone mode transition.
- Planet-property-to-decision connection.
- 3D factory readability for the machines, cables, and logistics included in the slice.
- Escape completion.
- Save/resume for Standard Probe split sessions. The Standard Probe is the only slice run long enough to require saving mid-session; if save/resume is not functional, the 3-5 hour pacing test cannot be completed.
- Field computer delivery surface. Persona and voice are not required; placeholder text is acceptable. The surface — the UI component that frames field computer messages — must exist so onboarding, hint, and world-context prompts can be delivered in the intended diegetic channel during playtesting.

These are part of the signal.

---

## 9. Success Criteria

The vertical slice is successful if:

- First-time players can describe one correct planet-specific inference.
- Players complete or nearly complete the Insight Run without external instructions.
- Drone exploration produces at least one meaningful decision, not just travel time.
- Players can diagnose basic machine and power failures using in-game UI.
- Players can read machine topology, cable/network connections, and bottlenecks in 3D without walking every machine one by one.
- Different seeds produce different plans.
- Repeat players still feel some discovery by run 3.
- The Standard Probe has an active middle, not just waiting for machines.
- Players understand what they would do next after stopping.

The vertical slice is not successful if:

- Players ignore planet properties.
- Players cannot explain why they chose a tech path.
- Players need external notes or spreadsheets.
- Remote mode feels like a chore.
- Power/logistics failures are mysterious.
- Players cannot understand their own factory layout, routing, or bottlenecks from the 3D presentation and overlays.
- Seed variation only changes quantities and locations.
- Repeat-run discovery collapses into parameter lookup by run 3.
- The middle of the longer run is passive.
- The first hour does not produce an insight moment.

Run 3 is the minimum failure threshold, not the confidence threshold. If discovery collapses by run 3, the replayability premise fails. If it survives run 3, the next required study is whether it still works across runs 5-10.

---

## 10. Outputs Before Implementation Plans

Before writing implementation plans, the following deeper designs should be created from this spec:

1. **Insight Run Design**
   - Exact seed templates
   - First-hour beats
   - Node list
   - Recipe list
   - Escape chain

2. **Standard Probe Design**
   - Added tier/phase
   - Mid-run pressure
   - Power transition
   - Second-site dependency

3. **Remote Mode Prototype Design**
   - Controls
   - Drone HUD
   - Scan model
   - Risk model
   - Deposit/return flow

4. **Planning UI Slice Design**
   - Required recipe browser views
   - Minimal graph/dependency view
   - Machine UI blocked states
   - Alerts
   - 3D topology/readability overlays

5. **Telemetry And Playtest Plan**
   - Event schema
   - Manual observation checklist
   - Playtest questions
   - Success/failure thresholds

6. **Procedural Graph Validator Design**
   - Reachability checks
   - Difficulty bounds
   - Resource and research-type availability checks
   - Pathology detection for circular gates, extreme machine counts, and unrecoverable scarcity

7. **Run-Stakes Test Design**
   - Failure model candidates
   - Recovery model
   - Risk/reward decisions
   - Marketing-positioning implications if failure remains non-destructive

Only after those designs exist should implementation plans be written.
