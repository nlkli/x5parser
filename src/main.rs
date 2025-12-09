use serde::{Deserialize, Serialize};
mod browser_utils;
mod db;
mod error;
mod parser;

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Config<'a> {
    pub db_path: Option<&'a str>,
    pub browser_executable: Option<&'a str>,
    pub cookies_store_path: Option<&'a str>,
    pub pyaterochka_stores_coord_path: Option<&'a str>,
    pub sleep_millis_for_each_catalog: Option<u64>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().collect::<Vec<_>>();
    let config_flag_pos = args.iter().position(|v| v == "-c");
    let config_path = config_flag_pos.and_then(|v| args.get(v+1));
    let config_json = config_path.and_then(|v| std::fs::read_to_string(v).ok()); 
    let config = config_json.as_ref()
        .and_then(|v| serde_json::from_str::<Config>(v).ok())
        .unwrap_or_default();
    let _ = db::init(config.db_path);
    println!("{:#?}", config);
    let parse_config = parser::pyaterochka::ParseConfig{ 
        browser_executable: config.browser_executable, 
        cookies_store_path: config.cookies_store_path, 
        pyaterochka_stores_coord_path: config.pyaterochka_stores_coord_path,
        sleep_millis_for_each_catalog: config.sleep_millis_for_each_catalog,
    };
    if let Err(e) = parser::pyaterochka::start_parsing(&parse_config).await {
        eprintln!("Error: {e}");
    }
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    Ok(())
}
