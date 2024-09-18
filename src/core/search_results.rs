use crate::core::aws_ip_prefix::AwsIpPrefix;
use crate::core::aws_ip_ranges::AwsIpRanges;
use ipnetwork::IpNetwork;
use std::collections::{BTreeMap, BTreeSet};

/*-------------------------------------------------------------------------------------------------
  Search Results
-------------------------------------------------------------------------------------------------*/

/// Search results containing the matching [AwsIpRanges], a map of found
/// prefixes, and the set of prefixes not found in the AWS IP Ranges.
#[derive(Clone, Debug, Default)]
pub struct SearchResults {
    /// [AwsIpRanges] object containing the matching AWS IP Prefixes.
    pub aws_ip_ranges: Box<AwsIpRanges>,

    /// Map of found [IpNetwork] prefixes to the sets of [AwsIpPrefix] records
    /// that contain the prefixes.
    pub prefix_matches: BTreeMap<IpNetwork, BTreeSet<AwsIpPrefix>>,

    /// Set of [IpNetwork] prefixes not found in the AWS IP Ranges.
    pub prefixes_not_found: BTreeSet<IpNetwork>,
}
