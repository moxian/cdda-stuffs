use crate::schema::{self, Material, Recipe, Requirement};
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

pub(crate) struct Db {
    pub raw: Vec<serde_json::Value>,
    pub recipes: Vec<Recipe>,
    pub requirements: Vec<Requirement>,
    pub materials: Vec<Material>,
    pub itemgroups: Vec<schema::ItemGroup>,
}

#[allow(dead_code)]
pub(crate) fn make_db_full() -> Db {
    let db = load_db_flat(crate::CATA_ROOT);
    Db {
        recipes: parse_recipes(&db),
        requirements: parse_requirements(&db),
        materials: parse_materials(&db),
        itemgroups: parse_itemgroups(&db),
        raw: db,
    }
}

fn load_db_flat<P: AsRef<std::path::Path>>(cata_root: P) -> Vec<serde_json::Value> {
    let json_root = cata_root.as_ref().join("data").join("json");

    // only because we get id collisions
    const BLACKLISTED_TYPES: &[&'static str] = &[
        "ascii_art",
        "ammo_effect",
        "json_flag",
        "clothing_mod",
        "harvest",
        "effect_type",
        "MIGRATION",
        "ammunition_type",
        "vehicle_part",
        "ITEM_CATEGORY",
        "item_action",
        "overmap_land_use_code",
        "overmap_location",
        "overmap_terrain",
    ];
    // because speed; temporary; works even without
    const WHITELISTED_TYPES: &[&'static str] = &["recipe", "requirement"];
    const WHITELISTED_DIRS: &[&'static str] = &["recipes", "requirements"];
    const USE_WHITELIST: bool = false;

    // read files
    let mut deserialized_raw = vec![];
    for entry in walkdir::WalkDir::new(json_root) {
        let entry = entry.unwrap();
        let filename: PathBuf = entry.path().into();
        if filename.extension().map(|e| e.to_str()) != Some(Some("json")) {
            // println!("skipping {}", filename.display());
            continue;
        }
        if USE_WHITELIST
            && !filename.components().any(|c| {
                WHITELISTED_DIRS.contains(&c.as_os_str().to_string_lossy().to_string().as_str())
            })
        {
            // println!("skipping {:?}", filename);
            continue;
        }
        // debug!("reading {:?}", filename);
        let file = std::io::BufReader::new(std::fs::File::open(&filename).unwrap());
        let deserialized: serde_json::Value = serde_json::from_reader(file).unwrap();
        let deserialized = match deserialized {
            serde_json::Value::Array(a) => a,
            x @ serde_json::Value::Object(..) => vec![x],
            _ => unimplemented!(),
        };
        deserialized_raw.extend(
            deserialized
                .into_iter()
                .filter(|des| !BLACKLISTED_TYPES.contains(&des["type"].as_str().unwrap()))
                .filter(|des| des.get("obsolete").and_then(|x| x.as_bool()) != Some(true))
                .filter(|des| {
                    !USE_WHITELIST || WHITELISTED_TYPES.contains(&des["type"].as_str().unwrap())
                })
                .filter(|des| {
                    const BLACKLISTED_RECIPE_IDS: &[(&str, Option<&str>)] = &[
                        ("icecream_choc", Some("from_bags")),
                        ("sleeveless_duster_faux_fur", None),
                        ("crude_picklock", None), // this one is legit, but i don't want to support it yet
                        ("crude_picklock", Some("from wire")),
                    ];
                    if des["type"] != "recipe" {
                        return true;
                    };
                    !BLACKLISTED_RECIPE_IDS.contains(&(
                        des["result"].as_str().unwrap(),
                        des.get("id_suffix").and_then(|x| x.as_str()),
                    ))
                })
                .filter(|des| {
                    const BLACKLISTED_PLAIN_IDS: &[&str] = &["debug_backpack"];
                    if let Some(id) = des.get("id").and_then(|id| id.as_str()) {
                        if BLACKLISTED_PLAIN_IDS.contains(&id) {
                            return false;
                        }
                    }
                    true
                }),
        );
    }

    // water is in data/core/basic.json
    deserialized_raw.push(serde_json::json!({
        "type": "COMESTIBLE",
        "id": "water",
        "name": "water",
        "description": "get me from data/core/basic.json",

        "comestible_type": "DRINK",
        "quench": 60,
    }));
    // build id->[indexes] mapping
    let mut id_map = HashMap::<String, Vec<usize>>::new();
    let mut abstract_map = HashMap::new();
    let mut recipe_map = HashMap::<String, usize>::new();
    for (index, entry) in deserialized_raw.iter().enumerate() {
        if let Some(id) = entry["id"].as_str().map(|s| s.to_owned()) {
            id_map.entry(id).or_default().push(index);
        }
        if let Some(abstract_id) = entry["abstract"].as_str().map(|s| s.to_owned()) {
            abstract_map.insert(abstract_id, index);
        }
        if let Some(result) = entry
            .get("result")
            .and_then(|x| x.as_str())
            .map(|s| s.to_owned())
        {
            if entry.get("id_suffix").is_none() {
                recipe_map.insert(result, index);
            }
        }
    }

    // resolve copy-from
    fn resolve_copy_from(
        entry: &serde_json::Value,
        db: &[serde_json::Value],
        id_map: &HashMap<String, Vec<usize>>,
        abstract_map: &HashMap<String, usize>,
        recipe_map: &HashMap<String, usize>,
    ) -> serde_json::Value {
        let entry_o = entry.as_object().unwrap();
        let typ = entry_o["type"].as_str().unwrap().to_owned();
        let from_id = entry_o.get("copy-from");
        // return as is if no copy-from
        let from_id = match from_id {
            Some(x) => x,
            None => return entry.clone(),
        }
        .as_str()
        .unwrap();
        // water is special
        if from_id == "water" {
            return entry.clone();
        }
        // println!("searching for {:?}", from_id);

        let obj_index = if typ == "recipe" {
            *recipe_map
                .get(from_id)
                .or_else(|| panic!("not found recipe {:?} for copy from", from_id))
                .unwrap()
        } else {
            let obj_indexes: &[usize] = id_map.get(from_id).map(|x| x.as_ref()).unwrap_or(&[]);
            let find_obj_index = || {
                match obj_indexes.len() {
                    0 => {
                        return abstract_map
                            .get(from_id)
                            .ok_or_else(|| {
                                format!(
                                    "failed to find id {:?} for copy-from, while working on:\n{:?}",
                                    from_id, entry
                                )
                            })
                            .unwrap()
                            .clone()
                    }
                    1 => return obj_indexes[0],
                    _ => {}
                };

                let candidates = obj_indexes
                    .iter()
                    .map(|ind| (ind, &db[*ind]))
                    .collect::<Vec<_>>();
                let same_cat = candidates
                    .iter()
                    .filter(|(_ind, c)| c["type"] == typ)
                    .collect::<Vec<_>>();
                if let &[single] = same_cat.as_slice() {
                    return *single.0;
                }

                if from_id == "bone" && typ == "GENERIC" {
                    let bone = candidates
                        .iter()
                        .filter(|(_ind, c)| c["type"] == "material")
                        .collect::<Vec<_>>();
                    assert_eq!(bone.len(), 1);
                    return *bone[0].0;
                }
                panic!(
                    "too few/many candidates!\n{:?}\n out of\n{}\nsearching for {:?}\nin in{:?}",
                    same_cat,
                    {
                        candidates
                            .iter()
                            .map(|x| format!("   - {:?}", x))
                            .collect::<Vec<_>>()
                            .join("\n")
                    },
                    from_id,
                    typ,
                );
            };
            let obj_index = find_obj_index();
            obj_index
        };

        let base_obj = &db[obj_index];
        let mut working_obj = resolve_copy_from(base_obj, db, id_map, abstract_map, recipe_map);
        for key in vec!["copy-from", "id", "abstract"] {
            working_obj.as_object_mut().unwrap().remove(key);
        }
        for (key, value) in entry_o.iter() {
            working_obj
                .as_object_mut()
                .unwrap()
                .insert(key.clone(), value.clone());
        }

        working_obj
    }
    let mut resolveds = Vec::with_capacity(deserialized_raw.len());
    for entry in deserialized_raw.iter() {
        let resolved = resolve_copy_from(
            entry,
            &deserialized_raw,
            &id_map,
            &abstract_map,
            &recipe_map,
        );
        resolveds.push(resolved)
    }

    // docs say that abstracts get removed after load, so let's remove them
    resolveds.retain(|entry| entry.get("abstract").is_none());

    resolveds
}

fn parse_recipes(db: &[serde_json::Value]) -> Vec<Recipe> {
    let mut recipes = vec![];
    for item in db {
        if item["type"] != "recipe" {
            continue;
        }
        // faction bases thingies
        if item["category"] == "CC_BUILDING" {
            continue;
        }

        let mut item = item.clone();
        // normalize the "using" part first
        let usings: Vec<schema::ComponentDesc> = match item.as_object_mut().unwrap().remove("using")
        {
            Some(using) => serde_json::from_value(using)
                .map_err(|e| format!("the \"using\" part: {}: {}", e, item))
                .unwrap(),
            None => vec![],
        };
        let usings = usings.into_iter().map(|cd| {
            use schema::ComponentDesc::*;
            match cd {
                Plain(s, i) => (s, i),
                List(s, i) => (s, i),
            }
        });
        let usings_as_component: Vec<Vec<(String, i32, &str)>> = usings
            .into_iter()
            .map(|(s, i)| vec![(s, i, "LIST")])
            .collect();
        let now_as_values = usings_as_component
            .into_iter()
            .map(|x| serde_json::to_value(x).unwrap());
        item.as_object_mut()
            .unwrap()
            .entry("components")
            .or_insert(serde_json::json!([]))
            .as_array_mut()
            .unwrap()
            .extend(now_as_values);

        let parsed: Recipe = serde_json::from_value(item.clone())
            .map_err(|e| panic!("{}:\n{:#?}", e, item))
            .unwrap();

        recipes.push(parsed);
    }

    // canned recipes are bad, mkay
    recipes.retain(|r: &Recipe| {
        ![Some("jarred_3l"), Some("jarred"), Some("canned")].contains(&r.id_suffix.as_deref())
    });

    // having two recipes with the same result and no id_suffix fucks things up, so remove all of those
    {
        let mut seen_ids = HashSet::new();
        let mut bads = HashSet::new();
        for recipe in recipes.iter() {
            let new = seen_ids.insert((recipe.result.as_str(), recipe.id_suffix.as_ref()));
            if !new {
                bads.insert((recipe.result.clone(), recipe.id_suffix.clone()));
            }
        }
        recipes.retain(|r: &Recipe| !bads.contains(&(r.result.clone(), r.id_suffix.clone())));
    }

    recipes
}

fn parse_requirements(db: &[serde_json::Value]) -> Vec<Requirement> {
    let mut requirements = vec![];
    for item in db {
        if item["type"] != "requirement" {
            continue;
        }
        let parsed: Requirement = serde_json::from_value(item.clone())
            .map_err(|e| panic!("on {}\ngot: {:?}", item.to_string(), e))
            .unwrap();
        requirements.push(parsed);
    }
    requirements
}

fn parse_materials(db: &[serde_json::Value]) -> Vec<schema::Material> {
    let mats = db
        .iter()
        .filter(|i| i["type"] == "material")
        .cloned()
        .map(|i| {
            serde_json::from_value(i.clone())
                .map_err(|e| panic!("{}: {:?}", e, i))
                .unwrap()
        })
        .collect();
    mats
}

fn parse_itemgroups(db: &[serde_json::Value]) -> Vec<schema::ItemGroup> {
    let mats = db
        .iter()
        .filter(|i| i["type"] == "item_group")
        .filter(|i| i["subtype"] != serde_json::json!("collection"))
        .cloned()
        .map(|i| {
            // println!("{}", serde_json::to_string_pretty(&i).unwrap());
            serde_json::from_value(i).unwrap()
        })
        .collect();
    mats
}

impl Db {
    pub fn lookup_item<'a>(&'a self, id: &str) -> Option<&'a serde_json::Value> {
        // is an item - not a requirement
        let candidates = self
            .raw
            .iter()
            .filter(|item| {
                let mut ok = true;
                let typ = item
                    .as_object()
                    .unwrap()
                    .get("type")
                    .unwrap()
                    .as_str()
                    .unwrap();

                ok = ok && typ != "requirement";
                ok = ok && typ != "material";
                ok = ok && item.as_object().unwrap().get("id").and_then(|x| x.as_str()) == Some(id);
                ok
            })
            .collect::<Vec<_>>();
        assert!(candidates.len() <= 1, "{:?} - {:#?}", id, candidates);
        candidates.get(0).cloned()
    }

    pub fn lookup_item_by_name<'a>(&'a self, want_name: &str) -> Option<&'a serde_json::Value> {
        // is an item - not a requirement
        let candidates = self
            .raw
            .iter()
            .filter(|item| {
                let mut ok = true;
                ok = ok
                    && item
                        .as_object()
                        .unwrap()
                        .get("type")
                        .unwrap()
                        .as_str()
                        .unwrap()
                        != "requirement";
                let item_name: Option<schema::Name> =
                    serde_json::from_value(item["name"].clone()).ok();
                ok = ok && item_name.map(|x| x.to_string()).as_deref() == Some(want_name);
                ok
            })
            .collect::<Vec<_>>();
        assert!(candidates.len() <= 1, "{:?} - {:#?}", want_name, candidates);
        candidates.get(0).cloned()
    }

    pub fn lookup_recipe<'a>(&'a self, result: &str, id_suffix: Option<&str>) -> &'a Recipe {
        // is an item - not a requirement
        let candidates = self
            .recipes
            .iter()
            .filter(|rec| rec.result == result && rec.id_suffix.as_deref() == id_suffix)
            .collect::<Vec<_>>();
        assert!(candidates.len() == 1, "{:?} - {:#?}", result, candidates);
        let item = candidates[0];
        item
    }

    pub fn lookup_requirement<'a>(&'a self, id: &str) -> &'a Requirement {
        // is an item - not a requirement
        let candidates = self
            .requirements
            .iter()
            .filter(|req| req.id == id)
            .collect::<Vec<_>>();
        assert!(candidates.len() == 1, "{:?} - {:#?}", id, candidates);
        let item = candidates[0];
        item
    }

    pub fn is_obtainable(&self, id: &str) -> bool {
        for rec in self.recipes.iter() {
            if rec.result == id {
                return true;
            }
        }
        for itemgroup in self.itemgroups.iter() {
            if itemgroup
                .items
                .iter()
                .any(|(item, _)| item.typ == schema::ItemGroupItemType::Item && item.id == id)
            {
                return true;
            }
        }
        false
    }
}

const COMPRESSED_PATH: &'static str = "cache/db.json";

#[allow(dead_code)]
pub(crate) fn dump_compressed(db: &Db) {
    let out = std::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(COMPRESSED_PATH)
        .unwrap();
    serde_json::to_writer(out, &db.raw).unwrap();
}

#[allow(dead_code)]
pub(crate) fn load_compressed() -> Option<Db> {
    let inp = std::io::BufReader::new(
        std::fs::OpenOptions::new()
            .read(true)
            .open(COMPRESSED_PATH)
            .ok()?,
    );
    let db: Vec<serde_json::Value> = serde_json::from_reader(inp).unwrap();
    Some(Db {
        recipes: parse_recipes(&db),
        requirements: parse_requirements(&db),
        materials: parse_materials(&db),
        itemgroups: parse_itemgroups(&db),
        raw: db,
    })
}

#[allow(dead_code)]
pub(crate) fn load_maybe_compressed() -> Db {
    if let Some(c) = load_compressed() {
        return c;
    }
    println!("Cached db does not exist. Rebuilding");
    let base = make_db_full();
    dump_compressed(&base);
    println!("Cache rebuilt");
    //
    return load_compressed().unwrap();
}
