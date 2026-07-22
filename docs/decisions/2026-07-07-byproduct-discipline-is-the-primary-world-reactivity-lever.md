---
type: decision
title: "Byproduct Discipline Is the Primary World-Reactivity Lever (sharpens §11)"
date: 2026-07-07
summary: "Byproduct Discipline Is the Primary World-Reactivity Lever (sharpens §11)"
tags: [recipe-graph, world-gen]
---
# Byproduct Discipline Is the Primary World-Reactivity Lever (sharpens §11)


**Decision:** World reactivity (§11) is given a concrete primary driver: **vented byproducts and unconsumed side-streams**. A closed-loop factory that consumes/recycles its waste runs quietly; one that dumps waste drives reactivity faster. The coupling is **soft and bidirectional** — venting some seeded streams degrades the environment, venting others triggers *beneficial* reactions (harvestable atmospheric product, enriched deposit). It never hard-blocks progress. This makes "bad run = ugly escape" mechanically true: an ugly run is ugly *because* it dumped waste.

**Rationale:** §11 already resolved that reactivity is factory-footprint-driven and two-sided (Q#4), but "footprint" was vague. Nullius Discord analysis (~30k messages, 1 year — see [[nullius]]) shows **byproduct management is by far the most-engaged activity** (~1,570 messages, #1 theme), and Nullius's strongest single mechanic is that **vented gases are tracked against the terraforming win condition**. Routing waste back into reactivity turns Exergon's deepest planning activity into its pressure system, with one lever, and reinforces the existing "clean vs ugly escape" stance. The user explicitly asked that the coupling be **soft, not hard**, and allow beneficial reactions — hence bidirectional, not a penalty meter.

**Alternatives considered:**
- *Hard penalty (Nullius-style tracked voiding blocking the win):* Rejected per direction — too punishing, conflicts with the no-forced-failure model (§16).
- *Leave §11 "footprint" generic:* Rejected — the vagueness was the gap; a single legible lever (waste) is more designable and more teachable.

**Implications:**
- §11 "Caused by" first bullet sharpened to name vented byproducts/side-streams; new paragraph "Byproduct discipline is the primary reactivity lever" added.
- Content schema (§17) will need per-waste-stream reactivity tags (harmful / neutral / beneficial), seeded per run — an unlock/discovery vector in its own right.
- Reinforces that recipe graphs must *generate* meaningful side-streams (not just clean input→output chains) for this lever to have teeth. Flag for recipe-graph content design.
- Still post-MVP for the beneficial/two-sided half, consistent with §11 and Q#4.

