use crate::core::awsipranges::AwsIpRanges;
use crate::core::errors::Result;
use log::{info, warn};
use reqwest;
use std::env;
use std::fs;
use std::path::PathBuf;

/*-------------------------------------------------------------------------------------------------
  Primary Interface
-------------------------------------------------------------------------------------------------*/

/// Retrieves and parses the AWS IP Ranges, returning a boxed [AwsIpRanges] object that
/// allows you to quickly query ([search](AwsIpRanges::search()), [filter](AwsIpRanges::filter()),
/// etc.) the AWS IP Ranges data.
pub fn get_ranges() -> Result<Box<AwsIpRanges>> {
    BlockingClient::new().get_ranges()
}

/*-------------------------------------------------------------------------------------------------
  Blocking Client
-------------------------------------------------------------------------------------------------*/

pub struct BlockingClient {
    url: String,
    cache_file: PathBuf,
    cache_time: u64,
}

impl BlockingClient {
    pub fn new() -> Self {
        // Default the cache file path to ${HOME}/.aws/ip-ranges.json
        let mut default_cache_file = dirs::home_dir().unwrap();
        default_cache_file.push(".aws");
        default_cache_file.push("ip-ranges.json");

        Self {
            url: env::var("AWSIPRANGES_URL")
                .unwrap_or("https://ip-ranges.amazonaws.com/ip-ranges.json".to_string()),
            cache_file: env::var("AWSIPRANGES_CACHE_FILE")
                .ok()
                .and_then(|env_var| PathBuf::try_from(env_var).ok())
                .unwrap_or(default_cache_file),
            cache_time: env::var("AWSIPRANGES_CACHE_TIME")
                .ok()
                .and_then(|env_var| env_var.parse::<u64>().ok())
                .unwrap_or(24 * 60 * 60), // 24 hours
        }
    }

    /// Retrieves, parses, and returns boxed [AwsIpRanges]. Uses locally cached JSON
    /// data, when available and fresh. Requests the AWS IP Ranges JSON data from the
    /// URL when the local cache is stale or unavailable.
    pub fn get_ranges(self: &Self) -> Result<Box<AwsIpRanges>> {
        let json = self.get_json()?;
        AwsIpRanges::from_json(&json)
    }

    /// Set the URL used to retrieve the AWS IP Ranges; defaults to
    /// `https://ip-ranges.amazonaws.com/ip-ranges.json` - see
    /// [AWS IP address ranges](https://docs.aws.amazon.com/vpc/latest/userguide/aws-ip-ranges.html)
    /// in the Amazon Virtual Private Cloud (VPC) User Guide for details.
    pub fn url<'s>(&'s mut self, url: &str) -> &'s Self {
        self.url = url.to_string();
        self
    }

    /// Set the file path used to cache the AWS IP Ranges JSON data; defaults
    /// to `${HOME}/.aws/ip-ranges.json`.
    pub fn cache_file<'s>(&'s mut self, cache_file: &PathBuf) -> &'s Self {
        self.cache_file = cache_file.clone();
        self
    }

    /// Set the cache-time durration - the amount of time (in seconds) the
    /// locally cached AWS IP Ranges JSON data is considered fresh; defaults to
    /// 24 hours (`86400` seconds). When the elapsed time (the difference
    /// between the current system time and cache file's modified timestamp) is
    /// greater than the configured `cache_time`, calls to `get_ranges()` will
    /// attempt to refresh the cached JSON data from the AWS IP Ranges URL.
    pub fn cache_time<'s>(&'s mut self, cache_time: u64) -> &'s Self {
        self.cache_time = cache_time;
        self
    }

    fn get_json(self: &Self) -> Result<String> {
        info!("Cache file path {:?}", &self.cache_file);
        info!("Cache time {} seconds", self.cache_time);

        if let Ok(_) = fs::canonicalize(&self.cache_file) {
            info!("Cache file exists");
            let elapsed = fs::metadata(&self.cache_file)?.modified()?.elapsed()?;
            if elapsed.as_secs() <= self.cache_time {
                info!("IP ranges cache is fresh; use cache");
                self.get_json_from_file()
            } else {
                info!("IP ranges cache is stale; refresh cache");
                if let Ok(json) = self.get_json_from_url() {
                    info!("Successfully retrieved fresh IP Ranges JSON; update cache file");
                    self.cache_json_to_file(&json)?;
                    Ok(json)
                } else {
                    warn!("Unable to retrieve fresh IP Ranges JSON data; use stale file cache");
                    self.get_json_from_file()
                }
            }
        } else {
            info!("Cache file does not exist; get JSON from URL and attempt to cache the result");
            match self.get_json_from_url() {
                Ok(json) => {
                    let _ = self.cache_json_to_file(&json);
                    Ok(json)
                }
                Err(error) => Err(error),
            }
        }
    }

    fn get_json_from_url(self: &Self) -> Result<String> {
        let response = reqwest::blocking::get(&self.url)?;
        Ok(response.text()?)
    }

    fn cache_json_to_file(self: &Self, json: &str) -> Result<()> {
        // Ensure parent directories exist
        self.cache_file
            .parent()
            .map(|parent| fs::create_dir_all(parent));

        Ok(fs::write(&self.cache_file, json)?)
    }

    fn get_json_from_file(self: &Self) -> Result<String> {
        Ok(fs::read_to_string(&self.cache_file)?)
    }
}

/*-------------------------------------------------------------------------------------------------
  Unit Tests
-------------------------------------------------------------------------------------------------*/

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::json;

    #[test]
    fn test_get_ranges_function() {
        let aws_ip_ranges = get_ranges();
        assert!(aws_ip_ranges.is_ok());
    }

    #[test]
    fn test_client_get_ranges() {
        let client = BlockingClient::new();
        let aws_ip_ranges = client.get_ranges();
        assert!(aws_ip_ranges.is_ok());
    }

    #[test]
    fn test_set_url() {
        let mut client: BlockingClient = BlockingClient::new();
        client.url("https://ip-ranges.amazonaws.com/ip-ranges.json");
        let aws_ip_ranges = client.get_ranges();
        assert!(aws_ip_ranges.is_ok());
    }

    #[test]
    fn test_set_cache_file() {
        let test_cach_file: PathBuf = [".", "scratch", "ip-ranges.json"].iter().collect();
        let mut client: BlockingClient = BlockingClient::new();
        client.cache_file(&test_cach_file);
        let aws_ip_ranges = client.get_ranges();
        assert!(aws_ip_ranges.is_ok());
    }

    #[test]
    fn test_set_cache_time() {
        let mut client: BlockingClient = BlockingClient::new();
        client.cache_time(48 * 60 * 60);
        let aws_ip_ranges = client.get_ranges();
        assert!(aws_ip_ranges.is_ok());
    }

    #[test]
    fn test_get_json_from_url() {
        let client = BlockingClient::new();
        let json = client.get_json_from_url();
        assert!(json.is_ok());
    }

    #[test]
    fn test_cache_json_to_file() {
        let client = BlockingClient::new();
        let json = client.get_json_from_url().unwrap();
        let result = client.cache_json_to_file(&json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_json_from_file() {
        let client = BlockingClient::new();
        let json_from_url = client.get_json_from_url().unwrap();
        client.cache_json_to_file(&json_from_url).unwrap();
        let json_from_file = client.get_json_from_file();
        assert!(json_from_file.is_ok());
    }

    #[test]
    fn test_parse_json() {
        let client = BlockingClient::new();
        let json = client.get_json_from_url().unwrap();
        let json_ip_ranges = json::parse(&json);
        assert!(json_ip_ranges.is_ok());
    }

    #[test]
    fn test_serialize_json_ip_ranges() {
        let client = BlockingClient::new();
        let json_from_url = client.get_json_from_url().unwrap();
        let json_ip_ranges = json::parse(&json_from_url).unwrap();
        let serialized_json = serde_json::to_string(&json_ip_ranges);
        assert!(serialized_json.is_ok());
    }
}
