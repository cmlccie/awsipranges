// Copyright ⓒ 2023-2024 Chris M. Lunsford and [`awsipranges` contributors](https://github.com/cmlccie/awsipranges/graphs/contributors)
// Licensed under the BSD-2-Clause-Patent license
// (see LICENSE or <https://opensource.org/license/bsdpluspatent>)

/*-------------------------------------------------------------------------------------------------
  Crates.io Library Doc
-------------------------------------------------------------------------------------------------*/

//! `awsipranges` allows you to quickly and efficiently search, filter, and use public
//! [AWS IP address ranges](https://docs.aws.amazon.com/vpc/latest/userguide/aws-ip-ranges.html)
//! answering questions like:
//!
//! - Is some IPv4/IPv6 `<address>` a public AWS IP address?
//!   - What region is it in?
//!   - What service(s) does it belong to?
//!   - What supernets does it belong to?
//! - What are the supernets of `<some-cidr-block>`?
//! - What services publish their IP ranges in the `ip-ranges.json` file?
//! - What IP ranges are used by `<some-supported-service>` in `<some-region>`?
//! - What Local / Wavelength Zones are attached to `<some-region>`?
//! - What are the IP ranges for `<some-local-zone>`?
//!
//! You could get answers to some of these ☝️ questions by downloading, parsing, and filtering the
//! [JSON file](https://docs.aws.amazon.com/vpc/latest/userguide/aws-ip-work-with.html#filter-json-file)
//! yourself, but `awsipranges` features make searching and filtering more accessible.
//! `awsipranges` parses and understands the structure of IPv4 and IPv6 CIDRs allowing you to work
//! with IP ranges as they were meant to - as structured data.
//!
//! If you find this project useful, please consider giving it a star ⭐ on
//! [GitHub](https://github.com/cmlccie/awsipranges). Your support is greatly appreciated!
//!
//! ## Features
//!
//! - **Retrieve & Cache**: [`ip-ranges.json`](https://ip-ranges.amazonaws.com/ip-ranges.json) to
//!   `${HOME}/.aws/ip-ranges.json`; refreshing the cache after 24 hours (by default).
//!
//! - **Search**: IP ranges for an _**IPv4/IPv6 address**_ or _**CIDR**_ (any prefix length) to
//!   view the AWS IP ranges that contain the provided address or CIDR.
//!
//! - **Filter**: IP ranges by region, service, network border group, and IP version (IPv4/IPv6).
//!
//! ## Example
//!
//! ```rust
#![doc = include_str!("../examples/lib_demo.rs")]
//! ```
//!
//! ## Configuration
//!
//! The [get_ranges] function, [Client::new], and [ClientBuilder::new] use environment variables
//! and default values to configure the client that retrieves the AWS IP Ranges. You can use the
//! [Client::default] and [ClientBuilder::default] methods to create a client with the default
//! configurations, ignoring environment variables. Use the [ClientBuilder] struct to build a
//! client with a custom configuration.
//!
#![doc = include_str!("../docs/lib_configuration_table.md")]

/*-------------------------------------------------------------------------------------------------
  Library Modules
-------------------------------------------------------------------------------------------------*/

mod core;

/*-------------------------------------------------------------------------------------------------
  Library Public Interface
-------------------------------------------------------------------------------------------------*/

pub use crate::core::aws_ip_prefix::AwsIpPrefix;
pub use crate::core::aws_ip_ranges::AwsIpRanges;
pub use crate::core::client::{get_ranges, Client, ClientBuilder};
pub use crate::core::errors::{Error, Result};
pub use crate::core::filter::{Filter, FilterBuilder};
pub use crate::core::search_results::SearchResults;

/*--------------------------------------------------------------------------------------
  Vendored Modules
--------------------------------------------------------------------------------------*/

pub use ipnetwork;
