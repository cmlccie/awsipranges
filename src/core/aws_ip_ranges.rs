use crate::core::aws_ip_prefix::AwsIpPrefix;
use crate::core::errors::Result;
use crate::core::filter::Filter;
use crate::core::filter::FilterBuilder;
use crate::core::json;
use crate::core::search_results::SearchResults;
use crate::core::utils;
use chrono::{DateTime, Utc};
use ipnetwork::IpNetwork;
use log::warn;
use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet};
use std::convert::From;
use std::ops::Bound::Included;
use std::rc::Rc;

/*-------------------------------------------------------------------------------------------------
  AWS IP Ranges
-------------------------------------------------------------------------------------------------*/

/// Collection of AWS IP ranges providing methods to access, [AwsIpRanges::search], and
/// [AwsIpRanges::filter] the AWS IP Ranges.
#[derive(Clone, Debug, Default)]
pub struct AwsIpRanges {
    pub(crate) sync_token: String,
    pub(crate) create_date: DateTime<Utc>,

    pub(crate) regions: BTreeSet<Rc<str>>,
    pub(crate) network_border_groups: BTreeSet<Rc<str>>,
    pub(crate) services: BTreeSet<Rc<str>>,

    pub(crate) prefixes: BTreeMap<IpNetwork, AwsIpPrefix>,
}

/*--------------------------------------------------------------------------------------
  AWS IP Ranges Implementation
--------------------------------------------------------------------------------------*/

impl AwsIpRanges {
    /*-------------------------------------------------------------------------
      Getters
    -------------------------------------------------------------------------*/

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

    /*-------------------------------------------------------------------------
      Get Prefix
    -------------------------------------------------------------------------*/

    /// Get the [AwsIpPrefix] record for the provided [IpNetwork] CIDR.
    pub fn get_prefix(&self, value: &IpNetwork) -> Option<&AwsIpPrefix> {
        self.prefixes.get(value)
    }

    /*-------------------------------------------------------------------------
      Get Longest Match Prefix
    -------------------------------------------------------------------------*/

    /// Get the longest matching [AwsIpPrefix] record for the provided [IpNetwork] CIDR.
    pub fn get_longest_match_prefix(&self, value: &IpNetwork) -> Option<&AwsIpPrefix> {
        self.supernet_prefixes(*value).next_back()
    }

    /*-------------------------------------------------------------------------
      Get Supernet Prefixes
    -------------------------------------------------------------------------*/

    /// Get all [AwsIpPrefix] records that are supernets of the provided [IpNetwork] CIDR.
    pub fn get_supernet_prefixes(&self, value: &IpNetwork) -> Option<BTreeSet<AwsIpPrefix>> {
        let aws_ip_prefixes: BTreeSet<AwsIpPrefix> =
            self.supernet_prefixes(*value).cloned().collect();

        if !aws_ip_prefixes.is_empty() {
            Some(aws_ip_prefixes)
        } else {
            None
        }
    }

    /// Iterate, shortest to longest, over the AWS IP Prefixes that are supernets of `value`.
    ///
    /// Supernets sort between the enclosing /8 (IPv4) or /16 (IPv6) network, the shortest
    /// published AWS prefix lengths, and `value`'s own network prefix. Values shorter than those
    /// lengths bound the scan at their own prefix length.
    fn supernet_prefixes(&self, value: IpNetwork) -> impl DoubleEndedIterator<Item = &AwsIpPrefix> {
        let shortest_aws_prefix = match value {
            IpNetwork::V4(_) => 8u8,
            IpNetwork::V6(_) => 16u8,
        };
        let lower_bound =
            utils::ipnetwork::new_network_prefix(&value, shortest_aws_prefix.min(value.prefix()));
        let upper_bound = utils::ipnetwork::network_prefix(&value);

        self.prefixes
            .range((Included(lower_bound), Included(upper_bound)))
            .map(|(_, aws_ip_prefix)| aws_ip_prefix)
            .filter(move |aws_ip_prefix| {
                utils::ipnetwork::is_supernet_of(aws_ip_prefix.prefix, value)
            })
    }

    /*-------------------------------------------------------------------------
      Get Reference Counted Strings
    -------------------------------------------------------------------------*/

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

    /*-------------------------------------------------------------------------
      Search
    -------------------------------------------------------------------------*/

    /// Search for the AWS IP Prefixes that contain the provided [IpNetwork] CIDRs.
    ///
    /// ```rust
    /// # fn main() -> awsipranges::Result<()> {
    /// use ipnetwork::IpNetwork;
    ///
    /// let aws_ip_ranges = awsipranges::get_ranges()?;
    ///
    /// let search_prefixes: Vec<IpNetwork> = vec!["3.141.102.225".parse().unwrap()];
    /// let search_results = aws_ip_ranges.search(&search_prefixes);
    /// # Ok(())
    /// # }
    /// ```
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

    /*-------------------------------------------------------------------------
      Filter
    -------------------------------------------------------------------------*/

    /// Create a [FilterBuilder] to filter the AWS IP Ranges.
    ///
    /// ```rust
    /// # fn main() -> awsipranges::Result<()> {
    /// let aws_ip_ranges = awsipranges::get_ranges()?;
    ///
    /// let filtered_ranges = aws_ip_ranges.filter_builder()
    ///     .ipv4()
    ///     .regions(["us-west-1"])?
    ///     .network_border_groups(["us-west-2-sea-1"])?
    ///     .services(["EC2"])?
    ///     .filter();
    /// # Ok(())
    /// # }
    /// ```
    pub fn filter_builder(&self) -> FilterBuilder<'_> {
        FilterBuilder::new(self)
    }

    /// Filter the AWS IP Ranges using the provided [Filter].
    ///
    /// ```rust
    /// # fn main() -> awsipranges::Result<()> {
    /// let aws_ip_ranges = awsipranges::get_ranges()?;
    ///
    /// let filter = awsipranges::FilterBuilder::new(&aws_ip_ranges)
    ///     .ipv4()
    ///     .regions(["us-west-1"])?
    ///     .network_border_groups(["us-west-2-sea-1"])?
    ///     .services(["EC2"])?
    ///     .build();
    ///
    /// let filtered_ranges = aws_ip_ranges.filter(&filter);
    /// # Ok(())
    /// # }
    /// ```
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

    /*-------------------------------------------------------------------------
      (Internal) AWS IP Ranges from JSON
    -------------------------------------------------------------------------*/

    pub(crate) fn from_json(json: &str) -> Result<Box<AwsIpRanges>> {
        let json_ip_ranges = json::parse(json)?;

        // (prefix, region, network border group, service) records from both prefix lists
        let records = || {
            json_ip_ranges
                .prefixes
                .iter()
                .map(|p| {
                    let prefix = IpNetwork::V4(p.ip_prefix);
                    (prefix, p.region, p.network_border_group, p.service)
                })
                .chain(json_ip_ranges.ipv6_prefixes.iter().map(|p| {
                    let prefix = IpNetwork::V6(p.ipv6_prefix);
                    (prefix, p.region, p.network_border_group, p.service)
                }))
        };

        let regions: BTreeSet<Rc<str>> = records().map(|r| Rc::from(r.1)).collect();
        let network_border_groups: BTreeSet<Rc<str>> = records().map(|r| Rc::from(r.2)).collect();
        let services: BTreeSet<Rc<str>> = records().map(|r| Rc::from(r.3)).collect();

        // Values are interned: every record's strings are in the sets built above
        let intern = |value: &str, set: &BTreeSet<Rc<str>>| {
            utils::get_rc_str_from_set(value, set).expect("value is in the set")
        };

        let mut prefixes: BTreeMap<IpNetwork, AwsIpPrefix> = BTreeMap::new();
        for (prefix, region, network_border_group, service) in records() {
            let service = intern(service, &services);
            match prefixes.entry(prefix) {
                Entry::Vacant(entry) => {
                    entry.insert(AwsIpPrefix {
                        prefix,
                        region: intern(region, &regions),
                        network_border_group: intern(network_border_group, &network_border_groups),
                        services: BTreeSet::from([service]),
                    });
                }
                // Duplicate IP prefix entries indicate that multiple AWS services use a prefix
                Entry::Occupied(mut entry) => {
                    let aws_ip_prefix = entry.get_mut();
                    // A prefix should always be assigned to a single region and network border
                    // group; keep the first assignment if the published data disagrees.
                    if *aws_ip_prefix.region != *region
                        || *aws_ip_prefix.network_border_group != *network_border_group
                    {
                        warn!(
                            "AWS IP prefix {prefix} is listed in multiple regions or network \
                             border groups; using {}/{}",
                            aws_ip_prefix.region, aws_ip_prefix.network_border_group
                        );
                    }
                    aws_ip_prefix.services.insert(service);
                }
            }
        }

        Ok(Box::new(AwsIpRanges {
            sync_token: json_ip_ranges.sync_token.to_string(),
            create_date: json_ip_ranges.create_date,
            regions,
            network_border_groups,
            services,
            prefixes,
        }))
    }
}

/*--------------------------------------------------------------------------------------
  Create AWS IP Ranges from BTreeSet of AWS IP Prefixes
--------------------------------------------------------------------------------------*/

impl From<BTreeSet<AwsIpPrefix>> for AwsIpRanges {
    fn from(value: BTreeSet<AwsIpPrefix>) -> Self {
        let aws_ip_prefix_map: BTreeMap<IpNetwork, AwsIpPrefix> = value
            .into_iter()
            .map(|aws_ip_prefix| (aws_ip_prefix.prefix, aws_ip_prefix))
            .collect();

        Self::from(aws_ip_prefix_map)
    }
}

/*--------------------------------------------------------------------------------------
  Create AWS IP Ranges from BTreeMap of AWS IP Prefixes
--------------------------------------------------------------------------------------*/

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
  Unit Tests
-------------------------------------------------------------------------------------------------*/

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::core::aws_ip_prefix::tests::{test_aws_ipv4_prefix, test_aws_ipv6_prefix};
    use crate::core::filter::FilterBuilder;
    use crate::core::test_utils::FIXTURE_JSON;

    /*----------------------------------------------------------------------------------
      Test Helper Functions
    ----------------------------------------------------------------------------------*/

    pub(crate) fn test_aws_ip_ranges() -> Box<AwsIpRanges> {
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
      AWS IP Ranges
    ----------------------------------------------------------------------------------*/

    /*-------------------------------------------------------------------------
      Getters
    -------------------------------------------------------------------------*/

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

    /*-------------------------------------------------------------------------
      Get Prefix
    -------------------------------------------------------------------------*/

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

    /*-------------------------------------------------------------------------
      Get Longest Match Prefix
    -------------------------------------------------------------------------*/

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

    /*-------------------------------------------------------------------------
      Get Supernet Prefixes
    -------------------------------------------------------------------------*/

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

    /*-------------------------------------------------------------------------
      Get Reference Counted Strings
    -------------------------------------------------------------------------*/

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

    /*-------------------------------------------------------------------------
      Search
    -------------------------------------------------------------------------*/

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

        assert!(
            search_results
                .prefix_matches
                .contains_key(&search_networks[0])
        ); // Full prefix match
        assert!(
            search_results
                .prefix_matches
                .contains_key(&search_networks[3])
        ); // Full prefix match

        assert_eq!(aws_ip_ranges.prefixes().len(), 6); // Original AWS IP ranges unchanged
        assert_eq!(search_results.aws_ip_ranges.prefixes.len(), 4); // Search results AWS IP ranges

        assert!(
            search_results
                .prefixes_not_found
                .contains(&search_networks[2])
        ); // No prefix match
        assert!(
            search_results
                .prefixes_not_found
                .contains(&search_networks[5])
        ); // No prefix match
    }

    /*-------------------------------------------------------------------------
      Filter
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

    /*-------------------------------------------------------------------------
      From JSON
    -------------------------------------------------------------------------*/

    #[test]
    fn test_from_json_merges_services_for_duplicate_prefixes() {
        let aws_ip_ranges = AwsIpRanges::from_json(FIXTURE_JSON).unwrap();

        let prefix = aws_ip_ranges
            .get_prefix(&"44.192.0.0/11".parse().unwrap())
            .unwrap();
        assert_eq!(&*prefix.region, "us-east-1");
        let services: Vec<&str> = prefix.services.iter().map(|s| &**s).collect();
        assert_eq!(services, ["AMAZON", "EC2"]);

        assert_eq!(aws_ip_ranges.sync_token(), "1790249826");
        assert!(aws_ip_ranges.regions().contains("GLOBAL"));
        assert!(
            aws_ip_ranges
                .network_border_groups()
                .contains("us-east-1-atl-1")
        );
        assert!(aws_ip_ranges.services().contains("CLOUDFRONT"));
    }

    #[test]
    fn test_from_json_keeps_first_assignment_for_conflicting_prefixes() {
        let json = r#"{
          "syncToken": "1640995200",
          "createDate": "2022-01-01-00-00-00",
          "prefixes": [
            {"ip_prefix": "10.0.0.0/8", "region": "us-east-1", "network_border_group": "us-east-1", "service": "AMAZON"},
            {"ip_prefix": "10.0.0.0/8", "region": "us-west-2", "network_border_group": "us-west-2", "service": "EC2"}
          ],
          "ipv6_prefixes": []
        }"#;

        let aws_ip_ranges = AwsIpRanges::from_json(json).unwrap();
        let prefix = aws_ip_ranges
            .get_prefix(&"10.0.0.0/8".parse().unwrap())
            .unwrap();
        assert_eq!(&*prefix.region, "us-east-1");
        assert_eq!(prefix.services.len(), 2);
    }

    #[test]
    fn test_from_json_invalid_json_is_a_json_error() {
        let error = AwsIpRanges::from_json("{}").unwrap_err();
        assert!(matches!(error, crate::Error::Json(_)), "{error:?}");
    }

    #[test]
    fn test_search_prefixes_shorter_than_aws_prefixes_do_not_panic() {
        let aws_ip_ranges = test_aws_ip_ranges();

        for value in ["0.0.0.0/0", "8.0.0.0/6", "::/0", "2000::/4"] {
            let value: IpNetwork = value.parse().unwrap();
            assert!(aws_ip_ranges.get_supernet_prefixes(&value).is_none());
            assert!(aws_ip_ranges.get_longest_match_prefix(&value).is_none());
        }
    }
}
