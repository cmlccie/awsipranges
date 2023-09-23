/*-------------------------------------------------------------------------------------------------
  Main Modules
-------------------------------------------------------------------------------------------------*/

mod cli;

/*-------------------------------------------------------------------------------------------------
Main CLI Function
-------------------------------------------------------------------------------------------------*/

use clap::Parser;

fn main() -> awsipranges::Result<()> {
    // Parse CLI arguments
    let args = cli::Args::parse();

    // Configure logging
    stderrlog::new()
        .module(module_path!())
        .verbosity(args.verbose.log_level_filter())
        .init()
        .unwrap();

    // Get AWS IP Ranges
    let aws_ip_ranges = awsipranges::get_ranges()?;

    // Search for CIDRs
    let search_cidrs = cli::parse_prefixes(&args);
    let search_results = search_cidrs
        .as_ref()
        .map(|search_prefixes| aws_ip_ranges.search(search_prefixes.iter()));

    // Apply Filters
    let filter = [
        args.ipv4,
        args.ipv6,
        args.include_regions.is_some(),
        args.include_network_border_groups.is_some(),
        args.include_services.is_some(),
    ]
    .iter()
    .any(|v| *v)
    .then(|| cli::build_filter(&args, &aws_ip_ranges));

    let filtered_results = match (&search_results, &filter) {
        (Some(search_results), Some(filter)) => Some(search_results.aws_ip_ranges.filter(&filter)),
        (None, Some(filter)) => Some(aws_ip_ranges.filter(&filter)),
        _ => None,
    };

    // Select AWS IP Ranges to output
    let display_aws_ip_ranges = filtered_results
        .as_ref()
        .or(search_results
            .as_ref()
            .map(|search_results| &search_results.aws_ip_ranges))
        .or(Some(&aws_ip_ranges))
        .unwrap();

    // Log CIDR search results
    cli::log::search_results(&search_cidrs, &search_results);

    // Display selected CLI output
    if display_aws_ip_ranges.prefixes.is_empty() {
        eprintln!("\nNo AWS IP Prefixes match the provided criteria.\n");
        std::process::exit(1);
    } else {
        if args.output.prefix_table {
            cli::output::prefix_table(display_aws_ip_ranges);
        } else if args.output.cidr_format {
            cli::output::prefixes_in_cidr_format(&display_aws_ip_ranges);
        } else if args.output.netmask_format {
            cli::output::prefixes_in_netmask_format(&display_aws_ip_ranges);
        } else if args.output.regions {
            cli::output::regions(&display_aws_ip_ranges);
        } else if args.output.network_border_groups {
            cli::output::network_border_groups(&display_aws_ip_ranges);
        } else if args.output.services {
            cli::output::services(&display_aws_ip_ranges);
        } else {
            // Default output
            cli::output::prefix_table(display_aws_ip_ranges);
        };
    };

    // Save results to CSV file
    if let Some(csv_file_path) = args.csv_file {
        cli::csv::save(&display_aws_ip_ranges, &csv_file_path)?;
    };

    Ok(())
}
