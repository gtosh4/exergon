# Bevy 0.18 — Rendering, Picking, Scene Save/Load

## 3D Rendering

```rust
// Camera — component directly, NOT Camera3dBundle (removed in 0.18)
commands.spawn((
    Camera3d::default(),
    Transform::from_xyz(0.0, 20.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y),
));

// Mesh entity — tuple, NOT PbrBundle (removed in 0.18)
commands.spawn((
    Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
    MeshMaterial3d(materials.add(StandardMaterial {
        base_color: Color::srgb(0.8, 0.4, 0.1),
        ..default()
    })),
    Transform::from_xyz(x, y, z),
));

// glTF scene
commands.spawn((
    SceneRoot(asset_server.load("models/machine.glb#Scene0")),
    Transform::from_xyz(x, y, z),
));

// Directional light (sun)
commands.spawn((
    DirectionalLight {
        illuminance: 10_000.0,
        shadows_enabled: true,
        ..default()
    },
    Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.5, 0.5, 0.0)),
));

// Point light
commands.spawn((
    PointLight { intensity: 1_500.0, shadows_enabled: true, ..default() },
    Transform::from_xyz(x, y, z),
));

// Ambient light (resource, not entity)
commands.insert_resource(AmbientLight { color: Color::WHITE, brightness: 200.0 });
```

**Atmospheric/fog**: Use `AtmosphericFog` or `FogSettings` components on the camera entity.

## Picking (Click/Hover on 3D Entities)

```rust
// MeshPickingPlugin included in DefaultPlugins when feature enabled
app.add_plugins((DefaultPlugins, MeshPickingPlugin));

// Attach observers — Pickable auto-added
commands.spawn((
    Mesh3d(mesh_handle),
    MeshMaterial3d(mat_handle),
    Transform::from_xyz(x, y, z),
))
.observe(|click: On<Pointer<Click>>, mut commands: Commands| {
    commands.entity(click.entity()).insert(Selected);
})
.observe(|over: On<Pointer<Over>>, mut highlights: Query<&mut MeshMaterial3d<StandardMaterial>>| {
    // highlight on hover
});

// Drag rotate (common pattern)
fn on_drag_rotate(drag: On<Pointer<Drag>>, mut transforms: Query<&mut Transform>) {
    if let Ok(mut transform) = transforms.get_mut(drag.entity()) {
        transform.rotate_y(drag.delta.x * 0.02);
        transform.rotate_x(drag.delta.y * 0.02);
    }
}
```

## Scene / Save-Load

```rust
// Save
fn save_run(world: &World, type_registry: Res<AppTypeRegistry>) {
    let scene = DynamicSceneBuilder::from_world(world)
        .extract_entities(entity_iter)
        .extract_resources()
        .build();
    let ron = scene.serialize(&type_registry).unwrap();
    std::fs::write("save/run.scn.ron", ron).unwrap();
}

// Load
fn load_run(mut commands: Commands, asset_server: Res<AssetServer>) {
    let handle: Handle<DynamicScene> = asset_server.load("save/run.scn.ron");
    commands.spawn(DynamicSceneRoot(handle));
}
```

Requires `#[derive(Reflect)]` + `#[reflect(Component/Resource)]` on everything saved.
