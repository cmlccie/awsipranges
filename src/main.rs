use awsipranges::{AwsIpRanges, SearchResults};
use clap::Parser;
use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::presets::UTF8_FULL;
use comfy_table::*;
use ipnetwork::IpNetwork;
use log::error;
use std::{collections::BTreeSet, path::PathBuf, rc::Rc};

/*-------------------------------------------------------------------------------------------------
  Command Line Interface (CLI) Arguments
-------------------------------------------------------------------------------------------------*/

#[derive(Parser, Debug)]
#[command(author, version, about="Query AWS IP ranges.", long_about = None)]
struct Args {
    /// Include IPv4 prefixes
    #[arg(short = '4', long)]
    ipv4: bool,

    /// Include IPv6 prefixes
    #[arg(short = '6', long)]
    ipv6: bool,

    /// Include prefixes from these AWS Regions
    #[arg(short = 'r', long = "region")]
    regions: Option<Vec<String>>,

    /// Include prefixes from these Network Border Groups
    #[arg(short = 'g', long = "network-border-group")]
    network_border_groups: Option<Vec<String>>,

    /// Include prefixes used by these AWS Services
    #[arg(short = 's', long = "service")]
    services: Option<Vec<String>>,

    /// Output Format: List of (RFC4632) CIDR-format prefixes
    #[arg(short = 'C', long)]
    cidr_format: bool,

    /// Output Format: List of IP networks in network mask format (n.n.n.n m.m.m.m)
    #[arg(short = 'N', long)]
    net_mask_format: bool,

    /// Include a summary of the matching prefixes
    #[arg(long)]
    summary: bool,

    /// Save the results to a CSV file
    #[arg(long = "csv")]
    csv_file: Option<PathBuf>,

    /// Logging verbosity
    #[command(flatten)]
    verbose: clap_verbosity_flag::Verbosity,

    /// Find IP prefixes containing these IP hosts or networks
    prefixes: Option<Vec<String>>,
}

/*-------------------------------------------------------------------------------------------------
  Main CLI Function
-------------------------------------------------------------------------------------------------*/

fn main() -> awsipranges::Result<()> {
    // Parse CLI arguments
    let args = Args::parse();

    // Configure logging
    stderrlog::new()
        .module(module_path!())
        .verbosity(args.verbose.log_level_filter())
        .init()
        .unwrap();

    // Get AWS IP Ranges
    let aws_ip_ranges = awsipranges::get_ranges()?;

    // Prefix Search
    let search_prefixes = parse_prefixes(&args);
    let search_results = search_prefixes
        .as_ref()
        .map(|search_prefixes| aws_ip_ranges.search(search_prefixes.iter()));

    // Apply Filters
    let filter = if [
        args.ipv4,
        args.ipv6,
        args.regions.is_some(),
        args.network_border_groups.is_some(),
        args.services.is_some(),
    ]
    .iter()
    .any(|v| *v)
    {
        Some(build_filter(&args, &aws_ip_ranges))
    } else {
        None
    };
    let filtered_results = match (&search_results, &filter) {
        (Some(search_results), Some(filter)) => Some(search_results.aws_ip_ranges.filter(&filter)),
        (None, Some(filter)) => Some(aws_ip_ranges.filter(&filter)),
        _ => None,
    };

    // CLI Output
    let display_aws_ip_ranges = filtered_results
        .as_ref()
        .or(search_results
            .as_ref()
            .map(|search_results| &search_results.aws_ip_ranges))
        .or(Some(&aws_ip_ranges))
        .unwrap();

    display_prefix_table(&display_aws_ip_ranges);
    display_search_summary(&search_prefixes, &search_results);

    Ok(())
}

/*--------------------------------------------------------------------------------------
  Helper Functions
--------------------------------------------------------------------------------------*/

fn parse_prefixes(args: &Args) -> Option<Vec<IpNetwork>> {
    args.prefixes.as_ref().map(|prefixes| {
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

fn build_filter(args: &Args, aws_ip_ranges: &AwsIpRanges) -> awsipranges::Filter {
    // Build Filters
    let prefix_type = match (args.ipv4, args.ipv6) {
        (true, false) => Some(awsipranges::PrefixType::IPv4),
        (false, true) => Some(awsipranges::PrefixType::IPv6),
        _ => None,
    };

    let regions: Option<BTreeSet<Rc<str>>> = args.regions.as_ref().map(|regions| {
        regions
            .iter()
            .filter_map(|region| {
                aws_ip_ranges.get_region(region).or_else(|| {
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
        args.network_border_groups
            .as_ref()
            .map(|network_border_groups| {
                network_border_groups
                    .iter()
                    .filter_map(|network_border_group| {
                        aws_ip_ranges.get_network_border_group(network_border_group).or_else(|| {
                                error!(
                                    "Invalid network border group or network border group not found in AWS IP Ranges: `{:?}`",
                                    network_border_group
                                );
                                None
                            })
                    })
                    .collect()
            });

    let services: Option<BTreeSet<Rc<str>>> = args.services.as_ref().map(|services| {
        services
            .iter()
            .filter_map(|service| {
                aws_ip_ranges.get_service(service).or_else(|| {
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

/*-------------------------------------------------------------------------------------------------
  CLI Display Functions
-------------------------------------------------------------------------------------------------*/

/*--------------------------------------------------------------------------------------
  Display Prefix Table
--------------------------------------------------------------------------------------*/

fn display_prefix_table(aws_ip_ranges: &AwsIpRanges) {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic);

    table.set_header(vec![
        Cell::new("IP Prefix")
            .add_attribute(Attribute::Bold)
            .fg(Color::Green),
        Cell::new("Region")
            .add_attribute(Attribute::Bold)
            .fg(Color::Green),
        Cell::new("Network Border Group")
            .add_attribute(Attribute::Bold)
            .fg(Color::Green),
        Cell::new("Services")
            .add_attribute(Attribute::Bold)
            .fg(Color::Green),
    ]);

    for prefix in aws_ip_ranges.prefixes.values() {
        let mut sorted_services = prefix
            .services
            .iter()
            .map(|service| service.to_string())
            .collect::<Vec<String>>();
        sorted_services.sort();
        let services = sorted_services.join(", ");

        table.add_row(vec![
            Cell::new(prefix.prefix).add_attribute(Attribute::Bold),
            Cell::new(&prefix.region),
            Cell::new(&prefix.network_border_group),
            Cell::new(services),
        ]);
    }

    // Right-align the IP Prefix column
    let column = table.column_mut(0).expect("The first column exists");
    column.set_cell_alignment(CellAlignment::Right);

    println!("{table}");
}

/*--------------------------------------------------------------------------------------
  Display Search Summary
--------------------------------------------------------------------------------------*/

fn display_search_summary(
    search_prefixes: &Option<Vec<IpNetwork>>,
    search_results: &Option<Box<SearchResults>>,
) {
    if search_prefixes.is_none() || search_results.is_none() {
        return;
    }

    let search_prefixes = search_prefixes.as_ref().unwrap();
    let search_results = search_results.as_ref().unwrap();

    let search_prefix_count = search_prefixes.len();
    let search_prefixes_found = search_results.prefix_matches.len();
    let search_prefixes_not_found = search_results.prefixes_not_found.len();

    let found_aws_ip_prefixes = search_results.aws_ip_ranges.prefixes.len();

    println!("");
    println!(
        "{search_prefixes_found}/{search_prefix_count} provided prefixes found in {found_aws_ip_prefixes} AWS IP Prefixes"
    );
    println!("{search_prefixes_not_found}/{search_prefix_count} provided prefixes were NOT found in the AWS IP Ranges");
}
