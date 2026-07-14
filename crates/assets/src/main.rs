//! `assets` — a Model Context Protocol (MCP) stdio server exposing the game's RON content
//! for read/write access, using the real deserializers so what a tool sees/writes is exactly
//! what the game loads.
//!
//! The write surface is **generic over a `kind` argument** rather than one tool per asset type:
//! `list_assets` / `get_asset` / `create_asset` / `update_asset` / `delete_asset` all take a
//! `kind` (see `list_kinds`). `describe_kind` returns the JSON schema for a kind's entity, so a
//! client still gets the exact shape to build a `create_asset` / `update_asset` payload. Plus
//! resolved-graph queries (`resolve_recipe`, `list_all_recipes`, `tech_path`, `item_uses`) that
//! expose template-expanded recipes / derived items the raw files don't contain, and a dedicated
//! pair for the (non-entity) block texture manifest.
//!
//! Writes are canonical RON (every field explicit). `update_asset` takes a JSON merge-patch:
//! `{ "energy_cost": 50 }` overwrites just that field; nested objects merge, arrays/scalars
//! replace wholesale.
//!
//! Must run with the repo root as the working directory so `assets/` is reachable.
//! stdout is the JSON-RPC channel — all logging goes to stderr; never `println!`.

use std::path::Path;

use rmcp::{
    ServerHandler, ServiceExt,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
    transport::stdio,
};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use exergon::asset_store;
use exergon::content::{BiomeDef, DepositDef, LayerDef, VeinDef, load_ron_dir};
use exergon::machine::{MachineFileDef, PlaceableDef};
use exergon::planet::PlanetArchetypeDef;
use exergon::recipe_graph::{
    ConcreteRecipe, FormGroup, ItemDef, MaterialDef, RecipeTemplate, build_recipe_graph,
};
use exergon::save::DifficultyTier;
use exergon::seed::CuratedSeedEntry;
use exergon::tech_tree::NodeDef;
use scenario_runner::{Target, run_smoke};

const SEEDS_PATH: &str = "assets/seeds/curated.ron";
const TEXTURES_PATH: &str = "assets/textures/blocks/manifest.ron";

// ---------------------------------------------------------------------------
// Tool argument structs
// ---------------------------------------------------------------------------

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct KindArg {
    /// Asset kind — see `list_kinds`.
    kind: String,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct KindIdArg {
    /// Asset kind — see `list_kinds`.
    kind: String,
    /// Entity identity: its `id`, or `item.id` (placeable), or `name` (planet_archetype, seed).
    id: String,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct KindValueArg {
    /// Asset kind — see `list_kinds`.
    kind: String,
    /// The full entity as a JSON object (validated against the kind's schema — see `describe_kind`).
    value: Value,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct KindUpdateArg {
    /// Asset kind — see `list_kinds`.
    kind: String,
    /// Identity of the entity to update.
    id: String,
    /// Fields to overwrite. Nested objects merge; arrays and scalars replace wholesale.
    patch: Value,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct IdArg {
    /// The id to query.
    id: String,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct ItemArg {
    /// Item id to search recipes for.
    item: String,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct StringListArg {
    /// Full replacement list.
    entries: Vec<String>,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct SmokeArg {
    /// What to prove reachable: `item`, `node`, or `recipe`.
    kind: String,
    /// The content id (an item id, tech-node id, or recipe id).
    id: String,
    /// Force a difficulty (`initiation` | `standard` | `advanced` | `pinnacle`). Omit to auto-pick
    /// the lowest that covers the target.
    #[serde(default)]
    difficulty: Option<String>,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct QueryArg {
    /// Asset kind — see `list_kinds`.
    kind: String,
    /// A jq program. Its input is the JSON array of every entity of `kind`; every value
    /// the program yields is returned. E.g. `[.[] | select(.energy_cost > 50) | .id]`.
    jq: String,
}

// ---------------------------------------------------------------------------
// Generic dispatch over `kind`
// ---------------------------------------------------------------------------

/// Runs `$op::<T, _>(dir, <extra args...>, id_of)` for the concrete type behind `$kind`.
/// Every directory-backed kind (one `.ron` file per entity) is listed once here; the `seed`
/// kind is a single list file and is handled before this macro by the dispatch fns.
macro_rules! by_kind {
    ($kind:expr, $op:ident $(, $arg:expr)*) => {
        match $kind {
            "recipe" => $op::<ConcreteRecipe, _>("assets/recipes" $(, $arg)*, |v| v.id.as_str()),
            "tech" => $op::<NodeDef, _>("assets/tech_nodes" $(, $arg)*, |v| v.id.as_str()),
            "item" => $op::<ItemDef, _>("assets/items" $(, $arg)*, |v| v.id.as_str()),
            "material" => $op::<MaterialDef, _>("assets/materials" $(, $arg)*, |v| v.id.as_str()),
            "form_group" => $op::<FormGroup, _>("assets/form_groups" $(, $arg)*, |v| v.id.as_str()),
            "recipe_template" => {
                $op::<RecipeTemplate, _>("assets/recipe_templates" $(, $arg)*, |v| v.id.as_str())
            }
            "vein" => $op::<VeinDef, _>("assets/veins" $(, $arg)*, |v| v.id.as_str()),
            "layer" => $op::<LayerDef, _>("assets/layers" $(, $arg)*, |v| v.id.as_str()),
            "biome" => $op::<BiomeDef, _>("assets/biomes" $(, $arg)*, |v| v.id.as_str()),
            "deposit" => $op::<DepositDef, _>("assets/deposits" $(, $arg)*, |v| v.id.as_str()),
            "machine" => $op::<MachineFileDef, _>("assets/machines" $(, $arg)*, |v| v.id.as_str()),
            "placeable" => {
                $op::<PlaceableDef, _>("assets/placeables" $(, $arg)*, |v| v.item.id.as_str())
            }
            "planet_archetype" => {
                $op::<PlanetArchetypeDef, _>("assets/planet/archetypes" $(, $arg)*, |v| {
                    v.name.as_str()
                })
            }
            other => Err(unknown_kind(other)),
        }
    };
}

fn dispatch_list(kind: &str) -> Result<String, String> {
    if kind == "seed" {
        return seed_list();
    }
    by_kind!(kind, list_kind)
}

fn dispatch_get(kind: &str, id: &str) -> Result<String, String> {
    if kind == "seed" {
        return seed_get(id);
    }
    by_kind!(kind, get_kind, id)
}

fn dispatch_create(kind: &str, value: Value) -> Result<String, String> {
    if kind == "seed" {
        return seed_create(value);
    }
    by_kind!(kind, create_kind, value)
}

fn dispatch_update(kind: &str, id: &str, patch: &Value) -> Result<String, String> {
    if kind == "seed" {
        return seed_update(id, patch);
    }
    by_kind!(kind, update_kind, id, patch)
}

fn dispatch_delete(kind: &str, id: &str) -> Result<String, String> {
    if kind == "seed" {
        return seed_delete(id);
    }
    by_kind!(kind, delete_kind, id)
}

/// JSON array of every entity of `kind`, used as the input document for `query_assets`.
fn dispatch_values(kind: &str) -> Result<String, String> {
    if kind == "seed" {
        let values: Vec<Value> = read_seeds()?
            .iter()
            .map(|s| serde_json::to_value(s).map_err(|e| format!("to_value: {e}")))
            .collect::<Result<_, _>>()?;
        return to_json(&values);
    }
    by_kind!(kind, values_kind)
}

fn schema_for_kind(kind: &str) -> Result<String, String> {
    let schema = match kind {
        "recipe" => schemars::schema_for!(ConcreteRecipe),
        "tech" => schemars::schema_for!(NodeDef),
        "item" => schemars::schema_for!(ItemDef),
        "material" => schemars::schema_for!(MaterialDef),
        "form_group" => schemars::schema_for!(FormGroup),
        "recipe_template" => schemars::schema_for!(RecipeTemplate),
        "vein" => schemars::schema_for!(VeinDef),
        "layer" => schemars::schema_for!(LayerDef),
        "biome" => schemars::schema_for!(BiomeDef),
        "deposit" => schemars::schema_for!(DepositDef),
        "machine" => schemars::schema_for!(MachineFileDef),
        "placeable" => schemars::schema_for!(PlaceableDef),
        "planet_archetype" => schemars::schema_for!(PlanetArchetypeDef),
        "seed" => schemars::schema_for!(CuratedSeedEntry),
        other => return Err(unknown_kind(other)),
    };
    to_json(&schema)
}

/// The kinds this server manages, with each kind's identity field, for `list_kinds`.
const KIND_CATALOG: &[(&str, &str)] = &[
    ("recipe", "id"),
    ("tech", "id"),
    ("item", "id"),
    ("material", "id"),
    ("form_group", "id"),
    ("recipe_template", "id"),
    ("vein", "id"),
    ("layer", "id"),
    ("biome", "id"),
    ("deposit", "id"),
    ("machine", "id"),
    ("placeable", "item.id"),
    ("planet_archetype", "name"),
    ("seed", "name"),
];

fn kinds_catalog() -> Result<String, String> {
    let kinds: Vec<Value> = KIND_CATALOG
        .iter()
        .map(|(kind, identity)| serde_json::json!({ "kind": kind, "identity": identity }))
        .collect();
    to_json(&serde_json::json!({
        "writable_kinds": kinds,
        "note": "Use these with list_assets/get_asset/create_asset/update_asset/delete_asset. \
                 Call describe_kind first to get a kind's JSON schema.",
        "special_tools": ["get_texture_manifest", "update_texture_manifest"],
        "graph_queries": ["resolve_recipe", "list_all_recipes", "tech_path", "item_uses"],
        "query": ["query_assets (jq program over all entities of a kind)"],
    }))
}

fn unknown_kind(kind: &str) -> String {
    let names: Vec<&str> = KIND_CATALOG.iter().map(|(k, _)| *k).collect();
    format!("unknown kind '{kind}'; valid kinds: {}", names.join(", "))
}

// ---------------------------------------------------------------------------
// Generic CRUD helpers over a one-file-per-entity directory
// ---------------------------------------------------------------------------

fn to_json<T: Serialize>(value: &T) -> Result<String, String> {
    serde_json::to_string_pretty(value).map_err(|e| format!("json encode failed: {e}"))
}

fn list_kind<T, F>(dir: &str, id_of: F) -> Result<String, String>
where
    T: DeserializeOwned,
    F: Fn(&T) -> &str,
{
    let ids = asset_store::list_ids::<T, _>(Path::new(dir), id_of)?;
    to_json(&ids)
}

fn values_kind<T, F>(dir: &str, _id_of: F) -> Result<String, String>
where
    T: Serialize + DeserializeOwned,
    F: Fn(&T) -> &str,
{
    let values: Vec<Value> = asset_store::load_all::<T>(Path::new(dir))?
        .iter()
        .map(|(_, v)| serde_json::to_value(v).map_err(|e| format!("to_value: {e}")))
        .collect::<Result<_, _>>()?;
    to_json(&values)
}

fn get_kind<T, F>(dir: &str, id: &str, id_of: F) -> Result<String, String>
where
    T: Serialize + DeserializeOwned,
    F: Fn(&T) -> &str,
{
    let (_, value) = asset_store::find_by::<T, _>(Path::new(dir), id, id_of)?
        .ok_or_else(|| format!("no '{id}' in {dir}"))?;
    to_json(&value)
}

/// Parse `value` into the concrete type (validating it), then create a new file.
fn create_kind<T, F>(dir: &str, value: Value, id_of: F) -> Result<String, String>
where
    T: Serialize + DeserializeOwned,
    F: Fn(&T) -> &str,
{
    let parsed: T = serde_json::from_value(value).map_err(|e| format!("invalid asset: {e}"))?;
    let path = asset_store::create(Path::new(dir), &parsed, id_of)?;
    Ok(format!("created {}", path.display()))
}

fn update_kind<T, F>(dir: &str, id: &str, patch: &Value, id_of: F) -> Result<String, String>
where
    T: Serialize + DeserializeOwned,
    F: Fn(&T) -> &str,
{
    let updated: T = asset_store::update(Path::new(dir), id, patch, id_of)?;
    to_json(&updated)
}

fn delete_kind<T, F>(dir: &str, id: &str, id_of: F) -> Result<String, String>
where
    T: DeserializeOwned,
    F: Fn(&T) -> &str,
{
    asset_store::delete::<T, _>(Path::new(dir), id, id_of)?;
    Ok(format!("deleted '{id}' from {dir}"))
}

// ---------------------------------------------------------------------------
// `seed` kind — a single list file (`curated.ron`) keyed by entry `name`
// ---------------------------------------------------------------------------

fn read_seeds() -> Result<Vec<CuratedSeedEntry>, String> {
    asset_store::read_file(Path::new(SEEDS_PATH))
}

fn write_seeds(seeds: &[CuratedSeedEntry]) -> Result<(), String> {
    asset_store::write_ron(Path::new(SEEDS_PATH), &seeds.to_vec())
}

fn seed_list() -> Result<String, String> {
    let names: Vec<String> = read_seeds()?.into_iter().map(|s| s.name).collect();
    to_json(&names)
}

fn seed_get(name: &str) -> Result<String, String> {
    let seeds = read_seeds()?;
    let entry = seeds
        .iter()
        .find(|s| s.name == name)
        .ok_or_else(|| format!("no seed '{name}'"))?;
    to_json(entry)
}

fn seed_create(value: Value) -> Result<String, String> {
    let entry: CuratedSeedEntry =
        serde_json::from_value(value).map_err(|e| format!("invalid seed: {e}"))?;
    let mut seeds = read_seeds()?;
    if seeds.iter().any(|s| s.name == entry.name) {
        return Err(format!("seed '{}' already exists", entry.name));
    }
    let name = entry.name.clone();
    seeds.push(entry);
    write_seeds(&seeds)?;
    Ok(format!("added seed '{name}'"))
}

fn seed_update(name: &str, patch: &Value) -> Result<String, String> {
    let mut seeds = read_seeds()?;
    let entry = seeds
        .iter_mut()
        .find(|s| s.name == name)
        .ok_or_else(|| format!("no seed '{name}'"))?;
    *entry = asset_store::apply_patch(entry, patch)?;
    let out = to_json(entry)?;
    write_seeds(&seeds)?;
    Ok(out)
}

fn seed_delete(name: &str) -> Result<String, String> {
    let mut seeds = read_seeds()?;
    let before = seeds.len();
    seeds.retain(|s| s.name != name);
    if seeds.len() == before {
        return Err(format!("no seed '{name}'"));
    }
    write_seeds(&seeds)?;
    Ok(format!("deleted seed '{name}'"))
}

/// Depth-first prerequisite walk: a node's prerequisites are pushed before the node itself.
fn walk_path(id: &str, nodes: &[NodeDef], seen: &mut Vec<String>) {
    if seen.iter().any(|s| s == id) {
        return;
    }
    let Some(node) = nodes.iter().find(|n| n.id == id) else {
        return;
    };
    for prereq in &node.prerequisites {
        walk_path(prereq, nodes, seen);
    }
    seen.push(id.to_string());
}

/// Run a jq `program` over the JSON array of every entity of `kind`, returning the
/// program's outputs as a pretty JSON array. Uses jaq (pure-Rust jq).
fn run_query(kind: &str, program: &str) -> Result<String, String> {
    use jaq_core::load::{Arena, File, Loader};
    use jaq_core::{Compiler, Ctx, Vars, data, unwrap_valr};
    use jaq_json::Val;

    let input: Val = serde_json::from_str(&dispatch_values(kind)?)
        .map_err(|e| format!("build query input: {e}"))?;

    let defs = jaq_core::defs()
        .chain(jaq_std::defs())
        .chain(jaq_json::defs());
    let funs = jaq_core::funs()
        .chain(jaq_std::funs())
        .chain(jaq_json::funs());
    let arena = Arena::default();
    let modules = Loader::new(defs)
        .load(
            &arena,
            File {
                code: program,
                path: (),
            },
        )
        .map_err(|errs| format!("jq parse error: {errs:?}"))?;
    let filter = Compiler::default()
        .with_funs(funs)
        .compile(modules)
        .map_err(|errs| format!("jq compile error: {errs:?}"))?;

    let ctx = Ctx::<data::JustLut<Val>>::new(&filter.lut, Vars::new([]));
    let mut outputs: Vec<Value> = filter
        .id
        .run((ctx, input))
        .map(unwrap_valr)
        .map(|r| {
            let val = r.map_err(|e| format!("jq runtime error: {e}"))?;
            // Val's Display is canonical JSON; re-parse so output formatting matches
            // the rest of the server.
            serde_json::from_str(&val.to_string()).map_err(|e| format!("decode jq output: {e}"))
        })
        .collect::<Result<_, _>>()?;
    // A jq program most often yields a single value (e.g. a `[...]` collection); return it
    // bare so the output isn't needlessly wrapped. A multi-value stream stays an array.
    if outputs.len() == 1 {
        return to_json(&outputs.remove(0));
    }
    to_json(&outputs)
}

// ---------------------------------------------------------------------------
// Server
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct AssetServer {
    tool_router: ToolRouter<Self>,
}

impl AssetServer {
    fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_router]
impl AssetServer {
    // --- discovery -----------------------------------------------------------
    #[tool(
        description = "List the asset kinds this server manages and each kind's identity field."
    )]
    fn list_kinds(&self) -> Result<String, String> {
        kinds_catalog()
    }
    #[tool(
        description = "Get the JSON schema for a kind's entity. Call before create_asset/update_asset to see the required fields. kind: see list_kinds."
    )]
    fn describe_kind(&self, Parameters(a): Parameters<KindArg>) -> Result<String, String> {
        schema_for_kind(&a.kind)
    }

    // --- generic CRUD over `kind` --------------------------------------------
    #[tool(description = "List all asset ids of a kind. kind: see list_kinds.")]
    fn list_assets(&self, Parameters(a): Parameters<KindArg>) -> Result<String, String> {
        dispatch_list(&a.kind)
    }
    #[tool(description = "Get one asset by id (the kind's identity field). kind: see list_kinds.")]
    fn get_asset(&self, Parameters(a): Parameters<KindIdArg>) -> Result<String, String> {
        dispatch_get(&a.kind, &a.id)
    }
    #[tool(
        description = "Create a new asset from a JSON object, validated against the kind's schema (errors if the id already exists)."
    )]
    fn create_asset(&self, Parameters(a): Parameters<KindValueArg>) -> Result<String, String> {
        dispatch_create(&a.kind, asset_store::coerce_json_arg(a.value))
    }
    #[tool(
        description = "Update asset fields by id via a JSON merge-patch (nested objects merge; arrays/scalars replace wholesale)."
    )]
    fn update_asset(&self, Parameters(a): Parameters<KindUpdateArg>) -> Result<String, String> {
        dispatch_update(&a.kind, &a.id, &asset_store::coerce_json_arg(a.patch))
    }
    #[tool(description = "Delete an asset by id.")]
    fn delete_asset(&self, Parameters(a): Parameters<KindIdArg>) -> Result<String, String> {
        dispatch_delete(&a.kind, &a.id)
    }

    // --- block texture manifest (a single bare-string list; not loaded by the game yet) ---
    #[tool(
        description = "Get the block texture manifest (ordered atlas list). Note: not loaded by the game yet."
    )]
    fn get_texture_manifest(&self) -> Result<String, String> {
        let list: Vec<String> = asset_store::read_file(Path::new(TEXTURES_PATH))?;
        to_json(&list)
    }
    #[tool(description = "Replace the entire block texture manifest with a new ordered list.")]
    fn update_texture_manifest(
        &self,
        Parameters(a): Parameters<StringListArg>,
    ) -> Result<String, String> {
        asset_store::write_ron(Path::new(TEXTURES_PATH), &a.entries)?;
        Ok(format!("wrote {} texture entries", a.entries.len()))
    }

    // --- resolved-graph read queries -----------------------------------------
    #[tool(
        description = "Resolve a recipe from the full graph, including template-expanded recipes"
    )]
    fn resolve_recipe(&self, Parameters(a): Parameters<IdArg>) -> Result<String, String> {
        let graph = build_recipe_graph();
        graph
            .recipes
            .get(&a.id)
            .ok_or_else(|| format!("no recipe '{}' in resolved graph", a.id))
            .and_then(to_json)
    }
    #[tool(description = "List every recipe id in the resolved graph (incl. template-expanded)")]
    fn list_all_recipes(&self) -> Result<String, String> {
        let graph = build_recipe_graph();
        let mut ids: Vec<&String> = graph.recipes.keys().collect();
        ids.sort();
        to_json(&ids)
    }
    #[tool(description = "Prerequisite chain to reach a tech node, in dependency order")]
    fn tech_path(&self, Parameters(a): Parameters<IdArg>) -> Result<String, String> {
        let nodes = load_ron_dir::<NodeDef>("assets/tech_nodes", "tech node");
        let mut seen = Vec::new();
        walk_path(&a.id, &nodes, &mut seen);
        to_json(&seen)
    }
    #[tool(description = "Recipes that produce / consume an item, from the resolved graph")]
    fn item_uses(&self, Parameters(a): Parameters<ItemArg>) -> Result<String, String> {
        let graph = build_recipe_graph();
        let empty = Vec::new();
        let produced = graph.producers.get(&a.item).unwrap_or(&empty);
        let consumed = graph.consumers.get(&a.item).unwrap_or(&empty);
        to_json(&serde_json::json!({
            "produced_by": produced,
            "consumed_by": consumed,
        }))
    }
    #[tool(
        description = "Query assets of a kind with a jq program. The input document is the JSON array of every entity of `kind`; every value the program yields is returned. E.g. kind=machine jq='[.[] | select(.power_draw > 100) | .id]'."
    )]
    fn query_assets(&self, Parameters(a): Parameters<QueryArg>) -> Result<String, String> {
        run_query(&a.kind, &a.jq)
    }

    #[tool(
        description = "Prove one piece of content is reachable + functional in a real (headless) run \
        without authoring a scenario. kind = item | node | recipe. Auto-picks the lowest difficulty \
        that covers the target (unless one is forced), derives from the matching e2e baseline, runs \
        it, and reports whether the target was reached. Returns JSON: on a content/config problem \
        (unknown id, an input with no producer, a broken prerequisite chain, or no baseline for the \
        difficulty) `{ok:false, failure_reason}` — fix the content and retry; otherwise \
        `{ok:true, reached, difficulty, virtual_secs}`. Runs a full simulation, so expect several \
        seconds per call."
    )]
    fn smoke_test(&self, Parameters(a): Parameters<SmokeArg>) -> Result<String, String> {
        let target = match a.kind.as_str() {
            "item" => Target::Item(a.id.clone()),
            "node" => Target::Node(a.id.clone()),
            "recipe" => Target::Recipe(a.id.clone()),
            other => {
                return Err(format!(
                    "unknown kind `{other}` (expected item | node | recipe)"
                ));
            }
        };
        let difficulty = match a.difficulty.as_deref() {
            None => None,
            Some(d) => Some(parse_difficulty(d)?),
        };

        match run_smoke(&target, difficulty) {
            Ok(r) => to_json(&serde_json::json!({
                "ok": true,
                "reached": r.reached,
                "difficulty": format!("{:?}", r.difficulty),
                "virtual_secs": r.report.virtual_secs,
            })),
            // A content/config problem (not a server error): report it so the caller can fix and retry.
            Err(failure_reason) => to_json(&serde_json::json!({
                "ok": false,
                "failure_reason": failure_reason,
            })),
        }
    }
}

fn parse_difficulty(s: &str) -> Result<DifficultyTier, String> {
    match s.to_ascii_lowercase().as_str() {
        "initiation" => Ok(DifficultyTier::Initiation),
        "standard" => Ok(DifficultyTier::Standard),
        "advanced" => Ok(DifficultyTier::Advanced),
        "pinnacle" => Ok(DifficultyTier::Pinnacle),
        other => Err(format!(
            "unknown difficulty `{other}` (expected initiation | standard | advanced | pinnacle)"
        )),
    }
}

#[tool_handler]
impl ServerHandler for AssetServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Exergon RON content server. Generic CRUD over a `kind` argument: list_assets, \
                 get_asset, create_asset, update_asset, delete_asset. Call list_kinds for the \
                 kinds and describe_kind for a kind's JSON schema. update_asset takes a JSON \
                 merge-patch. query_assets runs a jq program over all entities of a kind. \
                 Also: get/update_texture_manifest, and resolved-graph queries \
                 (resolve_recipe, list_all_recipes, tech_path, item_uses). smoke_test proves a \
                 piece of content (item/node/recipe) is reachable in a real headless run. The \
                 server must run from the repo root so assets/ is reachable."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // stdout is the JSON-RPC channel — route all diagnostics to stderr.
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();

    let service = AssetServer::new().serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
