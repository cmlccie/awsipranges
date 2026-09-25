use crate::core::aws_ip_ranges::AwsIpRanges;
use crate::core::errors::{Error, Result};
use log::{debug, info, warn};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

/*-------------------------------------------------------------------------------------------------
  Simple Interface
-------------------------------------------------------------------------------------------------*/

/// _**Simple library interface**_ quickly retrieves and parses the AWS IP Ranges using the default
/// client configuration. Returns a boxed [AwsIpRanges] object that allows you to quickly query
/// ([search](AwsIpRanges::search), [filter](AwsIpRanges::filter_builder), etc.) the AWS IP Ranges.
///
/// ```rust
/// # fn main() -> awsipranges::Result<()> {
/// use ipnetwork::IpNetwork;
///
/// // Get the AWS IP Ranges
/// let aws_ip_ranges = awsipranges::get_ranges()?;
///
/// // Search for IP Prefixes
/// let search_prefixes: Vec<IpNetwork> = vec!["3.141.102.225".parse().unwrap()];
/// let search_results = aws_ip_ranges.search(&search_prefixes);
///
/// // Filter the AWS IP Ranges
/// let filtered_results = aws_ip_ranges.filter_builder()
///    .ipv4()
///    .regions(["us-east-2"])?
///    .services(["S3"])?
///    .filter();
/// # Ok(())
/// # }
/// ```
pub fn get_ranges() -> Result<Box<AwsIpRanges>> {
    Client::new().get_ranges()
}

/*-------------------------------------------------------------------------------------------------
  Cache Mode
-------------------------------------------------------------------------------------------------*/

/// Controls how the [Client] uses the local cache file and the AWS IP Ranges URL.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CacheMode {
    /// Use the cache file when it is fresh (see [ClientBuilder::cache_time]); otherwise download
    /// the AWS IP Ranges and update the cache, falling back to a stale cache file if the download
    /// fails.
    #[default]
    Auto,

    /// Always download the AWS IP Ranges and update the cache; fail if the download fails.
    Refresh,

    /// Never download; use the cache file regardless of its age and fail if it cannot be read.
    Offline,
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
///     .cache_mode(awsipranges::CacheMode::Auto)
///     .retry_count(4)
///     .retry_initial_delay(200) // 200 ms
///     .retry_backoff_factor(2)
///     .retry_timeout(30_000) // 30 seconds
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
    cache_mode: CacheMode,
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
    /// assert_eq!(client.cache_mode(), awsipranges::CacheMode::Auto);
    /// assert_eq!(client.retry_count(), 4);
    /// assert_eq!(client.retry_initial_delay(), 200);
    /// assert_eq!(client.retry_backoff_factor(), 2);
    /// assert_eq!(client.retry_timeout(), 30_000);
    /// ```
    fn default() -> Self {
        Self {
            url: "https://ip-ranges.amazonaws.com/ip-ranges.json".to_string(),
            cache_file: dirs::home_dir()
                .unwrap()
                .join(".aws")
                .join("ip-ranges.json"), // ${HOME}/.aws/ip-ranges.json
            cache_time: 24 * 60 * 60, // 24 hours
            cache_mode: CacheMode::Auto,
            retry_count: 4,
            retry_initial_delay: 200, // 200 ms
            retry_backoff_factor: 2,
            retry_timeout: 30_000, // 30 seconds
        }
    }
}

impl ClientBuilder {
    //! Create a new [ClientBuilder] reading initial configuration values from
    //! environment variables when set and default values when the environment
    //! variables are not set.
    //!
    #![doc = include_str!("../../docs/lib_configuration_table.md")]

    pub fn new() -> Self {
        Self::from_env(|name| env::var(name).ok())
    }

    /// Build a [ClientBuilder] from configuration values provided by `lookup` (the process
    /// environment in [ClientBuilder::new]), using defaults for missing or invalid values.
    fn from_env(lookup: impl Fn(&str) -> Option<String>) -> Self {
        let default = ClientBuilder::default();

        Self {
            url: parse_env_var(&lookup, "AWSIPRANGES_URL", default.url),
            cache_file: parse_env_var(&lookup, "AWSIPRANGES_CACHE_FILE", default.cache_file),
            cache_time: parse_env_var(&lookup, "AWSIPRANGES_CACHE_TIME", default.cache_time),
            cache_mode: default.cache_mode,
            retry_count: parse_env_var(&lookup, "AWSIPRANGES_RETRY_COUNT", default.retry_count),
            retry_initial_delay: parse_env_var(
                &lookup,
                "AWSIPRANGES_RETRY_INITIAL_DELAY",
                default.retry_initial_delay,
            ),
            retry_backoff_factor: parse_env_var(
                &lookup,
                "AWSIPRANGES_RETRY_BACKOFF_FACTOR",
                default.retry_backoff_factor,
            ),
            retry_timeout: parse_env_var(
                &lookup,
                "AWSIPRANGES_RETRY_TIMEOUT",
                default.retry_timeout,
            ),
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

    /// Set how the client uses the cache file and the AWS IP Ranges URL; defaults to
    /// [CacheMode::Auto].
    pub fn cache_mode(&mut self, cache_mode: CacheMode) -> &mut Self {
        self.cache_mode = cache_mode;
        self
    }

    /// Set the maximum number of attempts to retrieve the AWS IP Ranges JSON
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

    /// Set the maximum total time (in milliseconds) to spend retrieving the
    /// AWS IP Ranges JSON from the URL, including all retry attempts and the
    /// delays between them; defaults to `30000` milliseconds (30 seconds).
    pub fn retry_timeout(&mut self, retry_timeout: u64) -> &mut Self {
        self.retry_timeout = retry_timeout;
        self
    }

    /*-------------------------------------------------------------------------
      Build Method
    -------------------------------------------------------------------------*/

    /// Build a new [Client] instance with the configured values.
    pub fn build(&self) -> Client {
        Client {
            url: self.url.clone(),
            cache_file: self.cache_file.clone(),
            cache_time: self.cache_time,
            cache_mode: self.cache_mode,
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
    cache_mode: CacheMode,
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
    /// assert_eq!(client.cache_mode(), awsipranges::CacheMode::Auto);
    /// assert_eq!(client.retry_count(), 4);
    /// assert_eq!(client.retry_initial_delay(), 200);
    /// assert_eq!(client.retry_backoff_factor(), 2);
    /// assert_eq!(client.retry_timeout(), 30_000);
    /// ```
    fn default() -> Self {
        ClientBuilder::default().build()
    }
}

impl Client {
    //! Create a new [Client] reading initial configuration values from
    //! environment variables when set and default values when the environment
    //! variables are not set.
    //!
    #![doc = include_str!("../../docs/lib_configuration_table.md")]
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

    /// Get how the client uses the cache file and the AWS IP Ranges URL.
    /// Defaults to [CacheMode::Auto].
    ///
    /// ```
    /// let client = awsipranges::Client::default();
    /// assert_eq!(client.cache_mode(), awsipranges::CacheMode::Auto);
    /// ```
    pub fn cache_mode(&self) -> CacheMode {
        self.cache_mode
    }

    /// Get the maximum number of attempts to retrieve the AWS IP Ranges JSON
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

    /// Get the maximum total time (in milliseconds) to spend retrieving the
    /// AWS IP Ranges JSON from the URL, including all retry attempts.
    /// Defaults to 30000 milliseconds (30 seconds).
    ///
    /// ```
    /// let client = awsipranges::Client::default();
    /// assert_eq!(client.retry_timeout(), 30_000);
    /// ```
    pub fn retry_timeout(&self) -> u64 {
        self.retry_timeout
    }

    /*-------------------------------------------------------------------------
      Get Ranges
    -------------------------------------------------------------------------*/

    /// Retrieves, parses, and returns a boxed [AwsIpRanges] object. Uses the
    /// cache file and the AWS IP Ranges URL as configured by the client's
    /// [CacheMode].
    pub fn get_ranges(&self) -> Result<Box<AwsIpRanges>> {
        let json = self.get_json()?;
        AwsIpRanges::from_json(&json)
    }

    /*-------------------------------------------------------------------------
      Private Methods
    -------------------------------------------------------------------------*/

    /// Get the AWS IP Ranges JSON from the cache file or URL.
    fn get_json(&self) -> Result<String> {
        info!("Cache mode: {:?}", self.cache_mode);
        info!("Cache file path: {:?}", self.cache_file);

        match self.cache_mode {
            CacheMode::Offline => self.get_json_from_file(),
            CacheMode::Refresh => self.get_json_from_url_and_cache(),
            CacheMode::Auto => self
                .fresh_cache_json()
                .map_or_else(|| self.get_json_from_url_or_stale_cache(), Ok),
        }
    }

    /// Get the cached JSON when the cache file is fresh and readable.
    fn fresh_cache_json(&self) -> Option<String> {
        info!("Cache time {} seconds", self.cache_time);

        // A modified time in the future (clock skew) is treated as fresh
        let cache_is_fresh = fs::metadata(&self.cache_file)
            .and_then(|metadata| metadata.modified())
            .map(|modified| modified.elapsed().unwrap_or_default().as_secs() <= self.cache_time)
            .unwrap_or(false);

        if cache_is_fresh {
            info!("Cache file is fresh");
            self.get_json_from_file().ok()
        } else {
            info!("Cache file is stale or missing; refresh cache");
            None
        }
    }

    /// Get the JSON from the URL, falling back to a stale cache file when the download fails.
    fn get_json_from_url_or_stale_cache(&self) -> Result<String> {
        self.get_json_from_url_and_cache().or_else(|url_error| {
            self.get_json_from_file()
                .inspect(|_| warn!("Using stale cached AWS IP Ranges: {url_error}"))
                .map_err(|_| url_error)
        })
    }

    /// Get the JSON from the URL and write it to the cache file.
    fn get_json_from_url_and_cache(&self) -> Result<String> {
        let json = self.get_json_from_url()?;
        // A cache write failure is logged and does not fail the lookup
        let _ = self.cache_json_to_file(&json);
        Ok(json)
    }

    /// Get the AWS IP Ranges JSON from the URL, retrying with exponential backoff until
    /// `retry_count` attempts have been made or `retry_timeout` has elapsed.
    fn get_json_from_url(&self) -> Result<String> {
        let deadline = Instant::now() + Duration::from_millis(self.retry_timeout);
        let http_error = |source: Box<dyn std::error::Error + Send + Sync>| Error::Http {
            url: self.url.clone(),
            source,
        };

        let http_client = reqwest::blocking::Client::builder()
            .connect_timeout(Duration::from_millis(self.retry_timeout.min(10_000)))
            .build()
            .map_err(|error| http_error(error.into()))?;

        let mut attempt: u32 = 0;
        loop {
            info!(
                "Get AWS IP Ranges from URL; Attempt {attempt}: GET {}",
                self.url
            );
            let remaining = deadline.saturating_duration_since(Instant::now());

            let json = http_client
                .get(&self.url)
                .timeout(remaining)
                .send()
                .and_then(|response| response.error_for_status())
                .and_then(|response| response.text())
                .map_err(|error| http_error(error.into()))
                .and_then(validate_json);

            match json {
                Ok(json) => {
                    info!("Get AWS IP Ranges from URL; Attempt {attempt}: Ok");
                    break Ok(json);
                }
                Err(error) => {
                    warn!("Get AWS IP Ranges from URL; Attempt {attempt}: FAILED: {error}");

                    let delay = Duration::from_millis(
                        self.retry_initial_delay
                            .saturating_mul(self.retry_backoff_factor.saturating_pow(attempt)),
                    );
                    attempt += 1;

                    if attempt < self.retry_count && Instant::now() + delay < deadline {
                        thread::sleep(delay);
                    } else {
                        break Err(error);
                    }
                }
            }
        }
    }

    /// Write the AWS IP Ranges JSON to the cache file.
    fn cache_json_to_file(&self, json: &str) -> Result<()> {
        let cache_write_error = |source| Error::CacheWrite {
            path: self.cache_file.clone(),
            source,
        };

        // Ensure parent directories exist
        if let Some(parent) = self.cache_file.parent() {
            fs::create_dir_all(parent).map_err(cache_write_error)?;
        }

        fs::write(&self.cache_file, json)
            .map_err(cache_write_error)
            .inspect(|_| {
                info!(
                    "Successfully cached AWS IP Ranges to: {:?}",
                    self.cache_file
                )
            })
            .inspect_err(|error| warn!("{error}"))
    }

    /// Get the AWS IP Ranges JSON from the cache file.
    fn get_json_from_file(&self) -> Result<String> {
        fs::read_to_string(&self.cache_file)
            .map_err(|source| Error::CacheRead {
                path: self.cache_file.clone(),
                source,
            })
            .and_then(validate_json)
            .inspect(|_| {
                info!(
                    "Successfully read AWS IP Ranges JSON from: {:?}",
                    self.cache_file
                )
            })
            .inspect_err(|error| debug!("{error}"))
    }
}

/*-------------------------------------------------------------------------------------------------
  Helper Functions
-------------------------------------------------------------------------------------------------*/

/// Parse an environment variable value or return a default value.
fn parse_env_var<T: std::str::FromStr>(
    lookup: impl Fn(&str) -> Option<String>,
    name: &str,
    default: T,
) -> T {
    lookup(name)
        .and_then(|value| {
            value
                .parse::<T>()
                .inspect(|_| info!("Using {name}: {value}"))
                .inspect_err(|_| warn!("Invalid {name}: {value}"))
                .ok()
        })
        .unwrap_or(default)
}

/// Validate a string contains parsable JSON.
fn validate_json(json: String) -> Result<String> {
    serde_json::from_str::<serde::de::IgnoredAny>(&json)?;
    Ok(json)
}

/*-------------------------------------------------------------------------------------------------
  Unit Tests
-------------------------------------------------------------------------------------------------*/

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::errors::log_error;
    use crate::core::test_utils::{FIXTURE_JSON, HttpResponse, serve};
    use std::collections::HashMap;
    use test_log::test;

    /// Build a client that talks to `url` and caches to a fresh temporary directory.
    fn test_client(url: &str, cache_dir: &tempfile::TempDir) -> ClientBuilder {
        let mut builder = ClientBuilder::default();
        builder
            .url(url)
            .cache_file(cache_dir.path().join("ip-ranges.json"))
            .retry_initial_delay(10)
            .retry_timeout(5_000);
        builder
    }

    /// Write the fixture JSON to `path` with a modified time two days in the past.
    fn write_stale_cache(path: &Path) {
        fs::write(path, FIXTURE_JSON).unwrap();
        let two_days_ago = std::time::SystemTime::now() - Duration::from_secs(2 * 24 * 60 * 60);
        fs::File::options()
            .write(true)
            .open(path)
            .and_then(|file| file.set_modified(two_days_ago))
            .unwrap();
    }

    /*-------------------------------------------------------------------------
      Test Environment Variable Configuration
    -------------------------------------------------------------------------*/

    #[test]
    fn test_environment_variable_configuration() {
        let env_vars: HashMap<&str, &str> = HashMap::from([
            ("AWSIPRANGES_URL", "https://my-ip-ranges.com/ip-ranges.json"),
            ("AWSIPRANGES_CACHE_FILE", "/tmp/ip-ranges-cache.json"),
            ("AWSIPRANGES_CACHE_TIME", "60"),
            ("AWSIPRANGES_RETRY_COUNT", "2"),
            ("AWSIPRANGES_RETRY_INITIAL_DELAY", "100"),
            ("AWSIPRANGES_RETRY_BACKOFF_FACTOR", "3"),
            ("AWSIPRANGES_RETRY_TIMEOUT", "1000"),
        ]);

        // No environment variables: defaults
        let default = ClientBuilder::default().build();
        let unset = ClientBuilder::from_env(|_| None).build();
        assert_eq!(unset.url(), default.url());
        assert_eq!(unset.cache_file(), default.cache_file());
        assert_eq!(unset.cache_time(), default.cache_time());
        assert_eq!(unset.retry_count(), default.retry_count());
        assert_eq!(unset.retry_initial_delay(), default.retry_initial_delay());
        assert_eq!(unset.retry_backoff_factor(), default.retry_backoff_factor());
        assert_eq!(unset.retry_timeout(), default.retry_timeout());

        // All environment variables set
        let env_config =
            ClientBuilder::from_env(|name| env_vars.get(name).map(|v| v.to_string())).build();
        assert_eq!(env_config.url(), "https://my-ip-ranges.com/ip-ranges.json");
        assert_eq!(
            env_config.cache_file(),
            PathBuf::from("/tmp/ip-ranges-cache.json")
        );
        assert_eq!(env_config.cache_time(), 60);
        assert_eq!(env_config.retry_count(), 2);
        assert_eq!(env_config.retry_initial_delay(), 100);
        assert_eq!(env_config.retry_backoff_factor(), 3);
        assert_eq!(env_config.retry_timeout(), 1000);

        // Invalid values fall back to defaults
        let invalid = ClientBuilder::from_env(|_| Some("not-a-number".to_string())).build();
        assert_eq!(invalid.cache_time(), default.cache_time());
        assert_eq!(invalid.retry_count(), default.retry_count());
    }

    /*-------------------------------------------------------------------------
      Test Getter and Setter Methods
    -------------------------------------------------------------------------*/

    #[test]
    fn test_getter_and_setter_methods() {
        let client = ClientBuilder::default()
            .url("https://my-ip-ranges.com/ip-ranges.json")
            .cache_file("/tmp/ip-ranges-cache.json")
            .cache_time(60)
            .cache_mode(CacheMode::Offline)
            .retry_count(2)
            .retry_initial_delay(100)
            .retry_backoff_factor(3)
            .retry_timeout(1000)
            .build();

        assert_eq!(client.url(), "https://my-ip-ranges.com/ip-ranges.json");
        assert_eq!(
            client.cache_file(),
            PathBuf::from("/tmp/ip-ranges-cache.json")
        );
        assert_eq!(client.cache_time(), 60);
        assert_eq!(client.cache_mode(), CacheMode::Offline);
        assert_eq!(client.retry_count(), 2);
        assert_eq!(client.retry_initial_delay(), 100);
        assert_eq!(client.retry_backoff_factor(), 3);
        assert_eq!(client.retry_timeout(), 1000);
    }

    /*-------------------------------------------------------------------------
      Test Retrieval from the URL
    -------------------------------------------------------------------------*/

    #[test]
    fn test_get_ranges_from_url_updates_cache() {
        let url = serve(vec![HttpResponse::ok(FIXTURE_JSON)]);
        let cache_dir = tempfile::tempdir().unwrap();
        let client = test_client(&url, &cache_dir).build();

        let aws_ip_ranges = client.get_ranges().inspect_err(log_error).unwrap();
        assert!(!aws_ip_ranges.prefixes().is_empty());
        assert_eq!(
            fs::read_to_string(client.cache_file()).unwrap(),
            FIXTURE_JSON
        );
    }

    #[test]
    fn test_retry_after_http_error_status() {
        let url = serve(vec![
            HttpResponse::status(503),
            HttpResponse::ok(FIXTURE_JSON),
        ]);
        let cache_dir = tempfile::tempdir().unwrap();
        let client = test_client(&url, &cache_dir).build();

        assert!(client.get_json_from_url().inspect_err(log_error).is_ok());
    }

    #[test]
    fn test_http_error_status_is_an_http_error() {
        let url = serve(vec![HttpResponse::status(404)]);
        let cache_dir = tempfile::tempdir().unwrap();
        let client = test_client(&url, &cache_dir).retry_count(1).build();

        let error = client.get_json_from_url().unwrap_err();
        assert!(matches!(error, Error::Http { .. }), "{error:?}");
    }

    #[test]
    fn test_invalid_json_is_a_json_error() {
        let url = serve(vec![HttpResponse::ok("<html>Not JSON</html>")]);
        let cache_dir = tempfile::tempdir().unwrap();
        let client = test_client(&url, &cache_dir).retry_count(1).build();

        let error = client.get_json_from_url().unwrap_err();
        assert!(matches!(error, Error::Json(_)), "{error:?}");
    }

    #[test]
    fn test_retry_timeout_bounds_a_stalled_request() {
        let url = serve(vec![HttpResponse::Stall]);
        let cache_dir = tempfile::tempdir().unwrap();
        let client = test_client(&url, &cache_dir).retry_timeout(500).build();

        let start = Instant::now();
        let error = client.get_json_from_url().unwrap_err();
        assert!(matches!(error, Error::Http { .. }), "{error:?}");
        assert!(
            start.elapsed() < Duration::from_secs(3),
            "{:?}",
            start.elapsed()
        );
    }

    /*-------------------------------------------------------------------------
      Test Cache Modes
    -------------------------------------------------------------------------*/

    #[test]
    fn test_auto_mode_uses_fresh_cache() {
        let cache_dir = tempfile::tempdir().unwrap();
        // The URL is never contacted when the cache is fresh
        let client = test_client("http://127.0.0.1:9", &cache_dir).build();
        fs::write(client.cache_file(), FIXTURE_JSON).unwrap();

        assert_eq!(client.get_json().unwrap(), FIXTURE_JSON);
    }

    #[test]
    fn test_auto_mode_falls_back_to_stale_cache() {
        let url = serve(vec![HttpResponse::status(503)]);
        let cache_dir = tempfile::tempdir().unwrap();
        let client = test_client(&url, &cache_dir).retry_count(1).build();
        write_stale_cache(client.cache_file());

        assert_eq!(client.get_json().unwrap(), FIXTURE_JSON);
    }

    #[test]
    fn test_refresh_mode_ignores_fresh_cache() {
        let url = serve(vec![HttpResponse::status(503)]);
        let cache_dir = tempfile::tempdir().unwrap();
        let client = test_client(&url, &cache_dir)
            .cache_mode(CacheMode::Refresh)
            .retry_count(1)
            .build();
        fs::write(client.cache_file(), FIXTURE_JSON).unwrap();

        let error = client.get_json().unwrap_err();
        assert!(matches!(error, Error::Http { .. }), "{error:?}");
    }

    #[test]
    fn test_offline_mode_uses_stale_cache() {
        let cache_dir = tempfile::tempdir().unwrap();
        let client = test_client("http://127.0.0.1:9", &cache_dir)
            .cache_mode(CacheMode::Offline)
            .build();
        write_stale_cache(client.cache_file());

        assert_eq!(client.get_json().unwrap(), FIXTURE_JSON);
    }

    #[test]
    fn test_offline_mode_without_cache_is_a_cache_error() {
        let cache_dir = tempfile::tempdir().unwrap();
        let client = test_client("http://127.0.0.1:9", &cache_dir)
            .cache_mode(CacheMode::Offline)
            .build();

        let error = client.get_json().unwrap_err();
        assert!(matches!(error, Error::CacheRead { .. }), "{error:?}");
    }

    /*-------------------------------------------------------------------------
      Test Live Retrieval (network)
    -------------------------------------------------------------------------*/

    /// Smoke test against the real AWS IP Ranges URL.
    /// URL: https://ip-ranges.amazonaws.com/ip-ranges.json
    #[test]
    fn test_get_ranges_from_aws() {
        let cache_dir = tempfile::tempdir().unwrap();
        let client = ClientBuilder::default()
            .cache_file(cache_dir.path().join("ip-ranges.json"))
            .build();

        let aws_ip_ranges = client.get_ranges().inspect_err(log_error);
        assert!(aws_ip_ranges.is_ok());
    }
}
