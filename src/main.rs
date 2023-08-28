use awsipranges::AwsIpPrefix;
use clap::Parser;
use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::presets::UTF8_FULL;
use comfy_table::*;
use ipnetwork::IpNetwork;
use std::{collections::HashSet, path::PathBuf, rc::Rc};

// ------------------------------------------------------------------------------------------------
// Command Line Interface (CLI) Arguments
// ------------------------------------------------------------------------------------------------

#[derive(Parser, Debug)]
#[command(author, version, about="Query AWS IP ranges.", long_about = None)]
struct Args {
    /// Include IPv4 prefixes
    #[arg(short = '4', long)]
    ipv4: bool,

    /// Include IPv6 prefixes
    #[arg(short = '6', long)]
    ipv6: bool,

    /// Include prefixes from one or more AWS Region(s)
    #[arg(short = 'r', long = "region")]
    regions: Option<Vec<String>>,

    /// Include prefixes in one or more Network Border Group(s)
    #[arg(short = 'g', long = "network-border-group")]
    network_border_groups: Option<Vec<String>>,

    /// Include prefixes used by one or more AWS Services
    #[arg(short = 's', long = "service")]
    services: Option<Vec<String>>,

    /// Output matching prefixes as a list of (RFC4632) CIDR blocks
    #[arg(short = 'C', long)]
    cidr_format: bool,

    /// Output matching prefixes as a list of IP networks in network mask format (n.n.n.n m.m.m.m)
    #[arg(short = 'N', long)]
    net_mask_format: bool,

    /// Include a summary of the matching prefixes
    #[arg(long)]
    summary: bool,

    /// Save the matching prefixes to a CSV file
    #[arg(long = "csv")]
    csv_file: Option<PathBuf>,

    /// Increase the logging verbosity
    #[command(flatten)]
    verbosity: clap_verbosity_flag::Verbosity,

    /// Find IP prefixes containing these IP hosts or networks
    prefixes: Option<Vec<String>>,
}

// ------------------------------------------------------------------------------------------------
// Main CLI Function
// ------------------------------------------------------------------------------------------------

fn main() -> awsipranges::AwsIpRangesResult<()> {
    // Parse CLI arguments
    let args = Args::parse();

    // Configure logging
    stderrlog::new()
        .module(module_path!())
        .verbosity(args.verbosity.log_level_filter())
        .init()
        .unwrap();

    // Get AWS IP Ranges
    let aws_ip_ranges = awsipranges::get_ranges()?;

    // Build Filters
    let prefix_type = match (args.ipv4, args.ipv6) {
        (true, false) => Some(awsipranges::PrefixType::IPv4),
        (false, true) => Some(awsipranges::PrefixType::IPv6),
        _ => None,
    };

    let prefixes = match args.prefixes {
        Some(prefixes) => Some(vec_to_hashset_ipnetwork(&prefixes)?),
        None => None,
    };

    let regions = args
        .regions
        .map(|regions| vec_to_hashset_rc_string(&regions));

    let network_border_groups = args
        .network_border_groups
        .map(|network_border_groups| vec_to_hashset_rc_string(&network_border_groups));

    let services = args
        .services
        .map(|services| vec_to_hashset_rc_string(&services));

    let filter = awsipranges::Filter {
        prefix_type,
        prefixes,
        regions,
        network_border_groups,
        services,
        ..awsipranges::Filter::default()
    };

    display_prefix_table(aws_ip_ranges.filter(&filter));

    Ok(())
}

// ----------------------------------------------------------------------------
// Helper Functions
// ----------------------------------------------------------------------------

fn vec_to_hashset_ipnetwork(v: &Vec<String>) -> awsipranges::AwsIpRangesResult<HashSet<IpNetwork>> {
    let mut set = HashSet::new();
    for s in v {
        set.insert(s.parse()?);
    }
    Ok(set)
}

fn vec_to_hashset_rc_string(v: &Vec<String>) -> HashSet<Rc<String>> {
    v.iter().map(|s| Rc::new(s.clone())).collect()
}

// ------------------------------------------------------------------------------------------------
// CLI Display Functions
// ------------------------------------------------------------------------------------------------

fn display_prefix_table<'a, T>(prefixes: T)
where
    T: Iterator<Item = &'a AwsIpPrefix>,
{
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

    for prefix in prefixes {
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
