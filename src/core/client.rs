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
  Client Builder
-------------------------------------------------------------------------------------------------*/

/// A builder for the [Client] struct that allows you to customize the client configuration. The
/// [ClientBuilder] struct provides setters for each configuration value and a
/// [ClientBuilder::build] method to create a [Client] instance.
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
/// The [ClientBuilder::new] method attempts to source configuration values from environment
/// variables when set and uses default values when the environment variables are not set.
///
/// If you want to use the default configuration values, ignoring any environment variables, use
/// the [ClientBuilder::default] method to create a new [ClientBuilder] instance.
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

/*--------------------------------------------------------------------------------------
  Client Builder Implementation
--------------------------------------------------------------------------------------*/

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
            url: get_env_var("AWSIPRANGES_URL", default.url),
            cache_file: get_env_var("AWSIPRANGES_CACHE_FILE", default.cache_file),
            cache_time: get_env_var("AWSIPRANGES_CACHE_TIME", default.cache_time),
            retry_count: get_env_var("AWSIPRANGES_RETRY_COUNT", default.retry_count),
            retry_initial_delay: get_env_var(
                "AWSIPRANGES_RETRY_INITIAL_DELAY",
                default.retry_initial_delay,
            ),
            retry_backoff_factor: get_env_var(
                "AWSIPRANGES_RETRY_BACKOFF_FACTOR",
                default.retry_backoff_factor,
            ),
            retry_timeout: get_env_var("AWSIPRANGES_RETRY_TIMEOUT", default.retry_timeout),
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
    /// locally cached AWS IP Ranges JSON is considered fresh; defaults to
    /// 24 hours (`86400` seconds). When the elapsed time (the difference
    /// between the current system time and cache file's modified timestamp) is
    /// greater than the configured `cache_time`, calls to `get_ranges()` will
    /// attempt to refresh the cached JSON from the AWS IP Ranges URL.
    pub fn cache_time(&mut self, cache_time: u64) -> &mut Self {
        self.cache_time = cache_time;
        self
    }

    /// Set the number of retry attempts to retrieve the AWS IP Ranges JSON
    /// data from the URL; defaults to `4` attempts.
    pub fn retry_count(&mut self, retry_count: u32) -> &mut Self {
        self.retry_count = retry_count;
        self
    }

    /// Set the initial delay (in milliseconds) between retry attempts to
    /// retrieve the AWS IP Ranges JSON from the URL; defaults to `200`
    /// milliseconds.
    ///
    /// The delay between retry attempts is calculated as:
    /// `retry_initial_delay * (retry_backoff_factor ^ attempt)`.
    pub fn retry_initial_delay(&mut self, retry_initial_delay: u64) -> &mut Self {
        self.retry_initial_delay = retry_initial_delay;
        self
    }

    /// Set the backoff factor used to increase the delay between retry
    /// attempts to retrieve the AWS IP Ranges JSON from the URL; defaults
    /// to `2`.
    ///
    /// The delay between retry attempts is calculated as:
    /// `retry_initial_delay * (retry_backoff_factor ^ attempt)`.
    pub fn retry_backoff_factor(&mut self, retry_backoff_factor: u64) -> &mut Self {
        self.retry_backoff_factor = retry_backoff_factor;
        self
    }

    /// Set the maximum time (in milliseconds) to wait for the AWS IP Ranges
    /// JSON to be retrieved from the URL; defaults to `5000` milliseconds
    /// (5 seconds).
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

/*-------------------------------------------------------------------------------------------------
  Client
-------------------------------------------------------------------------------------------------*/

/// A client for retrieving the AWS IP Ranges from the cached JSON file, when available and fresh,
/// or from the URL when the cache is stale or unavailable. Client implements a simple exponential-
/// backoff retry mechanism to retrieve the JSON from the URL.
///
/// The [Client::new] method attempts to source configuration values from environment variables
/// when set and uses default values when the environment variables are not set.
///
/// If you want to use the default configuration values, ignoring any environment variables, use
/// the [Client::default] method to create a new [Client] instance.
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
    /// let client = awsipranges::Client::default();
    /// assert_eq!(client.url(), "https://ip-ranges.amazonaws.com/ip-ranges.json");
    /// ```
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Get the file path used to cache the AWS IP Ranges JSON.
    /// Defaults to `${HOME}/.aws/ip-ranges.json`.
    pub fn cache_file(&self) -> &Path {
        &self.cache_file
    }

    /// Get the cache-time duration - the amount of time (in seconds) the
    /// locally cached AWS IP Ranges JSON is considered fresh.
    /// Defaults to 24 hours (86400 seconds).
    ///
    /// ```
    /// let client = awsipranges::Client::default();
    /// assert_eq!(client.cache_time(), 86400);
    /// ```
    pub fn cache_time(&self) -> u64 {
        self.cache_time
    }

    /// Get the number of retry attempts to retrieve the AWS IP Ranges JSON
    /// data from the URL. Defaults to 4 attempts.
    ///
    /// ```
    /// let client = awsipranges::Client::default();
    /// assert_eq!(client.retry_count(), 4);
    /// ```
    pub fn retry_count(&self) -> u32 {
        self.retry_count
    }

    /// Get the initial delay (in milliseconds) between retry attempts to
    /// retrieve the AWS IP Ranges JSON from the URL. Defaults to 200
    /// milliseconds.
    ///
    /// ```
    /// let client = awsipranges::Client::default();
    /// assert_eq!(client.retry_initial_delay(), 200);
    /// ```
    pub fn retry_initial_delay(&self) -> u64 {
        self.retry_initial_delay
    }

    /// Get the backoff factor used to increase the delay between retry
    /// attempts to retrieve the AWS IP Ranges JSON from the URL. Defaults
    /// to 2.
    ///
    /// ```
    /// let client = awsipranges::Client::default();
    /// assert_eq!(client.retry_backoff_factor(), 2);
    /// ```
    pub fn retry_backoff_factor(&self) -> u64 {
        self.retry_backoff_factor
    }

    /// Get the maximum time (in milliseconds) to wait for the AWS IP Ranges
    /// JSON to be retrieved from the URL. Defaults to 5000 milliseconds
    /// (5 seconds).
    ///
    /// ```
    /// let client = awsipranges::Client::default();
    /// assert_eq!(client.retry_timeout(), 5000);
    /// ```
    pub fn retry_timeout(&self) -> u64 {
        self.retry_timeout
    }

    /*-------------------------------------------------------------------------
      Get Ranges
    -------------------------------------------------------------------------*/

    /// Retrieves, parses, and returns a boxed [AwsIpRanges] object. Uses
    /// locally cached JSON, when available and fresh. Requests the AWS IP
    /// Ranges JSON from the URL when the local cache is stale or
    /// unavailable.
    pub fn get_ranges(&self) -> Result<Box<AwsIpRanges>> {
        let json = self.get_json()?;
        AwsIpRanges::from_json(&json)
    }

    /*-------------------------------------------------------------------------
      Private Methods
    -------------------------------------------------------------------------*/

    /// Get the AWS IP Ranges JSON from the cache file or URL.
    fn get_json(&self) -> Result<String> {
        info!("Cache time {} seconds", self.cache_time);
        info!("Cache file path: {:?}", &self.cache_file);

        // Check if cache file exists
        let cache_exists = fs::metadata(&self.cache_file).is_ok();
        if cache_exists {
            info!("Cache file exists");
        } else {
            info!("Cache file not found");
        };

        // Check if cache file is fresh
        let cache_is_fresh = cache_exists
            && fs::metadata(&self.cache_file)?
                .modified()?
                .elapsed()?
                .as_secs()
                <= self.cache_time;
        if cache_is_fresh {
            info!("Cache file is fresh");
        } else {
            info!("Cache file is stale; refresh cache");
        };

        // Fresh cached JSON
        if cache_is_fresh {
            let fresh_cached_json = self.get_json_from_file();
            if fresh_cached_json.is_ok() {
                return fresh_cached_json;
            }
        };

        // Fresh URL JSON
        let fresh_url_json = self.get_json_from_url();
        if let Ok(fresh_url_json) = fresh_url_json {
            let _ = self.cache_json_to_file(&fresh_url_json);
            return Ok(fresh_url_json);
        };
        let url_result = fresh_url_json;

        // Stale cached JSON
        if cache_exists && !cache_is_fresh {
            let stale_cache_json = self.get_json_from_file();
            if stale_cache_json.is_ok() {
                return stale_cache_json;
            }
        };

        // Return result (Err) retrieving AWS IP Ranges JSON from URL
        url_result
    }

    /// Get the AWS IP Ranges JSON from the URL.
    fn get_json_from_url(&self) -> Result<String> {
        let start_time = time::Instant::now();
        let max_elapsed_time = time::Duration::from_millis(self.retry_timeout);

        let mut attempt: u32 = 0;
        loop {
            info!(
                "Get AWS IP Ranges from URL; Attempt {}: GET {}",
                attempt, self.url
            );
            let json: Result<String> = reqwest::blocking::get(&self.url)
                .map_err(Error::from)
                .and_then(|response| response.text().map_err(Error::from))
                .and_then(validate_json);

            match json {
                Ok(json) => {
                    info!("Get AWS IP Ranges from URL; Attempt {}: Ok", attempt);
                    break Ok(json);
                }
                Err(error) => {
                    log::error!(
                        "Get AWS IP Ranges from URL; Attempt {}: FAILED: {}",
                        attempt,
                        error
                    );

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

    /// Write the AWS IP Ranges JSON to the cache file.
    fn cache_json_to_file(&self, json: &str) -> Result<()> {
        // Ensure parent directories exist
        self.cache_file.parent().map(fs::create_dir_all);

        fs::write(&self.cache_file, json)
            .inspect(|_| {
                info!(
                    "Successfully cached AWS IP Ranges to: {:?}",
                    &self.cache_file
                )
            })
            .map_err(Error::from)
            .inspect_err(|error| {
                log::error!(
                    "Failed to cache AWS IP Ranges to `{:?}`: {}",
                    &self.cache_file,
                    error
                )
            })
    }

    /// Get the AWS IP Ranges JSON from the cache file.
    fn get_json_from_file(&self) -> Result<String> {
        fs::read_to_string(&self.cache_file)
            .map_err(Error::from)
            .and_then(validate_json)
            .inspect(|_| {
                info!(
                    "Successfully read AWS IP Ranges JSON from: {:?}",
                    &self.cache_file
                )
            })
            .inspect_err(|error| {
                log::error!(
                    "Failed to read AWS IP Ranges JSON from `{:?}`: {}",
                    &self.cache_file,
                    error
                )
            })
    }
}

/*-------------------------------------------------------------------------------------------------
  Helper Functions
-------------------------------------------------------------------------------------------------*/

/// Get and parse an environment variable value or return a default value.
fn get_env_var<T: std::str::FromStr>(env_var: &str, default: T) -> T {
    env::var(env_var)
        .ok()
        .and_then(|value| {
            value
                .parse::<T>()
                .inspect(|_| info!("Using {}: {}", env_var, value))
                .inspect_err(|_| warn!("Invalid {}: {}", env_var, value))
                .ok()
        })
        .unwrap_or(default)
}

/// Validate a string contains parsable JSON.
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
    use env::VarError;
    use test_log::test;

    /*-------------------------------------------------------------------------
      Test Simple Interface
    -------------------------------------------------------------------------*/

    /// Test the library's simple interface function.
    /// ENV_VAR: AWSIPRANGES_CACHE_FILE
    /// ENV_VAR: AWSIPRANGES_CACHE_TIME
    /// ENV_VAR: AWSIPRANGES_RETRY_COUNT
    /// ENV_VAR: AWSIPRANGES_RETRY_INITIAL_DELAY
    /// ENV_VAR: AWSIPRANGES_RETRY_BACKOFF_FACTOR
    /// ENV_VAR: AWSIPRANGES_RETRY_TIMEOUT
    /// FILE: {HOME}/.aws/ip-ranges.json
    #[test]
    fn test_get_ranges_function() {
        let aws_ip_ranges = get_ranges().inspect_err(log_error);
        assert!(aws_ip_ranges.is_ok());
    }

    /*-------------------------------------------------------------------------
      Test Environment Variable Configuration
    -------------------------------------------------------------------------*/

    /// ENV_VAR: AWSIPRANGES_CACHE_FILE
    /// ENV_VAR: AWSIPRANGES_CACHE_TIME
    /// ENV_VAR: AWSIPRANGES_RETRY_COUNT
    /// ENV_VAR: AWSIPRANGES_RETRY_INITIAL_DELAY
    /// ENV_VAR: AWSIPRANGES_RETRY_BACKOFF_FACTOR
    /// ENV_VAR: AWSIPRANGES_RETRY_TIMEOUT
    #[test]
    fn test_environment_variable_configuration() {
        let test_env_vars = [
            ("AWSIPRANGES_URL", "https://my-ip-ranges.com/ip-ranges.json"),
            (
                "AWSIPRANGES_CACHE_FILE",
                "./scratch/test_environment_variable_configuration_cache_file.json",
            ),
            ("AWSIPRANGES_CACHE_TIME", "60"),
            ("AWSIPRANGES_RETRY_COUNT", "2"),
            ("AWSIPRANGES_RETRY_INITIAL_DELAY", "100"),
            ("AWSIPRANGES_RETRY_BACKOFF_FACTOR", "3"),
            ("AWSIPRANGES_RETRY_TIMEOUT", "1000"),
        ];

        let default = Client::default();

        // Store environment variable values
        let stored_env_vars: Vec<(String, std::result::Result<std::string::String, VarError>)> =
            test_env_vars
                .iter()
                .map(|(env_var, _)| (env_var.to_string(), env::var(env_var)))
                .collect();

        // Unset all environment variables
        test_env_vars.iter().for_each(|(env_var, _)| unsafe {
            std::env::remove_var(env_var);
        });

        // Test default cases
        let new = Client::new();
        assert_eq!(new.url(), default.url());
        assert_eq!(new.cache_file(), default.cache_file());
        assert_eq!(new.cache_time(), default.cache_time());
        assert_eq!(new.retry_count(), default.retry_count());
        assert_eq!(new.retry_initial_delay(), default.retry_initial_delay());
        assert_eq!(new.retry_backoff_factor(), default.retry_backoff_factor());
        assert_eq!(new.retry_timeout(), default.retry_timeout());

        // Set all environment variables
        for (env_var, value) in test_env_vars.iter() {
            unsafe { std::env::set_var(env_var, value) };
        }

        // Test environment variable configuration
        let env_config = Client::new();
        assert_eq!(env_config.url(), "https://my-ip-ranges.com/ip-ranges.json");
        assert_eq!(
            env_config.cache_file(),
            PathBuf::from("./scratch/test_environment_variable_configuration_cache_file.json")
        );
        assert_eq!(env_config.cache_time(), 60);
        assert_eq!(env_config.retry_count(), 2);
        assert_eq!(env_config.retry_initial_delay(), 100);
        assert_eq!(env_config.retry_backoff_factor(), 3);
        assert_eq!(env_config.retry_timeout(), 1000);

        // Reset environment variables
        for (env_var, value) in stored_env_vars {
            match value {
                Ok(value) => unsafe { std::env::set_var(env_var, value) },
                Err(VarError::NotPresent) => unsafe { std::env::remove_var(env_var) },
                Err(VarError::NotUnicode(value)) => unsafe { std::env::set_var(env_var, value) },
            }
        }
    }

    /*-------------------------------------------------------------------------
      Test Getter and Setter Methods
    -------------------------------------------------------------------------*/

    #[test]
    fn test_getter_and_setter_methods() {
        let client = ClientBuilder::default()
            .url("https://my-ip-ranges.com/ip-ranges.json")
            .cache_file("./scratch/test_getter_and_setter_methods_cache_file.json")
            .cache_time(60)
            .retry_count(2)
            .retry_initial_delay(100)
            .retry_backoff_factor(3)
            .retry_timeout(1000)
            .build();

        assert_eq!(client.url(), "https://my-ip-ranges.com/ip-ranges.json");
        assert_eq!(
            client.cache_file(),
            PathBuf::from("./scratch/test_getter_and_setter_methods_cache_file.json")
        );
        assert_eq!(client.cache_time(), 60);
        assert_eq!(client.retry_count(), 2);
        assert_eq!(client.retry_initial_delay(), 100);
        assert_eq!(client.retry_backoff_factor(), 3);
        assert_eq!(client.retry_timeout(), 1000);
    }

    /*-------------------------------------------------------------------------
      Test JSON Retrieval Methods
    -------------------------------------------------------------------------*/

    /// Test getting the JSON from the URL.
    /// URL: https://ip-ranges.amazonaws.com/ip-ranges.json
    #[test]
    fn test_get_json_from_url() {
        let client = ClientBuilder::default().build();
        let json = client.get_json_from_url().inspect_err(log_error);
        assert!(json.is_ok());
    }

    /// Test caching the JSON to a file.
    /// FILE: ./scratch/test_cache_json_to_file.json
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

    /// Test getting the JSON from a file.
    /// FILE: ./scratch/test_get_json_from_file.json
    #[test]
    fn test_get_json_from_file() {
        // Write JSON to test cache file
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

    /// Test parsing the JSON.
    /// URL: https://ip-ranges.amazonaws.com/ip-ranges.json
    #[test]
    fn test_parse_json() {
        let client = Client::default();
        let json = client.get_json_from_url().unwrap();
        let json_ip_ranges = json::parse(&json).inspect_err(log_error);
        assert!(json_ip_ranges.is_ok());
    }

    /// Test serializing the JSON.
    /// URL: https://ip-ranges.amazonaws.com/ip-ranges.json
    #[test]
    fn test_serialize_json_ip_ranges() {
        let client = Client::default();
        let json_from_url = client.get_json_from_url().unwrap();
        let json_ip_ranges = json::parse(&json_from_url).unwrap();
        let serialized_json = serde_json::to_string(&json_ip_ranges);
        assert!(serialized_json.is_ok());
    }
}
