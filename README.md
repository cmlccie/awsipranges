# awsipranges

_Quickly query the AWS IP Ranges_

[![License](https://img.shields.io/badge/license-BSD%E2%80%932%E2%80%93Clause%E2%80%93Patent-blue)](https://opensource.org/license/bsdpluspatent)
[![Crates.io Version](https://img.shields.io/crates/v/awsipranges)](https://crates.io/crates/awsipranges)
[![docs.rs](https://img.shields.io/docsrs/awsipranges)](https://docs.rs/awsipranges/latest/awsipranges/)
[![Tests](https://github.com/cmlccie/awsipranges/actions/workflows/tests.yml/badge.svg?branch=main)](https://github.com/cmlccie/awsipranges/actions/workflows/tests.yml)
[![Code Coverage](https://codecov.io/gh/cmlccie/awsipranges/graph/badge.svg?token=2NS0NOYQ0Y)](https://codecov.io/gh/cmlccie/awsipranges)

---

![Demo](https://vhs.charm.sh/vhs-10iTXUYl2aeKdyYoMvI6C0.gif)

`awsipranges` allows you to search, filter, and use public [AWS IP address ranges](https://docs.aws.amazon.com/vpc/latest/userguide/aws-ip-ranges.html) from the command line without writing complicated JSON parsing scripts or commands. This single-purpose CLI tool allows you to quickly answer questions like:

- Is some IPv4/IPv6 `<address>` (or `<hostname>`) a public AWS IP address?
  - What region is it in?
  - What service(s) does it belong to?
  - What supernets does it belong to?
- What are the supernets of `<some-cidr-block>`?
- What services publish their IP ranges in the `ip-ranges.json` file?
- What IP ranges are used by `<some-supported-service>` in `<some-region>`?
- What Local / Wavelength Zones are attached to `<some-region>`?
- What are the IP ranges for `<some-local-zone>`?

You could get answers to some of these ☝️ questions by [filtering the JSON file](https://docs.aws.amazon.com/vpc/latest/userguide/aws-ip-work-with.html#filter-json-file), but `awsipranges` [features](#features) make filtering more accessible. `awsipranges` parses and understands the structure of IPv4 and IPv6 CIDRs allowing you to work with IP ranges as they were meant to - as structured data - enabling you to output the results in human and machine friendly formats.

If you find this project useful, please consider giving it a star ⭐ on [GitHub](https://github.com/cmlccie/awsipranges). Your support is greatly appreciated!

## Features

- **Retrieve & Cache**: [`ip-ranges.json`](https://ip-ranges.amazonaws.com/ip-ranges.json) to `${HOME}/.aws/ip-ranges.json`; refreshing the cache after 24 hours (by default).
- **Search**: IP ranges for an _**IPv4/IPv6 address**_, _**CIDR**_ (any prefix length), or _**hostname**_ to view the AWS IP ranges that contain the provided address, CIDR, or the hostname's IP addresses.
- **Filter**: IP ranges by region, service, network border group, and IP version (IPv4/IPv6).
- **Multiple Output Formats**: Table, CIDR, and netmask output formats for easy integration with other tools.
- **Save Results to CSV**: Save your search and filter results to CSV for programmatic use or analysis in your favorite spreadsheet app.
- **Rust Crate:** This CLI tool's core functionality is also available as a library, allowing you to easily add it to your Rust utility or application.

## Installation

You can build and install `awsipranges` from source or install pre-built binaries. `awsipranges` supports:

| OS              | arm64 (AArch64)                         | amd64 (x86_64)                          |
| --------------- | --------------------------------------- | --------------------------------------- |
| **macOS**       | ✅ ⏬ Apple silicon                       | ✅ ⏬ Intel silicon                       |
| **Linux**       | ✅ ⏬ GNU (glibc) </BR> ✅ ⏬ Musl (static) | ✅ ⏬ GNU (glibc) </BR> ✅ ⏬ Musl (static) |
| **Windows 10+** |                                         | ✅ ⏬ MSVC                                |

✅ = Supported ⏬ = Pre-built binaries available

Following [Rust Platform Support](https://doc.rust-lang.org/nightly/rustc/platform-support.html) for supported targets and OS and library version dependencies.

### Pre-Built Binaries

You can download and install pre-built binaries from the [releases](https://github.com/cmlccie/awsipranges/releases/) page or use the following installation scripts, which select and install the correct binary for your platform.

#### Homebrew

```Shell
brew install cmlccie/tap/awsipranges
```

#### Shell Script

See the [releases](https://github.com/cmlccie/awsipranges/releases/) page for the latest `{{version}}`.

```Shell
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/cmlccie/awsipranges/releases/download/{{version}}/awsipranges-installer.sh | sh
```

#### PowerShell Script

See the [releases](https://github.com/cmlccie/awsipranges/releases/) page for the latest `{{version}}`.

```PowerShell
powershell -c "irm https://github.com/cmlccie/awsipranges/releases/download/{{version}}/awsipranges-installer.ps1 | iex"
```

### Build from source

To build and install `awsipranges` from source, you will need the [Rust toolchain installed](https://www.rust-lang.org/tools/install) (Rust 1.88 or newer) on your system. Then, you can use `cargo install` to download and build your desired version.

#### Crates.io

Install published releases of `awsipranges` from [crates.io](https://crates.io/crates/awsipranges/).

```bash
cargo install awsipranges
```

#### GitHub

Install the latest, potentially _unreleased_, `awsipranges` from the `main` branch on [GitHub](https://github.com/cmlccie/awsipranges).

```bash
cargo install --git https://github.com/cmlccie/awsipranges.git
```

## Usage

```text
Usage: awsipranges [OPTIONS] [IP|CIDR|HOSTNAME]...

Arguments:
  [IP|CIDR|HOSTNAME]...  Find AWS IP Prefixes containing these IP addresses, networks (CIDRs), or
                         hostnames (resolved to their IPv4 and IPv6 addresses)

Options:
  -4, --ipv4                                         Include: IPv4 prefixes
  -6, --ipv6                                         Include: IPv6 prefixes
  -r, --region <REGION>...                           Include: Region
  -g, --network-border-group <NETWORK_BORDER_GROUP>...
                                                     Include: Network Border Group
  -s, --service <SERVICE>...                         Include: Service
  -o, --output <OUTPUT>                              Output format [default: table]
                                                     [possible values: table, json, cidr, netmask,
                                                     regions, network-border-groups, services]
      --csv <CSV_FILE>                               Save the results to a CSV file
      --refresh                                      Download the AWS IP Ranges even if the cache is fresh
      --offline                                      Use only the cached AWS IP Ranges; never download
      --completions <SHELL>                          Print a shell completion script and exit
                                                     [possible values: bash, elvish, fish, powershell, zsh]
  -v, --verbose...                                   Increase logging verbosity
  -q, --quiet...                                     Decrease logging verbosity
  -h, --help                                         Print help
  -V, --version                                      Print version
```

### Examples

```shell
# Is this an AWS IP address? Which region and services use it?
awsipranges 44.192.140.65

# Search for several addresses and CIDRs (IPv4 and IPv6) at once
awsipranges 3.141.102.225 2600:1f1a:4000:a03a::/64

# Is this website hosted on AWS? (checks all of its IPv4 and IPv6 addresses)
awsipranges aws.amazon.com

# IPv4 prefixes used by S3 in us-west-2, one CIDR per line
awsipranges --ipv4 --service S3 --region us-west-2 --output cidr

# JSON for other tools: the services that use an address
awsipranges --output json 44.192.140.65 | jq -r '.prefixes[].services[]'

# List the network border groups (Local and Wavelength Zones) in the data set
awsipranges --output network-border-groups

# Save the EC2 prefixes in us-east-1 to a CSV file
awsipranges --service EC2 --region us-east-1 --csv ec2-us-east-1.csv

# Work without network access, using the cached data
awsipranges --offline 44.192.140.65
```

Filters are combined: a prefix must match every filter you provide, and any of the values you provide for a single filter. Region and network-border-group names are case-insensitive (`GLOBAL` is accepted for global prefixes), as are service names.

### Hostnames

`awsipranges` resolves hostnames to their IPv4 (A) and IPv6 (AAAA) addresses with your system's resolver (so the hosts file, VPN, and corporate DNS settings apply), prints the addresses to stderr, and searches for all of them. Use `-4` or `-6` to consider only one address family.

When you search, the results show which search matched each AWS IP Prefix: the table and CSV output add a **Matches** column (listing a hostname with the resolved addresses in that prefix), and JSON output adds a `matches` list to each prefix:

```shell
awsipranges --output json ip-ranges.amazonaws.com 44.192.140.65 \
  | jq -c '.prefixes[] | {prefix, matches}'
```

Keep in mind:

- DNS answers can vary by location and over time (CDNs, load balancers, geo-DNS), so results reflect what your resolver returns at that moment.
- If a hostname can't be resolved, `awsipranges` reports the error, still shows results for the other arguments, and exits with status `2`.
- `--offline` applies to the AWS IP Ranges data only; resolving hostnames still uses DNS.
- Pass a hostname, not a URL (`example.com`, not `https://example.com/path`). Internationalized names must use their ASCII (`xn--`) form.

### Exit Status

| Status | Meaning                                                                                         |
| ------ | ----------------------------------------------------------------------------------------------- |
| `0`    | One or more AWS IP Prefixes matched                                                             |
| `1`    | No AWS IP Prefixes matched the provided criteria                                                |
| `2`    | An error occurred (invalid input, unknown filter value, hostname lookup or download failure...) |

This makes `awsipranges` easy to use in scripts:

```shell
awsipranges --quiet --output cidr "$IP" > /dev/null
case $? in
  0) echo "$IP is an AWS IP address" ;;
  1) echo "$IP is not an AWS IP address" ;;
  *) echo "Lookup failed" >&2 ;;
esac
```

### Shell Completions

Generate a completion script for your shell and load it from your shell's configuration, for example:

```shell
awsipranges --completions bash > ~/.local/share/bash-completion/completions/awsipranges
awsipranges --completions zsh > "${fpath[1]}/_awsipranges"
awsipranges --completions fish > ~/.config/fish/completions/awsipranges.fish
```

### Configuration

`awsipranges` caches `ip-ranges.json` locally and retries failed downloads with exponential backoff. You can adjust this behavior with environment variables:

| Environment Variable               | Default Value                                    | Description                                               |
| ---------------------------------- | ------------------------------------------------ | --------------------------------------------------------- |
| `AWSIPRANGES_URL`                  | `https://ip-ranges.amazonaws.com/ip-ranges.json` | URL used to retrieve the AWS IP Ranges                    |
| `AWSIPRANGES_CACHE_FILE`           | `${HOME}/.aws/ip-ranges.json`                    | Path of the local cache file                              |
| `AWSIPRANGES_CACHE_TIME`           | `86400` seconds (24 hours)                       | How long the cached file is considered fresh              |
| `AWSIPRANGES_RETRY_COUNT`          | `4`                                              | Maximum number of download attempts                       |
| `AWSIPRANGES_RETRY_INITIAL_DELAY`  | `200` milliseconds                               | Delay before the first retry                              |
| `AWSIPRANGES_RETRY_BACKOFF_FACTOR` | `2`                                              | Multiplier applied to the delay after each failed attempt |
| `AWSIPRANGES_RETRY_TIMEOUT`        | `30000` milliseconds (30 seconds)                | Maximum total time for the download, including retries    |

If a download fails and a stale cache file exists, `awsipranges` falls back to the stale cache. Use `--refresh` to force a download (and fail if it fails) or `--offline` to use only the cache. HTTPS connections verify certificates against your operating system's trust store and honor the standard `HTTPS_PROXY`/`NO_PROXY` environment variables.

## Rust Library

The CLI's core functionality is available as a library crate. See the [API documentation on docs.rs](https://docs.rs/awsipranges/latest/awsipranges/) and the [`lib_demo.rs`](examples/lib_demo.rs) example.

```shell
cargo add awsipranges
```

```rust
fn main() -> awsipranges::Result<()> {
    let aws_ip_ranges = awsipranges::get_ranges()?;

    let s3_us_west_2 = aws_ip_ranges
        .filter_builder()
        .ipv4()
        .regions(["us-west-2"])?
        .services(["S3"])?
        .filter();

    for prefix in s3_us_west_2.prefixes().keys() {
        println!("{prefix}");
    }

    Ok(())
}
```

## Issues and Enhancements

If you encounter any issues or bugs or have ideas for enhancements and new features, please report them on our [GitHub Issues](https://github.com/cmlccie/awsipranges/issues) page. Your feedback is a gift and helps us improve the tool for everyone!

## Acknowledgements

I appreciate the following teams and individuals without which this tool would not be possible or as quickly constructed:

- The AWS Networking team that publishes and maintains the [AWS IP address ranges](https://docs.aws.amazon.com/vpc/latest/userguide/aws-ip-ranges.html).
- Abhishek Chanda ([@achanda](https://www.github.com/achanda)) for publishing the excellent [ipnetwork](https://crates.io/crates/ipnetwork) crate, which makes parsing and working with IPv4 and IPv6 prefixes a breeze.
- [VHS](https://github.com/charmbracelet/vhs) - Straightforward and powerful terminal GIF recorder! I love how easy it is to make a [demo tape](https://github.com/cmlccie/awsipranges/blob/main/demo/demo.tape)! 😎
- Orhun Parmaksız ([@orhun](https://github.com/orhun)) for his excellent blog on [Fully Automated Releases for Rust Projects](https://blog.orhun.dev/automated-rust-releases/).

## Other Works

- [`netrange`](https://crates.io/crates/netrange) - Use LUA scripts to download and filter IP ranges from multiple cloud providers.
- [`aws-ip-ranges`](https://crates.io/crates/aws-ip-ranges) - Provides the AWS IP range data as a const struct.

## Why did I make this?

I frequently need to get answers from the AWS IP ranges. I published a similar [Python library](https://github.com/aws-samples/awsipranges) while working at AWS. Then, when learning Rust, I needed something to build! 😎 This tool has been helpful to me - perhaps it will be useful to you.
