# Exergon: bevy 0.18 → 0.19 migration

Status as of **2026-06-26**: **DONE.** Bumped to bevy 0.19; builds clean, 264 tests pass.

## Versions landed

| Dep | Version | Notes |
|-----|---------|-------|
| `bevy` | 0.19 | requires **rustc ≥ 1.95** (`rustup update stable`) |
| `avian3d` | 0.7 | |
| `bevy-tnua` / `bevy-tnua-avian3d` | 0.32 / 0.12 | integration crate (0.12) was the long pole |
| `moonshine-save` | 0.7 | now built on `bevy_world_serialization`, not `bevy_scene` |

The physics deps all shipped 0.19 releases on 2026-06-22; the blocker is cleared.

## When unblocked — Cargo.toml

```toml
bevy = { version = "0.19", features = ["dynamic_linking"] }
avian3d = "<0.19-compatible>"
bevy-tnua = "<0.19-compatible>"
bevy-tnua-avian3d = "<0.19-compatible>"
```
Note: in 0.19 `ui` and `audio` are no longer implied by `2d`/`3d` — they're explicit default
features. The default `bevy` (no `default-features = false`) still includes them, so no action
unless we later trim features.

## Code changes (by impact)

### 1. UI text — `TextFont` (biggest churn)
~10 files: `debug/mod.rs`, `ui/menus/{loading,pause}.rs`, `ui/panels/{inventory,hotbar,storage}.rs`,
`ui/hud/{escape,power,research,tooltip}.rs`.
- `font: <handle>` → `font: <handle>.into()`
- `font_size: <f32>` → `font_size: FontSize::Px(<f32>)`
- `TextLayout::new_with_*` → `justify` / `linebreak` / `no_wrap`
See [`rendering.md`](./rendering.md) "UI Text".

### 2. Resources-as-components
- Scan for `#[derive(Component, Resource)]` doubles — **none currently**, but re-check.
- `#[derive(MapEntities)]` on resources in `power/mod.rs`, `logistics/mod.rs`, `save/mod.rs`:
  if the derive is on a **Resource**, drop it (auto now); if on a **Component**, keep. Verify each.
- Broad queries (`Query<Entity>`, `Query<()>`, `Query<Option<&T>>`) may panic on resource conflict.
  Compile, and where it complains add `Without<ThatResource>`.

### 3. Renames (grep, only if present)
- `DefaultErrorHandler` → `FallbackErrorHandler`
- `set_executor_kind` / `ExecutorKind` → `set_executor(MultiThreadedExecutor::new())`
- `System::type_id` → `System::system_type`
- `Assets::get_mut` returns `AssetMut<A>` now — deref still works; only matters if you stored `&mut A`.
- `AssetPath::resolve`/`resolve_embed` (str overloads) → `resolve_str`/`resolve_embed_str`

### 4. Scene API → `bevy_world_serialization` (what actually broke)
In 0.19 `bevy_scene` became the BSN system (`Scene` is now a trait). The old asset-scene types
moved to `bevy_world_serialization` (re-exported as `bevy::world_serialization`, in the prelude):
- `Scene` → `WorldAsset`, `SceneRoot` → `WorldAssetRoot` (used in `machine/{placement,visuals}`,
  `world/generation`, `power/mod`, `main.rs`). GLTF `#Scene0` loads as `Handle<WorldAsset>`.
- `SceneFilter` → `WorldFilter` (`save/mod.rs` save whitelist). Same `deny_all`/`allow` API.
- `bevy::scene::serde::SceneDeserializer` → `bevy::world_serialization::serde::WorldDeserializer`
  (a `DeserializeSeed` needing a `load_from_path: &mut dyn bevy::asset::LoadFromPath`). The
  header-only read in `save/mod.rs::read_run_header` now passes a no-op `LoadFromPath` (header
  components carry no asset handles). `DynamicWorld.entities[*].components` field names unchanged.

### 5. moonshine-save 0.7
Built on `bevy_world_serialization`; `SaveWorld.components`/`.resources` are now `WorldFilter`.
Save/load flow otherwise unchanged.

### 6. Misc renames hit
- `DirectionalLight.shadows_enabled` → `shadow_maps_enabled` (`world/player.rs`).
- `SystemCondition::or(..)` deprecated → `.or_else(..)` (`planet/mod.rs`).
- `SystemState::get`/`get_mut` now return `Result` — `.unwrap()` in tests (`power/mod.rs`).

## Verify
1. `cargo build` — fix compile errors top-down (deps first, then renames, then text).
2. `cargo fmt && cargo clippy && cargo test`.
3. Run the app (`/run`), confirm UI text renders and physics/movement work.

## Next commit (after the bump): bsn
Migrate `commands.spawn((..))` tuple spawns to `bsn!`. **Not in this migration.** Separate commit.
