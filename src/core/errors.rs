/*-------------------------------------------------------------------------------------------------
  Errors and Results
-------------------------------------------------------------------------------------------------*/

// TODO: Add descriptive error messages and handling for the various errors that can occur in the
//       crate.

// Error type alias used throughout the crate.
pub type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

// Result type alias used throughout the crate.
pub type Result<T> = std::result::Result<T, Error>;

/*--------------------------------------------------------------------------------------
  Log Error Function
--------------------------------------------------------------------------------------*/

#[cfg(test)]
pub(crate) fn log_error(error: &Error) {
    log::error!("{}", error);
}
