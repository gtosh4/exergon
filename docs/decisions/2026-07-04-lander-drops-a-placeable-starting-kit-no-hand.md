---
type: decision
title: "Lander Drops a Placeable Starting Kit; No Hand Scanner (revises 2026-05-23)"
date: 2026-07-04
summary: "Lander Drops a Placeable Starting Kit; No Hand Scanner (revises 2026-05-23)"
tags: [building]
---
# Lander Drops a Placeable Starting Kit; No Hand Scanner (revises 2026-05-23)


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
- End-to-end coverage: `tests/standard_full_run.rs` drives the real placement + cable + mining + recipe + research systems from placing machines through unlocking the first node.

