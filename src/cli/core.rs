use crate::cli;
use awsipranges::{AwsIpRanges, Filter, Result};
use cli::utils::to_lowercase;
use ipnetwork::IpNetwork;
use log::error;

/*-------------------------------------------------------------------------------------------------
  Core functions
-------------------------------------------------------------------------------------------------*/

/*--------------------------------------------------------------------------------------
  Parse IP Network prefixes from CLI arguments
--------------------------------------------------------------------------------------*/

pub fn parse_prefixes(args: &cli::Args) -> Option<Vec<IpNetwork>> {
    args.search_cidrs.as_ref().map(|prefixes| {
        prefixes
            .iter()
            .filter_map(|prefix| {
                prefix.parse().ok().or_else(|| {
                    error!("Invalid IP prefix: {:?}", prefix);
                    None
                })
            })
            .collect()
    })
}

/*--------------------------------------------------------------------------------------
  Build AWS IP Ranges filter from CLI arguments
--------------------------------------------------------------------------------------*/

pub fn build_filter(args: &cli::Args, aws_ip_ranges: &AwsIpRanges) -> Result<Filter> {
    let mut filter = aws_ip_ranges.filter_builder();

    // Prefix Type
    if args.ipv4 {
        filter = filter.ipv4();
    };

    if args.ipv6 {
        filter = filter.ipv6();
    };

    // Regions
    if let Some(include_regions) = &args.include_regions {
        let include_regions: Vec<String> = include_regions
            .iter()
            .map(|region| to_lowercase(region, ["GLOBAL"]))
            .collect();
        filter = filter.regions(include_regions)?;
    };

    // Network Border Groups
    if let Some(include_network_border_groups) = &args.include_network_border_groups {
        let include_network_border_groups: Vec<String> = include_network_border_groups
            .iter()
            .map(|group| to_lowercase(group, ["GLOBAL"]))
            .collect();
        filter = filter.network_border_groups(include_network_border_groups)?;
    };

    // Services
    if let Some(include_services) = &args.include_services {
        let include_services: Vec<String> = include_services
            .iter()
            .map(|service| service.to_uppercase())
            .collect();
        filter = filter.services(include_services)?;
    };

    Ok(filter.build())
}
