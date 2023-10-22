use std::collections::BTreeSet;
use std::rc::Rc;

/*-------------------------------------------------------------------------------------------------
  Utilities
-------------------------------------------------------------------------------------------------*/

/*--------------------------------------------------------------------------------------
  Work with Reference Counted String Slices
--------------------------------------------------------------------------------------*/

pub fn get_rc_str_from_set(value: &str, set: &BTreeSet<Rc<str>>) -> Option<Rc<str>> {
    set.get(value).map(|item| Rc::clone(item))
}

/*--------------------------------------------------------------------------------------
  IP Network Supplemental Functions
--------------------------------------------------------------------------------------*/

pub mod ipnetwork {
    use crate::core::errors::Result;
    use ipnetwork::{IpNetwork, Ipv4Network, Ipv6Network};

    /*
        The IpNetwork type does not reduce (or provide a method to reduce) an
        interface CIDR prefix to network prefix (where all host bits are set to
        `0`). It does provide a network() method that will extract the network IP.

        These helper functions extract the network prefix from an IpNetwork and
        build a new network prefiex from an existing IpNetwork with a specified
        number of mask bits.
    */

    pub fn network_prefix(ip_network: &IpNetwork) -> IpNetwork {
        match ip_network {
            IpNetwork::V4(ipv4_network) => IpNetwork::V4(
                Ipv4Network::new(ipv4_network.network(), ipv4_network.prefix()).unwrap(),
            ),
            IpNetwork::V6(ipv6_network) => IpNetwork::V6(
                Ipv6Network::new(ipv6_network.network(), ipv6_network.prefix()).unwrap(),
            ),
        }
    }

    pub fn new_network_prefix(ip_network: &IpNetwork, mask_bits: u8) -> Result<IpNetwork> {
        let new_prefix = match ip_network {
            IpNetwork::V4(ipv4_network) => {
                IpNetwork::V4(Ipv4Network::new(ipv4_network.ip(), mask_bits)?)
            }
            IpNetwork::V6(ipv6_network) => {
                IpNetwork::V6(Ipv6Network::new(ipv6_network.ip(), mask_bits)?)
            }
        };

        Ok(network_prefix(&new_prefix))
    }

    /*
        The Ipv4Network and Ipv6Network types implement an is_supernet_of() method;
        however, the IpNetwork type does not.

        This helper function implements the is_supernet_of() functionality to
        compare two IpNetwork objects.
    */

    pub fn is_supernet_of(supernet: IpNetwork, subnet: IpNetwork) -> bool {
        match (supernet, subnet) {
            (IpNetwork::V4(ipv4_supernet), IpNetwork::V4(ipv4_subnet)) => {
                ipv4_supernet.is_supernet_of(ipv4_subnet)
            }
            (IpNetwork::V6(ipv6_supernet), IpNetwork::V6(ipv6_subnet)) => {
                ipv6_supernet.is_supernet_of(ipv6_subnet)
            }
            _ => false,
        }
    }
}

/*-------------------------------------------------------------------------------------------------
  Unit Tests
-------------------------------------------------------------------------------------------------*/

#[cfg(test)]
mod tests {
    use super::get_rc_str_from_set;
    use super::ipnetwork::{is_supernet_of, network_prefix, new_network_prefix};
    use ipnetwork::IpNetwork;
    use std::collections::BTreeSet;
    use std::rc::Rc;

    /*-----------------------------------------------------------------------------
      Work with Reference Counted String Slices
    -----------------------------------------------------------------------------*/

    #[test]
    fn test_get_rc_str_from_set() {
        let set: BTreeSet<Rc<str>> = [Rc::from("foo"), Rc::from("bar"), Rc::from("baz")]
            .into_iter()
            .collect();

        let foo = get_rc_str_from_set("foo", &set).unwrap();
        let bar = get_rc_str_from_set("bar", &set).unwrap();
        let baz = get_rc_str_from_set("baz", &set).unwrap();
        let nope = get_rc_str_from_set("nope", &set);

        assert_eq!(Rc::strong_count(&foo), 2);
        assert_eq!(Rc::strong_count(&bar), 2);
        assert_eq!(Rc::strong_count(&baz), 2);
        assert!(nope.is_none());
    }

    /*-----------------------------------------------------------------------------
      IP Network Supplemental Functions
    -----------------------------------------------------------------------------*/

    #[test]
    fn test_network_prefix() {
        let ipv4_interface: IpNetwork = "10.0.0.1/8".parse().unwrap();
        let ipv6_interface: IpNetwork = "2001:db8::1/32".parse().unwrap();

        let expected_ipv4_network_prefix: IpNetwork = "10.0.0.0/8".parse().unwrap();
        let expected_ipv6_network_prefix: IpNetwork = "2001:db8::/32".parse().unwrap();

        let actual_ipv4_prefix: IpNetwork = network_prefix(&ipv4_interface);
        let actual_ipv6_prefix: IpNetwork = network_prefix(&ipv6_interface);

        assert_eq!(actual_ipv4_prefix, expected_ipv4_network_prefix);
        assert_eq!(actual_ipv6_prefix, expected_ipv6_network_prefix);
    }

    #[test]
    fn test_new_prefix() {
        let original_ipv4_network: IpNetwork = "10.0.1.0/24".parse().unwrap();
        let original_ipv6_network: IpNetwork = "2001:db8:0:1::/64".parse().unwrap();

        let expected_ipv4_prefix: IpNetwork = "10.0.0.0/16".parse().unwrap();
        let expected_ipv6_prefix: IpNetwork = "2001:db8::/48".parse().unwrap();

        let actual_ipv4_prefix: IpNetwork = new_network_prefix(&original_ipv4_network, 16).unwrap();
        let actual_ipv6_prefix: IpNetwork = new_network_prefix(&original_ipv6_network, 48).unwrap();

        assert_eq!(actual_ipv4_prefix, expected_ipv4_prefix);
        assert_eq!(actual_ipv6_prefix, expected_ipv6_prefix);
    }

    #[test]
    fn test_is_supernet_of() {
        let ipv4_supernet: IpNetwork = "10.0.0.0/8".parse().unwrap();
        let ipv4_subnet: IpNetwork = "10.1.0.0/16".parse().unwrap();

        let ipv6_supernet: IpNetwork = "2001:db8::/32".parse().unwrap();
        let ipv6_subnet: IpNetwork = "2001:db8:0:1::/64".parse().unwrap();

        assert!(is_supernet_of(ipv4_supernet, ipv4_subnet)); // IPv4 subnet of supernet
        assert!(is_supernet_of(ipv6_supernet, ipv6_subnet)); // IPv6 subnet of supernet

        assert!(!is_supernet_of(ipv4_subnet, ipv4_supernet)); // IPv4 subnet is not supernet
        assert!(!is_supernet_of(ipv6_subnet, ipv6_supernet)); // IPv6 subnet is not supernet

        assert!(!is_supernet_of(ipv4_supernet, ipv6_supernet)); // Comparing IPv4 and IPv6 prefixes returns false
        assert!(!is_supernet_of(ipv6_supernet, ipv4_supernet)); // Comparing IPv6 and IPv4 prefixes returns false
    }
}
