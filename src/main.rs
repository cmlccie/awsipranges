/*-------------------------------------------------------------------------------------------------
  Command Line Interface (CLI) Modules
-------------------------------------------------------------------------------------------------*/

mod cli;

use clap::Parser;

/*-------------------------------------------------------------------------------------------------
  Main CLI Function
-------------------------------------------------------------------------------------------------*/

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

    // Prefix Search
    let search_prefixes = cli::parse_prefixes(&args);
    let search_results = search_prefixes
        .as_ref()
        .map(|search_prefixes| aws_ip_ranges.search(search_prefixes.iter()));

    // Apply Filters
    let filter = [
        args.ipv4,
        args.ipv6,
        args.regions.is_some(),
        args.network_border_groups.is_some(),
        args.services.is_some(),
    ]
    .iter()
    .any(|v| *v)
    .then(|| cli::build_filter(&args, &aws_ip_ranges));

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

    cli::output::log_search_summary(&search_prefixes, &search_results);
    cli::output::display_prefix_table(&display_aws_ip_ranges);

    Ok(())
}

/*-------------------------------------------------------------------------------------------------
  CLI Display Functions
-------------------------------------------------------------------------------------------------*/
