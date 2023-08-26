use clap::Parser;
use std::path::PathBuf;

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
    regions: Vec<String>,

    /// Include prefixes in one or more Network Border Group(s)
    #[arg(short = 'g', long = "network-border-group")]
    network_border_groups: Vec<String>,

    /// Include prefixes used by one or more AWS Services
    #[arg(short = 's', long = "service")]
    services: Vec<String>,

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

    /// Find IP prefixes containing this IP host or network
    ip_prefix: Option<String>,
}

fn main() -> awsipranges::AwsIpRangesResult<()> {
    let args = Args::parse();

    println!("Arguments:");
    println!("  Regions: {:?}", args.regions);
    println!("  Network Border Groups: {:?}", args.network_border_groups);
    println!("  Services: {:?}", args.services);

    let aws_ip_ranges = awsipranges::AwsIpRanges::new()?;

    println!("Number of Prefixes: {}", aws_ip_ranges.prefixes.len());
    println!("");

    println!("sync_token:    {}", aws_ip_ranges.sync_token);
    println!("creation_date: {}", aws_ip_ranges.create_date);
    println!("");
    println!("First {:?}", aws_ip_ranges.prefixes.iter().next().unwrap());
    println!("");
    println!("Regions {:?}", aws_ip_ranges.regions);
    println!("");
    println!(
        "Network Border Groups {:?}",
        aws_ip_ranges.network_border_groups
    );
    println!("");
    println!("Services {:?}", aws_ip_ranges.services);
    Ok(())
}
