use chrono::{DateTime, Utc};
use ipnetwork::{Ipv4Network, Ipv6Network};
use reqwest;
use serde::{Deserialize, Serialize};

// -------------------------------------------------------------------------------------
// Low-Level AWS IP Ranges
// -------------------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
pub struct IpRanges {
    #[serde(rename = "syncToken")]
    pub sync_token: String,

    #[serde(rename = "createDate", with = "aws_ip_ranges_datetime_format")]
    pub create_date: DateTime<Utc>,

    pub prefixes: Vec<IpPrefix>,

    pub ipv6_prefixes: Vec<Ipv6Prefix>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct IpPrefix {
    pub ip_prefix: Ipv4Network,
    pub region: String,
    pub network_border_group: String,
    pub service: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Ipv6Prefix {
    pub ipv6_prefix: Ipv6Network,
    pub region: String,
    pub network_border_group: String,
    pub service: String,
}

pub fn get_ip_ranges() -> IpRanges {
    reqwest::blocking::get("https://ip-ranges.amazonaws.com/ip-ranges.json")
        .expect("Error getting https://ip-ranges.amazonaws.com/ip-ranges.json")
        .json::<IpRanges>()
        .expect("Error parsing JSON from response")
}

// ----------------------------------------------------------------------------
// AWS IP Ranges DateTime Format
// ----------------------------------------------------------------------------

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
