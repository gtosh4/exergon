# Bevy 0.18 — Assets & Custom Loaders

## Asset Loading

```rust
// Load (async, returns Handle immediately — data may not be ready yet)
let scene: Handle<Scene> = asset_server.load("models/machine.glb#Scene0");
let mesh:  Handle<Mesh>  = asset_server.load("models/machine.glb#Mesh0");

// Check loaded (including all dependencies)
asset_server.is_loaded_with_dependencies(&handle)

// Access data in a system
fn use_recipe(recipes: Res<Assets<RecipeData>>, handle: Res<CurrentRecipe>) {
    if let Some(recipe) = recipes.get(&handle.0) { /* ready */ }
}
```

Hot reload: add `file_watcher` feature in dev profile.

## Custom Asset Loader (for RON data files)

```rust
// Asset must derive Asset + TypePath
#[derive(Asset, TypePath, Reflect, serde::Deserialize)]
pub struct RecipeData {
    pub inputs:      Vec<(ItemId, f32)>,
    pub outputs:     Vec<(ItemId, f32)>,
    pub energy_cost: f32,
}

#[derive(Default, TypePath)]
pub struct RecipeLoader;

#[derive(Debug, thiserror::Error)]
pub enum RecipeLoaderError {
    #[error("IO: {0}")]  Io(#[from] std::io::Error),
    #[error("RON: {0}")] Ron(#[from] ron::error::SpannedError),
}

impl AssetLoader for RecipeLoader {
    type Asset    = RecipeData;
    type Settings = ();
    type Error    = RecipeLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        _load_context: &mut LoadContext<'_>,
    ) -> Result<RecipeData, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        Ok(ron::de::from_bytes(&bytes)?)
    }

    fn extensions(&self) -> &[&str] { &["recipe.ron"] }
}

// Register in plugin — use register_asset_loader for runtime instances
app.init_asset::<RecipeData>()
   .register_type::<RecipeData>()
   .register_asset_loader(RecipeLoader);
// Or init_asset_loader::<RecipeLoader>() if RecipeLoader implements FromWorld
```

## Exergon Asset Types

- `.recipe.ron` → `RecipeData` via `RecipeLoader` — recipe graph nodes
- `.node.ron` → tech tree nodes with `tier_range`, `rarity`, `unlock_vectors`
- `#[derive(Asset)]` without `TypePath` is a **0.18 breaking change** — always derive both
