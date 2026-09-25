use awsipranges::SearchResults;
use ipnetwork::IpNetwork;
use log::{info, warn};

/*-------------------------------------------------------------------------------------------------
  Logging Functions
-------------------------------------------------------------------------------------------------*/

/*--------------------------------------------------------------------------------------
  Search Results
--------------------------------------------------------------------------------------*/

pub fn search_results(search_networks: &[IpNetwork], search_results: Option<&SearchResults>) {
    let Some(search_results) = search_results else {
        return;
    };

    let count_search_cidrs = search_networks.len();
    info!("Searched for {count_search_cidrs} CIDR(s) in the AWS IP Ranges");

    let count_search_cidrs_found = search_results.prefix_matches.len();
    let count_containing_prefixes = search_results.aws_ip_ranges.prefixes().len();
    if count_search_cidrs_found > 0 {
        info!(
            "Found {count_search_cidrs_found} search CIDR(s) contained in {count_containing_prefixes} AWS IP Prefix(es)"
        );
    };

    let count_search_cidrs_not_found = search_results.prefixes_not_found.len();
    if count_search_cidrs_not_found > 0 {
        warn!("Did not find {count_search_cidrs_not_found} search CIDR(s)");
    };
}
