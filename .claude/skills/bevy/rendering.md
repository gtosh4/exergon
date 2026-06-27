# Bevy 0.19 — Rendering, Picking, Scene Save/Load

## 3D Rendering

## Picking (Click/Hover on 3D Entities)

## UI Text (0.19 — Parley migration)
Biggest UI churn at the version bump. `TextFont` fields changed:
```rust
// 0.18
TextFont { font: asset_server.load("font.ttf"), font_size: 24.0, ..default() }
// 0.19
TextFont { font: asset_server.load("font.ttf").into(), font_size: FontSize::Px(24.0), ..default() }
```
- `font: Handle<Font>` → `font: FontSource` (use `.into()` on the handle).
- `font_size: f32` → `font_size: FontSize` (wrap literals in `FontSize::Px(..)`).
- `TextLayout::new_with_justify/new_with_linebreak/new_with_no_wrap` → `justify`/`linebreak`/`no_wrap`.
- `Font::try_from_bytes(..).unwrap()` → `Font::from_bytes(..)` (no longer `Result`).
- `UiWidgetsPlugin` + `InputDispatchPlugin` are now in `DefaultPlugins` — remove explicit `add_plugins`.

## Camera / 3D (0.19)
- `Skybox.image: Handle<Image>` → `Option<Handle<Image>>` (`Some(..)`).
- `Hdr` moved `bevy_render` → `bevy_camera`; read via `ExtractedCamera::hdr`, not `ExtractedView::hdr`.
- `Atmosphere` is now its own entity; `bevy::pbr::Atmosphere` → `bevy::light::Atmosphere`, `earthlike` → `earth`. (Exergon doesn't use it yet.)
