use crate::core::awsipranges::AwsIpRanges;
use crate::core::errors::{Error, Result};
use log::{info, warn};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::{thread, time};

/*-------------------------------------------------------------------------------------------------
  Primary Interface
-------------------------------------------------------------------------------------------------*/

/// _**Primary library interface**_ that allows you to quickly retrieve and parse the AWS IP
/// Ranges. Returns a boxed [AwsIpRanges] object that allows you to quickly query
/// ([search](AwsIpRanges::search()), [filter](AwsIpRanges::filter()),
/// etc.) the AWS IP Ranges data.
///
/// ```
/// use ipnetwork::IpNetwork;
///
/// // Get the AWS IP Ranges
/// let aws_ip_ranges = awsipranges::get_ranges().unwrap();
///
/// // Search for IP Prefixes
/// let search_prefixes: Vec<IpNetwork> = vec!["3.141.102.225".parse().unwrap()];
/// let search_results = aws_ip_ranges.search(search_prefixes.iter());
///
/// // Filter the AWS IP Ranges
/// let region = aws_ip_ranges.get_region("us-east-2").unwrap();
/// let service = aws_ip_ranges.get_service("S3").unwrap();
///
/// let filter = awsipranges::Filter {
///     prefix_type: Some(awsipranges::PrefixType::IPv4),
///     regions: Some(vec![region].into_iter().collect()),
///     network_border_groups: None,
///     services: Some(vec![service].into_iter().collect()),
/// };
/// let filtered_results = aws_ip_ranges.filter(&filter);
/// ```
pub fn get_ranges() -> Result<Box<AwsIpRanges>> {
    BlockingClient::new().get_ranges()
}

/*-------------------------------------------------------------------------------------------------
  Blocking Client
-------------------------------------------------------------------------------------------------*/

/// A synchronous (blocking) client for retrieving and caching the AWS IP Ranges JSON data. You can
/// use the client to customize the URL used to retrieve the JSON file, the file path used to cache
/// the data, and the cache-time duration.
/// ```
/// let mut client = awsipranges::BlockingClient::new();
/// client.url("https://ip-ranges.amazonaws.com/ip-ranges.json");
/// client.cache_file("/tmp/ip-ranges.json");
/// client.cache_time(60 * 60); // 1 hour
///
/// let aws_ip_ranges = client.get_ranges();
/// ```
pub struct BlockingClient {
    url: String,
    cache_file: PathBuf,
    cache_time: u64,
    retry_count: u32,
    retry_initial_delay: u64,
    retry_backoff_factor: u64,
    retry_timeout: u64,
}

impl Default for BlockingClient {
    fn default() -> Self {
        Self {
            url: "https://ip-ranges.amazonaws.com/ip-ranges.json".to_string(),
            cache_file: dirs::home_dir()
                .unwrap()
                .join(".aws")
                .join("ip-ranges.json"), // ${HOME}/.aws/ip-ranges.json
            cache_time: 24 * 60 * 60, // 24 hours
            retry_count: 4,
            retry_initial_delay: 200, // 200 ms
            retry_backoff_factor: 2,
            retry_timeout: 5000, // 5 seconds
        }
    }
}

impl BlockingClient {
    pub fn new() -> Self {
        let default = BlockingClient::default();

        Self {
            url: env::var("AWSIPRANGES_URL").ok().unwrap_or(default.url),
            cache_file: env::var("AWSIPRANGES_CACHE_FILE")
                .ok()
                .map(PathBuf::from)
                .unwrap_or(default.cache_file),
            cache_time: env::var("AWSIPRANGES_CACHE_TIME")
                .ok()
                .and_then(|env_var| env_var.parse::<u64>().ok())
                .unwrap_or(default.cache_time),
            retry_count: env::var("AWSIPRANGES_RETRY_COUNT")
                .ok()
                .and_then(|env_var| env_var.parse::<u32>().ok())
                .unwrap_or(default.retry_count),
            retry_initial_delay: env::var("AWSIPRANGES_RETRY_INITIAL_DELAY")
                .ok()
                .and_then(|env_var| env_var.parse::<u64>().ok())
                .unwrap_or(default.retry_initial_delay),
            retry_backoff_factor: env::var("AWSIPRANGES_RETRY_BACKOFF_FACTOR")
                .ok()
                .and_then(|env_var| env_var.parse::<u64>().ok())
                .unwrap_or(default.retry_backoff_factor),
            retry_timeout: env::var("AWSIPRANGES_RETRY_TIMEOUT")
                .ok()
                .and_then(|env_var| env_var.parse::<u64>().ok())
                .unwrap_or(default.retry_timeout),
        }
    }

    /// Retrieves, parses, and returns a boxed [AwsIpRanges] object. Uses locally cached
    /// JSON data, when available and fresh. Requests the AWS IP Ranges JSON data from
    /// the URL when the local cache is stale or unavailable.
    pub fn get_ranges(&self) -> Result<Box<AwsIpRanges>> {
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
    pub fn cache_file<P: AsRef<Path>>(&'_ mut self, cache_file: P) -> &'_ Self {
        self.cache_file = cache_file.as_ref().to_path_buf();
        self
    }

    /// Set the cache-time duration - the amount of time (in seconds) the
    /// locally cached AWS IP Ranges JSON data is considered fresh; defaults to
    /// 24 hours (`86400` seconds). When the elapsed time (the difference
    /// between the current system time and cache file's modified timestamp) is
    /// greater than the configured `cache_time`, calls to `get_ranges()` will
    /// attempt to refresh the cached JSON data from the AWS IP Ranges URL.
    pub fn cache_time(&mut self, cache_time: u64) -> &Self {
        self.cache_time = cache_time;
        self
    }

    /// Set the number of retry attempts to retrieve the AWS IP Ranges JSON data
    /// from the URL; defaults to `4` attempts.
    pub fn retry_count(&mut self, retry_count: u32) -> &Self {
        self.retry_count = retry_count;
        self
    }

    /// Set the initial delay (in milliseconds) between retry attempts to
    /// retrieve the AWS IP Ranges JSON data from the URL; defaults to `200`
    /// milliseconds.
    ///
    /// The delay between retry attempts is calculated as:
    /// `retry_initial_delay * (retry_backoff_factor ^ attempt)`.
    pub fn retry_initial_delay(&mut self, retry_initial_delay: u64) -> &Self {
        self.retry_initial_delay = retry_initial_delay;
        self
    }

    /// Set the backoff factor used to increase the delay between retry attempts
    /// to retrieve the AWS IP Ranges JSON data from the URL; defaults to `2`.
    ///
    /// The delay between retry attempts is calculated as:
    /// `retry_initial_delay * (retry_backoff_factor ^ attempt)`.
    pub fn retry_backoff_factor(&mut self, retry_backoff_factor: u64) -> &Self {
        self.retry_backoff_factor = retry_backoff_factor;
        self
    }

    /// Set the maximum time (in milliseconds) to wait for the AWS IP Ranges JSON
    /// data to be retrieved from the URL; defaults to `5000` milliseconds (5
    /// seconds).
    pub fn retry_timeout(&mut self, retry_timeout: u64) -> &Self {
        self.retry_timeout = retry_timeout;
        self
    }

    fn get_json(&self) -> Result<String> {
        info!("Cache file path {:?}", &self.cache_file);
        info!("Cache time {} seconds", self.cache_time);

        if fs::canonicalize(&self.cache_file).is_ok() {
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

    fn get_json_from_url(&self) -> Result<String> {
        let start_time = time::Instant::now();
        let max_elapsed_time = time::Duration::from_millis(self.retry_timeout);

        let mut attempt: u32 = 0;
        loop {
            info!("Get JSON from URL - Attempt {}: GET {}", attempt, self.url);
            let json: Result<String> = reqwest::blocking::get(&self.url)
                .map_err(Error::from)
                .and_then(|response| response.text().map_err(Error::from))
                .and_then(validate_json);

            match json {
                Ok(json) => break Ok(json),
                Err(error) => {
                    warn!("Get JSON from URL - Attempt {}: FAILED: {}", attempt, error);

                    let delay = time::Duration::from_millis(
                        self.retry_initial_delay * (self.retry_backoff_factor.pow(attempt)),
                    );

                    attempt += 1;

                    if (start_time.elapsed() + delay < max_elapsed_time)
                        && (attempt < self.retry_count)
                    {
                        thread::sleep(delay);
                        continue;
                    } else {
                        break Err(error);
                    }
                }
            }
        }
    }

    fn cache_json_to_file(&self, json: &str) -> Result<()> {
        // Ensure parent directories exist
        self.cache_file.parent().map(fs::create_dir_all);

        Ok(fs::write(&self.cache_file, json)?)
    }

    fn get_json_from_file(&self) -> Result<String> {
        fs::read_to_string(&self.cache_file)
            .map_err(Error::from)
            .and_then(validate_json)
    }
}

/*-------------------------------------------------------------------------------------------------
  Helper Functions
-------------------------------------------------------------------------------------------------*/

fn validate_json(json: String) -> Result<String> {
    serde_json::from_str::<serde::de::IgnoredAny>(&json)
        .and(Ok(json))
        .or(Err("Invalid JSON".into()))
}

/*-------------------------------------------------------------------------------------------------
  Unit Tests
-------------------------------------------------------------------------------------------------*/

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::errors::log_error;
    use crate::core::json;
    use test_log::test;

    #[test]
    fn test_get_ranges_function() {
        let aws_ip_ranges = get_ranges().inspect_err(log_error);
        assert!(aws_ip_ranges.is_ok());
    }

    #[test]
    fn test_client_get_ranges() {
        let client = BlockingClient::new();
        let aws_ip_ranges = client.get_ranges().inspect_err(log_error);
        assert!(aws_ip_ranges.is_ok());
    }

    #[test]
    fn test_set_url() {
        let mut client: BlockingClient = BlockingClient::new();
        client.url("https://ip-ranges.amazonaws.com/ip-ranges.json");
        let aws_ip_ranges = client.get_ranges().inspect_err(log_error);
        assert!(aws_ip_ranges.is_ok());
    }

    #[test]
    fn test_set_cache_file() {
        let test_cache_file: PathBuf = [".", "scratch", "ip-ranges.json"].iter().collect();
        let mut client: BlockingClient = BlockingClient::new();
        client.cache_file(&test_cache_file);
        let aws_ip_ranges = client.get_ranges().inspect_err(log_error);
        assert!(aws_ip_ranges.is_ok());
    }

    #[test]
    fn test_set_cache_time() {
        let mut client: BlockingClient = BlockingClient::new();
        client.cache_time(48 * 60 * 60);
        let aws_ip_ranges = client.get_ranges().inspect_err(log_error);
        assert!(aws_ip_ranges.is_ok());
    }

    #[test]
    fn test_get_json_from_url() {
        let client = BlockingClient::new();
        let json = client.get_json_from_url().inspect_err(log_error);
        assert!(json.is_ok());
    }

    #[test]
    fn test_cache_json_to_file() {
        let client = BlockingClient::new();
        let json = client.get_json_from_url().unwrap();
        let result = client.cache_json_to_file(&json).inspect_err(log_error);
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_json_from_file() {
        let client = BlockingClient::new();
        let json_from_url = client.get_json_from_url().unwrap();
        client.cache_json_to_file(&json_from_url).unwrap();
        let json_from_file = client.get_json_from_file().inspect_err(log_error);
        assert!(json_from_file.is_ok());
    }

    #[test]
    fn test_parse_json() {
        let client = BlockingClient::new();
        let json = client.get_json_from_url().unwrap();
        let json_ip_ranges = json::parse(&json).inspect_err(log_error);
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
