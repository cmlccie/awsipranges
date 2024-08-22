use crate::core::awsipranges::AwsIpRanges;
use crate::core::errors::{Error, Result};
use log::{info, warn};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::{thread, time};

/*-------------------------------------------------------------------------------------------------
  Simple Interface
-------------------------------------------------------------------------------------------------*/

/// _**Simple library interface**_ quickly retrieves and parses the AWS IP Ranges using the default
/// client configuration. Returns a boxed [AwsIpRanges] object that allows you to quickly query
/// ([search](AwsIpRanges::search()), [filter](AwsIpRanges::filter()), etc.) the AWS IP Ranges.
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
    Client::new().get_ranges()
}

/*-------------------------------------------------------------------------------------------------
  Client
-------------------------------------------------------------------------------------------------*/

/// A client for retrieving the AWS IP Ranges. The client retrieves the AWS IP Ranges from the
/// cached JSON file, when available and fresh, or from the URL when the cache is stale or
/// unavailable. The client implements a simple exponential backoff retry mechanism to retrieve the
/// JSON data from the URL.
///
/// The [Client] and [ClientBuilder] structs attempt to source configuration values from
/// environment variables when set and use default values when the environment variables are not
/// set - see the [ClientBuilder] struct for details on the environment variables used.
///
/// ```
/// let client = awsipranges::Client::new();
/// let aws_ip_ranges = client.get_ranges().unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct Client {
    url: String,
    cache_file: PathBuf,
    cache_time: u64,
    retry_count: u32,
    retry_initial_delay: u64,
    retry_backoff_factor: u64,
    retry_timeout: u64,
}

/*--------------------------------------------------------------------------------------
  Client Builder
--------------------------------------------------------------------------------------*/

/// A builder for the [Client] struct that allows you to customize the client
/// configuration. The [ClientBuilder] struct provides setters for each
/// configuration value and a [ClientBuilder::build] method to create a [Client]
/// instance.
///
/// ```
/// let client = awsipranges::ClientBuilder::new()
///     .url("https://ip-ranges.amazonaws.com/ip-ranges.json")
///     .cache_file("/tmp/ip-ranges.json")
///     .cache_time(60 * 60) // 1 hour
///     .retry_count(4)
///     .retry_initial_delay(200) // 200 ms
///     .retry_backoff_factor(2)
///     .retry_timeout(5000) // 5 seconds
///     .build();
/// ```
///
/// The [ClientBuilder::new] method attempts to source configuration values from
/// environment variables when set and uses default values when the environment
/// variables are not set.
///
/// If you want to use the default configuration values, ignoring any set
/// environment variables, use the [ClientBuilder::default] method to create a
/// new [ClientBuilder] instance.
#[derive(Debug, Clone)]
pub struct ClientBuilder {
    url: String,
    cache_file: PathBuf,
    cache_time: u64,
    retry_count: u32,
    retry_initial_delay: u64,
    retry_backoff_factor: u64,
    retry_timeout: u64,
}

impl Default for ClientBuilder {
    /// Create a new [ClientBuilder] with default configuration values.
    ///
    /// ```
    /// let client = awsipranges::ClientBuilder::default().build();
    ///
    /// assert_eq!(client.url(), "https://ip-ranges.amazonaws.com/ip-ranges.json");
    /// assert_eq!(client.cache_file(), dirs::home_dir().unwrap().join(".aws").join("ip-ranges.json"));
    /// assert_eq!(client.cache_time(), 86400);
    /// assert_eq!(client.retry_count(), 4);
    /// assert_eq!(client.retry_initial_delay(), 200);
    /// assert_eq!(client.retry_backoff_factor(), 2);
    /// assert_eq!(client.retry_timeout(), 5000);
    /// ```
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

impl ClientBuilder {
    /// Create a new [ClientBuilder] reading initial configuration values from
    /// environment variables when set and default values when the environment
    /// variables are not set.
    ///
    /// The environment variables used to set the initial configuration values
    /// are:
    /// - `AWSIPRANGES_URL`
    /// - `AWSIPRANGES_CACHE_FILE`
    /// - `AWSIPRANGES_CACHE_TIME`
    /// - `AWSIPRANGES_RETRY_COUNT`
    /// - `AWSIPRANGES_RETRY_INITIAL_DELAY`
    /// - `AWSIPRANGES_RETRY_BACKOFF_FACTOR`
    /// - `AWSIPRANGES_RETRY_TIMEOUT`
    pub fn new() -> Self {
        let default = ClientBuilder::default();

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

    /*-------------------------------------------------------------------------
      Setters
    -------------------------------------------------------------------------*/

    /// Set the URL used to retrieve the AWS IP Ranges; defaults to
    /// `https://ip-ranges.amazonaws.com/ip-ranges.json` - see
    /// [AWS IP address ranges](https://docs.aws.amazon.com/vpc/latest/userguide/aws-ip-ranges.html)
    /// in the Amazon Virtual Private Cloud (VPC) User Guide for details.
    pub fn url<'s>(&'s mut self, url: &str) -> &'s mut Self {
        self.url = url.to_string();
        self
    }

    /// Set the file path used to cache the AWS IP Ranges JSON data; defaults
    /// to `${HOME}/.aws/ip-ranges.json`.
    pub fn cache_file<P: AsRef<Path>>(&mut self, cache_file: P) -> &mut Self {
        self.cache_file = cache_file.as_ref().to_path_buf();
        self
    }

    /// Set the cache-time duration - the amount of time (in seconds) the
    /// locally cached AWS IP Ranges JSON data is considered fresh; defaults to
    /// 24 hours (`86400` seconds). When the elapsed time (the difference
    /// between the current system time and cache file's modified timestamp) is
    /// greater than the configured `cache_time`, calls to `get_ranges()` will
    /// attempt to refresh the cached JSON data from the AWS IP Ranges URL.
    pub fn cache_time(&mut self, cache_time: u64) -> &mut Self {
        self.cache_time = cache_time;
        self
    }

    /// Set the number of retry attempts to retrieve the AWS IP Ranges JSON data
    /// from the URL; defaults to `4` attempts.
    pub fn retry_count(&mut self, retry_count: u32) -> &mut Self {
        self.retry_count = retry_count;
        self
    }

    /// Set the initial delay (in milliseconds) between retry attempts to
    /// retrieve the AWS IP Ranges JSON data from the URL; defaults to `200`
    /// milliseconds.
    ///
    /// The delay between retry attempts is calculated as:
    /// `retry_initial_delay * (retry_backoff_factor ^ attempt)`.
    pub fn retry_initial_delay(&mut self, retry_initial_delay: u64) -> &mut Self {
        self.retry_initial_delay = retry_initial_delay;
        self
    }

    /// Set the backoff factor used to increase the delay between retry attempts
    /// to retrieve the AWS IP Ranges JSON data from the URL; defaults to `2`.
    ///
    /// The delay between retry attempts is calculated as:
    /// `retry_initial_delay * (retry_backoff_factor ^ attempt)`.
    pub fn retry_backoff_factor(&mut self, retry_backoff_factor: u64) -> &mut Self {
        self.retry_backoff_factor = retry_backoff_factor;
        self
    }

    /// Set the maximum time (in milliseconds) to wait for the AWS IP Ranges JSON
    /// data to be retrieved from the URL; defaults to `5000` milliseconds (5
    /// seconds).
    pub fn retry_timeout(&mut self, retry_timeout: u64) -> &mut Self {
        self.retry_timeout = retry_timeout;
        self
    }

    /*-------------------------------------------------------------------------
      Build Method
    -------------------------------------------------------------------------*/

    pub fn build(&self) -> Client {
        Client {
            url: self.url.clone(),
            cache_file: self.cache_file.clone(),
            cache_time: self.cache_time,
            retry_count: self.retry_count,
            retry_initial_delay: self.retry_initial_delay,
            retry_backoff_factor: self.retry_backoff_factor,
            retry_timeout: self.retry_timeout,
        }
    }
}

/*--------------------------------------------------------------------------------------
  Client Implementation
--------------------------------------------------------------------------------------*/

impl Default for Client {
    /// Create a new [Client] with default configuration values.
    ///
    /// ```
    /// let client = awsipranges::Client::default();
    ///
    /// assert_eq!(client.url(), "https://ip-ranges.amazonaws.com/ip-ranges.json");
    /// assert_eq!(client.cache_file(), dirs::home_dir().unwrap().join(".aws").join("ip-ranges.json"));
    /// assert_eq!(client.cache_time(), 86400);
    /// assert_eq!(client.retry_count(), 4);
    /// assert_eq!(client.retry_initial_delay(), 200);
    /// assert_eq!(client.retry_backoff_factor(), 2);
    /// assert_eq!(client.retry_timeout(), 5000);
    /// ```
    fn default() -> Self {
        ClientBuilder::default().build()
    }
}

impl Client {
    pub fn new() -> Self {
        ClientBuilder::new().build()
    }

    /*-------------------------------------------------------------------------
      Getters
    -------------------------------------------------------------------------*/

    /// Get the URL used to retrieve the AWS IP Ranges.
    /// Defaults to `https://ip-ranges.amazonaws.com/ip-ranges.json`.
    /// See [AWS IP address ranges](https://docs.aws.amazon.com/vpc/latest/userguide/aws-ip-ranges.html)
    /// in the Amazon Virtual Private Cloud (VPC) User Guide for details.
    ///
    /// ```
    /// let client = awsipranges::Client::new();
    /// assert_eq!(client.url(), "https://ip-ranges.amazonaws.com/ip-ranges.json");
    /// ```
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Get the file path used to cache the AWS IP Ranges JSON data.
    /// Defaults to `${HOME}/.aws/ip-ranges.json`.
    pub fn cache_file(&self) -> &Path {
        &self.cache_file
    }

    /// Get the cache-time duration - the amount of time (in seconds) the
    /// locally cached AWS IP Ranges JSON data is considered fresh.
    /// Defaults to 24 hours (86400 seconds).
    /// ```
    /// let client = awsipranges::Client::new();
    /// assert_eq!(client.cache_time(), 86400);
    /// ```
    pub fn cache_time(&self) -> u64 {
        self.cache_time
    }

    /// Get the number of retry attempts to retrieve the AWS IP Ranges JSON data
    /// from the URL. Defaults to 4 attempts.
    /// ```
    /// let client = awsipranges::Client::new();
    /// assert_eq!(client.retry_count(), 4);
    /// ```
    pub fn retry_count(&self) -> u32 {
        self.retry_count
    }

    /// Get the initial delay (in milliseconds) between retry attempts to
    /// retrieve the AWS IP Ranges JSON data from the URL. Defaults to 200
    /// milliseconds.
    /// ```
    /// let client = awsipranges::Client::new();
    /// assert_eq!(client.retry_initial_delay(), 200);
    /// ```
    pub fn retry_initial_delay(&self) -> u64 {
        self.retry_initial_delay
    }

    /// Get the backoff factor used to increase the delay between retry attempts
    /// to retrieve the AWS IP Ranges JSON data from the URL. Defaults to 2.
    /// ```
    /// let client = awsipranges::Client::new();
    /// assert_eq!(client.retry_backoff_factor(), 2);
    /// ```
    pub fn retry_backoff_factor(&self) -> u64 {
        self.retry_backoff_factor
    }

    /// Get the maximum time (in milliseconds) to wait for the AWS IP Ranges JSON
    /// data to be retrieved from the URL. Defaults to 5000 milliseconds (5
    /// seconds).
    /// ```
    /// let client = awsipranges::Client::new();
    /// assert_eq!(client.retry_timeout(), 5000);
    /// ```
    pub fn retry_timeout(&self) -> u64 {
        self.retry_timeout
    }

    /*-------------------------------------------------------------------------
      Get Ranges
    -------------------------------------------------------------------------*/

    /// Retrieves, parses, and returns a boxed [AwsIpRanges] object. Uses locally cached
    /// JSON data, when available and fresh. Requests the AWS IP Ranges JSON data from
    /// the URL when the local cache is stale or unavailable.
    pub fn get_ranges(&self) -> Result<Box<AwsIpRanges>> {
        let json = self.get_json()?;
        AwsIpRanges::from_json(&json)
    }

    /*-------------------------------------------------------------------------
      Private Methods
    -------------------------------------------------------------------------*/

    /// Get the AWS IP Ranges JSON data from the cache file or URL.
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
                    match self.cache_json_to_file(&json) {
                        Ok(_) => info!(
                            "Successfully cached IP Ranges JSON data to: {:?}",
                            &self.cache_file
                        ),
                        Err(error) => warn!(
                            "Unable to cache IP Ranges JSON data to `{:?}`: {}",
                            &self.cache_file, error
                        ),
                    };
                    Ok(json)
                }
                Err(error) => Err(error),
            }
        }
    }

    /// Get the AWS IP Ranges JSON data from the URL.
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

    /// Write the AWS IP Ranges JSON data to the cache file.
    fn cache_json_to_file(&self, json: &str) -> Result<()> {
        // Ensure parent directories exist
        self.cache_file.parent().map(fs::create_dir_all);

        Ok(fs::write(&self.cache_file, json)?)
    }

    /// Get the AWS IP Ranges JSON data from the cache file.
    fn get_json_from_file(&self) -> Result<String> {
        fs::read_to_string(&self.cache_file)
            .map_err(Error::from)
            .and_then(validate_json)
    }
}

/*-------------------------------------------------------------------------------------------------
  Helper Functions
-------------------------------------------------------------------------------------------------*/

/// Validate a string contains parsable JSON data.
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

    /*-------------------------------------------------------------------------
      Test Simple Interface
    -------------------------------------------------------------------------*/

    #[test]
    fn test_get_ranges_function() {
        let aws_ip_ranges = get_ranges().inspect_err(log_error);
        assert!(aws_ip_ranges.is_ok());
    }

    /*-------------------------------------------------------------------------
      Test Getter and Setter Methods
    -------------------------------------------------------------------------*/

    #[test]
    fn test_set_url() {
        let url = "https://my-ip-ranges.com/ip-ranges.json";
        let client = ClientBuilder::default().url(url).build();
        assert!(client.url() == url);
    }

    #[test]
    fn test_set_cache_file() {
        let test_cache_file: PathBuf = [".", "scratch", "ip-ranges.json"].iter().collect();
        let client: Client = ClientBuilder::default()
            .cache_file(&test_cache_file)
            .build();
        assert!(client.cache_file() == test_cache_file);
    }

    #[test]
    fn test_set_cache_time() {
        let cache_time = 60; // 1 minute
        let client: Client = ClientBuilder::default().cache_time(cache_time).build();
        assert!(client.cache_time() == cache_time);
    }

    #[test]
    fn test_set_retry_count() {
        let retry_count = 2;
        let client: Client = ClientBuilder::default().retry_count(retry_count).build();
        assert!(client.retry_count() == retry_count);
    }

    #[test]
    fn test_set_retry_initial_delay() {
        let retry_initial_delay = 100; // 100 ms
        let client: Client = ClientBuilder::default()
            .retry_initial_delay(retry_initial_delay)
            .build();
        assert!(client.retry_initial_delay() == retry_initial_delay);
    }

    #[test]
    fn test_set_retry_backoff_factor() {
        let retry_backoff_factor = 3;
        let client: Client = ClientBuilder::default()
            .retry_backoff_factor(retry_backoff_factor)
            .build();
        assert!(client.retry_backoff_factor() == retry_backoff_factor);
    }

    #[test]
    fn test_set_retry_timeout() {
        let retry_timeout = 1000; // 1 second
        let client: Client = ClientBuilder::default()
            .retry_timeout(retry_timeout)
            .build();
        assert!(client.retry_timeout() == retry_timeout);
    }

    /*-------------------------------------------------------------------------
      Test JSON Retrieval Methods
    -------------------------------------------------------------------------*/

    #[test]
    fn test_get_json_from_url() {
        let client = ClientBuilder::default().build();
        let json = client.get_json_from_url().inspect_err(log_error);
        assert!(json.is_ok());
    }

    #[test]
    fn test_cache_json_to_file() {
        let test_cache_file: PathBuf = [".", "scratch", "test_cache_json_to_file.json"]
            .iter()
            .collect();
        let client: Client = ClientBuilder::default()
            .cache_file(&test_cache_file)
            .build();
        let json = client.get_json_from_url().unwrap();
        let result = client.cache_json_to_file(&json).inspect_err(log_error);
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_json_from_file() {
        // Write JSON data to test cache file
        let test_cache_file: PathBuf = [".", "scratch", "test_get_json_from_file.json"]
            .iter()
            .collect();
        let client: Client = ClientBuilder::default()
            .cache_file(&test_cache_file)
            .build();
        let json_from_url = client.get_json_from_url().unwrap();
        client.cache_json_to_file(&json_from_url).unwrap();

        // Get JSON from test cache file
        let json_from_file = client.get_json_from_file().inspect_err(log_error);
        assert!(json_from_file.is_ok());
    }

    /*-------------------------------------------------------------------------
      Test JSON Parsing
    -------------------------------------------------------------------------*/

    #[test]
    fn test_parse_json() {
        let client = Client::new();
        let json = client.get_json_from_url().unwrap();
        let json_ip_ranges = json::parse(&json).inspect_err(log_error);
        assert!(json_ip_ranges.is_ok());
    }

    #[test]
    fn test_serialize_json_ip_ranges() {
        let client = Client::new();
        let json_from_url = client.get_json_from_url().unwrap();
        let json_ip_ranges = json::parse(&json_from_url).unwrap();
        let serialized_json = serde_json::to_string(&json_ip_ranges);
        assert!(serialized_json.is_ok());
    }
}
