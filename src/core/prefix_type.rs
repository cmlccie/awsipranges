/*-------------------------------------------------------------------------------------------------
  Prefix Type
-------------------------------------------------------------------------------------------------*/

/// IP prefix type (IPv4 or IPv6) used to filter the AWS IP Prefixes.
#[derive(Debug, Clone, Copy)]
pub(crate) enum PrefixType {
    IPv4,
    IPv6,
}

impl PrefixType {
    pub(crate) fn is_ipv4(&self) -> bool {
        match self {
            PrefixType::IPv4 => true,
            PrefixType::IPv6 => false,
        }
    }

    pub(crate) fn is_ipv6(&self) -> bool {
        match self {
            PrefixType::IPv4 => false,
            PrefixType::IPv6 => true,
        }
    }
}

/*-------------------------------------------------------------------------------------------------
  Unit Tests
-------------------------------------------------------------------------------------------------*/

#[cfg(test)]
mod tests {
    use super::*;

    /*----------------------------------------------------------------------------------
      PrefixType
    ----------------------------------------------------------------------------------*/

    #[test]
    fn test_prefix_type_is_ipv4() {
        let ipv4 = PrefixType::IPv4;
        assert!(ipv4.is_ipv4());
        assert!(!ipv4.is_ipv6());
    }

    #[test]
    fn test_prefix_type_is_ipv6() {
        let ipv6 = PrefixType::IPv6;
        assert!(!ipv6.is_ipv4());
        assert!(ipv6.is_ipv6());
    }
}
