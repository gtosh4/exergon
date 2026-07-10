# Recipe Graph

Definition and data model of the recipe graph: materials, form groups, items, machines, wildcard recipes, concrete recipes, and the runtime `RecipeGraph` resource. Read `gdd.md §8` for design intent.

**Scope.** This document covers the graph's **definition and data model** only. Runtime execution (recipe start, progress, completion, dispatch, catalyst reservation) is specified in [`crafting.md`](crafting.md). Tech-tree gating of recipe availability is in [`tech-tree-design.md`](../tech-tree-design.md). Procedural generation of the graph from the `recipes` domain seed is deferred to **post-VS** (see [`README.md`](README.md) Post-VS section).

**VS scope.** Data model and runtime types only. Recipe content is curated assets (`assets/materials/`, `assets/form_groups/`, `assets/recipes/`, `assets/items/`). Procedural variance, parameter bounds, and a graph generator are **post-VS**.

**Read before:** modifying `src/recipe_graph/`, adding a material/form group/recipe/item asset, or touching `RecipeGraph` consumers (`src/crafting/`, `src/logistics/`, planner UI).

---

## Table of Contents

1. [Overview](#1-overview)
2. [Vocabulary: Materials, Form Groups, Items, Machines, Recipes](#2-vocabulary-materials-form-groups-items-machines-recipes)
3. [Item Kinds](#3-item-kinds)
4. [Machines as Items](#4-machines-as-items)
5. [Wildcard Recipes & Expansion](#5-wildcard-recipes--expansion)
6. [ConcreteRecipe Data Model](#6-concreterecipe-data-model)
7. [RecipeGraph Resource](#7-recipegraph-resource)
8. [Graph Structure, Cycles, and Reachability](#8-graph-structure-cycles-and-reachability)
9. [Outputs Routing](#9-outputs-routing)
10. [Validity Invariants](#10-validity-invariants)
11. [Seed Integration](#11-seed-integration)
12. [VS / MVP Scope](#12-vs--mvp-scope)
13. [Integration Test Invariants](#13-integration-test-invariants)

---

## 1. Overview

The recipe graph is a **directed acyclic graph (DAG) with three node kinds**:

- **Item nodes** — every concrete item (derived, composite, unique, machine).
- **Recipe nodes** — every concrete recipe; an "edge bundle" carrying inputs, outputs, machine, tier, time, energy.
- **Machine nodes** — items with `ItemKind::Machine`; they are simultaneously item nodes (they have producer recipes) *and* are referenced by recipes as the machine that runs them.

Edges (direction = dependency: source must exist before sink can fire):

- `item → recipe` if the recipe consumes that item (input edge).
- `item → recipe` if the recipe requires that item as its machine (machine edge). Machine is a dependency of the recipe, same as a regular input — the recipe cannot run without it.
- `recipe → item` if the recipe produces that item (output edge).

A recipe node fans in from all its inputs **and** its machine, and fans out to all its outputs. **Multiple recipes may produce the same item** — that item simply has multiple incoming recipe-nodes (alternative producers).

A machine must itself be **craftable**: it is an item like any other, and the graph must contain a recipe chain that produces every machine referenced by any recipe in the run. This is enforced by an invariant (see §10).

The graph is **immutable for the duration of a run** — built once at run start from content assets (VS) or from `DomainSeeds.recipes` (post-VS), inserted as the `RecipeGraph` resource, and then read-only.

Two layers exist:

- **Content / definition layer.** Materials, form groups, wildcard recipes, concrete recipes, items. All loaded from `assets/` or generated procedurally.
- **Runtime layer.** The `RecipeGraph` resource and its lookup indexes (`by_output`, `by_input`, `by_machine`). Built once from the content layer.

The recipe graph does **not** know about machine entities (placed in world), catalyst reservations, plans, jobs, or networks. Those are runtime concerns in `crafting.md`. The graph only knows: *which recipes exist this run, what they require, what they produce, on what machine item, at what tier.*

---

## 2. Vocabulary: Materials, Form Groups, Items, Machines, Recipes

### Materials

Abstract substance identities. A material is not a recipe node; it is the identity items inherit. Asset format (`assets/materials/<id>.ron`):

```rust
pub enum MaterialKind { Base, Exotic }

pub struct MaterialDef {
    pub id:          MaterialId,        // e.g. "copper", "tin_oxide", "resonite"
    pub name:        String,            // display name
    pub kind:        MaterialKind,      // Base = real-world-inspired; Exotic = seeded per run
    pub form_groups: HashSet<FormGroupId>,  // e.g. {"metal"} or {"exotic", "combustible"}
}
```

- **Base** materials are consistent across runs and form the early-tier vocabulary.
- **Exotic** materials are unique per run (post-VS: drawn from the `recipes` domain seed; VS: curated). They populate the final tier and the run's goal items.
- A material may belong to multiple form groups; it gets the **union** of their forms.
- Material IDs are lowercase snake_case (e.g. `tin_oxide`). Underscores stand in for spaces within a multi-word material name.

### Form Groups

Content-defined sets of physical states. Asset format (`assets/form_groups/<id>.ron`):

```rust
pub struct FormGroup {
    pub id:    FormGroupId,        // e.g. "metal", "exotic", "combustible"
    pub forms: HashSet<FormId>,    // e.g. {"ore", "ingot", "wire"}
}
```

The `forms` set is **unordered**. No semantics depend on iteration order — UI ordering of forms uses the recipe graph's topological order when display order matters; procedural generation (post-VS) does not index into form groups by position.

A form group exists independently of any material. Materials reference groups by `FormGroupId`.

### Items

Items are the **node set** of the recipe graph (alongside recipe nodes). Four kinds (see §3). Items are identified by `ItemId` (`String`).

**Derived item IDs use `:` as the material/form separator**: `{material}:{form}` — e.g. `copper:ingot`, `tin_oxide:wire`. The `:` cleanly separates the material from the form even when materials use `_` for multi-word names. Composite, unique, and machine item IDs are content-defined free-form strings (conventionally lowercase snake_case).

### Machines

Machines are items with `ItemKind::Machine` (see §4). A machine item has a `machine_type` (e.g. `assembler`, `smelter`) and a `tier` (e.g. `1`, `2`, `3`). Different tiers of the same type are **different items** (`assembler_mk1`, `assembler_mk2`, …) with their own producer recipes.

### Recipes

Recipes are **nodes** in the graph (not edges). Two flavors at the asset level:

- **Wildcard recipes** — recipes that use `$var:form` placeholders to bind across materials (see §5). Expanded at build time into concrete recipes.
- **Concrete recipes** — fully expanded recipe instances, the only form the runtime sees (see §6).

Wildcard recipes exist to author one rule that applies to many materials. Concrete recipes exist as authored assets when no wildcard fits (composites, uniques, machine recipes, special cases).

---

## 3. Item Kinds

```rust
pub enum ItemKind {
    Derived   { material: MaterialId, form: FormId },
    Composite { family: Option<RecipeId> },
    Unique,
    Machine   { machine_type: MachineTypeId, tier: u8 },
}

pub struct ItemDef {
    pub id:        ItemId,
    pub name:      String,
    pub kind:      ItemKind,
    pub tags:      HashSet<ItemTag>,   // union of author-declared and auto-derived (see "Tags" below)
}
```

### Derived

A `(material, form)` pair. **Generated automatically** at graph construction time from material × form-group membership — *no asset file required*. ID: `{material}:{form}`. Name: `{Material} {Form}` (capitalized). Derived items exist whenever the material is present in the run.

> A material gaining a form group at construction time automatically gains all derived items for its forms — and via wildcard-recipe expansion, all recipes that use those forms.

### Composite

An item defined by a content asset (`assets/items/<id>.ron`) that is the output of one or more recipes (wildcard or concrete). The optional `family: Option<RecipeId>` field marks composite items that participate in a wildcard-recipe family (e.g. `[material]:cable` for each material with a wire form) — used by the planner UI to group alternatives.

### Unique

A one-off asset-defined item with no material-form derivation and no wildcard family. Goal items (`gateway_key`, `power_cell`-analogs, narrative artifacts) are typically Unique. Decorative placeables that are not machines can also be Unique.

### Machine

An item that, when placed in the world, runs recipes referencing its `machine_type` at compatible tier. See §4. Machine items have their own producer recipes (an "assembler_mk1" recipe whose output is the `assembler_mk1` item).

### Tags

`tags: HashSet<ItemTag>` is the flexible mechanism for marking items with semantic roles. The set is **a union of author-declared tags and auto-derived tags**:

**Auto-derived tags** are inserted by the graph builder from `ItemKind`:

| Source | Tags emitted |
|---|---|
| `ItemKind::Derived { material, form }` | `kind:derived`, `material:{material}`, `form:{form}` |
| `ItemKind::Composite { family }` | `kind:composite`, `family:{family}` if `Some` |
| `ItemKind::Unique` | `kind:unique` |
| `ItemKind::Machine { machine_type, tier }` | `kind:machine`, `machine_type:{machine_type}`, `tier:{tier}` |

These let consumers (planner UI, wildcards, victory predicates, codex) query the graph by structural properties without re-deriving them from `ItemKind`.

**Author-declared tags** come from asset files (e.g. `["goal", "consumable", "catalyst_provided_by_world"]`). The set of valid author tags is content-defined and not enumerated by `RecipeGraph` — consumers interpret tags they care about.

Tags namespaced with `:` are reserved for auto-derived (e.g. `material:copper`); author tags should avoid the `:` separator to prevent collision.

Tag-based wildcards (e.g. matching items by `tag:metal` rather than by form group) are a candidate future extension; the dual-source tag model is designed to support this.

---

## 4. Machines as Items

Machines are first-class items in the graph:

- A machine has `ItemKind::Machine { machine_type, tier }`.
- A machine item has one or more **producer recipes** that craft it from materials/components.
- A recipe references a machine by `machine: ItemId` (the machine item that runs it).

This means the same DAG simultaneously encodes:
- "What does this recipe consume and produce?"
- "What machine is needed to run this recipe?"
- "What recipe produces that machine?" (recursively)

The graph builder verifies that every machine item referenced by any recipe is itself producible — i.e. it appears as an output of at least one recipe (or is provided by world placement / starter inventory, see invariant #14 in §10).

### Tier semantics

A tier-N machine of a given `machine_type` can run any recipe with `machine_type` matching and `machine_tier ≤ N`. Tier ordering is encoded on the machine item's `ItemKind::Machine.tier` field; the dispatcher reads this metadata.

A higher-tier machine is **not automatically a refinement** of a lower-tier item — `assembler_mk2` is its own item with its own recipe. Players upgrade by crafting the next-tier machine, not by transforming the existing one in place. (Whether upgrade-in-place is a future mechanic is out of scope here.)

### Why machines as items (not a separate node kind)

Folding machines into the item type unifies storage, logistics, and crafting:

- Machines flow through logistics like any other item.
- "Build the assembler" and "build a circuit board" are the same kind of operation: craft an item.
- The DAG's reachability check naturally covers "can the player obtain every machine they need?" — no separate machine-reachability pass required.

---

## 5. Wildcard Recipes & Expansion

A **wildcard recipe** is a recipe asset that uses `$var:form` placeholders to bind across materials. The graph builder expands each wildcard recipe across all material assignments that satisfy the placeholder constraints, producing one concrete recipe per binding.

```rust
pub struct WildcardItem {
    pub item:     WildcardItemRef,   // either a concrete ItemId or a "$var:form" pattern
    pub quantity: u32,
    pub consumed: bool,              // inputs only; outputs always true
}

pub enum WildcardItemRef {
    Concrete(ItemId),                // matches exactly this item
    Pattern { var: String, form: FormId },  // matches "$var:form" where $var binds to a material
}

pub struct WildcardRecipe {
    pub id:               RecipeId,                  // base id; expanded as "{id}:{$a=copper}:{$b=tin}"
    pub inputs:           Vec<WildcardItem>,
    pub outputs:          Vec<WildcardItem>,         // consumed ignored on outputs
    pub machine:          ItemId,                    // machine item this recipe runs on
    pub machine_tier:     u8,
    pub min_voltage_tier: u8,
    pub processing_time:  f32,                       // seconds
    pub energy_cost:      f32,                       // joules
    pub var_groups:       HashMap<String, FormGroupId>,  // each $var must belong to this form group
}
```

### Pattern semantics

- A pattern entry `$a:wire` matches any item of the form `wire` whose material has been bound to `$a`.
- Variables `$a`, `$b`, … are independent unless constrained.
- The same variable across multiple entries **must bind to the same material**: in `$a:wire + $b:screw -> $a:cable + $b:dust`, every `$a` resolves to one material and every `$b` to another (possibly the same, possibly different).
- `var_groups[$a] = "metal"` constrains `$a` to materials whose `form_groups` includes `"metal"` AND whose `form_groups`' union of forms contains every form referenced by `$a` in this recipe.

### Expansion rule

For each `WildcardRecipe w`:

1. Collect the set of variables `V = {$a, $b, …}` referenced in `inputs ∪ outputs`.
2. For each `$v ∈ V`, compute the candidate material set: materials whose `form_groups` includes `var_groups[$v]` AND which have all forms `$v` references in `w`.
3. Enumerate the Cartesian product of candidate sets across all variables in `V`.
4. For each binding assignment, emit a `ConcreteRecipe`:
   - `id = "{w.id}:{$a=<material>}:{$b=<material>}:…"` (variables listed in lexicographic order of variable name).
   - Each `WildcardItem` is lifted: `Pattern { var, form } → ItemId = "{materials[var].id}:{form}"`; `Concrete(id) → id` unchanged.
   - All other fields (`machine`, `machine_tier`, `min_voltage_tier`, `processing_time`, `energy_cost`) copy unchanged.
5. Skip an expansion (warn) if any resolved item ID does not resolve to an existing `ItemDef` (e.g. the material lacks the form derivation).

### Why wildcard recipes

Wildcard recipes look like normal recipes with placeholders — there is **no separate "template" abstraction**. Authoring `draw_wire` once with `inputs: [{$a:ingot, 1}], outputs: [{$a:wire, 1}], var_groups: {$a: "metal"}` instantly produces a concrete recipe for every metal in the run. Multi-variable patterns enable alloy-style and compound recipes (`$a:wire + $b:screw → $a:cable + $b:dust`) without per-pair authoring.

### Wildcards and machine items

A wildcard recipe's `machine: ItemId` is **always concrete** (no wildcards on machines). If a recipe family needs to vary the machine across bindings, author each as a separate recipe asset.

---

## 6. ConcreteRecipe Data Model

`ConcreteRecipe` is the only recipe representation the runtime sees. Wildcard recipes are expanded into concrete recipes at graph-build time and retained on the resource only for planner-UI grouping.

```rust
pub struct RecipeInput {
    pub item:     ItemId,
    pub quantity: u32,
    pub consumed: bool,  // true = pulled and destroyed; false = catalyst (held, not pulled)
}

pub enum RecipeOutput {
    /// Item byproduct or primary item output. Routes through output-eligible logistics ports.
    Item   { item: ItemId, quantity: u32, chance: f32 },
    /// Energy injected into the host machine's `GeneratorUnit.buffer_joules` at completion.
    /// Only valid on machines that carry `GeneratorUnit` (see `power.md`).
    Energy { joules: f32, chance: f32 },
}

pub struct ConcreteRecipe {
    pub id:               RecipeId,
    pub inputs:           Vec<RecipeInput>,    // consumed:true regular; consumed:false catalyst
    pub outputs:          Vec<RecipeOutput>,   // all outputs unified — primary + byproducts share this list
    pub machine:          ItemId,              // machine item that runs this recipe (must be ItemKind::Machine)
    pub machine_tier:     u8,                  // minimum machine tier that can run this recipe
    pub min_voltage_tier: u8,                  // minimum network voltage tier; independent of machine_tier
    pub processing_time:  f32,                 // seconds at base speed
    pub energy_cost:      f32,                 // joules per completion
    pub tags:             HashSet<RecipeTag>,  // free-form recipe tags; "recycle" is a reserved tag (see §8)
}
```

### Quantity type

All item quantities (`RecipeInput.quantity`, `RecipeOutput::Item.quantity`) are `u32` — strictly integer. Fractional quantities are disallowed. A recipe cannot consume "0.5 of copper:ingot"; if a real-world fractional cost is intended, scale both inputs and outputs (e.g. one recipe consuming 1 ingot and producing 2 wires).

`RecipeOutput::Energy.joules`, `processing_time`, `energy_cost`, and output `chance` remain `f32` (they are continuous, not item counts).

### Probabilistic outputs

`chance ∈ [0.0, 1.0]` is the probability that the output is emitted on a given completion (field present on both `RecipeOutput::Item` and `RecipeOutput::Energy`). `chance = 1.0` (the common case, including all primary outputs) means the output is always produced. Lower values express probabilistic byproducts (e.g. ore processing yielding trace amounts of a secondary metal `20%` of runs) or stochastic energy yield (e.g. rare exotic generator doubling output `10%` of completions).

- At runtime, on completion, each output's emission is rolled against `chance` using a recipe-execution RNG seeded from `(plan_id, completion_index)` (see `crafting.md §5`, deterministic save-friendly).
- An output that fails its roll is **not produced** — no item flows to logistics, no routing constraint applies to it for that completion.
- Routing constraints (`crafting.md §5` step 4) still apply pre-roll: if a recipe *could* produce an output, the machine must have an output-eligible port for it (a probabilistic byproduct must be routable, even if the specific completion doesn't roll it).

### Chance modifiability

`chance` is a base value on the concrete recipe. Per-run modifiers from tech-tree nodes or world properties (e.g. a "quality" tech node that improves byproduct yields) may **multiplicatively** adjust the rolled chance at completion time. The modifier source is a separate system (post-VS); the recipe graph only declares the base. Effective chance is clamped to `[0.0, 1.0]`.

For VS: all outputs have `chance = 1.0` (no probabilistic outputs in the curated set). The field is present for the post-VS modifier system; tests assert VS recipes set it to `1.0`.

### Field semantics

| Field | Authority | Notes |
|---|---|---|
| `id` | Asset author (or `"{wildcard}:{$a=…}:…"` for expansions) | Globally unique across the run's recipe set. Treated as opaque string. |
| `inputs` | Asset or wildcard expansion | `consumed: false` entries are **catalysts**: reserved on the network, not pulled (see `crafting.md §6`). |
| `outputs` | Asset or wildcard expansion | A **single unified list** of `RecipeOutput` (`Item` or `Energy` variants). Primary outputs and byproducts share the same shape and runtime path. The first entry is conventionally the primary output (used by planner UI). For recipes whose host machine has a `GeneratorUnit`, `outputs[0]` **must** be `RecipeOutput::Energy` — see §9 and `power.md §7`. |
| `machine` | Asset or wildcard | An `ItemId` that must resolve to an `ItemDef` with `ItemKind::Machine`. |
| `machine_tier` | Asset; wildcards copy from asset | Minimum machine tier that can run this recipe. A tier-`N` machine of the matching `machine_type` runs recipes with `machine_tier ≤ N`. |
| `min_voltage_tier` | Asset or recipe-graph builder | **Independent of `machine_tier`.** Authored on the recipe asset; defaults to `machine_tier` if absent. Checked at `recipe_start_system` step 2a (see `crafting.md §5`). |
| `processing_time` | Asset or wildcard | Seconds at base speed. `effective_processing_time = processing_time * speed_multiplier` (see `crafting.md §7`). |
| `energy_cost` | Asset or wildcard | Joules per recipe completion. `draw_rate = (energy_cost / processing_time) * efficiency_multiplier`. |

### Default for min_voltage_tier

If a concrete-recipe asset omits `min_voltage_tier`, the graph builder sets `min_voltage_tier = machine_tier`. Wildcard expansions copy what the wildcard asset specified (which may be left to the same default). Asset authors override `min_voltage_tier` only when it must diverge from `machine_tier`.

### Catalyst variance

`consumed: false` entries are **not subject to parameter variance** (post-VS). They are passed through unchanged from the recipe asset. Variance applies to `consumed: true` input quantities, output quantities (item `quantity` and energy `joules` alike), processing time, and energy cost — bounded per `gdd.md §8` (post-VS). Item quantity variance is integer-rounded (any fractional intermediate is rounded toward the nearest integer with a minimum of 1); energy `joules` variance is continuous (`f32`).

---

## 7. RecipeGraph Resource

Inserted at run start (Startup schedule); never mutated during a run.

```rust
#[derive(Resource, Clone, Debug)]
pub struct RecipeGraph {
    pub materials:   HashMap<MaterialId, MaterialDef>,
    pub form_groups: HashMap<FormGroupId, FormGroup>,
    pub wildcards:   HashMap<RecipeId, WildcardRecipe>,
    pub items:       HashMap<ItemId, ItemDef>,
    pub recipes:     HashMap<RecipeId, ConcreteRecipe>,

    // Lookup indexes — all derived from `recipes` at build time. Stored as HashSet because
    // iteration order is not semantically meaningful; consumers that need stable order must sort.
    pub by_output:        HashMap<ItemId, HashSet<RecipeId>>,    // item -> recipes that produce it
    pub by_input:         HashMap<ItemId, HashSet<RecipeId>>,    // item -> recipes that consume it
    pub by_machine:       HashMap<ItemId, HashSet<RecipeId>>,    // machine item -> recipes it runs
    pub by_energy_output: HashSet<RecipeId>,                     // recipes whose outputs include any RecipeOutput::Energy
}
```

Goal items are **not** tracked in `RecipeGraph` — they belong to a separate resource owned by the victory / objective system, outside this document's scope.

### Index construction

After `recipes` is populated (from concrete-recipe assets ∪ wildcard expansions):

```text
for r in recipes.values():
    for out in r.outputs:
        match out:
            RecipeOutput::Item { item, .. }  => by_output[item].insert(r.id)
            RecipeOutput::Energy { .. }      => by_energy_output.insert(r.id)
    for input in r.inputs:     by_input[input.item].insert(r.id)
    by_machine[r.machine].insert(r.id)
```

`by_machine` keys on `ItemId` of the machine item. The dispatcher resolves "what recipes can this placed machine run?" by looking up `by_machine[machine_item_id]` and filtering by `machine_tier ≤ machine_item.tier`.

`by_energy_output` is the canonical "what produces power?" index — the tech-tree UI, planner power picker, and codex query against this set. Energy outputs are not keyed by item (energy is not an item; virtual items like `sunlight_tick` are inputs, not outputs).

### Lookup order

`HashSet<RecipeId>` makes the lack of ordering explicit at the type level. Any system that needs a stable ordering of producers / consumers (e.g. for "first alternative" tie-breaking in the planner) must sort by `RecipeId` after fetch.

### Query methods

`RecipeGraph` exposes reachability and lookup helpers so consumers (victory validator, tech-tree generator, planner, debug overlays) share a single implementation. Sketch:

```rust
impl RecipeGraph {
    /// Recipes that produce `item`.
    pub fn producers(&self, item: &ItemId) -> &HashSet<RecipeId>;

    /// Recipes that consume `item`.
    pub fn consumers(&self, item: &ItemId) -> &HashSet<RecipeId>;

    /// Recipes that run on `machine` (a machine ItemId).
    pub fn recipes_for_machine(&self, machine: &ItemId) -> &HashSet<RecipeId>;

    /// Recipes that produce energy as an output (any `RecipeOutput::Energy`).
    pub fn energy_producers(&self) -> &HashSet<RecipeId>;

    /// Items reachable from a starting inventory + a set of available recipes.
    /// A recipe contributes outputs iff all of its inputs (consumed and catalyst) and its machine
    /// are already reachable. Fixed-point BFS to convergence.
    ///
    /// `ignore_tags` filters out recipes carrying any of these tags. Defaults to `{"recycle"}`
    /// when called via `reachable_from_default`.
    pub fn reachable_items(
        &self,
        starter_items: &HashSet<ItemId>,
        available_recipes: &HashSet<RecipeId>,
        ignore_tags: &HashSet<RecipeTag>,
    ) -> HashSet<ItemId>;

    /// Convenience wrapper: reachable items using all recipes in the graph, ignoring `recycle`.
    pub fn reachable_from_default(&self, starter_items: &HashSet<ItemId>) -> HashSet<ItemId>;

    /// True iff `target` ∈ reachable_items(starter_items, available_recipes, ignore_tags).
    pub fn can_reach(
        &self,
        target: &ItemId,
        starter_items: &HashSet<ItemId>,
        available_recipes: &HashSet<RecipeId>,
        ignore_tags: &HashSet<RecipeTag>,
    ) -> bool;
}
```

These methods are the canonical reachability surface — the validator, tech-tree generator, and integration tests must call them rather than re-implementing traversal. The signature accepts an explicit `available_recipes` set so callers can ask reachability questions under partial tech-tree unlocks (e.g. "is the goal still reachable if we cut this node?").

### Reading the resource

All read-only. Systems that need the graph add `Res<RecipeGraph>`. No mutation paths exist after `Startup`.

### Implementation: petgraph

The internal graph representation uses [`petgraph`](https://docs.rs/petgraph) — Rust's standard graph crate. Rationale:

- Provides `toposort`, `is_cyclic_directed`, `tarjan_scc`, BFS/DFS visitors, and `has_path_connecting` out of the box — covers every algorithm the query API needs.
- `EdgeFiltered` / `NodeFiltered` adapters give zero-copy filtered views, so "non-recycle subgraph" queries (default reachability, DAG invariant check) reuse the same graph without rebuilding.
- Mature, widely depended on; already pulled in transitively via `bevy_ecs`'s schedule graph.

**Not** `daggy`: daggy enforces acyclicity at edge insertion, which conflicts with recycle recipes that *intentionally* close cycles. Filtering at query time via `EdgeFiltered` is the cleaner pattern for our two-mode (DAG / cyclic) graph.

Suggested internal type:

```rust
use petgraph::graph::{DiGraph, NodeIndex};

pub enum GraphNode {
    Item(ItemId),
    Recipe(RecipeId),
}

pub enum GraphEdge {
    Input { consumed: bool, quantity: u32 },   // item -> recipe
    Machine,                                   // item -> recipe (machine item required to run)
    Output { quantity: u32, chance: f32 },     // recipe -> item  (item outputs only)
}

// Stored alongside the HashMaps as an internal index — not part of the public API.
struct RecipeGraphIndex {
    graph:        DiGraph<GraphNode, GraphEdge>,
    item_nodes:   HashMap<ItemId, NodeIndex>,
    recipe_nodes: HashMap<RecipeId, NodeIndex>,
}
```

Query methods (`reachable_items`, `can_reach`, …) implement traversal on `graph` using `EdgeFiltered` to skip recycle-tagged recipe nodes. The `HashMap`/`HashSet` lookup indexes (§7 "Index construction") are still maintained alongside the petgraph index for O(1) lookup by id — petgraph is used only for traversal algorithms, not as the canonical store.

**Energy outputs are not graph edges.** Energy is not a `GraphNode::Item`; reachability and DAG analyses operate over the item-level graph only. Recipes producing energy still appear as `Recipe` nodes (with their item inputs/outputs intact) — they are simply edge-less on the energy side. The `by_energy_output: HashSet<RecipeId>` index is the canonical "what produces power?" surface; consumers iterate it directly.

### Save integration

`RecipeGraph` is derived from content assets (VS) or from `DomainSeeds.recipes` + asset pools (post-VS). Save files must be validated against the loaded asset state to ensure the graph the save was authored against still exists. Concretely: a save records the content hash (or asset manifest digest) of the graph; on load, the rebuilt graph must hash to the same value. Mismatches are a load error (game cannot safely resume against a divergent graph).

This consistency check is owned by the save system. See [`save.md`](save.md) — *to be written* — for the schema and load-time validation. The recipe graph itself only commits to being deterministically reproducible from `(content assets, DomainSeeds.recipes)`.

---

## 8. Graph Structure, Cycles, and Reachability

### Reachability is a graph operation

The recipe graph provides reachability as a query (see §7 query methods). Goal definitions, victory conditions, and "is this run solvable?" predicates live in **other systems** that call into `RecipeGraph::reachable_items` / `can_reach`. This document does not enumerate goals.

A common reachability question: *"given the player's starter inventory and the recipes unlocked by the current tech-tree state, which items can the player obtain?"* — answered by `RecipeGraph::reachable_items(starter, unlocked_recipes, ignore_tags)`.

### Cycles: recycle recipes

The recipe graph is **not strictly acyclic in general**. A common pattern is a recycle recipe that converts a downstream item back to an upstream form (e.g. `cable → ingot`). Without further constraints, this creates a cycle: `ingot → wire → cable → ingot`.

The graph handles this by **tagging recycle recipes** at authoring time:

- A recipe asset that closes a cycle is tagged `tags: ["recycle"]` (or another author-chosen tag conveying "this recipe is not part of a forward production chain").
- Reachability methods accept `ignore_tags: &HashSet<RecipeTag>`. The default helpers (`reachable_from_default`) pass `{"recycle"}` to exclude these edges, so cycle-closing recipes are skipped for "what can the player make?" queries.
- Recycle recipes still **exist** in `recipes` and still **execute at runtime** — the runtime ignores tags. Tags only affect static reachability/validity analyses.

This makes the cycle problem explicit: content authors declare which recipes are recycle (cycle-closing) edges, and analyses get clean acyclicity over the non-recycle subgraph.

**Alternative considered:** ignoring cycles silently in BFS (visited-set termination). Rejected because: (a) it hides the data-modeling fact that some recipes are structurally different; (b) it gives the planner UI no way to distinguish "primary production chain" from "recycle return path"; (c) it doesn't let post-VS generators reason about the forward graph independently.

### Invariant: DAG over non-recycle subgraph

The graph must be acyclic when all recipes tagged with any "cycle-closing" tag (default: `recycle`) are removed. Cycles within the recycle-tagged subgraph are permitted but flagged as authoring suspicion.

### Cross-tier reuse

A recipe authored for tier-1 use (e.g. `smelt_metal:copper`, machine = `smelter_mk1`) remains valid at higher tiers — a `smelter_mk3` (tier 3) runs it too (since `machine_tier: 1 ≤ 3`). The `RecipeGraph` does not partition recipes by tier; the tech tree's `unlocked_recipes` set is the source of truth for *current* availability.

### Verification

Validity is **expressed as integration test invariants** (§10, §13), not enforced by a runtime validator. There is no startup panic / warn for an invalid graph in VS — invalid curated content is caught by tests at CI time. Post-VS, the generator is responsible for emitting valid output; failed reachability checks are a test-time failure of the generator, not a runtime concern.

---

## 9. Outputs Routing

A "byproduct" is **not a distinct data concept**. All recipe outputs live in the single `outputs: Vec<RecipeOutput>` list and follow identical runtime routing rules (see `crafting.md §5` step "produce outputs"): every output rolled-emitted is dispatched by variant — `Item` to the machine's output-eligible logistics ports via `PortPolicy`, `Energy` to the machine's `GeneratorUnit.buffer_joules` (see `power.md §7`).

### Convention

By **convention** the first entry in `outputs` is the primary output — the item the recipe is named for, the headline output shown in the planner UI's recipe row, and the output that `RecipeGraph.by_output` lookups target first when resolving "what produces item X?". Entries beyond the first are byproducts in the colloquial sense.

For non-generator recipes this is convention, not invariant. For wildcard expansions: `outputs[0]` follows from the wildcard asset's first declared output.

**Generator-recipe invariant.** Recipes whose host machine carries a `GeneratorUnit` (i.e. `machine` resolves to a `MachineDef` with a `GeneratorDef`) **must** have `RecipeOutput::Energy` as `outputs[0]`. The primary function of a generator is generating power; the planner UI, codex, and tech-tree presentations all key off this convention to surface the energy yield as the headline number. Byproducts (ash, depleted_rod, etc.) follow in subsequent entries. Asserted by invariant #16 (§10).

### Output routing constraints

A recipe with multiple outputs requires the host machine to have at least one output-eligible logistics port reachable for **each** output item, or the recipe blocks with `RecipeBlockedOutputs` at start (per `crafting.md §5` step 4). The runtime does not route some outputs and drop others — all or none.

This means: a recipe that produces an unwanted byproduct cannot be "filtered" by omitting destination ports; the machine must accept all outputs or not run the recipe. Players solve this by adding a sink (storage, void module — post-VS) for the unwanted output.

### Parameter variance (post-VS)

Output quantities (both primary and byproduct) are subject to parameter variance per `gdd.md §8` (60%–150% of base), integer-rounded for `RecipeOutput::Item.quantity`; continuous (`f32`) for `RecipeOutput::Energy.joules`. An output's `chance` may also vary across runs (e.g. seeded byproduct yields). Tech-tree "quality" nodes can further apply multiplicative `chance` modifiers at runtime (see §6). This authoring is curated for VS and generator-controlled post-VS.

---

## 10. Validity Invariants

The graph must satisfy these invariants. **VS:** asserted by integration tests over curated content. **Post-VS:** asserted by the recipe-graph generator (its responsibility to produce valid output) and by integration tests over generated content.

| # | Invariant |
|---|---|
| 1 | **DAG over non-recycle subgraph.** When recipes tagged `recycle` are removed, the directed graph over (item-nodes + recipe-nodes) with edges `item → recipe` (input or machine) and `recipe → item` (output) is acyclic. Cycles within the recycle-tagged subgraph are allowed. Machine self-bootstrapping (a tier-N machine recipe requires a tier-N machine) would form a cycle — handled by starter inventory providing the base tier (invariant #4). |
| 2 | **All referenced items exist.** Every `RecipeInput.item`, every `RecipeOutput::Item.item`, and every `ConcreteRecipe.machine` resolves to an entry in `items`. |
| 3 | **Machine field references a machine item.** Every `ConcreteRecipe.machine` resolves to an `ItemDef` with `ItemKind::Machine`. |
| 4 | **Every referenced machine is producible.** For every `machine ∈ {r.machine for r in recipes.values()}`, `by_output[machine]` is non-empty **or** the machine is provided by starter inventory / world placement (declared in a content manifest, curated for VS). |
| 5 | **All referenced materials exist.** Every `MaterialId` referenced by a Derived item resolves to an entry in `materials`. |
| 6 | **All referenced form groups exist.** Every `FormGroupId` on a `MaterialDef`, `WildcardRecipe.var_groups`, or material→group reference resolves to an entry in `form_groups`. |
| 7 | **Wildcard patterns reference valid forms.** Every form in a wildcard's `$var:form` patterns exists in `form_groups[var_groups[var]].forms`. |
| 8 | **Recipe IDs unique.** No two entries in `recipes` share an ID. Concrete-recipe assets must not collide with wildcard-expansion IDs (`"{wildcard}:{$a=…}:…"`). |
| 9 | **Item IDs unique.** No two entries in `items` share an ID. Derived item IDs (`"{material}:{form}"`) must not collide with composite/unique/machine IDs. |
| 10 | **Positive integer quantities.** Every `RecipeInput.quantity` is `≥ 1` (`u32`). Every `RecipeOutput::Item.quantity` is `≥ 1` (`u32`). |
| 11 | **Output chance bounded.** Every output's `chance ∈ (0.0, 1.0]` (both variants). (Zero would be a no-op output; negative or >1 is invalid.) |
| 12 | **Positive time and energy.** Every recipe's `processing_time > 0` and `energy_cost ≥ 0`. Every `RecipeOutput::Energy.joules > 0`. |
| 13 | **Tier sanity.** `machine_tier ≥ 1` and `min_voltage_tier ≥ 1` for every recipe. `ItemKind::Machine.tier ≥ 1` for every machine item. |
| 14 | **Catalyst items exist in some network source.** Every `RecipeInput { consumed: false }` references an item that is producible somewhere in `by_output` *or* is provided by world placement (declared in a content manifest, curated for VS, e.g. lens items). |
| 15 | **Auto-derived tags consistent.** Every `ItemDef.tags` superset includes the auto-derived tags implied by its `ItemKind` (see §3 table). The graph builder asserts this after construction. |
| 16 | **Generator-recipe primary is Energy.** For every recipe `r` whose `r.machine` resolves to a `MachineDef` carrying a `GeneratorDef` (see `power.md §3`), `r.outputs[0]` is a `RecipeOutput::Energy` variant. Subsequent entries may be `Item` (byproducts) or additional `Energy` entries (uncommon — e.g. probabilistic bonus yield). |
| 17 | **Energy output target has a generator.** For every recipe `r` containing any `RecipeOutput::Energy`, `r.machine` resolves to a `MachineDef` carrying a `GeneratorDef`. Non-generator machines may not produce energy. |

---

## 11. Seed Integration

### VS

Recipe content is loaded from `assets/` at startup. `DomainSeeds.recipes` is **unused** in VS — the recipe graph is identical across all runs.

### Post-VS

The recipe graph is generated from `DomainSeeds.recipes` (see [`seed.md §4`](seed.md#4-domain-seed-derivation)). Generator inputs:

- Curated base materials (always present)
- Pool of exotic materials, selected by seed
- Pool of wildcard and concrete recipe templates
- Machine roster (with tier ladders)
- Bounded variance parameters (see `gdd.md §8`)

Generator output: the same `RecipeGraph` shape as VS, just procedurally populated. The goal/victory system (separate doc) consumes the generated graph and derives goal items from it.

**Generator spec is deferred** to a separate post-VS doc (`recipe-graph-generation.md`) — see `README.md` Post-VS section. This document covers only the data model the generator must emit.

---

## 12. VS / MVP Scope

| Feature | VS | MVP |
|---|---|---|
| `MaterialDef`, `FormGroup`, `ItemDef` (incl. `ItemKind::Machine`), `WildcardRecipe`, `ConcreteRecipe`, `RecipeOutput` types | ✓ | ✓ |
| `RecipeGraph` resource with `materials`, `form_groups`, `wildcards`, `items`, `recipes` | ✓ | ✓ |
| `HashSet`-backed lookup indexes (`by_output`, `by_input`, `by_machine`, `by_energy_output`) | ✓ | ✓ |
| Reachability/query API (`producers`, `consumers`, `reachable_items`, `can_reach`, `energy_producers`) | ✓ | ✓ |
| Auto-derived item tags from `ItemKind` (`kind:*`, `material:*`, `form:*`, `machine_type:*`, `tier:*`) | ✓ | ✓ |
| Derived-item generation from material × form-group with `:` separator | ✓ | ✓ |
| Wildcard-recipe expansion to concrete recipes (single and multi-variable patterns) | ✓ | ✓ |
| Unified `outputs: Vec<RecipeOutput>` enum (`Item` / `Energy` variants) with `chance: f32` field (all VS recipes set `chance = 1.0`) | ✓ | ✓ |
| Generator recipes producing `RecipeOutput::Energy` (combustion + solar at T1; invariants #16/#17 enforced in tests) | ✓ | ✓ |
| `RecipeInput { item, quantity, consumed }` with catalyst flag, integer quantities | ✓ | ✓ |
| `min_voltage_tier` independent field on `ConcreteRecipe` | ✓ | ✓ |
| Machines as items; recipes reference machine via `ItemId` | ✓ | ✓ |
| Recipe tags (incl. reserved `recycle`); reachability ignores recycle by default | ✓ | ✓ |
| Validity-invariant integration tests over curated content | ✓ | ✓ |
| Procedural graph generation from `DomainSeeds.recipes` | — | — (post-VS) |
| Parameter variance (input/output qty, time, energy, chance bounds) | — | — (post-VS) |
| Probabilistic outputs at runtime (chance rolls, tech-tree quality modifiers) | — | — (post-VS) |
| Save-time graph consistency hash (owned by `save.md`) | — | — (post-VS) |

For VS: one machine type (e.g. `assembler`), one tier. The curated recipe set produces a single end-to-end chain to the curated goal item (owned by the victory system). No procedural variance, no tech-tree gating of `unlocked_recipes` (every recipe in `recipes` is implicitly unlocked).

---

## 13. Integration Test Invariants

Tests live in `src/recipe_graph/` and run against curated `assets/` (VS) and against generator output (post-VS).

1. **Empty graph.** `RecipeGraph::from_vecs(vec![], …)` produces an empty graph with empty indexes.
2. **DAG over non-recycle subgraph** — invariant #1. Topological sort over the directed graph (item→recipe for inputs and machine, recipe→item for outputs) excluding recycle-tagged recipes succeeds.
3. **All referenced items exist** — invariant #2.
4. **Machine field references a machine item** — invariant #3. For every `r in recipes.values()`, `items[r.machine].kind` is `ItemKind::Machine`.
5. **Every referenced machine is producible** — invariant #4.
6. **Derived items materialize** — for any `MaterialDef` with non-empty `form_groups`, every `(material, form)` pair produces an entry in `items` with ID `{material}:{form}`.
7. **Auto-derived tags present** — for every item, the auto-derived tags from §3 are present in `ItemDef.tags` after construction. E.g. `items["copper:ingot"].tags ⊇ {"kind:derived", "material:copper", "form:ingot"}`.
8. **Wildcard expansion creates one recipe per binding assignment** — single-variable: one expansion per material in the form group; multi-variable: Cartesian product of candidate material sets.
9. **Producers index from outputs** — `by_output[item]` (as `HashSet<RecipeId>`) equals the set of recipe IDs whose `outputs` includes `item`.
10. **Consumers index from inputs** — `by_input[item]` equals the set of recipe IDs whose `inputs` includes `item`.
11. **Unified outputs.** Any test that previously asserted on a separate `byproducts` field must assert on `outputs` and find both primary and secondary outputs in the unified list.
12. **`min_voltage_tier` default.** A concrete-recipe asset that omits `min_voltage_tier` produces a `ConcreteRecipe` with `min_voltage_tier == machine_tier`.
13. **Recipe ID uniqueness** — invariant #8.
14. **Catalyst pass-through.** A wildcard recipe's `consumed: false` entries appear unchanged in each expansion. A concrete-recipe asset's catalyst entries appear unchanged in the loaded recipe.
15. **Integer quantity rejection.** Asset loading rejects any RON file with a non-integer quantity (fractional, negative, or zero).
16. **Output chance default and bounds.** A `RecipeOutput` with `chance` omitted defaults to `1.0`. Asset loading rejects `chance ∉ (0.0, 1.0]`.
17. **VS chance == 1.0.** Every output in every recipe in the VS curated set has `chance == 1.0` (probabilistic outputs are post-VS).
18. **Recycle-tag reachability semantics.** A reachability traversal that ignores `recycle` does not traverse cycle-closing edges, and removing all recycle recipes still leaves all goal items reachable from starters (asserted by the goal-validator system's tests, not this suite — listed here for coverage).
19. **API method correctness.** `producers(item)` equals `by_output[item]`; `consumers(item)` equals `by_input[item]`; `reachable_items` returns a fixed-point set (re-running it on its output produces the same set).

---

## Open issues

- **`producers` / `consumers` → `by_output` / `by_input` rename.** `src/recipe_graph/mod.rs` uses the older names. Either rename to match this doc and `crafting.md`, or add both as aliases on the resource.
- **`byproducts` field removal in `ConcreteRecipe`.** Current code has `pub byproducts: Vec<ItemStack>`. This spec unifies into `outputs`. Migration: move existing byproduct entries into `outputs`, drop the field, update asset RON files. Producer index already merges both; behavior preserved.
- **`RecipeInput { consumed }` vs `ItemStack` for inputs.** Current code's `ConcreteRecipe.inputs: Vec<ItemStack>` does not carry the `consumed` flag. Migration: rename `inputs` element type to `RecipeInput`; default `consumed: true` for existing assets.
- **`machine: ItemId` field migration.** Current code has `machine_type: MachineTypeId` (a string keying into a registry). Migration: replace with `machine: ItemId`; create `ItemKind::Machine` variant; convert existing machine registry entries to machine item assets with proper tier ladders.
- **`u32` quantity migration.** Current code uses `f32` for `quantity` on inputs and outputs. Migration: change to `u32`; update RON assets; update any logistics/crafting code that did fractional arithmetic on quantities.
- **`:` separator migration.** Existing derived IDs use `_` (e.g. `copper_ingot`). Migration: switch derived ID format to `{material}:{form}`; update any string-matched asset references; update tests asserting on derived IDs.
- **Goal/victory resource extraction.** Drop `is_terminal: bool` and `RecipeGraph.terminal`. Goal items move to a separate resource owned by the victory/objective system (spec lives outside this doc).
- **`tags: HashSet<ItemTag>` field addition + auto-derivation.** New `ItemDef.tags` field, populated as the union of author tags and `ItemKind`-derived tags during graph construction. Migration: add field; implement derivation in builder; update tests asserting on tag presence.
- **`RecipeOutput` + `chance` field.** New struct replacing `ItemStack` for outputs (inputs still use a distinct `RecipeInput`). All existing outputs default `chance = 1.0`. Migration: rename type in `ConcreteRecipe.outputs`; update RON loader (default `chance = 1.0`); update output routing code (no behavior change for `chance = 1.0`).
- **`RecipeTag` and recipe `tags` field.** New `tags: HashSet<RecipeTag>` on `ConcreteRecipe` and `WildcardRecipe`. Reserved tag: `recycle`. Migration: add field (default empty); update loader; reachability default-ignores `recycle`.
- **`HashSet<RecipeId>` lookup index migration.** Current code (if `Vec`-backed) changes to `HashSet`-backed `by_output` / `by_input` / `by_machine`. Migration: change types; update consumers that iterate (sort after collect if order needed).
- **Reachability/query API surface.** Implement `producers`, `consumers`, `recipes_for_machine`, `reachable_items`, `reachable_from_default`, `can_reach` as `impl RecipeGraph` methods. Replace any ad-hoc traversal in other modules with calls into these helpers.
- **Save consistency hash.** `RecipeGraph` reproducibility hash (over deterministic serialization of materials, form_groups, items, recipes) for save validation. Spec lives in `save.md` (to be written); this doc commits only to determinism.
- **Wildcard recipe asset format design.** Concrete RON schema for `WildcardRecipe` with pattern strings (e.g. `"$a:wire"`) needs design pass; loader must parse and validate variable references against `var_groups`.
