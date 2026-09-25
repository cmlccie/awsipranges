use crate::cli::search::Search;
use crate::cli::target::SearchTarget;
use ipnetwork::IpNetwork;
use std::collections::BTreeSet;
use std::io;
use std::net::{IpAddr, ToSocketAddrs};

/*-------------------------------------------------------------------------------------------------
  Hostname Resolution
-------------------------------------------------------------------------------------------------*/

/// A hostname could not be resolved to any IP addresses.
#[derive(Debug, thiserror::Error)]
#[error("failed to resolve {hostname}")]
pub struct ResolveError {
    pub hostname: String,
    #[source]
    pub source: io::Error,
}

/// The searches to perform and the hostnames that could not be resolved.
#[derive(Debug, Default)]
pub struct Resolved {
    /// One search per IP address, CIDR, or successfully resolved hostname, in target order.
    pub searches: Vec<Search>,

    /// Hostnames that could not be resolved.
    pub errors: Vec<ResolveError>,
}

impl Resolved {
    /// All networks to search for, across searches.
    pub fn networks(&self) -> Vec<IpNetwork> {
        self.searches
            .iter()
            .flat_map(|search| search.networks.iter().copied())
            .collect()
    }
}

/// Resolve a hostname to its IPv4 (A) and IPv6 (AAAA) addresses with the system resolver,
/// which also honors the hosts file and the system's DNS configuration.
pub fn system_resolver(hostname: &str) -> io::Result<BTreeSet<IpAddr>> {
    Ok((hostname, 0)
        .to_socket_addrs()?
        .map(|socket_address| socket_address.ip())
        .collect())
}

/// Resolve the hostname targets with `resolve` and build the searches to perform.
pub fn resolve_targets(
    targets: &[SearchTarget],
    resolve: impl Fn(&str) -> io::Result<BTreeSet<IpAddr>>,
) -> Resolved {
    targets
        .iter()
        .fold(Resolved::default(), |mut resolved, target| {
            let networks = match target {
                SearchTarget::Network(network) => Ok(vec![*network]),
                SearchTarget::Hostname(hostname) => resolve(hostname)
                    .and_then(non_empty)
                    .map(|addresses| addresses.into_iter().map(IpNetwork::from).collect())
                    .map_err(|source| ResolveError {
                        hostname: hostname.clone(),
                        source,
                    }),
            };
            match networks {
                Ok(networks) => resolved.searches.push(Search {
                    target: target.clone(),
                    networks,
                }),
                Err(error) => resolved.errors.push(error),
            }
            resolved
        })
}

/// Treat a successful lookup that returned no addresses as an error.
fn non_empty(addresses: BTreeSet<IpAddr>) -> io::Result<BTreeSet<IpAddr>> {
    if addresses.is_empty() {
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            "no addresses found",
        ))
    } else {
        Ok(addresses)
    }
}

/*-------------------------------------------------------------------------------------------------
  Unit Tests
-------------------------------------------------------------------------------------------------*/

#[cfg(test)]
mod tests {
    use super::*;

    /// A resolver with fixed answers.
    fn fake_resolver(hostname: &str) -> io::Result<BTreeSet<IpAddr>> {
        let addresses = |values: &[&str]| values.iter().map(|v| v.parse().unwrap()).collect();
        match hostname {
            "dual-stack.example" => Ok(addresses(&["44.192.140.65", "2600:1f1a:4000::1"])),
            "ipv4-only.example" => Ok(addresses(&["44.192.140.65"])),
            "empty.example" => Ok(BTreeSet::new()),
            _ => Err(io::Error::new(io::ErrorKind::NotFound, "unknown host")),
        }
    }

    fn targets(values: &[&str]) -> Vec<SearchTarget> {
        values.iter().map(|v| v.parse().unwrap()).collect()
    }

    #[test]
    fn test_networks_pass_through() {
        let resolved = resolve_targets(&targets(&["44.192.0.0/11", "2600::1"]), fake_resolver);
        assert_eq!(
            resolved.networks(),
            ["44.192.0.0/11".parse().unwrap(), "2600::1".parse().unwrap()]
        );
        assert_eq!(resolved.searches.len(), 2);
        assert!(resolved.errors.is_empty());
    }

    #[test]
    fn test_hostnames_resolve_to_host_networks() {
        let resolved = resolve_targets(&targets(&["dual-stack.example"]), fake_resolver);
        assert_eq!(
            resolved.searches,
            [Search {
                target: SearchTarget::Hostname("dual-stack.example".to_string()),
                networks: vec![
                    "44.192.140.65/32".parse().unwrap(),
                    "2600:1f1a:4000::1/128".parse().unwrap()
                ],
            }]
        );
    }

    #[test]
    fn test_resolution_failures_are_collected() {
        let resolved = resolve_targets(
            &targets(&[
                "ipv4-only.example",
                "missing.example",
                "empty.example",
                "10.0.0.1",
            ]),
            fake_resolver,
        );
        assert_eq!(resolved.searches.len(), 2);
        let failed: Vec<&str> = resolved
            .errors
            .iter()
            .map(|e| e.hostname.as_str())
            .collect();
        assert_eq!(failed, ["missing.example", "empty.example"]);
    }

    #[test]
    fn test_system_resolver_resolves_localhost() {
        let addresses = system_resolver("localhost").unwrap();
        assert!(addresses.iter().all(|address| address.is_loopback()));
        assert!(!addresses.is_empty());
    }
}
