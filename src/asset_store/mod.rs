//! Generic RON file-store helpers backing the `assets` MCP server.
//!
//! Engine-free (no Bevy, no ECS) so both the MCP tools and unit tests can round-trip
//! content through the real `serde`/`ron` path without spinning up an `App`. Every asset
//! kind is one entity per `.ron` file with a string identity field (`id`, `item.id`, or
//! `name`); the identity is resolved from the *deserialized* value, never the filename,
//! because filenames don't always match the id (e.g. planet archetypes).
//!
//! Writes are *canonical*: serializing re-emits every field, so `#[serde(default)]` fields
//! (e.g. `energy_output`, `template_id`, `max_reach`) become explicit. The file still loads
//! identically — this is a deliberate, accepted trade-off.

use std::path::{Path, PathBuf};

use serde::Serialize;
use serde::de::DeserializeOwned;

/// Serialize a value to canonical, anonymous-struct RON (matches the hand-authored files:
/// `(field: value)` with `Variant(..)` enums and 4-space indent).
pub fn to_ron<T: Serialize>(value: &T) -> Result<String, String> {
    let config = ron::ser::PrettyConfig::new()
        .struct_names(false)
        .indentor("    ".to_string());
    ron::ser::to_string_pretty(value, config).map_err(|e| format!("RON serialize failed: {e}"))
}

/// Parse RON text into `T` (the same path `load_ron_dir` uses, so errors mirror the game).
pub fn from_ron<T: DeserializeOwned>(text: &str) -> Result<T, String> {
    ron::from_str::<T>(text).map_err(|e| format!("RON parse failed: {e}"))
}

/// Read and deserialize a single `.ron` file.
pub fn read_file<T: DeserializeOwned>(path: &Path) -> Result<T, String> {
    let text =
        std::fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    from_ron(&text)
}

/// Write `value` as canonical RON to `path` (with a trailing newline).
pub fn write_ron<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let ron = to_ron(value)?;
    std::fs::write(path, format!("{ron}\n")).map_err(|e| format!("write {}: {e}", path.display()))
}

/// Load every `*.ron` under `dir` (recursively) as `T`, paired with its path. A missing
/// directory yields an empty list rather than an error, matching `load_ron_dir`.
pub fn load_all<T: DeserializeOwned>(dir: &Path) -> Result<Vec<(PathBuf, T)>, String> {
    let mut out = Vec::new();
    collect(dir, &mut out)?;
    out.sort_by(|(a, _), (b, _)| a.cmp(b));
    Ok(out)
}

fn collect<T: DeserializeOwned>(dir: &Path, out: &mut Vec<(PathBuf, T)>) -> Result<(), String> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Ok(());
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, out)?;
        } else if path.extension().is_some_and(|e| e == "ron") {
            let value = read_file::<T>(&path)?;
            out.push((path, value));
        }
    }
    Ok(())
}

/// Sorted list of identities across all files in `dir`.
pub fn list_ids<T, F>(dir: &Path, id_of: F) -> Result<Vec<String>, String>
where
    T: DeserializeOwned,
    F: Fn(&T) -> &str,
{
    let mut ids: Vec<String> = load_all::<T>(dir)?
        .iter()
        .map(|(_, v)| id_of(v).to_string())
        .collect();
    ids.sort();
    Ok(ids)
}

/// Find the `(path, value)` whose identity equals `id`, resolving by the deserialized field.
pub fn find_by<T, F>(dir: &Path, id: &str, id_of: F) -> Result<Option<(PathBuf, T)>, String>
where
    T: DeserializeOwned,
    F: Fn(&T) -> &str,
{
    for (path, value) in load_all::<T>(dir)? {
        if id_of(&value) == id {
            return Ok(Some((path, value)));
        }
    }
    Ok(None)
}

/// Create a new asset file for `value`, erroring if the identity already exists.
/// Returns the path written.
pub fn create<T, F>(dir: &Path, value: &T, id_of: F) -> Result<PathBuf, String>
where
    T: Serialize + DeserializeOwned,
    F: Fn(&T) -> &str,
{
    let id = id_of(value).to_string();
    if id.trim().is_empty() {
        return Err("identity field is empty".to_string());
    }
    if find_by(dir, &id, &id_of)?.is_some() {
        return Err(format!("'{id}' already exists in {}", dir.display()));
    }
    std::fs::create_dir_all(dir).map_err(|e| format!("mkdir {}: {e}", dir.display()))?;
    let path = dir.join(format!("{}.ron", sanitize(&id)));
    if path.exists() {
        return Err(format!("file {} already exists", path.display()));
    }
    write_ron(&path, value)?;
    Ok(path)
}

/// Apply a JSON merge-patch to the asset with identity `id`, validate the result deserializes
/// back to `T`, and write it. Objects merge recursively; arrays and scalars replace wholesale.
/// Returns the updated value.
pub fn update<T, F>(dir: &Path, id: &str, patch: &serde_json::Value, id_of: F) -> Result<T, String>
where
    T: Serialize + DeserializeOwned,
    F: Fn(&T) -> &str,
{
    let (path, current) = find_by::<T, _>(dir, id, &id_of)?
        .ok_or_else(|| format!("no '{id}' in {}", dir.display()))?;
    let updated = apply_patch(&current, patch)?;
    write_ron(&path, &updated)?;
    Ok(updated)
}

/// Apply a JSON merge-patch to a value and re-validate it as `T`, without touching the
/// filesystem. Objects merge recursively; arrays and scalars replace wholesale. Used by
/// [`update`] and by the list-file kinds (e.g. curated seeds) that aren't one-file-per-entity.
pub fn apply_patch<T>(value: &T, patch: &serde_json::Value) -> Result<T, String>
where
    T: Serialize + DeserializeOwned,
{
    let mut json = serde_json::to_value(value).map_err(|e| format!("to_value: {e}"))?;
    merge(&mut json, patch);
    serde_json::from_value(json)
        .map_err(|e| format!("patch produced an invalid {}: {e}", short_type::<T>()))
}

/// Delete the asset file with identity `id`.
pub fn delete<T, F>(dir: &Path, id: &str, id_of: F) -> Result<(), String>
where
    T: DeserializeOwned,
    F: Fn(&T) -> &str,
{
    let (path, _) = find_by::<T, _>(dir, id, &id_of)?
        .ok_or_else(|| format!("no '{id}' in {}", dir.display()))?;
    std::fs::remove_file(&path).map_err(|e| format!("delete {}: {e}", path.display()))
}

/// Recursive JSON merge: object keys merge, everything else (arrays, scalars) replaces.
fn merge(base: &mut serde_json::Value, patch: &serde_json::Value) {
    match (base, patch) {
        (serde_json::Value::Object(b), serde_json::Value::Object(p)) => {
            for (k, v) in p {
                merge(b.entry(k.clone()).or_insert(serde_json::Value::Null), v);
            }
        }
        (b, p) => *b = p.clone(),
    }
}

fn sanitize(id: &str) -> String {
    id.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn short_type<T>() -> &'static str {
    let name = std::any::type_name::<T>();
    name.rsplit("::").next().unwrap_or(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
    struct Widget {
        id: String,
        power: f32,
        tags: Vec<String>,
    }

    fn widget(id: &str, power: f32) -> Widget {
        Widget {
            id: id.to_string(),
            power,
            tags: vec!["a".into()],
        }
    }

    fn temp_dir(name: &str) -> PathBuf {
        let mut dir = std::env::temp_dir();
        dir.push(format!("exergon_asset_store_{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn id_of(w: &Widget) -> &str {
        &w.id
    }

    #[test]
    fn create_get_roundtrip() {
        let dir = temp_dir("create_get");
        create(&dir, &widget("miner", 5.0), id_of).unwrap();
        let (_, got) = find_by::<Widget, _>(&dir, "miner", id_of).unwrap().unwrap();
        assert_eq!(got, widget("miner", 5.0));
    }

    #[test]
    fn create_rejects_duplicate() {
        let dir = temp_dir("dup");
        create(&dir, &widget("miner", 5.0), id_of).unwrap();
        let err = create(&dir, &widget("miner", 9.0), id_of).unwrap_err();
        assert!(err.contains("already exists"), "got: {err}");
    }

    #[test]
    fn update_merges_single_field() {
        let dir = temp_dir("update");
        create(&dir, &widget("miner", 5.0), id_of).unwrap();
        let patched: Widget =
            update(&dir, "miner", &serde_json::json!({ "power": 42.0 }), id_of).unwrap();
        assert_eq!(patched.power, 42.0);
        // Untouched fields survive the patch.
        assert_eq!(patched.tags, vec!["a".to_string()]);
    }

    #[test]
    fn update_replaces_arrays_wholesale() {
        let dir = temp_dir("update_arr");
        create(&dir, &widget("miner", 5.0), id_of).unwrap();
        let patched: Widget = update(
            &dir,
            "miner",
            &serde_json::json!({ "tags": ["x", "y"] }),
            id_of,
        )
        .unwrap();
        assert_eq!(patched.tags, vec!["x".to_string(), "y".to_string()]);
    }

    #[test]
    fn update_rejects_invalid_patch() {
        let dir = temp_dir("update_bad");
        create(&dir, &widget("miner", 5.0), id_of).unwrap();
        // power must be a number; a string fails re-deserialization.
        let err = update::<Widget, _>(
            &dir,
            "miner",
            &serde_json::json!({ "power": "lots" }),
            id_of,
        )
        .unwrap_err();
        assert!(err.contains("invalid"), "got: {err}");
    }

    #[test]
    fn delete_removes_file() {
        let dir = temp_dir("delete");
        create(&dir, &widget("miner", 5.0), id_of).unwrap();
        delete::<Widget, _>(&dir, "miner", id_of).unwrap();
        assert!(
            find_by::<Widget, _>(&dir, "miner", id_of)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn missing_dir_lists_empty() {
        let dir = std::env::temp_dir().join("exergon_asset_store_does_not_exist_xyz");
        let _ = std::fs::remove_dir_all(&dir);
        assert!(list_ids::<Widget, _>(&dir, id_of).unwrap().is_empty());
    }

    #[test]
    fn ron_roundtrip_is_stable() {
        let w = widget("miner", 5.0);
        let ron = to_ron(&w).unwrap();
        let back: Widget = from_ron(&ron).unwrap();
        assert_eq!(w, back);
    }
}
