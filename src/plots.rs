use crate::db::Db;
use crate::schema;

use schema::Volume;

fn total_storage(item: &schema::Armor) -> Volume {
    let mut total = Volume::default();
    for pocket in &item.pocket_data {
        if let schema::PocketData::Normal(pocket) = pocket {
            total.ml += pocket.max_contains_volume.ml;
        }
    }
    total
}

fn enc_at_full(item: &schema::CataItem) -> i32 {
    let armor = item.as_armor();
    let mut base = if let Some(max) = armor.max_encumbrance {
        max
    } else {
        armor.encumbrance + total_storage(armor).ml / 250
    };
    if item.header.flags.contains(&"VARSIZE".to_string()) {
        base /= 2;
    }
    return base ;
}

fn enc_at_empty(item: &schema::CataItem) -> i32 {
    let armor = item.as_armor();
    let mut base = armor.encumbrance;
    if item.header.flags.contains(&"VARSIZE".to_string()) {
        base /= 2;
    }
    return base;
}

fn plot_belts(belts: &[&schema::CataItem], input: &BeltsInput) {
    // plot_stuffs( belts, "belts", 0f32..10f32, 0f32..7f32, true);
    plot_stuffs(belts, "belts", false, input).unwrap();
}

fn plot_stuffs(
    stuffs: &[&schema::CataItem],
    out: &str,
    extra_lines: bool,
    input: &BeltsInput
) -> Result<(), Box<dyn std::error::Error>> {
    use plotters::prelude::*;
    let out = "out/".to_string() + out + ".svg";
    let root = SVGBackend::new(&out, (800, 900)).into_drawing_area();
    root.fill(&WHITE)?;

    let (root_a, root_b) = root.split_vertically(600);

    let mut max_volume = 0.1;
    let mut max_relenc = 0.1;
    let mut max_denc = 0.1;
    let mut max_enc = 0.1;
    for stuff in stuffs {
        let armor = stuff.as_armor();
        let holds_l = total_storage(armor).ml as f32 / 1000.0;
        let e2 = enc_at_full(stuff) as f32;
        let denc = (e2 as f32 - armor.encumbrance as f32) / holds_l;
        let enc_per_l_at_full = e2 / holds_l;
        if holds_l > max_volume {
            max_volume = holds_l
        }
        if enc_per_l_at_full > max_relenc {
            max_relenc = enc_per_l_at_full
        };
        if extra_lines && armor.encumbrance as f32 > max_relenc {
            max_relenc = armor.encumbrance as f32;
        }
        if e2 > max_enc {
            max_enc = e2;
        }
        if denc > max_denc {
            max_denc = denc
        }
    }

    const MARGIN_FACTOR: f32 = 1.1;
    let mut chart_a = ChartBuilder::on(&root_a)
        // .caption("y=x^2", ("sans-serif", 50).into_font())
        .margin(5)
        .x_label_area_size(30)
        .y_label_area_size(30)
        .build_cartesian_2d(
            0f32..(max_volume * MARGIN_FACTOR),
            0.9f32..(max_denc * MARGIN_FACTOR),
        )?;

    let mut chart_b = ChartBuilder::on(&root_b)
        // .caption("y=x^2", ("sans-serif", 50).into_font())
        .margin(5)
        .x_label_area_size(30)
        .y_label_area_size(30)
        .build_cartesian_2d(
            0f32..(max_volume * MARGIN_FACTOR),
            0f32..(max_enc * MARGIN_FACTOR),
        )?;

    chart_a.configure_mesh().draw()?;
    chart_b.configure_mesh().draw()?;

    for (i, stuff) in stuffs.iter().enumerate() {
        
        // if belt.header.name.as_str() != "tool belt"{
        //     continue
        // }
        // aaaa ! They all get enc = volume/250ml!
        // let min_enc = belt.as_armor().encumbrance();
        let name = stuff.header.name.as_str();
        let armor = stuff.as_armor();
        let at_empty = enc_at_empty(stuff);
        let at_full = enc_at_full(stuff);
        let holds_ml = total_storage(armor).ml;
        let holds_l = holds_ml as f32 / 1000.0;
        let color = {
            // let mut s = std::collections::hash_map::DefaultHasher::new();
            // use std::hash::{Hash, Hasher};
            // name.hash(&mut s);
            // let h = s.finish();
            // Palette99::pick(h as usize) // yikes..
            Palette99::pick(i)
        };
        let enc_per_l_at_full = at_full as f32 / holds_l;
        let enc_per_l_delta = (at_full - at_empty) as f32 / holds_l as f32;
        // dbg!(enc_per_l_delta);

        if extra_lines {
            chart_a.draw_series(LineSeries::new(
                vec![(0.0, at_empty as f32), (holds_l, enc_per_l_at_full)],
                &color,
            ))?;
        }
        let labels = input.labels;
        chart_a
            .draw_series(PointSeries::of_element(
                vec![(holds_l, enc_per_l_delta)],
                5,
                ShapeStyle::from(&color).filled(),
                &|coord, size, style| {
                    EmptyElement::at(coord)
                        + Circle::new((0, 0), size, style)
                        + Text::new(
                            if labels { format!("{}", name) } else { "".to_string() },
                            (-15, 15),
                            ("sans-serif", 15),
                        )
                },
            ))?
            // .label(belt.header.name.as_str())
            // .legend(move |(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &Palette99::pick(i)))
            ;

        chart_b
            .draw_series(LineSeries::new(
                vec![(0.0, at_empty as f32), (holds_l, at_full as f32)],
                &color,
            ))?
            .label(name)
            .legend(move |(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &color));
    }

    chart_a
        .configure_series_labels()
        .background_style(&TRANSPARENT)
        // .border_style(&BLACK)
        .draw()?;

    chart_b
        .configure_series_labels()
        .background_style(&TRANSPARENT)
        // .border_style(&BLACK)
        .draw()?;
    Ok(())
}

// fn plot_belts(belts: &[&schema::CataItem]) -> Result<(), Box<dyn std::error::Error>> {
//     Ok(())
// }

#[derive(Debug, serde::Deserialize)]
struct BeltsInput {
    whitelist: Vec<String>,
    pocket_flag_any: Vec<String>,
    labels: bool,
}

#[allow(dead_code)]
pub(crate) fn belts(db: &Db) {
    let input: BeltsInput =
        json5::from_str(&std::fs::read_to_string("input/belts.json5").unwrap()).unwrap();

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

    let mut belts = vec![];
    for item in &items {
        let armor = item.as_armor();
        if total_storage(armor).ml == 0 {
            continue;
        }
        if input.pocket_flag_any.len() > 0 {
            let has_matching_pockets = armor
                .pocket_data
                .iter()
                .filter_map(|p: &schema::PocketData| p.as_normal())
                // .next().is_some();
                .any(|n: &schema::PocketNormal| {
                    n.flag_restriction
                        .iter()
                        .any(|f| input.pocket_flag_any.contains(&f))
                });
            if !has_matching_pockets {
                continue;
            }
        }
        if input.whitelist.len() > 0 {
            if !input
                .whitelist
                .contains(&item.header.name.as_str().to_string())
            {
                // println!("hi? {}", item.header.name.as_str());
                continue;
            }
        }
        if total_storage(armor).ml <= 1_000 {
            // continue;
        };
        if enc_at_full(item) as f32 / (total_storage(armor).ml as f32 / 1000.0) > 0.8 {
            // continue;
        }
        let holds_l = total_storage(armor).ml as f32 / 1000.0;
        let denc = (enc_at_full(item) - enc_at_empty(item)) as f32 / holds_l;
        if enc_at_full(item) == enc_at_empty(item){
            continue
        }
        if denc < 1.0 || denc >= 4.0 || holds_l > 10.0 {
            continue;
        }
        if !item.header.name.as_str().contains("survivor"){
            // continue
        }
        // println!("{} {} {}", denc, enc_at_full(item), enc_at_empty(item));
        belts.push(item);
    }

    plot_belts(&belts, &input);
}
