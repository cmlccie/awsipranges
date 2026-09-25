use ipnetwork::IpNetwork;
use std::fmt;
use std::str::FromStr;

/*-------------------------------------------------------------------------------------------------
  Search Target
-------------------------------------------------------------------------------------------------*/

/// A search argument: an IP address or CIDR, or a hostname to resolve to IP addresses.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SearchTarget {
    Network(IpNetwork),
    Hostname(String),
}

impl FromStr for SearchTarget {
    type Err = String;

    /// Parse an IP address or CIDR, falling back to a hostname.
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        value
            .parse::<IpNetwork>()
            .map(SearchTarget::Network)
            .or_else(|_| parse_hostname(value).map(SearchTarget::Hostname))
    }
}

impl fmt::Display for SearchTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SearchTarget::Network(network) => write!(f, "{network}"),
            SearchTarget::Hostname(hostname) => write!(f, "{hostname}"),
        }
    }
}

/*--------------------------------------------------------------------------------------
  Hostname Validation
--------------------------------------------------------------------------------------*/

const INVALID: &str = "not a valid IP address, CIDR, or hostname";

/// Validate a DNS hostname: dot-separated labels of 1-63 ASCII letters, digits, hyphens, or
/// underscores (as in SRV-style names) that don't start or end with a hyphen; at most 253
/// characters, with an optional trailing dot. The last label must not be all digits, so
/// malformed IPv4 addresses (e.g. `1.2.3`) are rejected rather than looked up as hostnames.
fn parse_hostname(value: &str) -> Result<String, String> {
    if value.contains("://") {
        return Err("looks like a URL; pass only the hostname (e.g. example.com)".to_string());
    }
    if !value.is_ascii() {
        return Err(
            "internationalized hostnames are not supported; use the ASCII (xn--) form".to_string(),
        );
    }

    let name = value.strip_suffix('.').unwrap_or(value);
    let valid_label = |label: &str| {
        (1..=63).contains(&label.len())
            && label
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
            && !label.starts_with('-')
            && !label.ends_with('-')
    };
    let numeric_last_label = name
        .rsplit('.')
        .next()
        .is_some_and(|label| label.bytes().all(|b| b.is_ascii_digit()));

    if (1..=253).contains(&name.len()) && name.split('.').all(valid_label) && !numeric_last_label {
        Ok(name.to_ascii_lowercase())
    } else {
        Err(INVALID.to_string())
    }
}

/*-------------------------------------------------------------------------------------------------
  Unit Tests
-------------------------------------------------------------------------------------------------*/

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_networks() {
        for value in [
            "44.192.140.65",
            "44.192.0.0/11",
            "2600:1f1a::1",
            "2600:1f1a::/36",
        ] {
            assert_eq!(
                value.parse::<SearchTarget>(),
                Ok(SearchTarget::Network(value.parse().unwrap())),
                "{value}"
            );
        }
    }

    #[test]
    fn test_parse_hostnames() {
        for (value, hostname) in [
            ("example.com", "example.com"),
            ("Example.COM.", "example.com"),
            ("localhost", "localhost"),
            ("_sip._tcp.example.com", "_sip._tcp.example.com"),
            (
                "ec2-3-141-102-225.us-east-2.compute.amazonaws.com",
                "ec2-3-141-102-225.us-east-2.compute.amazonaws.com",
            ),
            ("xn--bcher-kva.example", "xn--bcher-kva.example"),
            ("1password.com", "1password.com"),
        ] {
            assert_eq!(
                value.parse::<SearchTarget>(),
                Ok(SearchTarget::Hostname(hostname.to_string())),
                "{value}"
            );
        }
    }

    #[test]
    fn test_parse_invalid_inputs() {
        let long_label = "a".repeat(64);
        let long_name = ["a"; 127].join(".") + ".com";
        for value in [
            "",
            ".",
            "10",
            "1.2.3",
            "010.1.1.1",
            "44.192.140.65/33",
            "2600:1f1a::/129",
            "example.com:443",
            "-example.com",
            "example-.com",
            "exa mple.com",
            "example..com",
            "example.com/path",
            &long_label,
            &long_name,
        ] {
            assert_eq!(
                value.parse::<SearchTarget>(),
                Err(INVALID.to_string()),
                "{value:?}"
            );
        }
    }

    #[test]
    fn test_parse_invalid_inputs_with_hints() {
        assert!(
            "https://example.com/path"
                .parse::<SearchTarget>()
                .unwrap_err()
                .contains("URL")
        );
        assert!(
            "bücher.example"
                .parse::<SearchTarget>()
                .unwrap_err()
                .contains("xn--")
        );
    }
}
