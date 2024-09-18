use ipnetwork::IpNetwork;
use std::collections::BTreeSet;
use std::rc::Rc;

/*-------------------------------------------------------------------------------------------------
  AWS IP Prefix
-------------------------------------------------------------------------------------------------*/

/// AWS IP Prefix record containing the IP prefix, region, network border group, and services
/// associated with the prefix.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct AwsIpPrefix {
    /// IPv4 or IPv6 prefix.
    pub prefix: IpNetwork,

    /// AWS region the IP prefix is associated with.
    pub region: Rc<str>,

    /// Network border group the IP prefix is associated with.
    pub network_border_group: Rc<str>,

    /// AWS services that use the IP prefix.
    pub services: BTreeSet<Rc<str>>,
}

/*-------------------------------------------------------------------------------------------------
  Unit Tests
-------------------------------------------------------------------------------------------------*/

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /*----------------------------------------------------------------------------------
      Test Helper Functions
    ----------------------------------------------------------------------------------*/

    pub(crate) fn test_aws_ipv4_prefix() -> AwsIpPrefix {
        AwsIpPrefix {
            prefix: "10.0.0.0/8".parse().unwrap(),
            region: Rc::from("us-east-1"),
            network_border_group: Rc::from("us-east-1"),
            services: [Rc::from("EC2")].into_iter().collect(),
        }
    }

    pub(crate) fn test_aws_ipv6_prefix() -> AwsIpPrefix {
        AwsIpPrefix {
            prefix: "2001:db8::/32".parse().unwrap(),
            region: Rc::from("us-east-1"),
            network_border_group: Rc::from("us-east-1"),
            services: [Rc::from("EC2")].into_iter().collect(),
        }
    }

    /*----------------------------------------------------------------------------------
      AwsIpPrefix
    ----------------------------------------------------------------------------------*/

    #[test]
    fn test_aws_ip_prefix_ordering() {
        let prefix1 = test_aws_ipv4_prefix();

        let prefix2 = AwsIpPrefix {
            prefix: "10.0.0.0/16".parse().unwrap(),
            ..test_aws_ipv4_prefix()
        };

        let prefix3 = AwsIpPrefix {
            prefix: "10.1.0.0/16".parse().unwrap(),
            ..test_aws_ipv4_prefix()
        };

        let prefix4 = AwsIpPrefix {
            region: Rc::from("us-east-2"),
            ..test_aws_ipv4_prefix()
        };

        let prefix5 = AwsIpPrefix {
            network_border_group: Rc::from("us-east-2"),
            ..test_aws_ipv4_prefix()
        };

        let prefix6 = AwsIpPrefix {
            services: [Rc::from("EC2"), Rc::from("ROUTE53")].into_iter().collect(),
            ..test_aws_ipv4_prefix()
        };

        let prefix7 = AwsIpPrefix {
            services: [Rc::from("EC2"), Rc::from("ROUTE53_HEALTHCHECKS")]
                .into_iter()
                .collect(),
            ..test_aws_ipv4_prefix()
        };

        assert!(prefix1 < prefix2); // Shorter prefix length is less than longer prefix length
        assert!(prefix2 < prefix3); // Lower prefix address is less than higher prefix address
        assert!(prefix1 < prefix4); // Lower region is less than higher region
        assert!(prefix1 < prefix5); // Lower network border group is less than higher network border group
        assert!(prefix1 < prefix6); // Lexicographically-equal shorter service set is less than longer set
        assert!(prefix6 < prefix7); // Lexicographically-lower service is less than higher service
    }

    #[test]
    fn test_aws_ip_prefix_equality() {
        let prefix1 = test_aws_ipv4_prefix();
        let prefix2 = test_aws_ipv4_prefix();
        let prefix3 = AwsIpPrefix {
            region: Rc::from("us-west-1"),
            ..test_aws_ipv4_prefix()
        };
        let prefix4 = AwsIpPrefix {
            network_border_group: Rc::from("us-west-1"),
            ..test_aws_ipv4_prefix()
        };
        let prefix5 = AwsIpPrefix {
            services: [Rc::from("EC2"), Rc::from("S3")].into_iter().collect(),
            ..test_aws_ipv4_prefix()
        };

        assert_eq!(prefix1, prefix2); // Equal prefixes
        assert_ne!(prefix1, prefix3); // Different regions
        assert_ne!(prefix1, prefix4); // Different network border groups
        assert_ne!(prefix1, prefix5); // Different services
    }
}
