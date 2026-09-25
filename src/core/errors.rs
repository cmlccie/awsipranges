use std::path::PathBuf;

/*-------------------------------------------------------------------------------------------------
  Errors and Results
-------------------------------------------------------------------------------------------------*/

/// Errors returned by the `awsipranges` library.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The AWS IP Ranges could not be retrieved from the URL (connection failure, timeout, or
    /// an HTTP error status).
    #[error("failed to retrieve the AWS IP Ranges from {url}")]
    Http {
        /// URL the client attempted to retrieve.
        url: String,
        /// Underlying HTTP client error.
        #[source]
        source: Box<dyn std::error::Error + Send + Sync + 'static>,
    },

    /// The AWS IP Ranges cache file could not be read.
    #[error("failed to read the AWS IP Ranges cache file {}", path.display())]
    CacheRead {
        /// Path of the cache file.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// The AWS IP Ranges cache file could not be written.
    #[error("failed to write the AWS IP Ranges cache file {}", path.display())]
    CacheWrite {
        /// Path of the cache file.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// The AWS IP Ranges JSON could not be parsed.
    #[error("failed to parse the AWS IP Ranges JSON")]
    Json(#[from] serde_json::Error),

    /// The requested region is not in the AWS IP Ranges.
    #[error("unknown region: {0}")]
    UnknownRegion(String),

    /// The requested network border group is not in the AWS IP Ranges.
    #[error("unknown network border group: {0}")]
    UnknownNetworkBorderGroup(String),

    /// The requested service is not in the AWS IP Ranges.
    #[error("unknown service: {0}")]
    UnknownService(String),
}

/// Result type alias used throughout the crate.
pub type Result<T> = std::result::Result<T, Error>;

/*--------------------------------------------------------------------------------------
  Log Error Function
--------------------------------------------------------------------------------------*/

#[cfg(test)]
pub(crate) fn log_error(error: &Error) {
    log::error!("{}", error);
}
