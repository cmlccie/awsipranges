/*-------------------------------------------------------------------------------------------------
  Main Modules
-------------------------------------------------------------------------------------------------*/

mod cli;

/*-------------------------------------------------------------------------------------------------
Main CLI Function
-------------------------------------------------------------------------------------------------*/

use clap::Parser;
use std::error::Error;
use std::process::ExitCode;

/// Exit status when no AWS IP Prefixes match the provided criteria.
const EXIT_NO_MATCH: u8 = 1;

/// Exit status when an error prevents the query from completing.
const EXIT_ERROR: u8 = 2;

fn main() -> ExitCode {
    // Parse CLI arguments
    let args = cli::Args::parse();

    // Configure logging
    stderrlog::new()
        .module(module_path!())
        .verbosity(args.verbose.log_level_filter())
        .init()
        .unwrap();

    run(&args).unwrap_or_else(|error| {
        report(error.as_ref());
        ExitCode::from(EXIT_ERROR)
    })
}

fn run(args: &cli::Args) -> Result<ExitCode, Box<dyn Error>> {
    // Get AWS IP Ranges
    let aws_ip_ranges = awsipranges::get_ranges()?;

    // Search for CIDRs
    let search_cidrs = cli::parse_prefixes(args);
    let search_results = search_cidrs
        .as_ref()
        .map(|search_prefixes| aws_ip_ranges.search(search_prefixes));

    // Apply Filters
    let filters_enabled = [
        args.ipv4,
        args.ipv6,
        args.include_regions.is_some(),
        args.include_network_border_groups.is_some(),
        args.include_services.is_some(),
    ]
    .iter()
    .any(|v| *v);

    let filter = if filters_enabled {
        Some(cli::build_filter(args, &aws_ip_ranges)?)
    } else {
        None
    };

    let filtered_results = match (&search_results, &filter) {
        (Some(search_results), Some(filter)) => Some(search_results.aws_ip_ranges.filter(filter)),
        (None, Some(filter)) => Some(aws_ip_ranges.filter(filter)),
        _ => None,
    };

    // Select AWS IP Ranges to output
    let display_aws_ip_ranges = filtered_results
        .as_ref()
        .or(search_results
            .as_ref()
            .map(|search_results| &search_results.aws_ip_ranges))
        .unwrap_or(&aws_ip_ranges);

    // Log CIDR search results
    cli::log::search_results(&search_cidrs, &search_results);

    // Display selected CLI output
    if display_aws_ip_ranges.prefixes().is_empty() {
        eprintln!("\nNo AWS IP Prefixes match the provided criteria.\n");
        return Ok(ExitCode::from(EXIT_NO_MATCH));
    }

    match args.output {
        cli::OutputFormat::Table => cli::output::prefix_table(display_aws_ip_ranges),
        cli::OutputFormat::Cidr => cli::output::prefixes_in_cidr_format(display_aws_ip_ranges),
        cli::OutputFormat::Netmask => {
            cli::output::prefixes_in_netmask_format(display_aws_ip_ranges)
        }
        cli::OutputFormat::Regions => cli::output::regions(display_aws_ip_ranges),
        cli::OutputFormat::NetworkBorderGroups => {
            cli::output::network_border_groups(display_aws_ip_ranges)
        }
        cli::OutputFormat::Services => cli::output::services(display_aws_ip_ranges),
    };

    // Save results to CSV file
    if let Some(csv_file_path) = &args.csv_file {
        cli::csv::save(display_aws_ip_ranges, csv_file_path)?;
    };

    Ok(ExitCode::SUCCESS)
}

/// Print an error, its causes, and a hint for resolving it (when there is one) to stderr.
fn report(error: &(dyn Error + 'static)) {
    eprintln!("error: {error}");

    let causes = std::iter::successors(error.source(), |&cause| cause.source());
    causes.for_each(|cause| eprintln!("  caused by: {cause}"));

    let hint = match error.downcast_ref::<awsipranges::Error>() {
        Some(awsipranges::Error::UnknownRegion(_)) => {
            Some("list the valid regions with `awsipranges --output regions`")
        }
        Some(awsipranges::Error::UnknownNetworkBorderGroup(_)) => Some(
            "list the valid network border groups with `awsipranges --output network-border-groups`",
        ),
        Some(awsipranges::Error::UnknownService(_)) => {
            Some("list the valid services with `awsipranges --output services`")
        }
        _ => None,
    };
    if let Some(hint) = hint {
        eprintln!("  hint: {hint}");
    }
}
