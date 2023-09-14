#[macro_use]
extern crate lazy_static;

pub mod blocking;
mod core;

pub use crate::blocking::get_ranges;
pub use crate::core::aws_ip_ranges::{AwsIpPrefix, AwsIpRanges, Filter, PrefixType, SearchResults};
pub use crate::core::errors_and_results::{Error, Result};
