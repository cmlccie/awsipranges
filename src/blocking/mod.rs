use crate::core::aws_ip_ranges::{AwsIpPrefix, AwsIpRanges};
use crate::core::config::AWS_IP_RANGES_CONFIG;
use crate::core::errors::Result;
use crate::core::json_ip_ranges::JsonIpRanges;
use crate::core::utils::get_rc_str_from_set;
use ipnetwork::IpNetwork;
use log::{info, warn};
use reqwest;
use std::collections::BTreeSet;
use std::convert::From;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

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
                info!("Successfully retrieved fresh IP Ranges JSON; update cache file");
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
