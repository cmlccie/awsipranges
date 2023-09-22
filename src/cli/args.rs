use clap::Parser;
use std::path::PathBuf;

/*-------------------------------------------------------------------------------------------------
  Command Line Interface (CLI) Arguments
-------------------------------------------------------------------------------------------------*/

#[derive(Parser, Debug)]
#[command(author, version, about="Quickly query the AWS IP Ranges.", long_about = None)]
pub struct Args {
    /// Include: IPv4 prefixes
    #[arg(short = '4', long)]
    pub ipv4: bool,

    /// Include: IPv6 prefixes
    #[arg(short = '6', long)]
    pub ipv6: bool,

    /// Include: Region
    #[arg(id = "REGION", short = 'r', long = "region", num_args(1..))]
    pub include_regions: Option<Vec<String>>,

    /// Include: Network Border Group
    #[arg(
        id = "NETWORK_BORDER_GROUP",
        short = 'g',
        long = "network-border-group",
        num_args(1..)
    )]
    pub include_network_border_groups: Option<Vec<String>>,

    /// Include: Service
    #[arg(id = "SERVICE", short = 's', long = "service", num_args(1..))]
    pub include_services: Option<Vec<String>>,

    #[command(flatten)]
    pub output: Output,

    /// Save the results to a CSV file
    #[arg(long = "csv")]
    pub csv_file: Option<PathBuf>,

    /// Logging verbosity
    #[command(flatten)]
    pub verbose: clap_verbosity_flag::Verbosity,

    /// Find AWS IP Prefixes containing these IP addresses or networks
    pub search_cidrs: Option<Vec<String>>,
}

#[derive(clap::Args, Debug)]
#[group(required = false, multiple = false)]
pub struct Output {
    /// Output: Prefix table (default)
    #[arg(long = "table")]
    pub prefix_table: bool,

    /// Output: Prefix list in CIDR format
    #[arg(long = "cidr")]
    pub cidr_format: bool,

    /// Output: Prefix list in netmask format
    #[arg(long = "netmask")]
    pub netmask_format: bool,

    /// Output: Regions list
    #[arg(long)]
    pub regions: bool,

    /// Output: Network Border Groups list
    #[arg(long)]
    pub network_border_groups: bool,

    /// Output: Services list
    #[arg(long)]
    pub services: bool,
}
