/*-------------------------------------------------------------------------------------------------
  Main Modules
-------------------------------------------------------------------------------------------------*/

mod cli;

/*-------------------------------------------------------------------------------------------------
Main CLI Function
-------------------------------------------------------------------------------------------------*/

use awsipranges::{CacheMode, ClientBuilder};
use clap::{CommandFactory, Parser};
use std::error::Error;
use std::io::{self, BufWriter, Write};
use std::process::ExitCode;

/// Exit status when no AWS IP Prefixes match the provided criteria.
const EXIT_NO_MATCH: u8 = 1;

/// Exit status when an error prevents the query from completing.
const EXIT_ERROR: u8 = 2;

fn main() -> ExitCode {
    // Parse CLI arguments
    let args = cli::Args::parse();

    // Print shell completions
    if let Some(shell) = args.completions {
        let mut command = cli::Args::command();
        let name = command.get_name().to_string();
        clap_complete::generate(shell, &mut command, name, &mut io::stdout());
        return ExitCode::SUCCESS;
    }

    // Configure logging
    stderrlog::new()
        .module(module_path!())
        .verbosity(args.verbose.log_level_filter())
        .init()
        .unwrap();

    run(&args).unwrap_or_else(|error| {
        if is_broken_pipe(error.as_ref()) {
            // The reader closed the pipe (e.g. `awsipranges -o cidr | head`); not an error
            return ExitCode::SUCCESS;
        }
        report(error.as_ref());
        ExitCode::from(EXIT_ERROR)
    })
}

fn run(args: &cli::Args) -> Result<ExitCode, Box<dyn Error>> {
    // Get AWS IP Ranges
    let cache_mode = match (args.refresh, args.offline) {
        (true, _) => CacheMode::Refresh,
        (_, true) => CacheMode::Offline,
        _ => CacheMode::Auto,
    };
    let aws_ip_ranges = ClientBuilder::new()
        .cache_mode(cache_mode)
        .build()
        .get_ranges()?;

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

    if display_aws_ip_ranges.prefixes().is_empty() {
        if !args.verbose.is_silent() {
            eprintln!("\nNo AWS IP Prefixes match the provided criteria.\n");
        }
        return Ok(ExitCode::from(EXIT_NO_MATCH));
    }

    // Save results to CSV file
    if let Some(csv_file_path) = &args.csv_file {
        cli::csv::save(display_aws_ip_ranges, csv_file_path)?;
    };

    // Display selected CLI output
    let mut out = BufWriter::new(io::stdout().lock());
    let output = match args.output {
        cli::OutputFormat::Table => cli::output::prefix_table,
        cli::OutputFormat::Json => cli::output::json,
        cli::OutputFormat::Cidr => cli::output::prefixes_in_cidr_format,
        cli::OutputFormat::Netmask => cli::output::prefixes_in_netmask_format,
        cli::OutputFormat::Regions => cli::output::regions,
        cli::OutputFormat::NetworkBorderGroups => cli::output::network_border_groups,
        cli::OutputFormat::Services => cli::output::services,
    };
    output(&mut out, display_aws_ip_ranges)?;
    out.flush()?;

    Ok(ExitCode::SUCCESS)
}

/// Whether an error is a write to a closed pipe.
fn is_broken_pipe(error: &(dyn Error + 'static)) -> bool {
    error
        .downcast_ref::<io::Error>()
        .is_some_and(|error| error.kind() == io::ErrorKind::BrokenPipe)
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
        // Only offline mode surfaces cache read errors
        Some(awsipranges::Error::CacheRead { .. }) => {
            Some("run without `--offline` to download the AWS IP Ranges")
        }
        _ => None,
    };
    if let Some(hint) = hint {
        eprintln!("  hint: {hint}");
    }
}
