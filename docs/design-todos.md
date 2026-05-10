# Design Todos

Systems that need a `networks.md`-depth spec before implementation: ECS components, system step-by-step logic, events/messages, edge cases, execution order — enough to write integration tests without guessing. Design these only considering other docs, not the current state of the code.

---

## Vertical Slice Priority

### Drone System

`technical-design.md §8` has control model intent. Needs ECS/system spec:

- Components on drone entity beyond the named fields (`type, pos, orientation, inventory, state`)
- Local↔Remote mode transition: which system handles camera/control transfer, what components change on the character entity vs. drone entity
- Fog-of-war reveal: component or resource, which system reveals on drone movement, data structure
- Sample collection: trigger, range check, what item is produced and where it lands
- Range scanning: which system, what radius, what data is exposed vs. withheld
- Multiple drone switching: selection mechanic, which system activates/deactivates

---

### Habitat System

Referenced in `gdd.md §10` and `milestones.md`. No mechanics doc exists. Needs:

- ECS structure: what component defines habitat zone (center + radius? AABB? voxel set?)
- System that checks player/drone position against habitat boundary
- Consequence of being outside habitat (blocked movement? atmospheric damage timer? something else?)
- Habitat expansion: how upgrading the generator entity changes the zone
- MVP: Outpost beacon full spec — power link requirement, body fabrication items and cost, switching mechanic, power-interruption collapse behavior

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

`technical-design.md §5` mentions modules snap to attachment points and carry functional tradeoffs. Needs:

- ECS components on module entity
- How module effects apply during recipe execution: where multiplier is stored, which system reads it
- Slot attachment: snap detection system, component recording slot occupancy
- Concrete tradeoff definitions: speed vs. efficiency (formula), parallel processing slots (how do parallel slots change recipe execution — run two recipes simultaneously? halve time?), buffer capacity (what buffer?)

---

### Catalyst Inputs (Recipe System Extension)

General recipe concept needed for gateway activation (key present for job duration, not consumed). Needs spec:

- Data model: `catalyst_inputs: Vec<(ItemId, u32)>` on `RecipeDef` — default empty, no effect on existing recipes
- Reservation semantics: how items are marked reserved in logistics network (flagged unavailable for other jobs but still present in inventory)
- Release on job complete or cancel: items remain in network, reservation clears
- Multi-job contention: two jobs requiring the same catalyst each need independent copies in the network
- Asset format: `catalyst_inputs` list in concrete and template recipe assets; generation algorithm passes field through unchanged (not a seeded variance axis)

---

### Auto-crafting Job Dispatch

`technical-design.md §6` has design intent. Needs ECS/system spec:

- `CraftingPlan` and `CraftingJob` entity components: all fields, status enum, prerequisite edge storage
- Job dispatcher system: what triggers it (machine idle event?), scan algorithm, assignment logic
- Machine capability auto-registration: trigger, which system runs, what component stores capability set
- Priority + filter configuration: component structure, how dispatcher reads and applies it
- Channel limit exceeded: what system detects, what event fires, how surfaced to player

---

## Cross-Cutting

### Recipe Graph Runtime Integration

`technical-design.md §2` specifies the data model. Missing runtime layer:

- How the generated graph is stored at runtime (resource? asset?)
- How `recipe_start_system` looks up matching recipes (by machine type + tier — index structure?)
- How tech tree unlock status gates recipe availability (`TechTreeProgress.unlocked_recipes` is referenced in `networks.md` but not specified)
- Recipe lookup performance: indexed by machine type, producing item, consuming item — where these indexes live

---

*See `networks.md` for the target documentation depth. Each todo above should produce a section at that level before implementation begins.*
