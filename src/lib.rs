#[macro_use]
extern crate lazy_static;
use chrono::{DateTime, Utc};
use config::{Config, Environment};
use ipnetwork::{IpNetwork, Ipv4Network, Ipv6Network};
use log::{info, trace, warn};
use reqwest;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::convert::From;
use std::fs;
use std::ops::Bound::Included;
use std::path::PathBuf;
use std::rc::Rc;

/*-------------------------------------------------------------------------------------------------
  Configuration
-------------------------------------------------------------------------------------------------*/

lazy_static! {
    static ref AWS_IP_RANGES_CONFIG: Config = {
        let home_dir = dirs::home_dir().unwrap();
        let cache_file: PathBuf = [&home_dir.to_str().unwrap(), ".aws", "ip-ranges.json"]
            .iter()
            .collect();

        let config_builder = Config::builder()
            .set_default("url", "https://ip-ranges.amazonaws.com/ip-ranges.json")
            .unwrap()
            .set_default("cache_file", cache_file.to_str())
            .unwrap()
            .set_default("cache_time", 24 * 60 * 60)
            .unwrap()
            .add_source(Environment::with_prefix("AWS_IP_RANGES"));

        config_builder.build().unwrap()
    };
}

/*-------------------------------------------------------------------------------------------------
  Primary Interface
-------------------------------------------------------------------------------------------------*/

pub fn get_ranges() -> Result<Box<AwsIpRanges>> {
    let json = get_json()?;
    let json_ip_ranges = parse_json(&json)?;

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
                    get_rc_str_from_set(json_ipv4_prefix.region, &aws_ip_ranges.regions).unwrap()
                );
                assert_eq!(
                    prefix.network_border_group,
                    get_rc_str_from_set(
                        json_ipv4_prefix.network_border_group,
                        &aws_ip_ranges.network_border_groups
                    )
                    .unwrap()
                );
                // Duplicate IP prefix entries are used to indicate multiple AWS services use a prefix
                prefix.services.insert(
                    get_rc_str_from_set(json_ipv4_prefix.service, &aws_ip_ranges.services).unwrap(),
                );
            })
            .or_insert(AwsIpPrefix {
                prefix: IpNetwork::V4(json_ipv4_prefix.ip_prefix),
                region: get_rc_str_from_set(json_ipv4_prefix.region, &aws_ip_ranges.regions)
                    .unwrap(),
                network_border_group: get_rc_str_from_set(
                    json_ipv4_prefix.network_border_group,
                    &aws_ip_ranges.network_border_groups,
                )
                .unwrap(),
                services: BTreeSet::from([get_rc_str_from_set(
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
                    get_rc_str_from_set(json_ipv6_prefix.region, &aws_ip_ranges.regions).unwrap()
                );
                assert_eq!(
                    prefix.network_border_group,
                    get_rc_str_from_set(
                        json_ipv6_prefix.network_border_group,
                        &aws_ip_ranges.network_border_groups
                    )
                    .unwrap()
                );
                // Duplicate IP prefix entries are used to indicate multiple AWS services use a prefix
                prefix.services.insert(
                    get_rc_str_from_set(json_ipv6_prefix.service, &aws_ip_ranges.services).unwrap(),
                );
            })
            .or_insert(AwsIpPrefix {
                prefix: IpNetwork::V6(json_ipv6_prefix.ipv6_prefix),
                region: get_rc_str_from_set(json_ipv6_prefix.region, &aws_ip_ranges.regions)
                    .unwrap(),
                network_border_group: get_rc_str_from_set(
                    json_ipv6_prefix.network_border_group,
                    &aws_ip_ranges.network_border_groups,
                )
                .unwrap(),
                services: BTreeSet::from([get_rc_str_from_set(
                    json_ipv6_prefix.service,
                    &aws_ip_ranges.services,
                )
                .unwrap()]),
            });
    }

    Ok(aws_ip_ranges)
}

/*-------------------------------------------------------------------------------------------------
  AWS IP Ranges
-------------------------------------------------------------------------------------------------*/

#[derive(Debug, Default, Clone)]
pub struct AwsIpRanges {
    pub sync_token: String,
    pub create_date: DateTime<Utc>,

    pub regions: BTreeSet<Rc<str>>,
    pub network_border_groups: BTreeSet<Rc<str>>,
    pub services: BTreeSet<Rc<str>>,

    pub prefixes: BTreeMap<IpNetwork, AwsIpPrefix>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct AwsIpPrefix {
    pub prefix: IpNetwork,
    pub region: Rc<str>,
    pub network_border_group: Rc<str>,
    pub services: BTreeSet<Rc<str>>,
}

#[derive(Clone, Debug, Default)]
pub struct SearchResults {
    pub aws_ip_ranges: Box<AwsIpRanges>,
    pub prefix_matches: BTreeMap<IpNetwork, BTreeSet<AwsIpPrefix>>,
    pub prefixes_not_found: BTreeSet<IpNetwork>,
}

impl AwsIpRanges {
    pub fn get(&self, value: &IpNetwork) -> Option<BTreeSet<AwsIpPrefix>> {
        let mut aws_ip_prefixes: BTreeSet<AwsIpPrefix> = BTreeSet::new();

        let lower_bound = match value {
            IpNetwork::V4(_) => new_network_prefix(value, 8u8).unwrap(),
            IpNetwork::V6(_) => new_network_prefix(value, 16u8).unwrap(),
        };
        let upper_bound = network_prefix(value);

        for (_, aws_ip_prefix) in self
            .prefixes
            .range((Included(lower_bound), Included(upper_bound)))
        {
            if is_supernet_of(aws_ip_prefix.prefix, *value) {
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
        get_rc_str_from_set(value, &self.regions)
    }

    pub fn get_network_border_group(&self, value: &str) -> Option<Rc<str>> {
        get_rc_str_from_set(value, &self.network_border_groups)
    }

    pub fn get_service(&self, value: &str) -> Option<Rc<str>> {
        get_rc_str_from_set(value, &self.services)
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
            if let Some(aws_ip_prefixes) = self.get(prefix) {
                aws_ip_prefixes.iter().for_each(|aws_ip_prefix| {
                    result_aws_ip_prefixes.insert(aws_ip_prefix.clone());
                });

                search_results
                    .prefix_matches
                    .insert(*prefix, aws_ip_prefixes);
            } else {
                warn!("Prefix not found in AWS IP ranges: {prefix}");
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

/*--------------------------------------------------------------------------------------
  Helper Functions
--------------------------------------------------------------------------------------*/

fn get_rc_str_from_set(value: &str, set: &BTreeSet<Rc<str>>) -> Option<Rc<str>> {
    set.get(value).map(|item| Rc::clone(item))
}

/*
    The IpNetwork type does not reduce (or provide a method to reduce) an
    interface CIDR prefix to network prefix (where all host bits are set to
    `0`). It does provide a network() method that will extract the network IP.

    These helper functions extract the network prefix from an IpNetwork and
    build a new network prefiex from an existing IpNetwork with a specified
    number of mask bits.
*/

fn network_prefix(ip_network: &IpNetwork) -> IpNetwork {
    match ip_network {
        IpNetwork::V4(ipv4_network) => {
            IpNetwork::V4(Ipv4Network::new(ipv4_network.network(), ipv4_network.prefix()).unwrap())
        }
        IpNetwork::V6(ipv6_network) => {
            IpNetwork::V6(Ipv6Network::new(ipv6_network.network(), ipv6_network.prefix()).unwrap())
        }
    }
}

fn new_network_prefix(ip_network: &IpNetwork, mask_bits: u8) -> Result<IpNetwork> {
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

fn is_supernet_of(supernet: IpNetwork, subnet: IpNetwork) -> bool {
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

/*-------------------------------------------------------------------------------------------------
  Errors and Results
-------------------------------------------------------------------------------------------------*/

pub type Error = Box<dyn std::error::Error + Send + Sync + 'static>;
pub type Result<T> = std::result::Result<T, Error>;

/*-------------------------------------------------------------------------------------------------
  Low-Level API
-------------------------------------------------------------------------------------------------*/

pub fn get_json() -> Result<String> {
    let cache_path: PathBuf = AWS_IP_RANGES_CONFIG.get_string("cache_file")?.into();
    let cache_time = AWS_IP_RANGES_CONFIG.get_int("cache_time")?.try_into()?;

    info!("Cache file path {:?}", &cache_path);
    info!("Cache time {cache_time} seconds");

    if let Ok(_) = fs::canonicalize(&cache_path) {
        info!("Cache file exists");
        let elapsed = fs::metadata(&cache_path)?.modified()?.elapsed()?;
        if elapsed.as_secs() <= cache_time {
            info!("IP ranges cache is fresh; use cache");
            get_json_from_file()
        } else {
            info!("IP ranges cache is stale; refresh cache");
            if let Ok(json) = get_json_from_url() {
                info!("Successfully retrieve fresh IP Ranges JSON; update cache file");
                cache_json_to_file(&json)?;
                Ok(json)
            } else {
                warn!("Unable to retrieve fresh IP Ranges JSON data; use stale file cache");
                get_json_from_file()
            }
        }
    } else {
        info!("Cache file does not exist; get JSON from URL and attempt to cache the result");
        match get_json_from_url() {
            Ok(json) => {
                let _ = cache_json_to_file(&json);
                Ok(json)
            }
            Err(error) => Err(error),
        }
    }
}

fn get_json_from_url() -> Result<String> {
    let response = reqwest::blocking::get(AWS_IP_RANGES_CONFIG.get_string("url")?)?;
    Ok(response.text()?)
}

fn cache_json_to_file(json: &str) -> Result<()> {
    let cache_path: PathBuf = AWS_IP_RANGES_CONFIG.get_string("cache_file")?.into();

    // Ensure parent directories exist
    cache_path.parent().map(|parent| fs::create_dir_all(parent));

    Ok(fs::write(cache_path, json)?)
}

fn get_json_from_file() -> Result<String> {
    let cache_path: PathBuf = AWS_IP_RANGES_CONFIG.get_string("cache_file")?.into();
    Ok(fs::read_to_string(cache_path)?)
}

pub fn parse_json<'j>(json: &'j str) -> Result<JsonIpRanges<'j>> {
    Ok(serde_json::from_str(json)?)
}

#[derive(Debug, Deserialize, Serialize)]
pub struct JsonIpRanges<'j> {
    #[serde(rename = "syncToken")]
    pub sync_token: &'j str,

    #[serde(rename = "createDate", with = "aws_ip_ranges_datetime_format")]
    pub create_date: DateTime<Utc>,

    pub prefixes: Vec<JsonIpPrefix<'j>>,

    pub ipv6_prefixes: Vec<JsonIpv6Prefix<'j>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct JsonIpPrefix<'j> {
    pub ip_prefix: Ipv4Network,
    pub region: &'j str,
    pub network_border_group: &'j str,
    pub service: &'j str,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct JsonIpv6Prefix<'j> {
    pub ipv6_prefix: Ipv6Network,
    pub region: &'j str,
    pub network_border_group: &'j str,
    pub service: &'j str,
}

/*-------------------------------------------------------------------------------------------------
  DateTime Format
-------------------------------------------------------------------------------------------------*/

mod aws_ip_ranges_datetime_format {
    use chrono::{DateTime, NaiveDateTime, Utc};
    use serde::{self, Deserialize, Deserializer, Serializer};

    const AWS_IP_RANGES_DATETIME_FORMAT: &'static str = "%Y-%m-%d-%H-%M-%S";

    pub fn serialize<S>(date: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = format!("{}", date.format(AWS_IP_RANGES_DATETIME_FORMAT));
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        NaiveDateTime::parse_from_str(&s, AWS_IP_RANGES_DATETIME_FORMAT)
            .map(|naive_date_time| naive_date_time.and_utc())
            .map_err(serde::de::Error::custom)
    }
}

/*-------------------------------------------------------------------------------------------------
  Unit Tests
-------------------------------------------------------------------------------------------------*/

#[cfg(test)]
mod tests {
    use super::*;

    /*----------------------------------------------------------------------------------
      Low Level API
    ----------------------------------------------------------------------------------*/

    #[test]
    fn test_get_json_from_url() {
        let json = get_json_from_url();
        assert!(json.is_ok());
    }

    #[test]
    fn test_cache_json_to_file() {
        let json = get_json_from_url().unwrap();
        let result = cache_json_to_file(&json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_json_from_file() {
        let json_from_url = get_json_from_file().unwrap();
        cache_json_to_file(&json_from_url).unwrap();
        let json_from_file = get_json_from_file();
        assert!(json_from_file.is_ok());
    }

    #[test]
    fn test_parse_json() {
        let json = get_json_from_url().unwrap();
        let json_ip_ranges = parse_json(&json);
        assert!(json_ip_ranges.is_ok());
    }

    #[test]
    fn test_serialize_json_ip_ranges() {
        let json_from_url = get_json_from_url().unwrap();
        let json_ip_ranges = parse_json(&json_from_url).unwrap();
        let serialized_json = serde_json::to_string(&json_ip_ranges);
        assert!(serialized_json.is_ok());
    }
}
