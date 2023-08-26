#[macro_use]
extern crate lazy_static;
use chrono::{DateTime, Utc};
use config::Config;
use ipnetwork::{IpNetwork, Ipv4Network, Ipv6Network};
use reqwest;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::convert::From;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

// -------------------------------------------------------------------------------------
// Configuration
// -------------------------------------------------------------------------------------

lazy_static! {
    // static ref AWS_IP_RANGES_CONFIG_BUILDER: ConfigBuilder<DefaultState> =
   static ref AWS_IP_RANGES_CONFIG: Config = {
        let config_builder = Config::builder()
        .set_default("url", "https://ip-ranges.amazonaws.com/ip-ranges.json").unwrap()
        .set_default(
            "cache_file",
            [
                dirs::home_dir().unwrap().to_str().unwrap(),
                ".aws",
                "ip-ranges.json",
            ]
            .iter()
            .collect::<PathBuf>().to_str(),
        ).unwrap()
        .set_default("cache_time", 24 * 60 * 60).unwrap();
        config_builder.build().unwrap()
   };
}

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

    pub prefixes: BTreeMap<IpNetwork, AwsIpPrefix>,
}

#[derive(Debug)]
pub struct AwsIpPrefix {
    pub prefix: IpNetwork,
    pub region: Rc<String>,
    pub network_border_group: Rc<String>,
    pub services: HashSet<Rc<String>>,
}

impl AwsIpRanges {
    pub fn new() -> AwsIpRangesResult<AwsIpRanges> {
        let json = get_json()?;
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

        let mut prefixes: BTreeMap<IpNetwork, AwsIpPrefix> = BTreeMap::new();

        for json_ipv4_prefix in &json_ip_ranges.prefixes {
            prefixes
                .entry(IpNetwork::V4(json_ipv4_prefix.ip_prefix))
                .and_modify(|prefix| {
                    // Verify IP prefix invariants
                    // An IP prefix should always be assigned to a single region and network border group
                    assert_eq!(
                        prefix.region,
                        get_rc_string(json_ipv4_prefix.region, &regions).unwrap()
                    );
                    assert_eq!(
                        prefix.network_border_group,
                        get_rc_string(
                            json_ipv4_prefix.network_border_group,
                            &network_border_groups
                        )
                        .unwrap()
                    );
                    // Duplicate IP prefix entries are used to indicate multiple AWS services use a prefix
                    prefix
                        .services
                        .insert(get_rc_string(json_ipv4_prefix.service, &services).unwrap());
                })
                .or_insert(AwsIpPrefix {
                    prefix: IpNetwork::V4(json_ipv4_prefix.ip_prefix),
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

        for json_ipv6_prefix in &json_ip_ranges.ipv6_prefixes {
            prefixes
                .entry(IpNetwork::V6(json_ipv6_prefix.ipv6_prefix))
                .and_modify(|prefix| {
                    // Verify IP prefix invariants
                    // An IP prefix should always be assigned to a single region and network border group
                    assert_eq!(
                        prefix.region,
                        get_rc_string(json_ipv6_prefix.region, &regions).unwrap()
                    );
                    assert_eq!(
                        prefix.network_border_group,
                        get_rc_string(
                            json_ipv6_prefix.network_border_group,
                            &network_border_groups
                        )
                        .unwrap()
                    );
                    // Duplicate IP prefix entries are used to indicate multiple AWS services use a prefix
                    prefix
                        .services
                        .insert(get_rc_string(json_ipv6_prefix.service, &services).unwrap());
                })
                .or_insert(AwsIpPrefix {
                    prefix: IpNetwork::V6(json_ipv6_prefix.ipv6_prefix),
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

        Ok(AwsIpRanges {
            sync_token,
            create_date,
            regions,
            network_border_groups,
            services,
            prefixes,
        })
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
// AWS IP Ranges Error(s)
// -------------------------------------------------------------------------------------

pub type AwsIpRangesError = Box<dyn std::error::Error + Send + Sync + 'static>;
pub type AwsIpRangesResult<T> = Result<T, AwsIpRangesError>;

// -------------------------------------------------------------------------------------
// Low-Level API
// -------------------------------------------------------------------------------------

pub fn get_json() -> AwsIpRangesResult<String> {
    let cache_path: PathBuf = AWS_IP_RANGES_CONFIG.get_string("cache_file")?.into();
    let cache_time = AWS_IP_RANGES_CONFIG.get_int("cache_time")?.try_into()?;

    println!("Home Directory: {:?}", dirs::home_dir());
    println!("Cache Path: {:?}", &cache_path);

    if let Ok(_) = fs::canonicalize(&cache_path) {
        // Cache file exists
        let elapsed = fs::metadata(&cache_path)?.modified()?.elapsed()?;
        if elapsed.as_secs() <= cache_time {
            println!("IP ranges cache is fresh; use cache");
            get_json_from_file()
        } else {
            println!("IP ranges cache is stale; refresh cache");
            if let Ok(json) = get_json_from_url() {
                println!("Successfully retrieve fresh IP Ranges JSON; update cache file");
                cache_json_to_file(&json)?;
                Ok(json)
            } else {
                println!("Unable to retrieve fresh IP Ranges JSON data; use stale file cache");
                get_json_from_file()
            }
        }
    } else {
        // Cache file does not exist
        println!("Cache file does not exist; get JSON from URL and attempt to cache the result");
        match get_json_from_url() {
            Ok(json) => {
                let _ = cache_json_to_file(&json);
                Ok(json)
            }
            Err(error) => Err(error),
        }
    }
}

fn cache_json_to_file(json: &str) -> AwsIpRangesResult<()> {
    let cache_path: PathBuf = AWS_IP_RANGES_CONFIG.get_string("cache_file")?.into();

    // Ensure parent directories exist
    cache_path.parent().map(|parent| fs::create_dir_all(parent));

    Ok(fs::write(cache_path, json)?)
}

fn get_json_from_file() -> AwsIpRangesResult<String> {
    let cache_path: PathBuf = AWS_IP_RANGES_CONFIG.get_string("cache_file")?.into();
    Ok(fs::read_to_string(cache_path)?)
}

fn get_json_from_url() -> AwsIpRangesResult<String> {
    let response = reqwest::blocking::get(AWS_IP_RANGES_CONFIG.get_string("url")?)?;
    Ok(response.text()?)
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
