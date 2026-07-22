---
type: decision
title: "Logistics segmentation nodes renamed to 'Sub-network Router I/II'"
date: 2026-07-13
summary: "Logistics segmentation nodes renamed to 'Sub-network Router I/II'"
tags: [logistics]
---
# Logistics segmentation nodes renamed to "Sub-network Router I/II"


**Decision:** The two logistics segmentation tech nodes are renamed: "Logistics Controller" → **Sub-network Router I** (T3, first segmentation node) and "Sub-network Router" → **Sub-network Router II** (T4, deeper segmentation, strict behavioral superset of I). The deeper node being a superset, this uses the internal tier convention (superset behavior → I/II naming); display names may be re-flavored later.

**Rationale:** "Controller" carries the wrong connotation for players coming from AE2 (modded Minecraft), where a *controller* is what *emits* channels. In Exergon's design these nodes do **not** emit channels — channels come from cable tier — the node is a **segmentation boundary** (the IRL network-router concept: it partitions a network into sub-nets, each with its own channel budget). "Router" names that behavior correctly and avoids the false AE2 mental model. Prose describing the boundary as "controller/interface" is updated to "router/interface" for the same reason ([[technical/networks|networks.md §5]], the 2026-07-12 channel-capacity entry).

**Notes:** Docs-only rename — these nodes are unauthored/unimplemented; no RON tech asset or `src/` symbol carries these names (confirmed). Unrelated "controller"s (the crafted Engineering `controller` assembly item, drone/body `Controller` code, power-network) are untouched.

