pub(crate) mod train;

use crate::db::Db;
use crate::schema::{self, ComponentDesc, Recipe, Requirement};
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

fn to_node(from: impl AsRef<str>, db: &Db) -> Node {
    let f = from.as_ref();
    if db.lookup_item(f).is_some() {
        Node::Item(f.to_string())
    } else if let Some(it) = db.lookup_item_by_name(f) {
        let id = it["id"].as_str().unwrap();
        Node::Item(id.to_string())
    } else {
        panic!("Failed to find item {:?}", f)
    }
}

fn to_node_list(from: &[impl AsRef<str>], db: &Db) -> Vec<Node> {
    let mut nodes = vec![];
    for f in from {
        nodes.push(to_node(f, db))
    }
    nodes
}

#[derive(Hash, PartialEq, Eq, Clone, Debug, PartialOrd, Ord)]
enum Node {
    Item(String),
    RequirementItem(String),
}
impl Node {
    fn as_item(&self) -> Option<&String> {
        match self {
            Self::Item(i) => Some(i),
            _ => None,
        }
    }
    fn as_requirement_item(&self) -> Option<&String> {
        match self {
            Self::RequirementItem(i) => Some(i),
            _ => None,
        }
    }
    fn name(&self) -> &String {
        match self {
            Self::RequirementItem(i) => i,
            Self::Item(i) => i,
        }
    }
}
#[derive(Hash, PartialEq, Eq, Clone, Debug, PartialOrd, Ord)]
struct Edge {
    source: Node,
    dest: Node,
    recipe_suffix: Option<String>,
}
#[derive(Default)]
struct CraftableGraph {
    nodes: HashSet<Node>,
    edges: HashSet<Edge>,
}

impl Node {
    fn from_recipe(rec: &Recipe) -> Node {
        Node::Item(rec.result.to_string())
    }
    fn from_requirement(req: &Requirement) -> Node {
        Node::RequirementItem(req.id.to_string())
    }
}
impl Edge {
    fn from_recipe(node: &Node, rec: &Recipe) -> Edge {
        Edge {
            source: node.clone(),
            dest: Node::from_recipe(rec),
            recipe_suffix: rec.id_suffix.clone(),
        }
    }
    fn from_requirement(node: &Node, req: &Requirement) -> Edge {
        Edge {
            source: node.clone(),
            dest: Node::from_requirement(req),
            recipe_suffix: None,
        }
    }
}
// returns true if at least one of the items is required in crafting
fn has_mandatory_ingredient(item_ids: &[Node], components: &Vec<Vec<ComponentDesc>>) -> bool {
    let plain_items: Vec<&String> = item_ids.iter().map(Node::as_item).flatten().collect();
    let req_items: Vec<&String> = item_ids
        .iter()
        .map(Node::as_requirement_item)
        .flatten()
        .collect();
    for component in components.iter() {
        let all_bad = component.iter().all(|alternative: &ComponentDesc| -> bool {
            match alternative {
                ComponentDesc::Plain(s, _) => plain_items.contains(&s),
                ComponentDesc::List(s, _) => req_items.contains(&s),
            }
        });
        if all_bad {
            return true;
        }
    }
    return false;
}

fn node_matches_component_desc(node: &Node, desc: &ComponentDesc) -> bool {
    match (node, desc) {
        (Node::Item(ni), ComponentDesc::Plain(nc, _)) => ni == nc,
        (Node::RequirementItem(ni), ComponentDesc::List(nc, _)) => ni == nc,
        _ => false,
    }
}
// iff using items provided is sufficient to satisfy the requirements,
// then returns all the items usable in recipe
// Otherwise returns None
fn has_enough_components<'a>(
    item_ids: impl IntoIterator<Item = &'a Node> + Copy,
    components: &Vec<Vec<ComponentDesc>>,
) -> Option<Vec<&'a Node>> {
    let mut usables = vec![];
    for alternatives in components {
        let mut found_alt = false;
        for alternative in alternatives.iter() {
            for item_id in item_ids.into_iter() {
                if node_matches_component_desc(item_id, alternative) {
                    found_alt = true;
                    usables.push(item_id);
                }
            }
        }
        if !found_alt {
            return None;
        }
    }
    return Some(usables);
}

fn find_everything_craftable_from(
    from: &[String],
    recipes_db: &[Recipe],
    requirements_db: &[Requirement],
    blacklist: &[String],
    unobtainables: &[String],
) -> CraftableGraph {
    let mut nodes: HashSet<Node> = from
        .iter()
        .map(|x| Node::Item(x.clone()))
        .collect::<HashSet<_>>();
    let mut edges: HashSet<Edge> = HashSet::new();
    // speedup/cache
    let mut any_changes = true;

    let blacklist_items: Vec<_> = blacklist
        .iter()
        .map(|x| {
            vec![
                Node::Item(x.to_string()),
                Node::RequirementItem(x.to_string()),
            ]
        })
        .flatten()
        .collect();
    let mut unobtainables_items: Vec<_> = unobtainables
        .iter()
        .map(|x| {
            vec![
                Node::Item(x.to_string()),
                Node::RequirementItem(x.to_string()),
            ]
        })
        .flatten()
        .collect();
    for req in requirements_db.iter() {
        if has_mandatory_ingredient(&unobtainables_items, &req.components) {
            unobtainables_items.push(Node::RequirementItem(req.id.to_string()));
        }
    }
    // let unobtainables_items = vec![];

    let blacklist_qualities = &[schema::Quality {
        id: "CHEM".into(),
        level: 2,
    }];

    fn do_stuff<'a, CI>(
        components: CI,
        nodes: &mut HashSet<Node>,
        edges: &mut HashSet<Edge>,
        my_node: Node,
        my_recipe_suffix: &Option<String>,
        blacklist_items: &[Node],
    ) -> bool
    where
        CI: Iterator<Item = &'a Vec<ComponentDesc>>,
    {
        if blacklist_items.contains(&my_node) {
            return false;
        }
        // // hack
        // if match &my_node {
        //     Node::Item(s) => s,
        //     Node::RequirementItem(s) => s,
        // }
        // .contains("mutagen")
        // {
        //     return false;
        // }
        // check unobtainables

        for component in components {
            // if component.iter().all(|alternative| match alternative{

            // });
            for alternative in component {
                match alternative {
                    ComponentDesc::Plain(s, _) => {
                        let source_node = Node::Item(s.to_string());
                        if nodes.contains(&source_node) {
                            let edge_added = edges.insert(Edge {
                                source: source_node.clone(),
                                dest: my_node.clone(),
                                recipe_suffix: my_recipe_suffix.clone(),
                            });
                            if edge_added {
                                // println!("adding {:?} -> {:?}", source_node, my_node);
                                nodes.insert(my_node);
                                return true;
                            }
                        }
                    }
                    ComponentDesc::List(s, _) => {
                        let source_node = Node::RequirementItem(s.to_string());
                        if nodes.contains(&source_node) {
                            let edge_added = edges.insert(Edge {
                                source: source_node.clone(),
                                dest: my_node.clone(),
                                recipe_suffix: my_recipe_suffix.clone(),
                            });
                            if edge_added {
                                // println!("adding {:?} -> {:?}", source_node, my_node);
                                nodes.insert(my_node);

                                return true;
                            }
                        }
                    }
                }
            }
        }
        return false;
    }
    while any_changes {
        any_changes = false;
        for requirement in requirements_db.iter() {
            let my_node = Node::RequirementItem(requirement.id.to_string());
            let my_recipe_suffix = &None;
            any_changes |= do_stuff(
                requirement.components.iter(),
                &mut nodes,
                &mut edges,
                my_node,
                my_recipe_suffix,
                &blacklist_items,
            );
        }

        let is_craftable = |recipe: &Recipe| -> bool {
            for quality in &recipe.qualities {
                for b_quality in blacklist_qualities.iter() {
                    if quality.id == b_quality.id && quality.level >= b_quality.level {
                        return false;
                    }
                }
            }
            true
        };
        for recipe in recipes_db.iter() {
            if !is_craftable(recipe) {
                continue;
            };
            // println!("recipe {} ; {:?} ", recipe.result, unobtainables_items);
            if has_mandatory_ingredient(&unobtainables_items, &recipe.components) {
                continue;
            }
            let my_node = Node::Item(recipe.result.to_string());
            let my_recipe_suffix = &recipe.id_suffix;

            any_changes |= do_stuff(
                recipe.components.iter(),
                &mut nodes,
                &mut edges,
                my_node,
                my_recipe_suffix,
                &blacklist_items,
            );
        }
    }
    CraftableGraph { nodes, edges }
}

fn expand_pantry(from: &[Node], db: &Db, blacklist: &[String]) -> CraftableGraph {
    let mut any_new = true;
    let mut nodes: HashSet<Node> = from.into_iter().cloned().collect();
    let mut edges: HashSet<Edge> = Default::default();

    while any_new {
        any_new = false;
        for req in &db.requirements {
            if blacklist.contains(&req.id) {
                continue;
            }
            if let Some(usables) = has_enough_components(&nodes, &req.components) {
                let usables: Vec<Node> = usables.into_iter().cloned().collect();
                let node = Node::from_requirement(req);
                any_new |= nodes.insert(node);
                for usable in usables {
                    let edge = Edge::from_requirement(&usable, req);
                    any_new |= edges.insert(edge);
                }
            };
        }
        for rec in &db.recipes {
            if blacklist.contains(&rec.result) {
                continue;
            }
            if let Some(usables) = has_enough_components(&nodes, &rec.components) {
                let usables: Vec<Node> = usables.into_iter().cloned().collect();
                let node = Node::from_recipe(rec);
                any_new |= nodes.insert(node);
                for usable in usables {
                    let edge = Edge::from_recipe(&usable, rec);
                    any_new |= edges.insert(edge);
                }
            };
        }
    }
    let graph = CraftableGraph {
        nodes: nodes.into_iter().collect(),
        edges: edges.into_iter().collect(),
    };
    graph
}

fn extract_relevant_raw_db_items<'a, It: Iterator<Item = &'a Node>>(
    db: &Db,
    nodes_it: It,
) -> HashMap<Node, &serde_json::Value> {
    let mut raw_db_small = HashMap::<Node, &serde_json::Value>::new();

    for node in nodes_it {
        match node {
            Node::Item(node_name) => {
                let item = db
                    .lookup_item(node_name.as_str())
                    .unwrap_or_else(|| panic!("{:?} not found", node_name));
                raw_db_small.insert(node.clone(), item);
            }
            _ => {}
        }
    }
    raw_db_small
}

#[allow(dead_code)]
fn break_cycles(mut graph: CraftableGraph, stating_set: &[Node]) -> CraftableGraph {
    let mut seen_nodes: HashSet<&Node> = stating_set.iter().collect();
    let mut any_new = true;
    let mut bad_edges: Vec<Edge> = vec![];
    while any_new {
        any_new = false;
        for edge in &graph.edges {
            if !seen_nodes.contains(&edge.source) {
                continue;
            }
            if seen_nodes.contains(&edge.dest) {
                bad_edges.push(edge.clone());
                continue;
            }
            any_new |= seen_nodes.insert(&edge.dest);
        }
    }
    drop(seen_nodes);
    graph.edges.retain(|e| !bad_edges.contains(e));
    todo!("this is broken and removes everything");
    #[allow(unreachable_code)]
    graph
}

fn prune_irrelevant_nodes(
    mut graph: CraftableGraph,
    db: &Db,
    healthy_min: Option<i32>,
    restrict_types: Option<&[String]>,
    hide: &[String],
) -> CraftableGraph {
    let raw_db_small = extract_relevant_raw_db_items(db, graph.nodes.iter());

    let mut to_hide: HashSet<Node> = HashSet::new();
    for node in &graph.nodes {
        if hide.contains(node.name()) {
            to_hide.insert(node.clone());
        }
    }
    graph.nodes.retain(|n| !to_hide.contains(n));
    graph
        .edges
        .retain(|e| !to_hide.contains(&e.source) && !to_hide.contains(&e.dest));

    let mut any_removed = true;
    while any_removed {
        any_removed = false;
        let mut irrelevant_nodes = vec![];
        for node in graph.nodes.iter() {
            if matches!(node, Node::Item(..)) {
                let item = raw_db_small[node];
                if let Some(health_min) = healthy_min {
                    let healthy_here = item
                        .as_object()
                        .unwrap()
                        .get("healthy")
                        .map(|x| x.as_i64().unwrap())
                        .unwrap_or(0);
                    if healthy_here < health_min as i64 {
                        irrelevant_nodes.push(node.clone());
                    }
                }
                if let Some(restrict) = restrict_types {
                    let type_here = item["type"].as_str().unwrap().to_string();
                    if !restrict.contains(&type_here) {
                        irrelevant_nodes.push(node.clone());
                    }
                }
            } else {
                irrelevant_nodes.push(node.clone());
            }
        }
        irrelevant_nodes.sort();
        irrelevant_nodes.dedup();
        for node in irrelevant_nodes {
            let has_outgoing = graph.edges.iter().any(|edge| edge.source == node);
            if !has_outgoing {
                any_removed = true;
                assert!(graph.nodes.remove(&node));
                graph.edges.retain(|edge| edge.dest != node)
            }
        }
    }
    graph
}

fn parse_spoils_in_bad(time_v: &serde_json::Value) -> String {
    match time_v {
        serde_json::Value::String(s) => s.to_string(),
        serde_json::Value::Number(n) => {
            let hours = n.as_i64().unwrap();
            match hours {
                0..=23 => format!("{}h", hours),
                _ => format!("{}d", hours / 24),
            }
        }
        _ => panic!(),
    }
}

fn get_item_name(item: &serde_json::Value) -> String {
    let name: schema::Name = serde_json::from_value(item["name"].clone()).unwrap();
    name.to_string()
}

fn get_ingredient_multiplicity(
    components: &Vec<Vec<ComponentDesc>>,
    ingredient: &str,
) -> Option<i32> {
    for component in components {
        for alternative in component {
            if alternative.name() == ingredient {
                return Some(alternative.amount());
            }
        }
    }
    None
}

fn make_graphviz_one(db: &Db, out_path: impl AsRef<std::path::Path>, graph: &CraftableGraph) {
    use itertools::Itertools;
    use std::io::Write;
    let out_path = out_path.as_ref();
    let out_dir = out_path.parent().unwrap();
    std::fs::create_dir_all(out_dir).unwrap();
    let out_file = &mut std::fs::OpenOptions::new()
        .truncate(true)
        .write(true)
        .create(true)
        .open(out_path)
        .unwrap();

    write!(out_file, "digraph {{\n").unwrap();

    let node_to_id = |node: &Node| match node {
        Node::Item(i) => i.to_string(),
        Node::RequirementItem(i) => format!("*{}", i),
    };
    for node in graph.nodes.iter().sorted() {
        write!(out_file, "{:?} ", node_to_id(node)).unwrap();
        match node {
            Node::Item(i) => {
                let other_item = db.lookup_item(i).unwrap();

                // println!("{}", other_item.to_string());

                // println!("{}", i);
                let mut label_stuff = vec![node_to_id(node)];
                let mut other_attrs = vec![];
                let name = get_item_name(other_item);
                if &name != i {
                    label_stuff.push(name);
                }
                if let Some(spoil) = other_item.get("spoils_in") {
                    label_stuff.push(parse_spoils_in_bad(spoil));
                }
                if let Some(healthy) = other_item.get("healthy") {
                    label_stuff.push(format!("healty: {}", healthy));
                    if healthy.as_i64().unwrap() > 0 {
                        other_attrs.push("color=blue shape=rectangle");
                    }
                }

                write!(out_file, r"[").unwrap();
                write!(out_file, "label=\"{}\" ", label_stuff.join(r"\n")).unwrap();
                write!(out_file, "{}", other_attrs.join(" ")).unwrap();
                write!(out_file, r"]").unwrap();
            }
            _ => {}
        };
        write!(out_file, ";\n").unwrap();
    }

    // collect into a vec first to deduplicate identical edges
    let mut edges_serialized = vec![];
    for edge in graph.edges.iter() {
        let mut edge_serialized = Vec::new();
        write!(
            edge_serialized,
            "{:?} -> {:?} ",
            node_to_id(&edge.source),
            node_to_id(&edge.dest)
        )
        .unwrap();
        let mut attributes = vec![];
        match &edge.dest {
            Node::RequirementItem(id) => {
                attributes.push(r#" color="red" "#.to_string());
                let req = db.lookup_requirement(&id);
                let from_count =
                    get_ingredient_multiplicity(&req.components, edge.source.name()).expect(
                        &format!("source not found?? for {} in {:?}", edge.source.name(), id),
                    );
                if from_count != 1 {
                    attributes.push(format!(
                        r#"label="{}:1 x{:0.2}""#,
                        from_count,
                        1.0 / from_count as f32
                    ));
                }
            }
            Node::Item(id) => {
                let rec = db.lookup_recipe(&id, edge.recipe_suffix.as_deref());
                let result = db.lookup_item(&rec.result).unwrap();
                let from_count = get_ingredient_multiplicity(&rec.components, edge.source.name())
                    .expect("source not found??");
                let to_count = rec
                    .charges
                    .or(result.get("charges").map(|c| c.as_i64().unwrap() as i32))
                    .unwrap_or(1)
                    * rec.result_mult.unwrap_or(1);
                // let result = lookup_item(db, id);
                if from_count != to_count {
                    attributes.push(format!(
                        "label=\"{}:{} x{:0.2}\" ",
                        from_count,
                        to_count,
                        to_count as f32 / from_count as f32
                    ));
                }
            }
        };
        write!(edge_serialized, "[{}]", attributes.join(" ")).unwrap();
        write!(edge_serialized, ";\n").unwrap();
        edges_serialized.push(edge_serialized);
    }
    edges_serialized.sort();
    edges_serialized.dedup();
    for edge in edges_serialized {
        out_file.write(&edge).unwrap();
    }

    write!(out_file, "}}").unwrap();
    out_file.sync_all().unwrap();
    drop(out_file);
    println!("invoking dot");
    std::process::Command::new(r#"C:\soft\graphviz\bin\dot.exe"#)
        .arg(out_path)
        .arg("-Tsvg")
        .arg("-O")
        .spawn()
        .unwrap()
        .wait()
        .unwrap();
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    #[serde(default)]
    pantry: Vec<String>,
    #[serde(default)]
    have: Vec<String>,
    // blacklist from crafting
    #[serde(default)]
    blacklist: Vec<String>,
    // allow crafting, but remove from resulting graph (with subsequent pruning)
    #[serde(default)]
    hide: Vec<String>,
    healthy_min: Option<i32>,
    #[serde(default)]
    unobtainables: Vec<String>,
    #[serde(default)]
    show_multiplier: bool,
    restrict_type: Option<Vec<String>>,
}

#[allow(dead_code)]
pub(crate) fn graphviz_all_inputs(db: &Db) {
    for entry in std::fs::read_dir("input/graphs").unwrap() {
        let entry = entry.unwrap();
        let filename: PathBuf = entry.path().into();
        if filename.extension().map(|e| e.to_str()) != Some(Some("json5")) {
            continue;
        }
        // let f = std::fs::File::open(filename).unwrap();
        let input: Input = json5::from_str(&std::fs::read_to_string(&filename).unwrap()).unwrap();
        let mut out;
        if !input.pantry.is_empty() {
            let start = to_node_list(&input.pantry, db);
            out = expand_pantry(&start, db, &input.blacklist);
        // out = break_cycles(out, &start);
        } else {
            out = find_everything_craftable_from(
                &input.have,
                &db.recipes,
                &db.requirements,
                &input.blacklist,
                &input.unobtainables,
            );
        }
        println!("nodes: {}", out.nodes.len());
        // if let Some(health_min) = input.healthy_min {
        //     out = prune_health_negative(out, &db, health_min);
        // }

        out = prune_irrelevant_nodes(
            out,
            db,
            input.healthy_min,
            input.restrict_type.as_deref(),
            &input.hide,
        );

        let out_path = std::path::Path::new("out")
            .join(filename.file_name().unwrap().to_owned())
            .with_extension("gv");
        make_graphviz_one(&db, out_path, &out);
    }
}
