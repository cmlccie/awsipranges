use crate::cli;
use awsipranges::AwsIpRanges;
use ipnetwork::IpNetwork;
use log::error;
use std::{collections::BTreeSet, rc::Rc};

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

pub fn build_filter(args: &cli::Args, aws_ip_ranges: &AwsIpRanges) -> awsipranges::Filter {
    // Build Filters
    let prefix_type = match (args.ipv4, args.ipv6) {
        (true, false) => Some(awsipranges::PrefixType::IPv4),
        (false, true) => Some(awsipranges::PrefixType::IPv6),
        _ => None,
    };

    let regions: Option<BTreeSet<Rc<str>>> = args.include_regions.as_ref().map(|regions| {
        regions
            .iter()
            .filter_map(|region| {
                let region = crate::cli::utils::to_lowercase(region, ["GLOBAL"]);
                aws_ip_ranges.get_region(&region).or_else(|| {
                    error!(
                        "Invalid region or region not found in AWS IP Ranges: {:?}",
                        region
                    );
                    None
                })
            })
            .collect()
    });

    let network_border_groups: Option<BTreeSet<Rc<str>>> =
        args.include_network_border_groups
            .as_ref()
            .map(|network_border_groups| {
                network_border_groups
                    .iter()
                    .filter_map(|network_border_group| {
                        let network_border_group = crate::cli::utils::to_lowercase(network_border_group, ["GLOBAL"]);
                        aws_ip_ranges.get_network_border_group(&network_border_group).or_else(|| {
                                error!(
                                    "Invalid network border group or network border group not found in AWS IP Ranges: `{:?}`",
                                    network_border_group
                                );
                                None
                            })
                    })
                    .collect()
            });

    let services: Option<BTreeSet<Rc<str>>> = args.include_services.as_ref().map(|services| {
        services
            .iter()
            .filter_map(|service| {
                let service = service.to_uppercase();
                aws_ip_ranges.get_service(&service).or_else(|| {
                    error!(
                        "Invalid service or service not found in AWS IP Ranges: {:?}",
                        service
                    );
                    None
                })
            })
            .collect()
    });

    awsipranges::Filter {
        prefix_type,
        regions,
        network_border_groups,
        services,
        ..awsipranges::Filter::default()
    }
}
