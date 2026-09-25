use crate::cli::target::SearchTarget;
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

    /// Output format
    #[clap(long, short)]
    #[clap(value_enum, default_value_t=OutputFormat::Table)]
    pub output: OutputFormat,

    /// Save the results to a CSV file
    #[arg(long = "csv")]
    pub csv_file: Option<PathBuf>,

    /// Download the AWS IP Ranges even if the cache is fresh
    #[arg(long, conflicts_with = "offline")]
    pub refresh: bool,

    /// Use only the cached AWS IP Ranges; never download
    #[arg(long)]
    pub offline: bool,

    /// Print a shell completion script and exit
    #[arg(long, value_name = "SHELL", exclusive = true)]
    pub completions: Option<clap_complete::Shell>,

    /// Logging verbosity
    #[command(flatten)]
    pub verbose: clap_verbosity_flag::Verbosity,

    /// Find AWS IP Prefixes containing these IP addresses, networks (CIDRs), or hostnames
    /// (resolved to their IPv4 and IPv6 addresses)
    #[arg(value_name = "IP|CIDR|HOSTNAME")]
    pub search: Vec<SearchTarget>,
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum OutputFormat {
    Table,
    Json,
    Cidr,
    Netmask,
    Regions,
    NetworkBorderGroups,
    Services,
}
