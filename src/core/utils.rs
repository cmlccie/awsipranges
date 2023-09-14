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
    use crate::core::errors_and_results::Result;
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
