//! Query the game's RON asset definitions from the terminal, using the real loaders
//! (so what you see is exactly what the game deserializes).
//!
//! Run from repo root so `assets/` is reachable:
//!   cargo run --bin assets recipe make_miner     # one recipe (inputs/outputs/machine/time)
//!   cargo run --bin assets recipes                # list every recipe id
//!   cargo run --bin assets tech ore_extraction    # one tech node (prereqs, cost, effects)
//!   cargo run --bin assets techs                   # list every tech node id
//!   cargo run --bin assets path drone_recon        # prerequisite chain to reach a node
//!   cargo run --bin assets uses stone              # recipes that produce / consume an item

use exergon::content::load_ron_dir;
use exergon::recipe_graph::{ConcreteRecipe, build_recipe_graph};
use exergon::tech_tree::NodeDef;

fn print_recipe(r: &ConcreteRecipe) {
    let stacks = |ss: &[exergon::recipe_graph::ItemStack]| {
        if ss.is_empty() {
            "-".to_string()
        } else {
            ss.iter()
                .map(|s| format!("{}x{}", s.quantity, s.item))
                .collect::<Vec<_>>()
                .join(", ")
        }
    };
    println!("{}", r.id);
    println!("  machine : {} (tier {})", r.machine_type, r.machine_tier);
    println!(
        "  time    : {}s   energy_cost: {}",
        r.processing_time, r.energy_cost
    );
    println!("  inputs  : {}", stacks(&r.inputs));
    println!("  outputs : {}", stacks(&r.outputs));
    if !r.byproducts.is_empty() {
        println!("  byproduct: {}", stacks(&r.byproducts));
    }
}

fn print_node(n: &NodeDef) {
    println!("{}  \"{}\"", n.id, n.name);
    println!("  tier {} / {:?} / {:?}", n.tier, n.category, n.rarity);
    println!("  unlock_via : {:?}", n.primary_unlock);
    println!(
        "  prereqs    : {}",
        if n.prerequisites.is_empty() {
            "-".to_string()
        } else {
            n.prerequisites.join(", ")
        }
    );
    for e in &n.effects {
        println!("  effect     : {e:?}");
    }
}

/// Depth-first prerequisite walk, printing each node once in dependency order
/// (a node's prerequisites are printed before the node itself).
fn walk_path(id: &str, nodes: &[NodeDef], seen: &mut Vec<String>) {
    if seen.iter().any(|s| s == id) {
        return;
    }
    let Some(node) = nodes.iter().find(|n| n.id == id) else {
        println!("  ??? unknown node: {id}");
        return;
    };
    for p in &node.prerequisites {
        walk_path(p, nodes, seen);
    }
    seen.push(id.to_string());
    println!("  {} ({:?})", id, node.primary_unlock);
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = args.first().map(String::as_str).unwrap_or("help");
    let arg = args.get(1).map(String::as_str);

    let graph = build_recipe_graph();
    let nodes = load_ron_dir::<NodeDef>("assets/tech_nodes", "tech node");

    match (cmd, arg) {
        ("recipe", Some(id)) => match graph.recipes.get(id) {
            Some(r) => print_recipe(r),
            None => eprintln!("no recipe '{id}' (try `recipes`)"),
        },
        ("recipes", _) => {
            let mut ids: Vec<&String> = graph.recipes.keys().collect();
            ids.sort();
            for id in ids {
                println!("{id}");
            }
        }
        ("tech", Some(id)) => match nodes.iter().find(|n| n.id == id) {
            Some(n) => print_node(n),
            None => eprintln!("no tech node '{id}' (try `techs`)"),
        },
        ("techs", _) => {
            let mut ids: Vec<&String> = nodes.iter().map(|n| &n.id).collect();
            ids.sort();
            for id in ids {
                println!("{id}");
            }
        }
        ("path", Some(id)) => {
            println!("prerequisite chain for {id}:");
            walk_path(id, &nodes, &mut Vec::new());
        }
        ("uses", Some(item)) => {
            let produces: Vec<&String> = graph
                .recipes
                .iter()
                .filter(|(_, r)| {
                    r.outputs
                        .iter()
                        .chain(&r.byproducts)
                        .any(|s| s.item == item)
                })
                .map(|(id, _)| id)
                .collect();
            let consumes: Vec<&String> = graph
                .recipes
                .iter()
                .filter(|(_, r)| r.inputs.iter().any(|s| s.item == item))
                .map(|(id, _)| id)
                .collect();
            println!("produced by: {produces:?}");
            println!("consumed by: {consumes:?}");
        }
        _ => {
            eprintln!(
                "usage: cargo run --bin assets <cmd>\n  \
                 recipe <id> | recipes | tech <id> | techs | path <node> | uses <item>"
            );
        }
    }
}
