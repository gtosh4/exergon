# Implementation Plan — Vertical Slice Sprint 1: First Escape
Always check off items as they are completed.

> **Scope note:** This plan implements Milestone 1 (Vertical Slice) — one complete Initiation run end-to-end with simplified/stubbed systems. Tech node names used here (`science_basics`, `basic_processing`, `advanced_processing`, `resonite_engineering`) are VS placeholder assets. Final T1 node names per `tech-tree-design.md §6` (Stone Furnace, Field Analyzer, etc.) will replace them in MVP.

## What You Can Test When This Is Done

Boot the game (`cargo run -- --test` for fast setup or `cargo run` for full run). Build a factory. Research up the tech tree. Explore to find the xalite deposit and the alien gateway ruins site — both fire discovery events that unlock key tech nodes. Research through to `escape_synthesis`. Build an assembler tier 2, run the `forge_gateway_key` recipe to craft a Gateway Key. Run cables from your power and logistics networks to the pre-existing gateway structure at the ruins site. Put the key in the logistics network. Right-click the gateway to activate it. Win screen appears showing your seed and elapsed time.

**Complete win-condition path:**
1. Mine iron/copper → smelt ingots → accumulate research
2. Drone explores → finds xalite deposit → `alien_materials` unlocked
3. Research `science_basics → basic_processing → advanced_processing` (ResearchSpend)
4. Research `resonite_engineering` (500 pts) → unlocks resonite circuit recipe
5. Explore → find gateway ruins site → `gateway_theory` unlocked (also enables cabling the gateway)
6. Research `escape_synthesis` (1000 pts) → unlocks `forge_gateway_key`
7. Craft gateway key in tier-2 assembler → put in logistics network cabled to gateway
8. Gateway powered + key present → right-click gateway → win

---

## Content layer (already in assets, verify on load)

The following content assets already exist and should just work — verify they load without errors on startup:

- `assets/items/gateway_key.ron` — Unique terminal item ✓
- `assets/machines/gateway.ron` — Machine type with power + logistics ports (world-spawned, not player-placed) ✓
- `assets/recipes/forge_gateway_key.ron` — assembler tier 2, resonite_circuit + power_cell → gateway_key ✓
- `assets/tech_nodes/escape_synthesis.ron` — ResearchSpend(1000) unlock ✓
- `assets/tech_nodes/gateway_theory.ron` — ExplorationDiscovery("gateway_ruins") unlock ✓
- `assets/tech_nodes/alien_materials.ron` — ExplorationDiscovery("xalite_deposit") unlock ✓

Note: `assets/items/gateway.ron` exists but the gateway is never placed from inventory — remove it from any hotbar/inventory give lists.

---

## Phase 1 — Discovery System

The `ExplorationDiscovery(String)` unlock trigger exists in the type system but is never handled.
Xalite deposit markers exist in the world. Gateway ruins does not exist yet.

**1-1. `DiscoveryEvent`** (`src/research/mod.rs`)
- [ ] Add `DiscoveryEvent(pub String)` Bevy event
- [ ] Register event in `ResearchPlugin`
- [ ] In `check_research_unlocks`: match `UnlockVector::ExplorationDiscovery(ref key)` — unlock if `DiscoveryEvent` with matching key was fired this frame
- [ ] Test: node with `ExplorationDiscovery("x")` unlocks when `DiscoveryEvent("x")` fires; does not unlock on wrong key

**1-2. Xalite deposit proximity trigger** (`src/drone/mod.rs`)
- [ ] `deposit_discovery_system`: in `DronePilot` state, check distance from drone to each `OreDeposit` entity whose ores contain xalite; within 8.0 units → fire `DiscoveryEvent("xalite_deposit")`; use a `Discovered` marker component to fire only once per deposit
- [ ] Test: system fires event once, not repeatedly

**1-3. Gateway ruins site** (`src/world/generation.rs` or new `src/world/ruins.rs`)
- [ ] `spawn_gateway_ruins_system`: runs once after terrain loads; picks seeded position (use `DomainSeeds::world` + offset, ensure above terrain surface); spawns:
  - Decorative ruins mesh (broken columns, rubble — or a simple distinctive shape for now)
  - The gateway machine entity itself (`Machine { machine_type: "gateway", tier: 1 }`) with IO port markers, collidable, right-clickable — same as any placed machine but world-spawned
  - `Collider::sphere(8.0)` sensor on ruins root for proximity detection
- [ ] `ruins_discovery_system`: in `DronePilot` state, drone within sensor radius → fire `DiscoveryEvent("gateway_ruins")` once (same `Discovered` marker pattern)
- [ ] Gateway machine cables/interaction locked until `gateway_theory` unlocked (check `TechTreeProgress::unlocked_machines` contains "gateway" before allowing cable snap or right-click)
- [ ] Gateway ruins position must be reachable on foot — place within 200 units of spawn, above terrain
- [ ] Test: ruins spawns at deterministic position for a given seed; `DiscoveryEvent` fires once on approach

---

## Phase 2 — Assembler Tier 2

`forge_gateway_key` requires `machine_tier: 2` assembler. Assembler asset only defines tier 1.

**2-1. Assembler tier 2 asset** (`assets/machines/assembler.ron`)
- [ ] Add tier 2 entry: same footprint, same IO layout as tier 1 (recipe system uses `machine_tier <= machine.tier`, so tier 2 machine can run tier 1 and tier 2 recipes)
- [ ] Add assembler tier 2 to `--test` hotbar so it's testable

**2-2. Verify recipe runs** (no new code needed if tier check already works)
- [ ] Integration test: assembler tier 2 + forge_gateway_key recipe + required inputs → gateway_key output
  - Pattern: same as smelter test in `tests/smelter_recipe.rs`

---

## Phase 3 — Gateway Activation

The gateway is a world-spawned machine. Need activation logic when player right-clicks it.

**3-1. Gateway activation interaction** (`src/machine/mod.rs` or `src/world/interaction.rs`)
- [ ] Right-click gateway machine in `Exploring` mode → `GatewayInteractEvent { gateway: Entity }`
- [ ] `gateway_activate_system`: on `GatewayInteractEvent` — check `gateway_theory` is unlocked (gating interaction on discovery) AND logistics network connected to gateway contains `gateway_key` (qty ≥ 1) AND gateway is on a powered network with `speed_factor >= 1.0` → fire `EscapeEvent`; else show diagnostic in machine status panel ("Undiscovered" / "Missing key" / "Insufficient power")

**3-2. `EscapeEvent`** (`src/lib.rs` or `src/game/mod.rs`)
- [ ] Add `EscapeEvent` Bevy event
- [ ] Handler: on `EscapeEvent` → record escape time in `RunState` → transition to `GameState::Escaped`

---

## Phase 4 — Run State + Win Screen

**4-1. `RunState` resource** (`src/game/mod.rs` or new `src/run_state.rs`)
- [ ] `RunState { seed: u64, status: RunStatus, start_time: f32, escape_time: Option<f32> }`
- [ ] `RunStatus` enum: `InProgress`, `Escaped`
- [ ] Populated at game start from `RunSeed` resource; `escape_time` set on `EscapeEvent`
- [ ] Test: RunState initializes with correct seed and InProgress status

**4-2. `GameState::Escaped`** (`src/game/mod.rs`)
- [ ] Add `Escaped` variant to `GameState` enum
- [ ] On enter: pause all simulation systems (same as `GameState::Paused`); show win screen

**4-3. Win screen** (`src/ui/mod.rs`)
- [ ] `win_screen_ui` system: runs in `GameState::Escaped`; full-screen egui `CentralPanel`
- [ ] Shows: "Run Complete", seed (formatted as hex or decimal), elapsed time (minutes:seconds), "Press Esc or Enter to return to main menu" (→ `GameState::MainMenu`)
- [ ] No new game / retry in this sprint — just main menu return

---

## Phase 5 — Gateway Compass HUD

Player needs to know where the ruins are without wandering randomly.

**5-1. Compass element** (`src/ui/mod.rs`)
- [ ] In `Playing` state HUD: show "⬡ Gateway: {distance:.0}m {bearing}°" (bearing from player/camera forward)
- [ ] Bearing: angle from camera forward projected onto XZ plane to ruins XZ position; display as 0–360°
- [ ] Only show when ruins position is known (always known after spawn; hide if drone has not discovered it yet — show "?" for distance until `DiscoveryEvent("gateway_ruins")` fires)
- [ ] Test: bearing calculation is correct (N/S/E/W sanity check)

---

## Bugs / Gaps to Fix

- [ ] `gateway_theory` primary_unlock is `ExplorationDiscovery("gateway_ruins")` — this will now work after Phase 1
- [ ] `alien_materials` primary_unlock is `ExplorationDiscovery("xalite_deposit")` — this will now work after Phase 1
- [ ] Assembler machine is in `assets/machines/` but not in `--test` hotbar — add it (with tier 2)
- [ ] `assets/items/gateway.ron` exists — ensure it is never given to player inventory or `--test` hotbar
- [ ] **Tier gate mechanics** — T1 gate ("Analyze first alien sample + deploy surface drone") and T1→2 gate ("Produce 100 units of refined base material") are specified in `tech-tree-design.md §3` and §6 but not implemented. For VS, `--test` mode must either bypass these gates or stub them so the full run path is testable.
- [ ] Drag-drop inventory to hotbar doesn't work
- [ ] Update power system to use volt/amp/watt as described in the docs
- [ ] 

---

## Out of Scope (next sprint)

- Underground layer + digger drone
- Save / load (full run resume)
- Procedural recipe graph generation
- Procedural tech tree generation
- Multiple escape types (Standard/Advanced/Pinnacle)
