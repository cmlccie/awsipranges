use crate::core::config;
use crate::core::errors::Result;
use log::{info, warn};
use reqwest;
use std::fs;
use std::path::PathBuf;

/*-------------------------------------------------------------------------------------------------
  Blocking JSON APIs
-------------------------------------------------------------------------------------------------*/

pub fn get_json() -> Result<String> {
    let cache_path: PathBuf = config::AWS_IP_RANGES_CONFIG
        .get_string("cache_file")?
        .into();
    let cache_time = config::AWS_IP_RANGES_CONFIG
        .get_int("cache_time")?
        .try_into()?;

    info!("Cache file path {:?}", &cache_path);
    info!("Cache time {cache_time} seconds");

    if let Ok(_) = fs::canonicalize(&cache_path) {
        info!("Cache file exists");
        let elapsed = fs::metadata(&cache_path)?.modified()?.elapsed()?;
        if elapsed.as_secs() <= cache_time {
            info!("IP ranges cache is fresh; use cache");
            get_json_from_file()
        } else {
            info!("IP ranges cache is stale; refresh cache");
            if let Ok(json) = get_json_from_url() {
                info!("Successfully retrieved fresh IP Ranges JSON; update cache file");
                cache_json_to_file(&json)?;
                Ok(json)
            } else {
                warn!("Unable to retrieve fresh IP Ranges JSON data; use stale file cache");
                get_json_from_file()
            }
        }
    } else {
        info!("Cache file does not exist; get JSON from URL and attempt to cache the result");
        match get_json_from_url() {
            Ok(json) => {
                let _ = cache_json_to_file(&json);
                Ok(json)
            }
            Err(error) => Err(error),
        }
    }
}

fn get_json_from_url() -> Result<String> {
    let response = reqwest::blocking::get(config::AWS_IP_RANGES_CONFIG.get_string("url")?)?;
    Ok(response.text()?)
}

fn cache_json_to_file(json: &str) -> Result<()> {
    let cache_path: PathBuf = config::AWS_IP_RANGES_CONFIG
        .get_string("cache_file")?
        .into();

    // Ensure parent directories exist
    cache_path.parent().map(|parent| fs::create_dir_all(parent));

    Ok(fs::write(cache_path, json)?)
}

fn get_json_from_file() -> Result<String> {
    let cache_path: PathBuf = config::AWS_IP_RANGES_CONFIG
        .get_string("cache_file")?
        .into();
    Ok(fs::read_to_string(cache_path)?)
}

/*-------------------------------------------------------------------------------------------------
  Unit Tests
-------------------------------------------------------------------------------------------------*/

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::json;

    #[test]
    fn test_get_json_from_url() {
        let json = get_json_from_url();
        assert!(json.is_ok());
    }

    #[test]
    fn test_cache_json_to_file() {
        let json = get_json_from_url().unwrap();
        let result = cache_json_to_file(&json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_json_from_file() {
        let json_from_url = get_json_from_file().unwrap();
        cache_json_to_file(&json_from_url).unwrap();
        let json_from_file = get_json_from_file();
        assert!(json_from_file.is_ok());
    }

    #[test]
    fn test_parse_json() {
        let json = get_json_from_url().unwrap();
        let json_ip_ranges = json::parse(&json);
        assert!(json_ip_ranges.is_ok());
    }

    #[test]
    fn test_serialize_json_ip_ranges() {
        let json_from_url = get_json_from_url().unwrap();
        let json_ip_ranges = json::parse(&json_from_url).unwrap();
        let serialized_json = serde_json::to_string(&json_ip_ranges);
        assert!(serialized_json.is_ok());
    }
}
