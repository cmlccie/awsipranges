use crate::core::errors::Result;
use chrono::{DateTime, Utc};
use ipnetwork::{Ipv4Network, Ipv6Network};
use serde::{Deserialize, Serialize};

/*-------------------------------------------------------------------------------------------------
  Parse JSON
-------------------------------------------------------------------------------------------------*/

pub fn parse<'j>(json: &'j str) -> Result<JsonIpRanges<'j>> {
    Ok(serde_json::from_str(json)?)
}

/*-------------------------------------------------------------------------------------------------
  JSON Data Structures
-------------------------------------------------------------------------------------------------*/

/*--------------------------------------------------------------------------------------
  JSON IP Ranges
--------------------------------------------------------------------------------------*/

#[derive(Debug, Deserialize, Serialize)]
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

#[derive(Debug, Deserialize, Serialize)]
pub struct JsonIpPrefix<'j> {
    pub ip_prefix: Ipv4Network,
    pub region: &'j str,
    pub network_border_group: &'j str,
    pub service: &'j str,
}

/*--------------------------------------------------------------------------------------
  JSON IPv6 Prefix
--------------------------------------------------------------------------------------*/

#[derive(Debug, Deserialize, Serialize)]
pub struct JsonIpv6Prefix<'j> {
    pub ipv6_prefix: Ipv6Network,
    pub region: &'j str,
    pub network_border_group: &'j str,
    pub service: &'j str,
}