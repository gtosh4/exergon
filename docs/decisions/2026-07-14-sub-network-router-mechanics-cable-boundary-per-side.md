---
type: decision
title: "Sub-network Router mechanics: cable-boundary + per-side channel + filtered crossings"
date: 2026-07-14
summary: "Sub-network Router mechanics: cable-boundary + per-side channel + filtered crossings"
tags: [logistics]
---
# Sub-network Router mechanics: cable-boundary + per-side channel + filtered crossings


**Decision:** A Sub-network Router is specified ([[technical/networks#Sub-network Routers|networks.md §Sub-network Routers]]) as a **non-conducting cable boundary**, not a divider on one shared net. Placing it partitions the connected cable graph into **N distinct `LogisticsNetwork` entities**, each with its own independent channel budget (`channel_capacity()` = min cable tier of *its own* segments). The router owns **one port per side**, each a member of that side's sub-net, and **each side-port consumes 1 channel** on its sub-net — so a 2-sided Router I costs 2 channels total (one per side). Cross-net item movement is **explicitly filtered**: a `RouterFilter` lists `Crossing { item_id, from, to }` entries and `router_transfer_system` relays only those items (`take_items` source → `give_items` dest, no per-tick rate cap). Router ports are priority-pinned so the over-budget shed severs leaf machine ports, never a boundary. Router II is a strict superset of I (multi-sided hubs ≥3, nested sub-nets, higher crossing cap).

**Rationale:** The 2026-07-13 rename settled the *name*; this settles the *behavior* the name implies. Making the router a real graph-partition (not a soft divider) is what gives each sub-net an **independent** budget — the entire payoff of segmentation (split a 40-port/8-channel net into two 20-port sub-nets that each fit). The **per-side channel cost** is the key friction knob: segmentation isn't free, so players cut along low-traffic seams rather than spamming routers to dodge every limit — architecture friction (Pillar-2), not a runtime throttle. **Filtered** (vs transparent-proxy) crossings make cross-net dependencies a *declared* design decision and preserve isolation; a transparent router would leak the very budget separation it exists to create. No flow-rate cap keeps this the item-flow layer's only lever = channels (GDD §10), consistent with the 2026-07-12 channel-capacity decision.

**Alternatives rejected:** *Soft divider on one net* (shared budget, router just tags a boundary) — gives no independent budget, defeats the purpose. *Transparent proxy* (router relays all `has_items`/`take_items` blindly) — leaks isolation, no planning intent, and makes cross-net routing free. *Per-tick throughput cap on crossings* — reintroduces the flow-rate model already rejected twice (2026-07-10, 2026-07-12); meters the clock, not the topology.

**Notes:** Docs-only, unimplemented — depends on the channel-capacity engine work (2026-07-12), same Demo-scope sequencing after the VS playtest gate. Crossing caps (N) and per-tier channel ratings are TBD content. New components `SubnetRouter` / `RouterFilter` and system `router_transfer_system` are design placeholders, not yet in `src/`.

