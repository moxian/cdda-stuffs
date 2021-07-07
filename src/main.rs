#[macro_use]
mod table;
mod db;
mod schema;

mod graphs;
mod lists;
mod plots;

// const CATA_ROOT: &'static str = r#"D:\games\cataclysm-dda\_this\"#;
const CATA_ROOT: &'static str = r#"D:\games\cataclysm-dda\launcher-autoupdated\"#;

fn main() {
    env_logger::from_env(env_logger::Env::default().default_filter_or("debug")).init();
    let db = &db::load_maybe_compressed();
    println!("loaded3");
    

    // lists::boots_stuff(db);
    // lists::drinks_stuff(db);
    // graphs::graphviz_all_inputs(db);
    graphs::train::train(db);
    // plots::belts(db);
    // lists::swords_stuff(db);
}
