---
type: decision
title: "Machine Config-Dedication: Hard Capability Gate for Combinatorial Lines"
date: 2026-07-10
summary: "Machine Config-Dedication: Hard Capability Gate for Combinatorial Lines"
tags: [machines]
---
# Machine Config-Dedication: Hard Capability Gate for Combinatorial Lines


**Decision:** Factory *scale* (why a run needs more than ~1 machine per type, and why the base looks impressive) comes from **machine dedication**, not item-belt lines. A machine's *effective* runnable recipe set is narrowed by its physical **config**; timesharing one machine across all its recipes is impossible, so distinct configs must be built as distinct dedicated instances, recombined into lines.

1. **Problem.** Item transport is ME-style by prior decision (GDD §10 — no belt-path puzzles, Pillar-2 "design is the game"). The item-network *does* have a throughput lever — AE2-style **channel capacity** (GDD §10) that forces sub-network segmentation (smelting net / processing net) — but that gates *item flow*, not *machine count*. On the machine side, `MachineCapability.capable` today = **all** unlocked recipes for a type+tier, and the dispatcher (`crafting.md §8`) timeshares them with zero switching cost. So one `chem_reactor` JIT-serves every chem recipe → a run clears with ~1 machine per *type* → no scale, no combinatorial factory.

2. **What "variety" actually is (user correction).** GTNH impressiveness is **combinatorial**, not roster size: ~10 workhorse archetypes (EBF, chem reactor, distillation tower) × a large recipe space × *arranged into dedicated lines*. The build is the recombination. The "line" is a chain of **committed** multiblocks (coil tier, casing, bed), and the commitment is what makes you build another instead of reusing one.

3. **Mechanism — config as a HARD capability gate.** Recipes carry `required_config`; a machine satisfies it via its installed config. Effective capability = `(type+tier ∩ unlocked_recipes) ∩ config-satisfied`. A `chem_reactor [bed: acidic]` cannot run alkaline recipes; run both live → build both. Config is a **hard** gate on `capable`, not a soft dispatch preference (soft collapses back to timesharing under optimization).

4. **Config is distinct from `RecipeCategory` / `category_filter`.** Category filter = player *preference* (soft policy, `machine-ui §3`). Config = physical *capability* (hard gate). Never conflate.

5. **Friction lives in build/config, not runtime — no added waiting.** Config is set when building/configuring a machine (the pillar-approved fun); recipe times stay short. Changeover requires the machine idle (same rule as the parallel-slot module, `crafting.md §7`), which *discourages* thrashing and *encourages* dedication. ME item convenience survives: a config-mismatched craft job simply queues until a matching machine exists (reuses the existing "no capable machine → job stays queued" behavior, `crafting.md §13`) — a signal to build one, not a failure.

6. **Granularity is the whole tuning knob.** ~**1 config axis per archetype, 3–5 values** (categorical `bed_type` or ordered `coil_tier`). ~10 archetypes × ~4 values ≈ a bounded, impressive combinatorial base — no machine explosion, no collapse to 1-of-each. Higher machine tiers add `module_slots`, so a late-tier machine can hold more axes at once → natural **sprawl-early, consolidate-late** progression.

**Rationale:** Two user worries — "a run shouldn't finish with 1 of each machine" and "players shouldn't wait on recipes instead of building/planning" — share one root: **no forcing function for machine scale, because a universal machine timeshares its whole recipe space for free.** Long recipe times (GregTech's answer) would force parallel banks but *is* the waiting sin Pillar-2 forbids, and banks of identical machines are fake bigness (copy-paste, not planning). Config-dedication forces scale as **combinatorial variety** — the impressive kind — with the friction in the build phase, not the clock. It sits exactly at the middle of the original main-net-JIT ↔ dedicated-lines dial: **ME-style item transport (no belts, Pillar-2 intact) + GTNH-style machine dedication (lines, scale, variety).** Engine cost is small: one filter in one existing system; dispatch, plans, catalysts, ports untouched.

**Alternatives considered:**
- *Item-throughput gate as the scale lever (rely on GDD §10 channel capacity / add logistics-cable rate):* Kept as the **item-network** lever, rejected as the answer to *machine* scale — channels force network segmentation, not dedicated machine instances, and give no combinatorial variety. The two are complementary layers, not substitutes.
- *Long recipe times to force parallel banks:* Rejected — Pillar-2 waiting sin; and identical-machine banks are fake scale.
- *Many unique machine types instead of few configurable archetypes:* Rejected — roster bloat; GTNH scale is recombination of few archetypes, not count (user's own correction).
- *Soft dispatch bias (config preferred, not required):* Rejected — collapses to timesharing under optimization.
- *Belts / physical item routing:* Rejected — reintroduces watch-and-fix, breaks the ME-network decision (GDD §10).
- *Recipe-changeover **time** cost as the friction:* Rejected as the primary lever — runtime friction = waiting; config-as-build friction preferred. A changeover-requires-idle rule is kept (point 5), but it is not a time sink.

**Implications:**
- **GDD §10:** "machine configuration" (already named as a complexity source) sharpened into a **Machine dedication** subsection — config narrows what a machine can run; dedicated instances recombined into lines are the machine-side scale lever, complementary to channel-capacity segmentation.
- **`crafting.md`:** §3 — `MachineCapability` gains a **config-satisfaction filter**; `machine_capability_register_system` recomputes on module/config change (new trigger) in addition to `TechNodeUnlocked` + placement. §7 — a new **config module class** that gates capability, alongside the existing speed/efficiency/parallel modules (which only multiply). New derived `MachineConfig` component + recipe `required_config` field. §8 dispatch **unchanged** (reads the narrowed `capable`). Forward-ref notes added to §3/§7; full spec deferred (see open sub-decisions).
- **Content schema:** recipes gain `required_config`; per-archetype config axes are authored/seeded content and the primary tuning surface (granularity knob, point 6). Supplies the **mid-run** scale pressure that the escape sustained-rate finale (2026-07-08 entry) supplies at the end.
- **Config source — RESOLVED (2026-07-10): config modules.** Config is a **durable installed module** (an item), not a placement-time variant. Rationale: Exergon has no physical multiblock assembly (GTNH's structural coils/casings don't map), a placement-time variant menu is poor UX, and modules-as-items get their own crafting cost and tech-tree unlock gating for free — a new config = a new gated item that expands an archetype's reach. The module is **installed/removable and occupies a config slot; not consumed per recipe** (durable fitting, unlike catalyst inputs which are network-reserved per job, §6). Reuses the module system (§7) and its per-tier slot-count consolidation arc. Yields a **double-gate**: a recipe class runs only when its recipe is tech-unlocked *and* the machine has the matching config module installed.
- **Axis model — RESOLVED (2026-07-10): both.** `required_config` supports categorical (`bed_type`, `==`) and ordered (`coil_tier`, `≥`) requirements — same check, `==` vs `≥`; covers the reactor-mode and EBF-tier shapes. **All sub-decisions now resolved; `crafting.md §3/§7` can be specced formally.**
- No code yet — direction lock only. Also flags the GDD-§10-vs-`networks.md` channel-capacity divergence (obs 2962) as still open, separate from this decision.

