use crate::db::Db;
use crate::schema;

use schema::Recipe;

type Map<K, V> = std::collections::HashMap<K, V>;

#[derive(serde::Deserialize, Debug)]
struct TrainInput {
    want_skill: String,
    forbidden_mats: Vec<String>,
    forbidden_meta: Vec<String>,
    skills: Map<String, i32>,
}

fn recipe_is_known(recipe: &Recipe, skills: &[(String, i32)]) -> bool {
    let need_skills = match &recipe.autolearn {
        schema::Autolearn::No => return false,
        schema::Autolearn::Yes => vec![(recipe.skill_used.clone(), recipe.difficulty)],
        schema::Autolearn::Complex(v) => v.clone(),
    };
    for (name, lvl) in skills {
        for (need_name, need_lvl) in &need_skills {
            if name != need_name {
                continue;
            };
            if lvl < need_lvl {
                return false;
            }
        }
    }
    return true;
}

#[allow(dead_code)]
pub(crate) fn train(db: &Db) {
    let input: TrainInput =
        json5::from_str(&std::fs::read_to_string("input/training.json5").unwrap()).unwrap();

    let mut forbidden_mats = super::to_node_list(&input.forbidden_mats, db);
    forbidden_mats.extend(
        input
            .forbidden_meta
            .iter()
            .map(|x| super::Node::RequirementItem(x.to_string())),
    );
    // let skills = vec![(input.want_skill.clone(), input.current_level)];
    let skills: Vec<_> = input.skills.clone().into_iter().collect();
    let current_level = input.skills[&input.want_skill].clone();

    let mut trainables = vec![];
    for recipe in &db.recipes {
        if recipe.result.starts_with("seed_") {
            continue;
        }
        if recipe.result.ends_with("_sharpened") {
            continue;
        }
        if recipe.skill_used != input.want_skill {
            continue;
        }
        if !recipe_is_known(recipe, &skills) {
            continue;
        }
        let recipe_cap = (recipe.difficulty as f32 * 1.25).floor() as i32;
        if recipe_cap < current_level {
            continue;
        }
        if super::has_mandatory_ingredient(&forbidden_mats, &recipe.components) {
            continue;
        }
        trainables.push(recipe);
    }

    trainables.sort_by_key(|x| x.time.to_seconds());
    let mut out_lines = Vec::new();
    for recipe in trainables {
        out_lines.push(format!(
            "{} ({}) {} - {:<20} - {}",
            recipe.time.to_seconds(),
            recipe.time.to_human(),
            if recipe.reversible { "*" } else { " " },
            recipe.result,
            super::get_item_name(db.lookup_item(&recipe.result).unwrap())
        ));
        for (skill, level) in &recipe.skills_required {
            out_lines.push(format!("      = {}+ {}", level, skill));
        }
        for r in &recipe.components {
            let words = r
                .iter()
                .map(|x| format!("{} x{}", x.name(), x.amount()))
                .collect::<Vec<_>>()
                .join(", ");
            out_lines.push(format!("  - {}", words));
        }
    }

    let mut out_file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open("out/train.txt")
        .unwrap();
    std::io::Write::write_all(&mut out_file, out_lines.join("\n").as_bytes()).unwrap();
}
