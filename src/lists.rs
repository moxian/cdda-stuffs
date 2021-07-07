use crate::db::Db;
use crate::schema;

#[allow(dead_code)]
pub(crate) fn drinks_stuff(db: &Db) {
    let mut maybe_drinks = vec![];
    for item in &db.raw {
        if item["type"] != "COMESTIBLE" {
            continue;
        }
        let item: crate::schema::Comestible =
            serde_json::from_value(item.clone()).expect(&format!("{:?}", item["id"]));
        if item.healthy <= 1 {
            continue;
        }
        if item.comestible_type != "DRINK" {
            // continue
        }
        maybe_drinks.push(item);
    }
    maybe_drinks.sort_unstable_by_key(|d| d.healthy);
    for d in maybe_drinks {
        println!(
            "{:>25}  {:5}  {:3}  {:3}",
            d.name.as_str(),
            d.calories,
            d.quench,
            d.healthy
        )
    }
}

fn extract_js_one<'a>(jo: &'a serde_json::Value, descr: &str) -> &'a serde_json::Value {
    let mut cur: &serde_json::Value = jo;
    for part in descr.split(".") {
        cur = &cur[part]
    }
    cur
}
fn extract_js_full<'a>(jo: &'a serde_json::Value, descr: &str) -> &'a serde_json::Value {
    let mut cur = &jo["nonexistent_key"];
    for part in descr.split("|") {
        cur = extract_js_one(jo, part);
        if !cur.is_null() {
            return cur;
        }
    }
    cur
    // panic!("empty description {:?}", descr);
}

fn show_json_value_plain(jv: &serde_json::Value) -> String {
    match jv {
        serde_json::Value::String(s) => s.to_string(),
        _ => serde_json::to_string(jv).unwrap(),
    }
}

#[derive(Debug, serde::Deserialize)]
struct BootsInput {
    covers_all: Vec<String>,
    covers_none: Vec<String>,
    columns: Vec<String>,
    sort_by: Vec<String>,
    #[serde(default)]
    need_flags: Vec<String>,
    #[serde(default)]
    covers_only: bool,
    #[serde(default)]
    allow_unobtainable: bool,
}

#[allow(dead_code)]
pub(super) fn boots_stuff(db: &Db) {
    let mut items = vec![];
    for item in &db.raw {
        if item["type"] != "ARMOR" {
            continue;
        }
        let item: schema::CataItem = serde_json::from_value(item.clone())
            .map_err(|e| panic!("{}:\n  {}", e, item))
            .unwrap();
        items.push(item.clone());
    }

    let input: BootsInput =
        json5::from_str(&std::fs::read_to_string("input/boots.json5").unwrap()).unwrap();
    #[derive(Clone, serde::Serialize)]
    struct Stat<T> {
        item: schema::CataItem,
        bash: i32,
        cut: i32,
        acid: i32,
        total: i32,
        encumbrance: i32,
        goodness: T,
    }
    let mut stats = vec![];
    'l: for item in items {
        let armor = item.as_armor();
        for cover in &input.covers_all {
            if !armor.covers.contains(&cover) {
                continue 'l;
            }
        }
        for cover in &input.covers_none {
            if armor.covers.contains(&cover) {
                continue 'l;
            }
        }
        if input.covers_only && armor.covers.len() > 1 {
            continue;
        }
        if !input
            .need_flags
            .iter()
            .all(|wf| item.header.flags.contains(wf))
        {
            continue;
        }
        if !input.allow_unobtainable && !db.is_obtainable(&item.header.id) {
            println!(
                "{} ({}) is unobtainable",
                item.header.id,
                item.header.name.as_str()
            );
            continue;
        }

        let armor = item.as_armor();
        let materials: Vec<&schema::Material> = armor
            .material
            .iter()
            .map(|mat| {
                db.materials
                    .iter()
                    .find(|dbm: &&schema::Material| &dbm.id == mat)
                    .unwrap()
            })
            .collect();
        let avg_bash =
            materials.iter().map(|m| m.bash_resist).sum::<i32>() as f32 / materials.len() as f32;
        let avg_cut =
            materials.iter().map(|m| m.cut_resist).sum::<i32>() as f32 / materials.len() as f32;
        let avg_acid =
            materials.iter().map(|m| m.acid_resist).sum::<i32>() as f32 / materials.len() as f32;

        let mut enc = armor.encumbrance;
        if item.header.flags.contains(&"VARSIZE".to_string()) {
            enc = enc / 2; // round down;
        } // this is not actually real but i want to sort easily

        let bash = (avg_bash * armor.material_thickness as f32).round() as i32;
        let cut = (avg_cut * armor.material_thickness as f32).round() as i32;
        // acid is funky
        // no mat thickness bonus for acid!
        let acid =
            (avg_acid * (armor.environmental_protection.min(10) as f32 / 10.0)).round() as i32;

        let enc_to_arm = enc * 100 / if (bash + cut) == 0 { 1 } else { bash + cut };
        // let goodness = ordered_float::OrderedFloat(enc_to_arm);
        if enc_to_arm > 100 {
            // continue;
        }
        let goodness = (acid, enc, enc_to_arm);
        let total = bash + cut + acid;

        stats.push(Stat {
            item,
            bash,
            cut,
            acid,
            total,
            encumbrance: enc,
            goodness,
        });
    }

    for sort_by in input.sort_by.as_slice().iter().rev() {
        if stats.len() == 0 {
            // idk
            continue;
        }
        match serde_json::to_value(stats[0].clone()).unwrap()[&sort_by] {
            serde_json::Value::Number(_) => {
                stats.sort_by_key(|s| {
                    ordered_float::OrderedFloat(
                        serde_json::to_value(s).unwrap()[&sort_by].as_f64().unwrap(),
                    )
                });
            }
            _ => stats.sort_by_key(|s| {
                serde_json::to_string(&serde_json::to_value(s).unwrap()[&sort_by]).unwrap()
            }),
        };
    }
    //

    let mut tbl = crate::table::Table::new();
    tbl.set_headers(row![
        "id", "name", "mats", "thick", "bash", "cut", "acid", "enc", "good",
    ]);
    tbl.set_headers(
        input
            .columns
            .iter()
            .map(|c| c.as_str().rsplit(".").next().unwrap().to_string())
            .collect::<Vec<_>>(),
    );
    for stat in stats {
        let stat_ser = serde_json::to_value(stat.clone()).unwrap();
        let mut row = vec![];
        for column_desc in &input.columns {
            let column = extract_js_full(&stat_ser, column_desc);
            row.push(show_json_value_plain(column));
        }
        // println!("{}", serde_json::to_string_pretty(&stat_ser).unwrap());
        tbl.add_row(row);

        // let item = stat.item;
        // let armor = item.as_armor();
        // tbl.add_row(row![
        //     item.header.id,
        //     item.header.name.as_str().to_string(),
        //     armor.material.join(","),
        //     armor.material_thickness,
        //     stat.bash,
        //     stat.cut,
        //     stat.acid,
        //     stat.encumbrance,
        //     format!("{:?}", stat.goodness),
        // ]);

        // println!(
        //     "{:>35} {:>23} {:>2} {:>2.0} {:>3.0} {:>2} {:>2} {:?}",
        //     item.header.name.as_str(),
        //     armor.material.join(","),
        // armor.material_thickness,
        // stat.bash,
        // stat.cut,
        // stat.acid,
        // stat.enc,
        // stat.goodness,
        // );
    }
    use std::io::Write;
    let mut out_file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open("out/boots.txt")
        .unwrap();
    out_file.write_all(tbl.format().as_bytes()).unwrap();
    // println!("{}", tbl.format());
}

fn attack_time(item: &schema::CataItem) -> Option<i32> {
    let time = (65.0
        + (item.header.volume?.ml as f32 / 62.5 + item.header.weight?.g as f32 / 60 as f32))
        as i32;
    return Some(time);
}
#[allow(dead_code)]
pub(crate) fn swords_stuff(db: &Db) {
    let mut times = vec![];
    for raw in &db.raw {
        if !["GENERIC", "TOOL"].contains(&raw["type"].as_str().unwrap()) {
            continue;
        }
        let item: schema::CataItem =
            serde_json::from_value(raw.clone()).unwrap_or_else(|e| panic!("{}:\n{:#}", e, raw));
        if item.header.category.as_deref() != Some("weapons") {
            continue;
        }
        let time = if let Some(t) = attack_time(&item) {
            t
        } else {
            continue;
        };
        times.push((item, time));
    }
    times.sort_by_key(|x| x.1);
    for (item, time) in times {
        println!(
            "{:>4} {:>4} {:>4} - {:?}",
            time,
            item.header.bashing,
            item.header.cutting,
            item.header.name.as_str()
        );
    }
}
