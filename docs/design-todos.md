# Design Todos

Systems that need a `networks.md`-depth spec before implementation: ECS components, system step-by-step logic, events/messages, edge cases, execution order — enough to write integration tests without guessing. Design these only considering other docs, not the current state of the code.

---

## MVP Priority

### World Reactivity

`technical-design.md §10` has the continuous + threshold model. Needs ECS/system layer:

- Component or resource tracking per-region reactivity score (0.0–1.0) and seeded rate multiplier
- Systems that increment reactivity: which system, which sources contribute, per-tick summation
- Continuous effect application: which system reads reactivity and modifies machine efficiency — modifier component? applied each tick? event-driven?
- Threshold events: how fired-threshold state is tracked per region (bitflags? component?), which system evaluates thresholds, what event fires
- Recovery: which system decrements, at what rate relative to buildup, clamping behavior
- Reactivity spread between adjacent regions: system and rate

---

### Module System

`technical-design.md §5` mentions modules snap to attachment points and carry functional tradeoffs. Runtime effects (multiplier storage, which system reads it) are specified in `technical/crafting.md §module-effects`. Still needs:

- ECS components on module entity
- Slot attachment: snap detection system, component recording slot occupancy
- Buffer capacity: what buffer does "buffer capacity" refer to?
- Parallel processing slots: run two recipes simultaneously, or halve time?

---

## Cross-Cutting

*See `networks.md` for the target documentation depth. Each todo above should produce a section at that level before implementation begins.*
