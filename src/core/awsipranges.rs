use crate::core::errors::Result;
use crate::core::json;
use crate::core::utils;
use chrono::{DateTime, Utc};
use ipnetwork::IpNetwork;
use log::{trace, warn};
use std::collections::{BTreeMap, BTreeSet};
use std::convert::From;
use std::ops::Bound::Included;
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
  AWS IP Ranges
-------------------------------------------------------------------------------------------------*/

/// Collection of AWS IP ranges that provides methods to search and filter the prefixes and
/// extract information about the regions, network border groups, and services represented in the
/// collection.
#[derive(Clone, Debug, Default)]
pub struct AwsIpRanges {
    pub(crate) sync_token: String,
    pub(crate) create_date: DateTime<Utc>,

    pub(crate) regions: BTreeSet<Rc<str>>,
    pub(crate) network_border_groups: BTreeSet<Rc<str>>,
    pub(crate) services: BTreeSet<Rc<str>>,

    pub(crate) prefixes: BTreeMap<IpNetwork, AwsIpPrefix>,
}

impl AwsIpRanges {
    /// Publication time of the current set of AWS IP Ranges in Unix epoch time format.
    pub fn sync_token(&self) -> &String {
        &self.sync_token
    }

    /// Publication time of the current set of AWS IP Ranges in UTC `DateTime` format.
    pub fn create_date(&self) -> &DateTime<Utc> {
        &self.create_date
    }

    /// AWS regions represented in the current set of AWS IP Ranges.
    pub fn regions(&self) -> &BTreeSet<Rc<str>> {
        &self.regions
    }

    /// Network border groups represented in the current set of AWS IP Ranges.
    pub fn network_border_groups(&self) -> &BTreeSet<Rc<str>> {
        &self.network_border_groups
    }

    /// AWS services represented in the current set of AWS IP Ranges.
    pub fn services(&self) -> &BTreeSet<Rc<str>> {
        &self.services
    }

    /// Map of [IpNetwork] CIDRs to [AwsIpPrefix] records.
    pub fn prefixes(&self) -> &BTreeMap<IpNetwork, AwsIpPrefix> {
        &self.prefixes
    }

    /// Get the [AwsIpPrefix] record for the provided [IpNetwork] CIDR.
    pub fn get_prefix(&self, value: &IpNetwork) -> Option<&AwsIpPrefix> {
        self.prefixes.get(value)
    }

    /// Get the longest matching [AwsIpPrefix] record for the provided [IpNetwork] CIDR.
    pub fn get_longest_match_prefix(&self, value: &IpNetwork) -> Option<&AwsIpPrefix> {
        let lower_bound = match value {
            IpNetwork::V4(_) => utils::ipnetwork::new_network_prefix(value, 8u8).unwrap(),
            IpNetwork::V6(_) => utils::ipnetwork::new_network_prefix(value, 16u8).unwrap(),
        };
        let upper_bound = utils::ipnetwork::network_prefix(value);

        self.prefixes
            .range((Included(lower_bound), Included(upper_bound)))
            .rev()
            .map(|(_, aws_ip_prefix)| aws_ip_prefix)
            .find(|&aws_ip_prefix| utils::ipnetwork::is_supernet_of(aws_ip_prefix.prefix, *value))
    }

    /// Get all [AwsIpPrefix] records that are supernets of the provided [IpNetwork] CIDR.
    pub fn get_supernet_prefixes(&self, value: &IpNetwork) -> Option<BTreeSet<AwsIpPrefix>> {
        let mut aws_ip_prefixes: BTreeSet<AwsIpPrefix> = BTreeSet::new();

        let lower_bound = match value {
            IpNetwork::V4(_) => utils::ipnetwork::new_network_prefix(value, 8u8).unwrap(),
            IpNetwork::V6(_) => utils::ipnetwork::new_network_prefix(value, 16u8).unwrap(),
        };
        let upper_bound = utils::ipnetwork::network_prefix(value);

        for (_, aws_ip_prefix) in self
            .prefixes
            .range((Included(lower_bound), Included(upper_bound)))
        {
            if utils::ipnetwork::is_supernet_of(aws_ip_prefix.prefix, *value) {
                aws_ip_prefixes.insert(aws_ip_prefix.clone());
            }
        }

        if !aws_ip_prefixes.is_empty() {
            Some(aws_ip_prefixes)
        } else {
            None
        }
    }

    /// Get a reference-counted string (`Rc<str>`) region for the provided region name.
    pub fn get_region(&self, value: &str) -> Option<Rc<str>> {
        utils::get_rc_str_from_set(value, &self.regions)
    }

    /// Get a reference-counted string (`Rc<str>`) network border group for the provided network border group name.
    pub fn get_network_border_group(&self, value: &str) -> Option<Rc<str>> {
        utils::get_rc_str_from_set(value, &self.network_border_groups)
    }

    /// Get a reference-counted string (`Rc<str>`) service for the provided service name.
    pub fn get_service(&self, value: &str) -> Option<Rc<str>> {
        utils::get_rc_str_from_set(value, &self.services)
    }

    /// Search for the AWS IP Prefixes that contain the provided [IpNetwork] CIDRs.
    pub fn search<'p, I>(&self, values: I) -> Box<SearchResults>
    where
        I: IntoIterator<Item = &'p IpNetwork>,
    {
        let mut search_results = Box::new(SearchResults {
            aws_ip_ranges: Box::new(AwsIpRanges::default()),
            prefix_matches: BTreeMap::new(),
            prefixes_not_found: BTreeSet::new(),
        });

        let mut result_aws_ip_prefixes: BTreeSet<AwsIpPrefix> = BTreeSet::new();

        for prefix in values.into_iter() {
            if let Some(aws_ip_prefixes) = self.get_supernet_prefixes(prefix) {
                aws_ip_prefixes.iter().for_each(|aws_ip_prefix| {
                    result_aws_ip_prefixes.insert(aws_ip_prefix.clone());
                });

                search_results
                    .prefix_matches
                    .insert(*prefix, aws_ip_prefixes);
            } else {
                warn!("Search CIDR not found in AWS IP ranges: {prefix}");
                search_results.prefixes_not_found.insert(*prefix);
            }
        }

        search_results.aws_ip_ranges = Box::new(AwsIpRanges::from(result_aws_ip_prefixes));
        search_results
            .aws_ip_ranges
            .sync_token
            .clone_from(&self.sync_token);
        search_results.aws_ip_ranges.create_date = self.create_date;

        search_results
    }

    /// Filter the AWS IP Prefixes using the provided `Filter`.
    pub fn filter(&self, filter: &Filter) -> Box<AwsIpRanges> {
        let filtered_aws_ip_prefix_map: BTreeMap<IpNetwork, AwsIpPrefix> = self
            .prefixes
            .iter()
            .filter(|(_, aws_ip_prefix)| filter.include_prefix(aws_ip_prefix))
            .map(|(prefix, aws_ip_prefix)| (*prefix, aws_ip_prefix.clone()))
            .collect();

        let mut aws_ip_ranges = Box::new(AwsIpRanges::from(filtered_aws_ip_prefix_map));
        aws_ip_ranges.sync_token.clone_from(&self.sync_token);
        aws_ip_ranges.create_date = self.create_date;

        aws_ip_ranges
    }

    pub(crate) fn from_json(json: &str) -> Result<Box<AwsIpRanges>> {
        let json_ip_ranges = json::parse(json)?;

        let mut aws_ip_ranges = Box::new(AwsIpRanges::default());

        aws_ip_ranges.sync_token = json_ip_ranges.sync_token.to_string();
        aws_ip_ranges.create_date = json_ip_ranges.create_date;

        aws_ip_ranges.regions = json_ip_ranges
            .prefixes
            .iter()
            .map(|prefix| prefix.region)
            .chain(
                json_ip_ranges
                    .ipv6_prefixes
                    .iter()
                    .map(|ipv6_prefix| ipv6_prefix.region),
            )
            .map(Rc::from)
            .collect();

        aws_ip_ranges.network_border_groups = json_ip_ranges
            .prefixes
            .iter()
            .map(|prefix| prefix.network_border_group)
            .chain(
                json_ip_ranges
                    .ipv6_prefixes
                    .iter()
                    .map(|ipv6_prefix| ipv6_prefix.network_border_group),
            )
            .map(Rc::from)
            .collect();

        aws_ip_ranges.services = json_ip_ranges
            .prefixes
            .iter()
            .map(|prefix| prefix.service)
            .chain(
                json_ip_ranges
                    .ipv6_prefixes
                    .iter()
                    .map(|ipv6_prefix| ipv6_prefix.service),
            )
            .map(Rc::from)
            .collect();

        for json_ipv4_prefix in &json_ip_ranges.prefixes {
            aws_ip_ranges
                .prefixes
                .entry(IpNetwork::V4(json_ipv4_prefix.ip_prefix))
                .and_modify(|prefix| {
                    // Verify IP prefix invariants
                    // An IP prefix should always be assigned to a single region and network border group
                    assert_eq!(
                        prefix.region,
                        utils::get_rc_str_from_set(json_ipv4_prefix.region, &aws_ip_ranges.regions)
                            .unwrap()
                    );
                    assert_eq!(
                        prefix.network_border_group,
                        utils::get_rc_str_from_set(
                            json_ipv4_prefix.network_border_group,
                            &aws_ip_ranges.network_border_groups
                        )
                        .unwrap()
                    );
                    // Duplicate IP prefix entries are used to indicate multiple AWS services use a prefix
                    prefix.services.insert(
                        utils::get_rc_str_from_set(
                            json_ipv4_prefix.service,
                            &aws_ip_ranges.services,
                        )
                        .unwrap(),
                    );
                })
                .or_insert(AwsIpPrefix {
                    prefix: IpNetwork::V4(json_ipv4_prefix.ip_prefix),
                    region: utils::get_rc_str_from_set(
                        json_ipv4_prefix.region,
                        &aws_ip_ranges.regions,
                    )
                    .unwrap(),
                    network_border_group: utils::get_rc_str_from_set(
                        json_ipv4_prefix.network_border_group,
                        &aws_ip_ranges.network_border_groups,
                    )
                    .unwrap(),
                    services: BTreeSet::from([utils::get_rc_str_from_set(
                        json_ipv4_prefix.service,
                        &aws_ip_ranges.services,
                    )
                    .unwrap()]),
                });
        }

        for json_ipv6_prefix in &json_ip_ranges.ipv6_prefixes {
            aws_ip_ranges
                .prefixes
                .entry(IpNetwork::V6(json_ipv6_prefix.ipv6_prefix))
                .and_modify(|prefix| {
                    // Verify IP prefix invariants
                    // An IP prefix should always be assigned to a single region and network border group
                    assert_eq!(
                        prefix.region,
                        utils::get_rc_str_from_set(json_ipv6_prefix.region, &aws_ip_ranges.regions)
                            .unwrap()
                    );
                    assert_eq!(
                        prefix.network_border_group,
                        utils::get_rc_str_from_set(
                            json_ipv6_prefix.network_border_group,
                            &aws_ip_ranges.network_border_groups
                        )
                        .unwrap()
                    );
                    // Duplicate IP prefix entries are used to indicate multiple AWS services use a prefix
                    prefix.services.insert(
                        utils::get_rc_str_from_set(
                            json_ipv6_prefix.service,
                            &aws_ip_ranges.services,
                        )
                        .unwrap(),
                    );
                })
                .or_insert(AwsIpPrefix {
                    prefix: IpNetwork::V6(json_ipv6_prefix.ipv6_prefix),
                    region: utils::get_rc_str_from_set(
                        json_ipv6_prefix.region,
                        &aws_ip_ranges.regions,
                    )
                    .unwrap(),
                    network_border_group: utils::get_rc_str_from_set(
                        json_ipv6_prefix.network_border_group,
                        &aws_ip_ranges.network_border_groups,
                    )
                    .unwrap(),
                    services: BTreeSet::from([utils::get_rc_str_from_set(
                        json_ipv6_prefix.service,
                        &aws_ip_ranges.services,
                    )
                    .unwrap()]),
                });
        }

        Ok(aws_ip_ranges)
    }
}

impl From<BTreeSet<AwsIpPrefix>> for AwsIpRanges {
    fn from(value: BTreeSet<AwsIpPrefix>) -> Self {
        let aws_ip_prefix_map: BTreeMap<IpNetwork, AwsIpPrefix> = value
            .into_iter()
            .map(|aws_ip_prefix| (aws_ip_prefix.prefix, aws_ip_prefix))
            .collect();

        Self::from(aws_ip_prefix_map)
    }
}

impl From<BTreeMap<IpNetwork, AwsIpPrefix>> for AwsIpRanges {
    fn from(value: BTreeMap<IpNetwork, AwsIpPrefix>) -> Self {
        let mut aws_ip_ranges = AwsIpRanges::default();

        aws_ip_ranges.prefixes = value;

        aws_ip_ranges.regions = aws_ip_ranges
            .prefixes
            .values()
            .map(|prefix| prefix.region.clone())
            .collect();

        aws_ip_ranges.network_border_groups = aws_ip_ranges
            .prefixes
            .values()
            .map(|prefix| prefix.network_border_group.clone())
            .collect();

        aws_ip_ranges.services = aws_ip_ranges
            .prefixes
            .values()
            .flat_map(|prefix| &prefix.services)
            .cloned()
            .collect();

        aws_ip_ranges
    }
}

/*-------------------------------------------------------------------------------------------------
  Search Results
-------------------------------------------------------------------------------------------------*/

/// Search results containing the matching [AwsIpRanges], a map of found
/// prefixes, and the set of prefixes not found in the AWS IP Ranges.
#[derive(Clone, Debug, Default)]
pub struct SearchResults {
    /// [AwsIpRanges] object containing the matching AWS IP Prefixes.
    pub aws_ip_ranges: Box<AwsIpRanges>,

    /// Map of found [IpNetwork] prefixes to the sets of [AwsIpPrefix] records
    /// that contain the prefixes.
    pub prefix_matches: BTreeMap<IpNetwork, BTreeSet<AwsIpPrefix>>,

    /// Set of [IpNetwork] prefixes not found in the AWS IP Ranges.
    pub prefixes_not_found: BTreeSet<IpNetwork>,
}

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

    fn match_prefix_type(&self, aws_ip_prefix: &AwsIpPrefix) -> bool {
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

    fn match_regions(&self, aws_ip_prefix: &AwsIpPrefix) -> bool {
        if let Some(filter_regions) = &self.regions {
            filter_regions.contains(&aws_ip_prefix.region)
        } else {
            trace!("No `regions` filter");
            true
        }
    }

    fn match_network_border_groups(&self, aws_ip_prefix: &AwsIpPrefix) -> bool {
        if let Some(filter_network_border_groups) = &self.network_border_groups {
            filter_network_border_groups.contains(&aws_ip_prefix.network_border_group)
        } else {
            trace!("No `network_border_groups` filter");
            true
        }
    }

    fn match_services(&self, aws_ip_prefix: &AwsIpPrefix) -> bool {
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

    fn include_prefix(&self, prefix: &AwsIpPrefix) -> bool {
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
  Prefix Type
-------------------------------------------------------------------------------------------------*/

/// IP prefix type (IPv4 or IPv6) used to filter the AWS IP Prefixes.
#[derive(Debug, Clone, Copy)]
pub enum PrefixType {
    IPv4,
    IPv6,
}

impl PrefixType {
    pub fn is_ipv4(&self) -> bool {
        match self {
            PrefixType::IPv4 => true,
            PrefixType::IPv6 => false,
        }
    }

    pub fn is_ipv6(&self) -> bool {
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
      Test Helper Functions
    ----------------------------------------------------------------------------------*/

    fn test_aws_ipv4_prefix() -> AwsIpPrefix {
        AwsIpPrefix {
            prefix: "10.0.0.0/8".parse().unwrap(),
            region: Rc::from("us-east-1"),
            network_border_group: Rc::from("us-east-1"),
            services: [Rc::from("EC2")].into_iter().collect(),
        }
    }

    fn test_aws_ipv6_prefix() -> AwsIpPrefix {
        AwsIpPrefix {
            prefix: "2001:db8::/32".parse().unwrap(),
            region: Rc::from("us-east-1"),
            network_border_group: Rc::from("us-east-1"),
            services: [Rc::from("EC2")].into_iter().collect(),
        }
    }

    fn test_aws_ip_ranges() -> Box<AwsIpRanges> {
        let create_date = Utc::now();
        let sync_token = create_date.timestamp().to_string();
        let prefixes: BTreeSet<AwsIpPrefix> = [
            test_aws_ipv4_prefix(),
            AwsIpPrefix {
                prefix: "10.0.0.0/16".parse().unwrap(),
                ..test_aws_ipv4_prefix()
            },
            AwsIpPrefix {
                prefix: "10.1.0.0/16".parse().unwrap(),
                region: Rc::from("us-west-1"),
                network_border_group: Rc::from("us-west-1"),
                services: [Rc::from("EC2"), Rc::from("S3")].into_iter().collect(),
            },
            test_aws_ipv6_prefix(),
            AwsIpPrefix {
                prefix: "2001:db8::/48".parse().unwrap(),
                ..test_aws_ipv6_prefix()
            },
            AwsIpPrefix {
                prefix: "2001:db8:1::/48".parse().unwrap(),
                region: Rc::from("us-west-1"),
                network_border_group: Rc::from("us-west-1"),
                services: [Rc::from("EC2"), Rc::from("S3")].into_iter().collect(),
            },
        ]
        .into_iter()
        .collect();

        let mut aws_ip_ranges = Box::new(AwsIpRanges::from(prefixes));
        aws_ip_ranges.sync_token = sync_token;
        aws_ip_ranges.create_date = create_date;

        aws_ip_ranges
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

    /*----------------------------------------------------------------------------------
      AwsIpRanges
    ----------------------------------------------------------------------------------*/

    #[test]
    fn test_aws_ip_ranges_sync_token() {
        let create_date = Utc::now();
        let sync_token = create_date.timestamp().to_string();
        let aws_ip_ranges = AwsIpRanges {
            sync_token: sync_token.clone(),
            ..Default::default()
        };
        assert_eq!(aws_ip_ranges.sync_token(), &sync_token);
    }

    #[test]
    fn test_aws_ip_ranges_create_date() {
        let create_date = Utc::now();
        let aws_ip_ranges = AwsIpRanges {
            create_date,
            ..Default::default()
        };
        assert_eq!(aws_ip_ranges.create_date(), &create_date);
    }

    #[test]
    fn test_get_prefix() {
        let aws_ip_ranges = test_aws_ip_ranges();

        let prefix_in_range: IpNetwork = "10.0.0.0/8".parse().unwrap();
        assert_eq!(
            aws_ip_ranges.get_prefix(&prefix_in_range).unwrap().prefix,
            prefix_in_range
        );

        let prefix_not_in_range: IpNetwork = "192.168.0.0/24".parse().unwrap();
        assert_eq!(aws_ip_ranges.get_prefix(&prefix_not_in_range), None);
    }

    #[test]
    fn test_get_longest_match_prefix() {
        let aws_ip_ranges = test_aws_ip_ranges();

        let prefix_in_range: IpNetwork = "10.0.0.0/32".parse().unwrap();
        let longest_match: IpNetwork = "10.0.0.0/16".parse().unwrap();
        assert_eq!(
            aws_ip_ranges
                .get_longest_match_prefix(&prefix_in_range)
                .unwrap()
                .prefix,
            longest_match
        );

        let prefix_not_in_range: IpNetwork = "192.168.0.0/24".parse().unwrap();
        assert_eq!(
            aws_ip_ranges.get_longest_match_prefix(&prefix_not_in_range),
            None
        );
    }

    #[test]
    fn test_get_supernet_prefixes() {
        let aws_ip_ranges = test_aws_ip_ranges();

        let prefix_in_range: IpNetwork = "10.0.0.0/24".parse().unwrap();
        let supernet_prefixes = aws_ip_ranges
            .get_supernet_prefixes(&prefix_in_range)
            .unwrap();
        assert_eq!(supernet_prefixes.len(), 2);
        assert!(supernet_prefixes.contains(&test_aws_ipv4_prefix()));

        let prefix_not_in_range: IpNetwork = "192.168.0.0/24".parse().unwrap();
        assert_eq!(
            aws_ip_ranges.get_longest_match_prefix(&prefix_not_in_range),
            None
        );
    }

    #[test]
    fn test_aws_ip_ranges_regions() {
        let regions: BTreeSet<Rc<str>> = [Rc::from("us-east-1"), Rc::from("us-west-1")]
            .into_iter()
            .collect();
        let aws_ip_ranges = AwsIpRanges {
            regions: regions.clone(),
            ..Default::default()
        };
        assert_eq!(aws_ip_ranges.regions(), &regions);
    }

    #[test]
    fn test_aws_ip_ranges_network_border_groups() {
        let network_border_groups: BTreeSet<Rc<str>> =
            [Rc::from("us-east-1"), Rc::from("us-west-1")]
                .into_iter()
                .collect();
        let aws_ip_ranges = AwsIpRanges {
            network_border_groups: network_border_groups.clone(),
            ..Default::default()
        };
        assert_eq!(
            aws_ip_ranges.network_border_groups(),
            &network_border_groups
        );
    }

    #[test]
    fn test_aws_ip_ranges_services() {
        let services: BTreeSet<Rc<str>> = [Rc::from("EC2"), Rc::from("S3")].into_iter().collect();
        let aws_ip_ranges = AwsIpRanges {
            services: services.clone(),
            ..Default::default()
        };
        assert_eq!(aws_ip_ranges.services(), &services);
    }

    #[test]
    fn test_aws_ip_ranges_get_region() {
        let region: Rc<str> = Rc::from("us-east-1");
        let aws_ip_ranges = AwsIpRanges {
            regions: [region.clone()].into_iter().collect(),
            ..Default::default()
        };
        assert_eq!(aws_ip_ranges.get_region("us-east-1").unwrap(), region);
    }

    #[test]
    fn test_aws_ip_ranges_get_network_border_group() {
        let network_border_group: Rc<str> = Rc::from("us-east-1");
        let aws_ip_ranges = AwsIpRanges {
            network_border_groups: [network_border_group.clone()].into_iter().collect(),
            ..Default::default()
        };
        assert_eq!(
            aws_ip_ranges.get_network_border_group("us-east-1").unwrap(),
            network_border_group
        );
    }

    #[test]
    fn test_aws_ip_ranges_get_service() {
        let service: Rc<str> = Rc::from("EC2");
        let aws_ip_ranges = AwsIpRanges {
            services: [service.clone()].into_iter().collect(),
            ..Default::default()
        };
        assert_eq!(aws_ip_ranges.get_service("EC2").unwrap(), service);
    }

    #[test]
    fn test_aws_ip_ranges_prefixes() {
        let prefixes: BTreeMap<IpNetwork, AwsIpPrefix> = [
            test_aws_ipv4_prefix(),
            AwsIpPrefix {
                prefix: "10.0.0.0/16".parse().unwrap(),
                ..test_aws_ipv4_prefix()
            },
            AwsIpPrefix {
                prefix: "10.1.0.0/16".parse().unwrap(),
                region: Rc::from("us-west-1"),
                network_border_group: Rc::from("us-west-1"),
                services: [Rc::from("EC2"), Rc::from("S3")].into_iter().collect(),
            },
        ]
        .iter()
        .map(|aws_ip_prefix| (aws_ip_prefix.prefix, aws_ip_prefix.clone()))
        .collect();

        let aws_ip_ranges = Box::new(AwsIpRanges::from(prefixes.clone()));

        assert_eq!(aws_ip_ranges.prefixes(), &prefixes); // Equal prefixes
    }

    #[test]
    fn test_aws_ip_ranges_search() {
        let aws_ip_ranges = test_aws_ip_ranges();

        let search_networks = [
            test_aws_ipv4_prefix().prefix,
            "10.0.0.1/32".parse().unwrap(),
            "192.168.0.0/24".parse().unwrap(),
            test_aws_ipv6_prefix().prefix,
            "2001:db8::1/128".parse().unwrap(),
            "2001:face:1::1/64".parse().unwrap(),
        ];

        let search_results = aws_ip_ranges.search(&search_networks);

        assert!(search_results
            .prefix_matches
            .contains_key(&search_networks[0])); // Full prefix match
        assert!(search_results
            .prefix_matches
            .contains_key(&search_networks[3])); // Full prefix match

        assert_eq!(aws_ip_ranges.prefixes().len(), 6); // Original AWS IP ranges unchanged
        assert_eq!(search_results.aws_ip_ranges.prefixes.len(), 4); // Search results AWS IP ranges

        assert!(search_results
            .prefixes_not_found
            .contains(&search_networks[2])); // No prefix match
        assert!(search_results
            .prefixes_not_found
            .contains(&search_networks[5])); // No prefix match
    }

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

    /*-------------------------------------------------------------------------
      Test Filter
    -------------------------------------------------------------------------*/

    #[test]
    fn test_filter() {
        let aws_ip_ranges = test_aws_ip_ranges();

        let filter = FilterBuilder::new(&aws_ip_ranges)
            .ipv4()
            .regions(["us-west-1"])
            .unwrap()
            .network_border_groups(["us-west-1"])
            .unwrap()
            .services(["EC2"])
            .unwrap()
            .build();

        let filtered_aws_ip_ranges = aws_ip_ranges.filter(&filter);

        assert_eq!(filtered_aws_ip_ranges.prefixes.len(), 1);
    }

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
