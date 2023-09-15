use config::{Config, Environment};
use lazy_static::lazy_static;
use std::path::PathBuf;

/*-------------------------------------------------------------------------------------------------
  Configuration
-------------------------------------------------------------------------------------------------*/

lazy_static! {
    pub static ref AWS_IP_RANGES_CONFIG: Config = {
        let home_dir = dirs::home_dir().unwrap();
        let cache_file: PathBuf = [&home_dir.to_str().unwrap(), ".aws", "ip-ranges.json"]
            .iter()
            .collect();

        let config_builder = Config::builder()
            .set_default("url", "https://ip-ranges.amazonaws.com/ip-ranges.json")
            .unwrap()
            .set_default("cache_file", cache_file.to_str())
            .unwrap()
            .set_default("cache_time", 24 * 60 * 60)
            .unwrap()
            .add_source(Environment::with_prefix("AWS_IP_RANGES"));

        config_builder.build().unwrap()
    };
}
