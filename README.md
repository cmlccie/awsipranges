# awsipranges

_Quickly query the AWS IP Ranges_

[![License](https://img.shields.io/github/license/cmlccie/awsipranges)](https://github.com/cmlccie/awsipranges/blob/main/LICENSE)
[![Tests](https://github.com/cmlccie/awsipranges/actions/workflows/tests.yml/badge.svg?branch=main)](https://github.com/cmlccie/awsipranges/actions/workflows/tests.yml)
[![Code Coverage](https://codecov.io/gh/cmlccie/awsipranges/graph/badge.svg?token=2NS0NOYQ0Y)](https://codecov.io/gh/cmlccie/awsipranges)

---

`awsipranges` allows you to filter, search, and use public [AWS IP address ranges](https://docs.aws.amazon.com/vpc/latest/userguide/aws-ip-ranges.html) from the command line without writing complicated JSON parsing scripts or commands. This simple single-purpose CLI tool allows you to quickly answer questions like:

- What IP ranges are used by `<some-supported-service>` in `<some-region>`?
- What services publish their IP ranges in the `ip-ranges.json` file?
- What Local / Wavelength Zones are attached to `<some-region>`?
- What are the IP ranges for `<some-local-zone>`?

Truthfully, you could get answers to these ☝️ types of questions by simply [filtering the JSON file](https://docs.aws.amazon.com/vpc/latest/userguide/aws-ip-work-with.html#filter-json-file). While `awsipranges` [features](#features) make filtering easier, it also allows you do get answers to questions you could not obtain via simple filtering:

- Is `<some-ip-address>` a public AWS IP address?
  - What region is it in?
  - What service(s) does it belong to?
  - What supernets does it belong to?
- What are the supernets of `<some-cidr-block>`?

`awsipranges` parses and understands the structure of IPv4 and IPv6 CIDRs allowing you to work with IP ranges as they were meant to - as structured data - and allows you to output the results in human and machine friendly formats.

## Features

- **Retrieve & Cache**: [`ip-ranges.json`](https://ip-ranges.amazonaws.com/ip-ranges.json) to `${HOME}/.aws/ip-ranges.json`; refreshing the cache after 24 hours (by default).
- **Filter**: IP ranges by region, service, network border group, and IP version (IPv4/IPv6).
- **Search**: IP ranges for an _**IPv4/IPv6 address**_ or _**CIDR**_ (any prefix length) to view the AWS IP ranges that contain the provided address or CIDR.
- **Multiple Output Formats**: Table, CIDR, and netmask output formats for easy integration with other tools.
- **Rust Crate:** The core functionality of this CLI tool is also available as a library allowing you to easily add this functionality to your own Rust utility or application.

## Installation

### Cargo

To build and install the latest `awsipranges` CLI from source, you will need the [Rust toolchain installed](https://www.rust-lang.org/tools/install) on your system, and then you can simply run:

```bash
cargo install --git https://github.com/cmlccie/awsipranges
```

## Why did I make this?

I frequently have need of getting answers from the AWS IP ranges. I published a similar [Python library](https://github.com/aws-samples/awsipranges) when I worked at AWS, and I was learning Rust and needed something to build. 😎 This tool has been useful to me; perhaps it will be useful to you.

## Acknowledgements

I appreciate the following teams and individuals without which this tool would not be possible:

- The AWS Networking team that publishes and maintains the [AWS IP address ranges](https://docs.aws.amazon.com/vpc/latest/userguide/aws-ip-ranges.html).
- Abhishek Chanda (@achanda) for publishing the excellent [`ipnetwork`](https://crates.io/crates/ipnetwork) crate, which makes parsing and working with IPv4 and IPv6 CIDRs a breeze.
