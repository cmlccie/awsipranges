use clap::Parser;
use std::path::PathBuf;

/*-------------------------------------------------------------------------------------------------
  Command Line Interface (CLI) Arguments
-------------------------------------------------------------------------------------------------*/

#[derive(Parser, Debug)]
#[command(author, version, about="Query AWS IP ranges.", long_about = None)]
pub struct Args {
    /// Include IPv4 prefixes
    #[arg(short = '4', long)]
    pub ipv4: bool,

    /// Include IPv6 prefixes
    #[arg(short = '6', long)]
    pub ipv6: bool,

    /// Include prefixes from these AWS Regions
    #[arg(short = 'r', long = "region")]
    pub regions: Option<Vec<String>>,

    /// Include prefixes from these Network Border Groups
    #[arg(short = 'g', long = "network-border-group")]
    pub network_border_groups: Option<Vec<String>>,

    /// Include prefixes used by these AWS Services
    #[arg(short = 's', long = "service")]
    pub services: Option<Vec<String>>,

    /// Output Format: List of (RFC4632) CIDR-format prefixes
    #[arg(short = 'C', long)]
    pub cidr_format: bool,

    /// Output Format: List of IP networks in network mask format (n.n.n.n m.m.m.m)
    #[arg(short = 'N', long)]
    pub net_mask_format: bool,

    /// Include a summary of the matching prefixes
    #[arg(long)]
    pub summary: bool,

    /// Save the results to a CSV file
    #[arg(long = "csv")]
    pub csv_file: Option<PathBuf>,

    /// Logging verbosity
    #[command(flatten)]
    pub verbose: clap_verbosity_flag::Verbosity,

    /// Search CIDRs - find AWS IP Prefixes that contain these IP addresses or networks
    pub search_cidrs: Option<Vec<String>>,
}
