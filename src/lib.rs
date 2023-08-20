use chrono::{DateTime, Utc};
use ipnetwork::{Ipv4Network, Ipv6Network};
use reqwest;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::rc::Rc;

// -------------------------------------------------------------------------------------
// AWS IP Ranges
// -------------------------------------------------------------------------------------

#[derive(Debug)]
pub struct AwsIpRanges {
    pub sync_token: String,
    pub create_date: DateTime<Utc>,

    pub regions: HashSet<Rc<String>>,
    pub network_border_groups: HashSet<Rc<String>>,
    pub services: HashSet<Rc<String>>,

    pub ipv4_prefixes: BTreeMap<Ipv4Network, AwsIPv4Prefix>,
    pub ipv6_prefixes: BTreeMap<Ipv6Network, AwsIPv6Prefix>,
}

#[derive(Debug)]
pub struct AwsIPv4Prefix {
    pub prefix: Ipv4Network,
    pub region: Rc<String>,
    pub network_border_group: Rc<String>,
    pub services: HashSet<Rc<String>>,
}

#[derive(Debug)]
pub struct AwsIPv6Prefix {
    pub prefix: Ipv6Network,
    pub region: Rc<String>,
    pub network_border_group: Rc<String>,
    pub services: HashSet<Rc<String>>,
}

impl AwsIpRanges {
    pub fn new() -> AwsIpRanges {
        let json = get_json_from_url();
        let json_ip_ranges = parse_json(&json);

        let sync_token = json_ip_ranges.sync_token.to_string();
        let create_date = json_ip_ranges.create_date;

        let regions: HashSet<Rc<String>> = json_ip_ranges
            .prefixes
            .iter()
            .map(|prefix| prefix.region)
            .chain(
                json_ip_ranges
                    .ipv6_prefixes
                    .iter()
                    .map(|ipv6_prefix| ipv6_prefix.region),
            )
            .map(|region| Rc::new(region.to_string()))
            .collect();

        let network_border_groups: HashSet<Rc<String>> = json_ip_ranges
            .prefixes
            .iter()
            .map(|prefix| prefix.network_border_group)
            .chain(
                json_ip_ranges
                    .ipv6_prefixes
                    .iter()
                    .map(|ipv6_prefix| ipv6_prefix.network_border_group),
            )
            .map(|network_border_group| Rc::new(network_border_group.to_string()))
            .collect();

        let services: HashSet<Rc<String>> = json_ip_ranges
            .prefixes
            .iter()
            .map(|prefix| prefix.service)
            .chain(
                json_ip_ranges
                    .ipv6_prefixes
                    .iter()
                    .map(|ipv6_prefix| ipv6_prefix.service),
            )
            .map(|service| Rc::new(service.to_string()))
            .collect();

        let mut ipv4_prefixes: BTreeMap<Ipv4Network, AwsIPv4Prefix> = BTreeMap::new();
        for json_ipv4_prefix in &json_ip_ranges.prefixes {
            ipv4_prefixes
                .entry(json_ipv4_prefix.ip_prefix)
                .and_modify(|ipv4_prefix| {
                    ipv4_prefix
                        .services
                        .insert(get_rc_string(json_ipv4_prefix.service, &services).unwrap());
                })
                .or_insert(AwsIPv4Prefix {
                    prefix: json_ipv4_prefix.ip_prefix,
                    region: get_rc_string(json_ipv4_prefix.region, &regions).unwrap(),
                    network_border_group: get_rc_string(
                        json_ipv4_prefix.network_border_group,
                        &network_border_groups,
                    )
                    .unwrap(),
                    services: HashSet::from([
                        get_rc_string(json_ipv4_prefix.service, &services).unwrap()
                    ]),
                });
        }

        let mut ipv6_prefixes: BTreeMap<Ipv6Network, AwsIPv6Prefix> = BTreeMap::new();
        for json_ipv6_prefix in &json_ip_ranges.ipv6_prefixes {
            ipv6_prefixes
                .entry(json_ipv6_prefix.ipv6_prefix)
                .and_modify(|ipv6_prefix| {
                    ipv6_prefix
                        .services
                        .insert(get_rc_string(json_ipv6_prefix.service, &services).unwrap());
                })
                .or_insert(AwsIPv6Prefix {
                    prefix: json_ipv6_prefix.ipv6_prefix,
                    region: get_rc_string(json_ipv6_prefix.region, &regions).unwrap(),
                    network_border_group: get_rc_string(
                        json_ipv6_prefix.network_border_group,
                        &network_border_groups,
                    )
                    .unwrap(),
                    services: HashSet::from([
                        get_rc_string(json_ipv6_prefix.service, &services).unwrap()
                    ]),
                });
        }

        AwsIpRanges {
            sync_token,
            create_date,
            regions,
            network_border_groups,
            services,
            ipv4_prefixes,
            ipv6_prefixes,
        }
    }
}

// -------------------------------------------------------------------------------------
// Helper Functions
// -------------------------------------------------------------------------------------

fn get_rc_string(value: &str, set: &HashSet<Rc<String>>) -> Option<Rc<String>> {
    set.get(&Rc::new(value.to_string()))
        .map(|item| Rc::clone(item))
}

// -------------------------------------------------------------------------------------
// Low-Level API
// -------------------------------------------------------------------------------------

pub fn get_json_from_url() -> String {
    reqwest::blocking::get("https://ip-ranges.amazonaws.com/ip-ranges.json")
        .expect("Error getting https://ip-ranges.amazonaws.com/ip-ranges.json")
        .text()
        .expect("Error downloading JSON")
}

pub fn parse_json<'j>(json: &'j str) -> JsonIpRanges<'j> {
    serde_json::from_str(json).expect("Error parsing JSON")
}

#[derive(Serialize, Deserialize, Debug)]
pub struct JsonIpRanges<'j> {
    #[serde(rename = "syncToken")]
    pub sync_token: &'j str,

    #[serde(rename = "createDate", with = "aws_ip_ranges_datetime_format")]
    pub create_date: DateTime<Utc>,

    pub prefixes: Vec<JsonIpPrefix<'j>>,

    pub ipv6_prefixes: Vec<JsonIpv6Prefix<'j>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct JsonIpPrefix<'j> {
    pub ip_prefix: Ipv4Network,
    pub region: &'j str,
    pub network_border_group: &'j str,
    pub service: &'j str,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct JsonIpv6Prefix<'j> {
    pub ipv6_prefix: Ipv6Network,
    pub region: &'j str,
    pub network_border_group: &'j str,
    pub service: &'j str,
}

// -------------------------------------------------------------------------------------
// AWS IP Ranges DateTime Format
// -------------------------------------------------------------------------------------

mod aws_ip_ranges_datetime_format {
    use chrono::{DateTime, TimeZone, Utc};
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
        Utc.datetime_from_str(&s, AWS_IP_RANGES_DATETIME_FORMAT)
            .map_err(serde::de::Error::custom)
    }
}
