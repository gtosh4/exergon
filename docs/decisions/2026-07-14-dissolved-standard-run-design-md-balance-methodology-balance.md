---
type: decision
title: "Dissolved `standard-run-design.md`; balance methodology → `balance.md`"
date: 2026-07-14
summary: "Dissolved standard-run-design.md; balance methodology → balance.md"
tags: [meta-progression]
---
# Dissolved `standard-run-design.md`; balance methodology → `balance.md`


**Decision:** The standalone Standard-run content-design doc was **deleted**. It had become a redundant, more-granular twin of `tech-tree-design.md` (which holds the holistic tier design — base/exotic ratios, unlock-rate targets, tier themes, gates) and its §5 tier tables were the main doc-vs-doc drift source. The content was dissolved as follows: **tier design** → `tech-tree-design.md` (§3 holistic, §6bis authored RON); **specific balance numbers** → the `.ron` assets + the scenario-runner baseline (never restated in prose); **general balance methodology + balance-flavored open questions** → the new hand-written `balance.md`; **byproduct→reactivity** → GDD §11 (already covered); **escape / second-site** → `technical/escape-condition.md §7.1`; **research ladder** → `technical/research.md §3`. Machine-dedication (§3.3) was already a shipped design-decision (config modules, 2026-07-10). All 13 inbound refs were repointed; the doc is git-tracked so the deletion is reversible.

**Rationale:** Docs are the source of truth, but a doc that re-states numbers and tier content already owned elsewhere is pure sync burden — it drifts silently the moment the RON or `tech-tree-design.md` changes. Removing the twin makes `tech-tree-design.md` + the `.ron` assets the single authority for tier content and quantities, and gives balance *method* its own home. Specific quantities were deliberately left to die into the assets/tests rather than migrated into prose.

**Notes:** `balance.md` added to `docs/README.md`. The current measured baseline per difficulty will live in a separate auto-generated `balance-state.md` (emitted by `scenario balance --emit`, a parallel code task — not created here). Deferred-scope flags from the old §9 (frontier-variant second config; machine-dedication config modules with no code yet) remain tracked in `escape-condition.md §7.1` and this log (2026-07-10) respectively — flagged to project-manager for any tasks.md backlog placement.

