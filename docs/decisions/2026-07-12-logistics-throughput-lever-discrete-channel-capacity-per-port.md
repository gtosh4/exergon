---
type: decision
title: "Logistics throughput lever = discrete channel capacity (per-port), not flow rate"
date: 2026-07-12
summary: "Logistics throughput lever = discrete channel capacity (per-port), not flow rate"
tags: [logistics]
---
# Logistics throughput lever = discrete channel capacity (per-port), not flow rate


**Decision:** The logistics item-network gets the throughput lever the GDD already promises — **discrete channel capacity, AE2-style** (GDD §10) — and it is **per connected machine port**: each logistics port a machine joins to a network consumes **1 channel**, a network's channel budget is the channel rating of its cable tier, and ports beyond the budget are not served until the player lays higher-tier cable or **segments** into sub-networks joined by router/interface boundaries. This is explicitly **not** a Factorio-style per-tick items/second flow rate. Resolves the open GDD-§10-vs-`networks.md` divergence flagged in the 2026-07-10 config-dedication entry (obs 2962): the GDD specified channel capacity for logistics, but `networks.md §2` modelled *no* capacity (an uncapped priority-ordered shared pool via `has_items`/`take_items`/`give_items`), so the intended segmentation friction never existed in the technical model or `src/logistics/items.rs` — a primary reason run execution is "free" on the item side.

**Rationale:** The instant shared pool means one network JIT-serves every machine with zero routing cost, so the item-flow pillar has no design pressure. The GDD (§10, `gdd.md:434`) already names the fix — discrete channel limits that "encourage players to segment their network into logical sub-networks… segmentation is a solution to be discovered, not a forced constraint" — and the upgrade nodes are already designed but unauthored (`tech-tree-design.md` Logistics category: Channel Upgrade → Sub-network Router I → Sub-network Router II). Per-port-per-channel is the simplest AE2-faithful unit and mirrors the power network's existing amp model exactly: `LogisticsNetworkMembers` gains `channel_capacity()` / `channels_in_use()` and gates at the same point `recipe_start_system` already gates power on `amp_capacity()`/`amps_in_use()` (`networks.md §3`). Friction lives in **network topology** (a planning/design decision), not the runtime clock — Pillar-2 intact. This is the **item-flow** layer; it is complementary to, not a substitute for, machine-count scale (config-dedication, 2026-07-10) and autocraft-depth scale (CPU complexity-ceiling, below) — all three are planning/build friction, none slows a recipe.

**Alternatives considered:**
- *Per-tick items/second flow rate (Factorio belt-rate model):* Rejected — already rejected as the machine-scale lever (2026-07-10, `:59`), reintroduces the watch-and-fix/belt-throughput model the ME-network decision (GDD §10) deliberately cut, and meters transport speed (a runtime-clock friction) rather than network architecture. A content-design pass initially drafted the cable ladder in items/second; that unit is discarded in favor of channel counts.
- *Leave logistics uncapped (status quo):* Rejected — it is the divergence itself; the item-flow pillar stays frictionless and execution stays free.
- *Channel = per-item-type or per-sub-network (coarser units):* Deferred, not chosen — per-port is the faithful AE2 unit and the finest-grained forcing function for segmentation; revisit only if per-port proves too punishing in playtest.

**Implications:**
- **`networks.md §2`:** add a logistics channel-capacity model mirroring §3 power — `LogisticsNetworkMembers.channel_capacity()` / `channels_in_use()`, the per-port consumption rule, cable-tier channel ratings, and the non-destructive over-budget behavior (ports past the budget go unserved, no cable/machine damage), plus segmentation via router/interface boundaries.
- **Content:** the Channel Upgrade / Sub-network Router I / Sub-network Router II nodes (`tech-tree-design.md` Logistics category) get authored with **channel-count** ratings per cable tier (numbers TBD in a content pass), not flow rates. New cable-tier items gate on Engineering research.
- **Engine (Demo-scope, sequenced after the VS playtest gate):** `src/logistics/items.rs` shared-pool access becomes channel-gated; `recipe_start_system` (`networks.md §2`, input/output checks) gains a channel-availability check alongside the existing item checks.
- **CPU complexity-ceiling (System B, corrected):** the crafting-CPU idea survives as an autocraft **depth** cap (a plan may schedule at most N *new* craft steps; raw/stocked intermediates don't count), forcing standing intermediate stock — planning friction, Pillar-2 safe. Its "job-slots serialize the finale" half (a runtime time-sink) is dropped.
- No code yet — direction + mechanism lock. Closes obs 2962.

