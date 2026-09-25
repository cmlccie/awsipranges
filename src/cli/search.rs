use crate::cli::target::SearchTarget;
use awsipranges::AwsIpPrefix;
use ipnetwork::IpNetwork;
use serde::Serialize;
use std::fmt;

/*-------------------------------------------------------------------------------------------------
  Searches and Matches
-------------------------------------------------------------------------------------------------*/

/// A search argument and the networks it stands for: the argument itself for an IP address or
/// CIDR, or the resolved addresses (as /32 or /128 networks) for a hostname.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Search {
    pub target: SearchTarget,
    pub networks: Vec<IpNetwork>,
}

/// A search that matched an AWS IP Prefix, with the search networks the prefix contains.
#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct Match {
    /// The search argument (IP address, CIDR, or hostname).
    pub search: String,

    /// The searched addresses (or CIDRs) within the AWS IP Prefix.
    pub addresses: Vec<String>,
}

/// The searches that matched an AWS IP Prefix, in search order.
pub fn matches(aws_ip_prefix: &AwsIpPrefix, searches: &[Search]) -> Vec<Match> {
    searches
        .iter()
        .filter_map(|search| {
            let addresses: Vec<String> = search
                .networks
                .iter()
                .filter(|network| contains(aws_ip_prefix.prefix, **network))
                .map(|network| display_network(*network))
                .collect();
            (!addresses.is_empty()).then(|| Match {
                search: match &search.target {
                    SearchTarget::Network(network) => display_network(*network),
                    SearchTarget::Hostname(hostname) => hostname.clone(),
                },
                addresses,
            })
        })
        .collect()
}

impl fmt::Display for Match {
    /// `hostname (address, ...)` for hostnames, or the IP address or CIDR searched for.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.addresses.as_slice() {
            [address] if *address == self.search => write!(f, "{}", self.search),
            addresses => write!(f, "{} ({})", self.search, addresses.join(", ")),
        }
    }
}

impl Match {
    /// Multi-line form for table cells: the search, then each matching address indented on its
    /// own line (omitted when the search is itself the single matching address).
    pub fn to_lines(&self) -> String {
        match self.addresses.as_slice() {
            [address] if *address == self.search => self.search.clone(),
            addresses => std::iter::once(self.search.clone())
                .chain(addresses.iter().map(|address| format!("  {address}")))
                .collect::<Vec<String>>()
                .join("\n"),
        }
    }
}

/// Whether `supernet` contains all of `network`.
fn contains(supernet: IpNetwork, network: IpNetwork) -> bool {
    supernet.prefix() <= network.prefix() && supernet.contains(network.network())
}

/// An IP address for single-address networks (/32 or /128), otherwise the CIDR.
fn display_network(network: IpNetwork) -> String {
    let single_address = match network {
        IpNetwork::V4(network) => network.prefix() == 32,
        IpNetwork::V6(network) => network.prefix() == 128,
    };
    if single_address {
        network.ip().to_string()
    } else {
        network.to_string()
    }
}

/*-------------------------------------------------------------------------------------------------
  Unit Tests
-------------------------------------------------------------------------------------------------*/

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use std::rc::Rc;

    fn aws_ip_prefix(prefix: &str) -> AwsIpPrefix {
        AwsIpPrefix {
            prefix: prefix.parse().unwrap(),
            region: Rc::from("us-east-1"),
            network_border_group: Rc::from("us-east-1"),
            services: BTreeSet::from([Rc::from("EC2")]),
        }
    }

    fn search(target: &str, networks: &[&str]) -> Search {
        Search {
            target: target.parse().unwrap(),
            networks: networks.iter().map(|n| n.parse().unwrap()).collect(),
        }
    }

    #[test]
    fn test_matches_by_search() {
        let searches = [
            search("44.192.140.65", &["44.192.140.65"]),
            search(
                "aws.example",
                &["44.192.140.66", "10.0.0.1", "2600:1f1a::1"],
            ),
            search("other.example", &["10.0.0.2"]),
            search("44.192.140.0/24", &["44.192.140.0/24"]),
        ];

        let found = matches(&aws_ip_prefix("44.192.0.0/11"), &searches);
        let displayed: Vec<String> = found.iter().map(ToString::to_string).collect();
        assert_eq!(
            displayed,
            [
                "44.192.140.65",
                "aws.example (44.192.140.66)",
                "44.192.140.0/24"
            ]
        );
    }

    #[test]
    fn test_hostname_with_multiple_addresses_in_one_prefix() {
        let searches = [search("aws.example", &["44.192.140.65", "44.192.140.66"])];
        let found = matches(&aws_ip_prefix("44.192.140.64/28"), &searches);
        assert_eq!(
            found,
            [Match {
                search: "aws.example".to_string(),
                addresses: vec!["44.192.140.65".to_string(), "44.192.140.66".to_string()],
            }]
        );
        assert_eq!(
            found[0].to_string(),
            "aws.example (44.192.140.65, 44.192.140.66)"
        );
        assert_eq!(
            found[0].to_lines(),
            "aws.example\n  44.192.140.65\n  44.192.140.66"
        );
    }

    #[test]
    fn test_broader_search_cidr_does_not_match_narrower_prefix() {
        let searches = [search("44.0.0.0/8", &["44.0.0.0/8"])];
        assert!(matches(&aws_ip_prefix("44.192.0.0/11"), &searches).is_empty());
    }

    #[test]
    fn test_address_families_do_not_match() {
        let searches = [search("2600:1f1a::1", &["2600:1f1a::1"])];
        assert!(matches(&aws_ip_prefix("0.0.0.0/0"), &searches).is_empty());
    }
}
