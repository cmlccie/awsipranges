use crate::core::errors::Result;
use chrono::{DateTime, Utc};
use ipnetwork::{Ipv4Network, Ipv6Network};
use serde::{Deserialize, Serialize};

/*-------------------------------------------------------------------------------------------------
  Parse JSON
-------------------------------------------------------------------------------------------------*/

pub fn parse(json: &str) -> Result<JsonIpRanges<'_>> {
    Ok(serde_json::from_str(json)?)
}

/*-------------------------------------------------------------------------------------------------
  JSON Data Structures
-------------------------------------------------------------------------------------------------*/

/*--------------------------------------------------------------------------------------
  JSON IP Ranges
--------------------------------------------------------------------------------------*/

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct JsonIpRanges<'j> {
    #[serde(rename = "syncToken")]
    pub sync_token: &'j str,

    #[serde(rename = "createDate", with = "crate::core::datetime")]
    pub create_date: DateTime<Utc>,

    pub prefixes: Vec<JsonIpPrefix<'j>>,

    pub ipv6_prefixes: Vec<JsonIpv6Prefix<'j>>,
}

/*--------------------------------------------------------------------------------------
  JSON IP (IPv4) Prefix
--------------------------------------------------------------------------------------*/

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct JsonIpPrefix<'j> {
    pub ip_prefix: Ipv4Network,
    pub region: &'j str,
    pub network_border_group: &'j str,
    pub service: &'j str,
}

/*--------------------------------------------------------------------------------------
  JSON IPv6 Prefix
--------------------------------------------------------------------------------------*/

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct JsonIpv6Prefix<'j> {
    pub ipv6_prefix: Ipv6Network,
    pub region: &'j str,
    pub network_border_group: &'j str,
    pub service: &'j str,
}

/*-------------------------------------------------------------------------------------------------
  Unit Tests
-------------------------------------------------------------------------------------------------*/

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use serde_json::{from_str, to_string};

    #[test]
    fn test_json_ip_ranges() {
        let aws_ip_ranges_test_json = r#"{
          "syncToken": "1640995200",
          "createDate": "2022-01-01-00-00-00",
          "prefixes": [
            {
              "ip_prefix": "10.0.0.0/8",
              "region": "us-east-1",
              "network_border_group": "us-east-1",
              "service": "AMAZON"
            }
          ],
          "ipv6_prefixes": [
            {
              "ipv6_prefix": "2001:db8::/32",
              "region": "us-east-1",
              "network_border_group": "us-east-1",
              "service": "AMAZON"
            }
          ]
        }"#;

        let parsed_value: JsonIpRanges = serde_json::from_str(aws_ip_ranges_test_json).unwrap();

        let create_date = Utc.with_ymd_and_hms(2022, 1, 1, 0, 0, 0).unwrap();
        let sync_token = create_date.timestamp().to_string();
        let expected_value = JsonIpRanges {
            sync_token: &sync_token,
            create_date,
            prefixes: vec![JsonIpPrefix {
                ip_prefix: "10.0.0.0/8".parse().unwrap(),
                region: "us-east-1",
                network_border_group: "us-east-1",
                service: "AMAZON",
            }],
            ipv6_prefixes: vec![JsonIpv6Prefix {
                ipv6_prefix: "2001:db8::/32".parse().unwrap(),
                region: "us-east-1",
                network_border_group: "us-east-1",
                service: "AMAZON",
            }],
        };

        assert_eq!(parsed_value, expected_value);

        // Round-trip test
        let serialized_value = serde_json::to_string(&expected_value).unwrap();
        let deserialized_value: JsonIpRanges = serde_json::from_str(&serialized_value).unwrap();
        assert_eq!(deserialized_value, expected_value);
    }

    #[test]
    fn test_json_ip_prefix() {
        let json_str = r#"{
          "ip_prefix": "10.0.0.0/8",
          "region": "us-east-1",
          "network_border_group": "us-east-1",
          "service": "AMAZON"
        }"#;

        let expected = JsonIpPrefix {
            ip_prefix: "10.0.0.0/8".parse().unwrap(),
            region: "us-east-1",
            network_border_group: "us-east-1",
            service: "AMAZON",
        };

        let actual: JsonIpPrefix = from_str(json_str).unwrap();
        assert_eq!(actual, expected);

        let serialized = to_string(&expected).unwrap();
        let deserialized: JsonIpPrefix = from_str(&serialized).unwrap();
        assert_eq!(deserialized, expected);
    }

    #[test]
    fn test_json_ipv6_prefix() {
        let json_str = r#"{
          "ipv6_prefix": "2001:db8::/32",
          "region": "us-east-1",
          "network_border_group": "us-east-1",
          "service": "AMAZON"
        }"#;

        let expected = JsonIpv6Prefix {
            ipv6_prefix: "2001:db8::/32".parse().unwrap(),
            region: "us-east-1",
            network_border_group: "us-east-1",
            service: "AMAZON",
        };

        let actual: JsonIpv6Prefix = from_str(json_str).unwrap();
        assert_eq!(actual, expected);

        let serialized = to_string(&expected).unwrap();
        let deserialized: JsonIpv6Prefix = from_str(&serialized).unwrap();
        assert_eq!(deserialized, expected);
    }
}
