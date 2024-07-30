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

- Is some IPv4/IPv6 `<address>` a public AWS IP address?
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
- **Search**: IP ranges for an _**IPv4/IPv6 address**_ or _**CIDR**_ (any prefix length) to view the AWS IP ranges that contain the provided address or CIDR.
- **Filter**: IP ranges by region, service, network border group, and IP version (IPv4/IPv6).
- **Multiple Output Formats**: Table, CIDR, and netmask output formats for easy integration with other tools.
- **Save Results to CSV**: Save your search and filter results to CSV for programmatic use or analysis in your favorite spreadsheet app.
- **Rust Crate:** This CLI tool's core functionality is also available as a library, allowing you to easily add it to your Rust utility or application.

## Installation

You can build and install `awsipranges` from source or install pre-built binaries. `awsipranges` supports:

- **macOS** (Apple and Intel silicon)
- **Linux** (x86_64 glibc 2.17+)
- **Windows 10+** (x86_64).

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

To build and install `awsipranges` from source, you will need the [Rust toolchain installed](https://www.rust-lang.org/tools/install) on your system. Then, you can use `cargo install` to download and build your desired version.

#### Crates.io

Install published releases of `awsipranges` from [crates.io](https://crates.io/crates/awsipranges/).

```bash
cargo install awsipranges
```

#### GitHub

Install the latest, potentially _unreleased_, `awsipranges` from the `main` branch on [GitHub](https://crates.io/crates/awsipranges/).

```bash
cargo install --git https://github.com/cmlccie/awsipranges.git
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
