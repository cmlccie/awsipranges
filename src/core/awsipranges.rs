use crate::core::errors::Result;
use crate::core::json;
use crate::core::utils;
use chrono::{DateTime, Utc};
use ipnetwork::IpNetwork;
use log::trace;
use log::warn;
use std::collections::{BTreeMap, BTreeSet};
use std::convert::From;
use std::ops::Bound::Included;
use std::rc::Rc;

/*-------------------------------------------------------------------------------------------------
  AWS IP Prefix
-------------------------------------------------------------------------------------------------*/

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct AwsIpPrefix {
    pub prefix: IpNetwork,
    pub region: Rc<str>,
    pub network_border_group: Rc<str>,
    pub services: BTreeSet<Rc<str>>,
}

/*-------------------------------------------------------------------------------------------------
  AWS IP Ranges
-------------------------------------------------------------------------------------------------*/

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
    /// The "sync token" is a string containing the publication time for the current set of AWS IP
    /// Ranges, in Unix epoch time format.
    ///
    /// ```
    /// # let aws_ip_ranges = awsipranges::get_ranges()?;
    /// let sync_token: &String = aws_ip_ranges.sync_token();
    /// println!("Sync Token: {sync_token}");
    /// # Ok::<(), awsipranges::Error>(())
    /// ```
    pub fn sync_token(&self) -> &String {
        &self.sync_token
    }

    pub fn create_date(&self) -> &DateTime<Utc> {
        &self.create_date
    }

    pub fn regions(&self) -> &BTreeSet<Rc<str>> {
        &self.regions
    }

    pub fn network_border_groups(&self) -> &BTreeSet<Rc<str>> {
        &self.network_border_groups
    }

    pub fn services(&self) -> &BTreeSet<Rc<str>> {
        &self.services
    }

    pub fn prefixes(&self) -> &BTreeMap<IpNetwork, AwsIpPrefix> {
        &self.prefixes
    }

    pub fn get_ip_network(&self, value: &IpNetwork) -> Option<BTreeSet<AwsIpPrefix>> {
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

        if aws_ip_prefixes.len() > 0 {
            Some(aws_ip_prefixes)
        } else {
            None
        }
    }

    pub fn get_region(&self, value: &str) -> Option<Rc<str>> {
        utils::get_rc_str_from_set(value, &self.regions)
    }

    pub fn get_network_border_group(&self, value: &str) -> Option<Rc<str>> {
        utils::get_rc_str_from_set(value, &self.network_border_groups)
    }

    pub fn get_service(&self, value: &str) -> Option<Rc<str>> {
        utils::get_rc_str_from_set(value, &self.services)
    }

    pub fn search<'p, P>(&self, values: P) -> Box<SearchResults>
    where
        P: Iterator<Item = &'p IpNetwork>,
    {
        let mut search_results = Box::new(SearchResults {
            aws_ip_ranges: Box::new(AwsIpRanges::default()),
            prefix_matches: BTreeMap::new(),
            prefixes_not_found: BTreeSet::new(),
        });

        let mut result_aws_ip_prefixes: BTreeSet<AwsIpPrefix> = BTreeSet::new();

        for prefix in values {
            if let Some(aws_ip_prefixes) = self.get_ip_network(prefix) {
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
        search_results.aws_ip_ranges.sync_token = self.sync_token.clone();
        search_results.aws_ip_ranges.create_date = self.create_date;

        search_results
    }

    pub fn filter(&self, filter: &Filter) -> Box<AwsIpRanges> {
        let filtered_aws_ip_prefix_map: BTreeMap<IpNetwork, AwsIpPrefix> = self
            .prefixes
            .iter()
            .filter(|(_, aws_ip_prefix)| filter.include_prefix(*aws_ip_prefix))
            .map(|(prefix, aws_ip_prefix)| (*prefix, aws_ip_prefix.clone()))
            .collect();

        let mut aws_ip_ranges = Box::new(AwsIpRanges::from(filtered_aws_ip_prefix_map));
        aws_ip_ranges.sync_token = self.sync_token.clone();
        aws_ip_ranges.create_date = self.create_date;

        aws_ip_ranges
    }

    pub(crate) fn from_json(json: &String) -> Result<Box<AwsIpRanges>> {
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
            .map(|region| Rc::from(region))
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
            .map(|network_border_group| Rc::from(network_border_group))
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
            .map(|service| Rc::from(service))
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
            .map(|service| service.clone())
            .collect();

        aws_ip_ranges
    }
}

/*-----------------------------------------------------------------------------
  Search Results
-----------------------------------------------------------------------------*/

#[derive(Clone, Debug, Default)]
pub struct SearchResults {
    pub aws_ip_ranges: Box<AwsIpRanges>,
    pub prefix_matches: BTreeMap<IpNetwork, BTreeSet<AwsIpPrefix>>,
    pub prefixes_not_found: BTreeSet<IpNetwork>,
}

/*--------------------------------------------------------------------------------------
  Filter
--------------------------------------------------------------------------------------*/

#[derive(Debug, Default)]
pub struct Filter {
    // Only include IPv4 or IPv6 AWS IP Prefixes.
    pub prefix_type: Option<PrefixType>,

    // Only include AWS IP Prefixes that contain these prefixes.
    pub prefixes: Option<BTreeSet<IpNetwork>>,

    // Only include AWS IP Prefixes from these AWS regions.
    pub regions: Option<BTreeSet<Rc<str>>>,

    // Only include AWS IP Prefixes from these network border groups.
    pub network_border_groups: Option<BTreeSet<Rc<str>>>,

    // Only include AWS IP Prefixes used by these services.
    pub services: Option<BTreeSet<Rc<str>>>,
}

impl Filter {
    fn match_prefix_type(&self, aws_ip_prefix: &AwsIpPrefix) -> bool {
        if let Some(prefix_type) = self.prefix_type {
            if prefix_type.is_ipv4() && aws_ip_prefix.prefix.is_ipv4() {
                true
            } else if prefix_type.is_ipv6() && aws_ip_prefix.prefix.is_ipv6() {
                true
            } else {
                false
            }
        } else {
            trace!("No `prefix_type` filter");
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

    pub fn include_prefix(&self, prefix: &AwsIpPrefix) -> bool {
        let filters = [
            Filter::match_prefix_type,
            Filter::match_regions,
            Filter::match_network_border_groups,
            Filter::match_services,
        ];
        filters.iter().all(|filter| filter(self, prefix))
    }
}

/*-----------------------------------------------------------------------------
  IP Prefix Type
-----------------------------------------------------------------------------*/

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
