use bevy::prelude::*;

/// Inserted before app startup so `WorldPlugin` can read tile count when building `VoxelWorldPlugin`.
#[derive(Resource)]
pub struct BlockAtlasLayers(pub u32);

/// Reads `assets/textures/blocks/manifest.ron`, loads each named PNG, stacks them vertically,
/// and writes `assets/textures/blocks.png`. Returns the layer count.
///
/// Called in `main()` before `App::run()` so the file exists when `bevy_voxel_world`'s asset
/// server load request is processed. See technical-design.md §4 "Block textures".
pub fn build_block_atlas() -> Result<u32, anyhow::Error> {
    let dir = "assets/textures/blocks";
    let manifest_path = format!("{dir}/manifest.ron");

    let src = std::fs::read_to_string(&manifest_path)
        .map_err(|e| anyhow::anyhow!("cannot read {manifest_path}: {e}"))?;
    let names: Vec<String> = ron::from_str(&src)
        .map_err(|e| anyhow::anyhow!("invalid manifest {manifest_path}: {e}"))?;

    let first_name = names
        .first()
        .ok_or_else(|| anyhow::anyhow!("block texture manifest is empty"))?;
    let first_path = format!("{dir}/{first_name}.png");
    let first = image::open(&first_path)
        .map_err(|e| anyhow::anyhow!("cannot load tile {first_path}: {e}"))?
        .into_rgba8();
    let (w, h) = first.dimensions();

    let n = names.len() as u32;
    let mut atlas = image::RgbaImage::new(w, h * n);

    for (i, name) in names.iter().enumerate() {
        let path = format!("{dir}/{name}.png");
        let tile = image::open(&path)
            .map_err(|e| anyhow::anyhow!("cannot load tile {path}: {e}"))?
            .into_rgba8();
        if tile.dimensions() != (w, h) {
            anyhow::bail!(
                "tile {name} is {}x{}, expected {w}x{h}",
                tile.width(),
                tile.height()
            );
        }
        image::imageops::replace(&mut atlas, &tile, 0, (i as i64) * i64::from(h));
    }

    let out = "assets/textures/blocks.png";
    atlas
        .save(out)
        .map_err(|e| anyhow::anyhow!("cannot write atlas {out}: {e}"))?;

    eprintln!("block atlas: {w}x{} ({n} layers) → {out}", h * n);
    Ok(n)
}
