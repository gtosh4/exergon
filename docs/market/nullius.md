# Nullius Market Note

> Date reviewed: 2026-07-07
> Subject: [Nullius](https://mods.factorio.com/mod/nullius), a total-conversion overhaul mod for Factorio, by GregorSamsanite
> Purpose: summarize what Nullius does, why it matters to Exergon's von Neumann + consistent-science direction, and extract practical lessons. Backed by a 1-year export of the Nullius Discord (~30k messages, Jul 2025–Jul 2026).

## Snapshot

Nullius (Latin, roughly "of no one / belonging to nobody") is a large Factorio overhaul mod by **GregorSamsanite** (as of 2026, actively maintained by a second dev, "Arthur L", who shipped **Nullius 2.0**). It replaces the base game's premise and is framed as a **Factorio prequel**: you are an **android** sent to terraform barren, lifeless planets and seed them with life, so that — eons later — the engineers of the base game have worlds to crash-land on. There are no humans, no combat by default, and no prior civilisation. Because there is no life, there is **no coal, oil, wood, biters, or free oxygen**; because most planets are poor in heavy elements, technology is built around the **abundant light elements**. You bootstrap an entire industrial base from a handful of raw feedstocks: **iron ore, sandstone, bauxite, calcite, air, seawater, and volcanic gas**.

Its defining trait is **scientifically grounded production**: recipes are built on real chemistry — air is nitrogen + CO₂ (feedstock for organic chemistry / plastics), seawater yields hydrogen, oxygen, chlorine, sodium, and trace deuterium/tritium/lithium, and processing runs on real reactions and electrolysis. The graph is dense, tightly interlocked, and symmetric (balanced science rather than a linear tier escalation), with heavy **byproduct and closed-loop management**. It sits in the top rank of Factorio overhauls alongside Space Exploration and the Py mods.

Useful source pages:

- Mod portal: <https://mods.factorio.com/mod/nullius>
- Forum thread ("Nullius: A Factorio prequel"): <https://forums.factorio.com/viewtopic.php?t=94853>
- Source: <https://github.com/GregorSamsanite/nullius>

## Why Nullius Matters To Exergon

Nullius is a late addition to the reference set because the **von Neumann probe reframe** (see `design-decisions.md`, 2026-07-07) made it doubly relevant:

1. **Theme.** A lone autonomous machine, no civilisation behind it, bootstrapping a dead world from raw materials, is almost exactly Exergon's new core fantasy: a self-replicating probe that lands with a machine-zero kit and must manufacture everything. Nullius proves this fantasy is coherent and motivating without any human narrative crutch.
2. **Depth model.** Nullius derives its depth from an **internally consistent science system**, not from a pile of arbitrary tiers. Players build durable expertise in *how the chemistry works*, then apply it — they are not memorising a recipe list. This is exactly Exergon's `§8` "fictional science grounding" goal, except Exergon swaps fixed real chemistry for **seeded alien science that is internally consistent within a run**.

The distinction matters: Nullius's ruleset is the same every playthrough, so mastery transfers wholesale and the wiki can fully solve it. Exergon keeps the *consistency* (which produces the depth) but reseeds the *specific configuration* each run (which preserves discovery). Nullius is the proof that consistency-as-depth works; Exergon's job is to keep that property while making each run unspoilable.

## What Works

### A Lone Machine Bootstrapping Nothing Is A Strong Fantasy
No humans, no aliens, no story hand-holding — just a machine turning seawater and rock into an industrial civilisation. The premise carries itself. Directly validates Exergon's probe framing and the "the lander *is* the bootstrap" starting-kit design.

### Consistency Is The Depth Engine
Because processes obey real chemistry, players reason forward ("I have brine and power, so I can get chlorine and sodium hydroxide, which unlock…") rather than looking up disconnected recipes. Understanding compounds. This is the intellectual loop Exergon wants from its recipe graph.

### Byproducts And Closed Loops Create Real Planning Problems
Nullius makes waste streams and side-products first-class: you must route, consume, or vent them. This produces genuine layout and balance puzzles instead of linear input→output chains — the kind of "read the system and plan" work Exergon's design phase is built around.

### Constructive Endgame
The goal is terraforming and seeding life — building the world up, not destroying an enemy. Parallels Exergon's constructive escape (fabricate and launch a copy) over a combat or survival win state.

## What To Watch (Cautions)

### Punishing Onboarding, Wiki Dependency
Same failure mode as GTNH: the consistent, deep graph is opaque to newcomers, and players lean on external references. Consistency does **not** guarantee legibility. Exergon must carry this with the codex, graph viewer, and partial-reveal tooling — the in-game tools have to be as good as an external wiki for type-level knowledge.

### Real Chemistry Is Not Directly Transferable To Exergon
Nullius rewards real-world chemistry knowledge. Exergon deliberately does **not** — its alien science is seeded so real-world knowledge cannot be imported (see `§8`). Borrow Nullius's *structural* consistency, not its literal chemistry.

### Symmetric / Marathon Pacing
Nullius is long and evenly demanding throughout. Exergon's run structure and "design phase is the game" stance deliberately compress watch-and-wait time. Take the graph coherence, not the marathon length.

## Comparison To Exergon

| Axis | Nullius | Exergon |
|---|---|---|
| Protagonist | Android terraforming a dead world to seed life (Factorio prequel) | Self-replicating von Neumann probe mastering a world to launch its next copy |
| Prior civilisation | None — you are the first/only machine | Earlier probe lineages + a distant origin; you are one generation of the lineage |
| Science basis | Real chemistry, fixed every playthrough | Internally consistent **seeded** alien science, unique per run |
| Source of depth | Consistent interlocked chemical processes | Consistent seeded graph, discovered through science |
| Player knowledge | External wiki can fully solve it | Codex gives type-level knowledge; run-specific values stay unspoilable |
| Pacing | Long, symmetric, marathon | Run-based, 4–50+ h by difficulty, watch-and-wait minimised |
| Endgame | Terraform / seed life (constructive) | Build + launch the next probe (constructive) |
| Bootstrap | From raw seawater and rock, near nothing | Machine-zero starting kit; manufacture everything |

## Community Signals (Discord, ~30k messages, 1 year)

A 1-year export of the Nullius Discord (general / questions / suggestions / development / media; Jul 2025 – Jul 2026) was analysed for recurring themes. Message counts are keyword-matched (indicative, not exact), quotes are verbatim excerpts.

**Byproduct management is the game.** It is the single most-discussed topic (~1,570 messages, far ahead of any other theme). The recurring problem is not making an item — it is disposing of or balancing what you make *alongside* it. Representative:

> "these production chains are so interconnected how am I meant to handle byproducts well…"
> "trying to balance consumption and production on processes drenched in spaghetti… I don't trust myself not to entirely lose track of what pipe is coming from where"
> "first time I played Nullius I ended up with chests and chests and chests of gravel"

This is exactly the "read the system and plan" work Exergon puts at its centre — and it confirms byproducts/side-streams are where that work lives.

**The killer mechanic: waste is coupled to the win condition.** Terraforming = oxygenating the atmosphere, and the game *tracks what you vent*:

> "Your oxygen voiding is being tracked and counts toward end game."
> "Water electrolysis doesn't work because it just makes extra hydrogen that counters your oxygenation. But the new carbon-sequestration recipes… work."

So sloppy factories that dump waste gases actively fight their own victory. This is a working proof-of-concept for Exergon's **world-reactivity pillar (§11)**: make factory footprint (venting, waste) the driver of reactivity, and make clean closed loops the elegant path — directly supporting the "bad run = ugly escape, good run = clean escape" stance.

**Onboarding pain concentrates in the early game** (~610 "confused / stuck / lost" messages; ~550 referencing external calculators/guides). The byproduct problem bites hardest when the player has the fewest tools:

> "That part was pretty annoying early game. Especially on a first go through."
> "setting up the first time is pain because nanofabricators are gigaslow… but after that it supports itself"
> "on my first run with default settings I spent way too much time catching my building up to research"

Confirms Exergon's stance: cut early friction, deliver an insight payoff in the first 30–60 min, and make the in-game codex/graph tools as good as the external calculators players currently lean on (Factory Planner / Helmod are constantly recommended).

**Pacing sags at the back, not the front** (~300 messages on early-vs-late and "too long"):

> "I launched the rocket. It seems that the first 80% of the game takes [most of the effort]…" *(one of the most-reacted general-channel posts)*
> "late game it's just not worth doing batteries… the stat boosts really fall off"

The symmetric, marathon pacing has a soggy endgame. Supports Exergon's variable-length runs and its explicit minimisation of late-game watch-and-wait.

**Execution tedium is the anti-pillar** — players resent the manual logistics, and love the *science*, not the belt-weaving:

> "I really do not enjoy the part of this that is just weaving belts and pipes of completely random things"
> "so satisfying once you get the bacteria really *frothing* and shitting out iron and copper"

The payoff moment is a designed process coming alive — precisely Exergon's "factory as proof of understanding" climax. The friction moment is manual routing — precisely what Exergon's "design phase is the game" pillar minimises.

**The audience already reads the android as a von Neumann probe.** A player, unprompted:

> "they found out the hard way that neumann ais get overwhelmed when they're allowed to play with the whole repertoire"

Direct external validation that Exergon's von Neumann framing is legible and native to this audience.

**Community shape:** highly engaged power users dominate (top posters have 1,500–2,500 messages each over the year), and the *questions* channel is one of the largest — a healthy but help-hungry community, consistent with a deep-but-opaque game. Note the maintainer handoff (original author GregorSamsanite → active dev Arthur L, Nullius 2.0): the pack's longevity depends on a tiny number of people, a risk any depth-first project should note.

## Lessons For Exergon

1. **Consistency, not tier count, is what makes a graph deep.** Prioritise an internally coherent alien-science ruleset over a large pool of shallow, disconnected recipes. A player who can *reason forward* through the graph is engaged; one who must look everything up is not.
2. **The lone-machine premise needs no narrative crutch.** The probe fantasy is self-justifying — resist adding human framing to "motivate" it.
3. **Elevate byproducts and loops** as planning content, within vertical-slice scope, because they turn linear chains into real layout puzzles.
4. **Consistency does not buy legibility.** Nullius and GTNH both prove a coherent deep graph can still be opaque. Budget codex + tool work as first-class, not polish.
5. **Keep the consistency, reseed the configuration.** Exergon's edge over a Nullius-style fixed pack is that the ruleset is coherent *and* different every run — protect both properties; losing either collapses toward "wiki-solved" or "arbitrary."
6. **Couple waste to consequence (candidate for §11).** Nullius's strongest single idea is that venting byproducts is *tracked and works against the win condition*. Exergon should consider making factory footprint — vented waste, unconsumed side-streams — the concrete driver of world reactivity, so that a clean closed-loop factory is both the elegant and the low-reactivity path, and an ugly run is ugly *because* it dumped waste. This turns the most-engaged Nullius activity (byproduct discipline) into Exergon's reactivity pillar. **Adopted 2026-07-07** as a *soft, bidirectional* coupling (venting can help or harm, never hard-blocks) — see GDD §11 and `design-decisions.md`.
