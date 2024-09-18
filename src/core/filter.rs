use crate::core::aws_ip_prefix::AwsIpPrefix;
use crate::core::aws_ip_ranges::AwsIpRanges;
use crate::core::errors::Result;
use crate::core::prefix_type::PrefixType;
use log::trace;
use std::collections::BTreeSet;
use std::rc::Rc;

/*-------------------------------------------------------------------------------------------------
  FilterBuilder
-------------------------------------------------------------------------------------------------*/

/// Builder used to construct a [Filter] object with the desired filter parameters.
#[derive(Debug)]
pub struct FilterBuilder<'a> {
    aws_ip_ranges: &'a AwsIpRanges,

    prefix_type: Option<PrefixType>,
    regions: Option<BTreeSet<Rc<str>>>,
    network_border_groups: Option<BTreeSet<Rc<str>>>,
    services: Option<BTreeSet<Rc<str>>>,
}

/*--------------------------------------------------------------------------------------
  Filter Builder Implementation
--------------------------------------------------------------------------------------*/

impl<'a> FilterBuilder<'a> {
    /// Create a new [FilterBuilder] object for an [AwsIpRanges] object. By default, no
    /// filter parameters are set. Set the desired filter parameters using the builder
    /// methods and then call the [FilterBuilder::build] method to create the [Filter]
    /// object.
    ///
    /// ```rust
    /// # fn main() -> awsipranges::Result<()> {
    /// # let aws_ip_ranges = awsipranges::get_ranges()?;
    /// let filter = awsipranges::FilterBuilder::new(&aws_ip_ranges)
    ///     .ipv4()
    ///     .regions(["us-west-1"])?
    ///     .network_border_groups(["us-west-2-sea-1"])?
    ///     .services(["EC2"])?
    ///     .build();
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(aws_ip_ranges: &'a AwsIpRanges) -> Self {
        Self {
            aws_ip_ranges,
            prefix_type: None,
            regions: None,
            network_border_groups: None,
            services: None,
        }
    }

    /*-------------------------------------------------------------------------
      Setters
    -------------------------------------------------------------------------*/

    /// Include IPv4 prefixes.
    pub fn ipv4(mut self) -> Self {
        self.prefix_type = match self.prefix_type {
            None => Some(PrefixType::IPv4),
            Some(PrefixType::IPv4) => Some(PrefixType::IPv4),

            // Include both IPv4 and IPv6 by removing the filter
            Some(PrefixType::IPv6) => None,
        };
        self
    }

    /// Include IPv6 prefixes.
    pub fn ipv6(mut self) -> Self {
        self.prefix_type = match self.prefix_type {
            None => Some(PrefixType::IPv6),
            Some(PrefixType::IPv6) => Some(PrefixType::IPv6),

            // Include both IPv4 and IPv6 by removing the filter
            Some(PrefixType::IPv4) => None,
        };
        self
    }

    /// Include AWS IP Prefixes from the provided AWS regions.
    pub fn regions<I, S>(mut self, regions: I) -> Result<Self>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let regions: Result<BTreeSet<Rc<str>>> = regions
            .into_iter()
            .map(|region| {
                self.aws_ip_ranges
                    .get_region(region.as_ref())
                    .ok_or(format!("Invalid region: {}", region.as_ref()).into())
            })
            .collect();
        self.regions = Some(regions?);
        Ok(self)
    }

    /// Include AWS IP Prefixes from the provided network border groups.
    pub fn network_border_groups<I, S>(mut self, network_border_groups: I) -> Result<Self>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let network_border_groups: Result<BTreeSet<Rc<str>>> = network_border_groups
            .into_iter()
            .map(|network_border_group| {
                self.aws_ip_ranges
                    .get_network_border_group(network_border_group.as_ref())
                    .ok_or(
                        format!(
                            "Invalid network border group: {}",
                            network_border_group.as_ref()
                        )
                        .into(),
                    )
            })
            .collect();
        self.network_border_groups = Some(network_border_groups?);
        Ok(self)
    }

    /// Include AWS IP Prefixes used by the provided services.
    pub fn services<I, S>(mut self, services: I) -> Result<Self>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let services: Result<BTreeSet<Rc<str>>> = services
            .into_iter()
            .map(|service| {
                self.aws_ip_ranges
                    .get_service(service.as_ref())
                    .ok_or(format!("Invalid service: {}", service.as_ref()).into())
            })
            .collect();
        self.services = Some(services?);
        Ok(self)
    }

    /*-------------------------------------------------------------------------
      Build Method
    -------------------------------------------------------------------------*/

    /// Build the [Filter] object with the provided filter parameters.
    pub fn build(self) -> Filter {
        Filter {
            prefix_type: self.prefix_type,
            regions: self.regions,
            network_border_groups: self.network_border_groups,
            services: self.services,
        }
    }
}

/*-------------------------------------------------------------------------------------------------
  Filter
-------------------------------------------------------------------------------------------------*/

/// Filter used to include AWS IP Prefixes based on the prefix type (IPv4/IPv6),
/// regions, network border groups, and services associated with the prefixes. Use the
/// [FilterBuilder] to construct a [Filter] object with the desired filter parameters.
#[derive(Debug, Default)]
pub struct Filter {
    /// Only include IPv4 or IPv6 AWS IP Prefixes.
    prefix_type: Option<PrefixType>,

    /// Include AWS IP Prefixes from these AWS regions.
    regions: Option<BTreeSet<Rc<str>>>,

    /// Include AWS IP Prefixes from these network border groups.
    network_border_groups: Option<BTreeSet<Rc<str>>>,

    /// Include AWS IP Prefixes used by these services.
    services: Option<BTreeSet<Rc<str>>>,
}

/*--------------------------------------------------------------------------------------
  Filter Implementation
--------------------------------------------------------------------------------------*/

impl Filter {
    /*-------------------------------------------------------------------------
      Getters
    -------------------------------------------------------------------------*/

    /// Check if the filter includes IPv4 prefixes.
    pub fn ipv4(&self) -> bool {
        match self.prefix_type {
            None => true, // No prefix type filter includes all prefix types
            Some(prefix_type) => prefix_type.is_ipv4(),
        }
    }

    /// Check if the filter includes IPv6 prefixes.
    pub fn ipv6(&self) -> bool {
        match self.prefix_type {
            None => true, // No prefix type filter includes all prefix types
            Some(prefix_type) => prefix_type.is_ipv6(),
        }
    }

    /// AWS regions included in the filter.
    pub fn regions(&self) -> Option<&BTreeSet<Rc<str>>> {
        self.regions.as_ref()
    }

    /// Network border groups included in the filter.
    pub fn network_border_groups(&self) -> Option<&BTreeSet<Rc<str>>> {
        self.network_border_groups.as_ref()
    }

    /// AWS services included in the filter.
    pub fn services(&self) -> Option<&BTreeSet<Rc<str>>> {
        self.services.as_ref()
    }

    /*-------------------------------------------------------------------------
      Filter Functions
    -------------------------------------------------------------------------*/

    pub(crate) fn match_prefix_type(&self, aws_ip_prefix: &AwsIpPrefix) -> bool {
        if let Some(prefix_type) = self.prefix_type {
            if prefix_type.is_ipv4() && aws_ip_prefix.prefix.is_ipv4() {
                true
            } else {
                prefix_type.is_ipv6() && aws_ip_prefix.prefix.is_ipv6()
            }
        } else {
            // No prefix type filter includes all prefix types
            true
        }
    }

    pub(crate) fn match_regions(&self, aws_ip_prefix: &AwsIpPrefix) -> bool {
        if let Some(filter_regions) = &self.regions {
            filter_regions.contains(&aws_ip_prefix.region)
        } else {
            trace!("No `regions` filter");
            true
        }
    }

    pub(crate) fn match_network_border_groups(&self, aws_ip_prefix: &AwsIpPrefix) -> bool {
        if let Some(filter_network_border_groups) = &self.network_border_groups {
            filter_network_border_groups.contains(&aws_ip_prefix.network_border_group)
        } else {
            trace!("No `network_border_groups` filter");
            true
        }
    }

    pub(crate) fn match_services(&self, aws_ip_prefix: &AwsIpPrefix) -> bool {
        if let Some(filter_services) = &self.services {
            filter_services
                .intersection(&aws_ip_prefix.services)
                .next()
                .is_some()
        } else {
            trace!("No `services` filter");
            true
        }
    }

    pub(crate) fn include_prefix(&self, prefix: &AwsIpPrefix) -> bool {
        let filters = [
            Filter::match_prefix_type,
            Filter::match_regions,
            Filter::match_network_border_groups,
            Filter::match_services,
        ];
        filters.iter().all(|filter| filter(self, prefix))
    }
}

/*-------------------------------------------------------------------------------------------------
  Unit Tests
-------------------------------------------------------------------------------------------------*/

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::aws_ip_prefix::tests::{test_aws_ipv4_prefix, test_aws_ipv6_prefix};
    use crate::core::aws_ip_ranges::tests::test_aws_ip_ranges;

    /*----------------------------------------------------------------------------------
      Filter Builder and Filter
    ----------------------------------------------------------------------------------*/

    /*-------------------------------------------------------------------------
      Test Getter and Setter Methods
    -------------------------------------------------------------------------*/

    #[test]
    fn test_getter_and_setter_methods() {
        let aws_ip_ranges = test_aws_ip_ranges();

        // Filter 1: IPv4, us-east-1, us-west-1, EC2, S3
        let filter1 = FilterBuilder::new(&aws_ip_ranges)
            .ipv4()
            .regions(["us-east-1", "us-west-1"])
            .unwrap()
            .network_border_groups(["us-east-1", "us-west-1"])
            .unwrap()
            .services(["EC2", "S3"])
            .unwrap()
            .build();

        assert!(filter1.ipv4());
        assert!(!filter1.ipv6());
        assert_eq!(filter1.regions().unwrap().len(), 2);
        assert_eq!(filter1.network_border_groups().unwrap().len(), 2);
        assert_eq!(filter1.services().unwrap().len(), 2);

        // Filter 2: IPv6, us-east-1, S3
        let filter2 = FilterBuilder::new(&aws_ip_ranges)
            .ipv6()
            .regions(["us-east-1"])
            .unwrap()
            .network_border_groups(["us-east-1"])
            .unwrap()
            .services(["S3"])
            .unwrap()
            .build();

        assert!(!filter2.ipv4());
        assert!(filter2.ipv6());
        assert_eq!(filter2.regions().unwrap().len(), 1);
        assert_eq!(filter2.network_border_groups().unwrap().len(), 1);
        assert_eq!(filter2.services().unwrap().len(), 1);

        // Filter 3: IPv4/Ipv6 Empty filter
        let filter3 = FilterBuilder::new(&aws_ip_ranges).ipv4().ipv6().build();

        assert!(filter3.ipv4());
        assert!(filter3.ipv6());
        assert!(filter3.regions().is_none());
        assert!(filter3.network_border_groups().is_none());
        assert!(filter3.services().is_none());
    }

    /*-------------------------------------------------------------------------
      Test Filter Functions
    -------------------------------------------------------------------------*/
    #[test]
    fn test_filter_match_prefix_type() {
        let aws_ip_ranges = test_aws_ip_ranges();

        let filter_ipv4 = FilterBuilder::new(&aws_ip_ranges).ipv4().build();
        let filter_ipv6 = FilterBuilder::new(&aws_ip_ranges).ipv6().build();
        let filter_none = FilterBuilder::new(&aws_ip_ranges).build();

        let ipv4_prefix = test_aws_ipv4_prefix();
        let ipv6_prefix = test_aws_ipv6_prefix();

        assert!(filter_ipv4.match_prefix_type(&ipv4_prefix)); // IPv4 filter matches IPv4 prefix
        assert!(!filter_ipv4.match_prefix_type(&ipv6_prefix)); // IPv4 filter does not match IPv6 prefix

        assert!(filter_ipv6.match_prefix_type(&ipv6_prefix)); // IPv6 filter matches IPv6 prefix
        assert!(!filter_ipv6.match_prefix_type(&ipv4_prefix)); // IPv6 filter does not match IPv4 prefix

        assert!(filter_none.match_prefix_type(&ipv4_prefix)); // No filter matches IPv4 prefix
        assert!(filter_none.match_prefix_type(&ipv6_prefix)); // No filter matches IPv6 prefix
    }

    #[test]
    fn test_filter_match_regions() {
        let aws_ip_ranges = test_aws_ip_ranges();

        let region_filter = FilterBuilder::new(&aws_ip_ranges)
            .regions(["us-east-1"])
            .unwrap()
            .build();
        let no_region_filter = Filter::default();

        let prefix1 = test_aws_ipv4_prefix();
        let prefix2 = AwsIpPrefix {
            region: Rc::from("us-west-1"),
            ..test_aws_ipv4_prefix()
        };

        assert!(region_filter.match_regions(&prefix1)); // Region filter matches prefix with correct region
        assert!(!region_filter.match_regions(&prefix2)); // Region filter does not match prefix with incorrect region

        assert!(no_region_filter.match_regions(&prefix1)); // No region filter matches any prefix region
        assert!(no_region_filter.match_regions(&prefix2)); // No region filter matches any prefix region
    }

    #[test]
    fn test_filter_match_network_border_group() {
        let aws_ip_ranges = test_aws_ip_ranges();

        let network_border_group_filter = FilterBuilder::new(&aws_ip_ranges)
            .network_border_groups(["us-east-1"])
            .unwrap()
            .build();
        let no_network_border_group_filter = Filter::default();

        let prefix1 = test_aws_ipv4_prefix();
        let prefix2 = AwsIpPrefix {
            network_border_group: Rc::from("us-west-1"),
            ..test_aws_ipv4_prefix()
        };

        assert!(network_border_group_filter.match_network_border_groups(&prefix1)); // Network border group filter matches prefix with correct network border group
        assert!(!network_border_group_filter.match_network_border_groups(&prefix2)); // Network border group filter does not match prefix with incorrect network border group

        assert!(no_network_border_group_filter.match_network_border_groups(&prefix1)); // No network border group filter matches any prefix network border group
        assert!(no_network_border_group_filter.match_network_border_groups(&prefix2));
        // No network border group filter matches any prefix network border group
    }

    #[test]
    fn test_filter_match_services() {
        let aws_ip_ranges = test_aws_ip_ranges();

        let service_filter = FilterBuilder::new(&aws_ip_ranges)
            .services(["EC2"])
            .unwrap()
            .build();
        let no_service_filter = FilterBuilder::new(&aws_ip_ranges).build();

        let prefix1 = AwsIpPrefix {
            services: [Rc::from("EC2"), Rc::from("S3")].into_iter().collect(),
            ..test_aws_ipv4_prefix()
        };
        let prefix2 = AwsIpPrefix {
            services: [Rc::from("S3")].into_iter().collect(),
            ..test_aws_ipv4_prefix()
        };

        assert!(service_filter.match_services(&prefix1)); // Service filter matches prefix containing service
        assert!(!service_filter.match_services(&prefix2)); // Service filter does not match prefix not containing service

        assert!(no_service_filter.match_services(&prefix1)); // No service filter matches any prefix
        assert!(no_service_filter.match_services(&prefix2)); // No service filter matches any prefix
    }
}
