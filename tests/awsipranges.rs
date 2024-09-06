use assert_cmd::Command;

/*-------------------------------------------------------------------------------------------------
  awsipranges Binary Tests
-------------------------------------------------------------------------------------------------*/

/*--------------------------------------------------------------------------------------
  No Arguments - Parse and Display All AWS IP Ranges
--------------------------------------------------------------------------------------*/

#[test]
fn command_no_args() {
    Command::cargo_bin("awsipranges")
        .unwrap()
        .assert()
        .success();
}

/*--------------------------------------------------------------------------------------
  Version
--------------------------------------------------------------------------------------*/

#[test]
fn command_version() {
    Command::cargo_bin("awsipranges")
        .unwrap()
        .arg("--version")
        .assert()
        .success();
}

/*--------------------------------------------------------------------------------------
  Output Formats
--------------------------------------------------------------------------------------*/

/*-----------------------------------------------------------------------------
  Output: Table
-----------------------------------------------------------------------------*/

#[test]
fn command_output_table() {
    Command::cargo_bin("awsipranges")
        .unwrap()
        .arg("--output")
        .arg("table")
        .assert()
        .success();
}

/*-----------------------------------------------------------------------------
  Output: CIDR
-----------------------------------------------------------------------------*/

#[test]
fn command_output_cidr() {
    Command::cargo_bin("awsipranges")
        .unwrap()
        .arg("--output")
        .arg("cidr")
        .assert()
        .success();
}

/*--------------------------------------------------------------------------------------
  Output: Netmask
--------------------------------------------------------------------------------------*/

#[test]
fn command_output_netmask() {
    Command::cargo_bin("awsipranges")
        .unwrap()
        .arg("--output")
        .arg("netmask")
        .assert()
        .success();
}

/*-----------------------------------------------------------------------------
  Output: Regions
-----------------------------------------------------------------------------*/

#[test]
fn command_output_regions() {
    Command::cargo_bin("awsipranges")
        .unwrap()
        .arg("--output")
        .arg("regions")
        .assert()
        .success();
}

/*-----------------------------------------------------------------------------
  Output: Network Border Groups
-----------------------------------------------------------------------------*/

#[test]
fn command_output_network_border_groups() {
    Command::cargo_bin("awsipranges")
        .unwrap()
        .arg("--output")
        .arg("network-border-groups")
        .assert()
        .success();
}

/*-----------------------------------------------------------------------------
  Output: Services
-----------------------------------------------------------------------------*/

#[test]
fn command_output_services() {
    Command::cargo_bin("awsipranges")
        .unwrap()
        .arg("--output")
        .arg("services")
        .assert()
        .success();
}

/*--------------------------------------------------------------------------------------
  Search
--------------------------------------------------------------------------------------*/

/*-----------------------------------------------------------------------------
  Search: IP Address
-----------------------------------------------------------------------------*/

#[test]
fn command_search_ip_address() {
    Command::cargo_bin("awsipranges")
        .unwrap()
        .arg("44.192.140.65")
        .assert()
        .success();
}

/*-----------------------------------------------------------------------------
  Search: IP Address Not Found
-----------------------------------------------------------------------------*/

#[test]
fn command_search_ip_address_not_found() {
    Command::cargo_bin("awsipranges")
        .unwrap()
        .arg("1.1.1.1")
        .assert()
        .failure()
        .code(1);
}

/*--------------------------------------------------------------------------------------
  Filter
--------------------------------------------------------------------------------------*/

/*-----------------------------------------------------------------------------
  Filter: IPv4
-----------------------------------------------------------------------------*/

#[test]
fn command_filter_ipv4() {
    Command::cargo_bin("awsipranges")
        .unwrap()
        .arg("--ipv4")
        .assert()
        .success();
}

/*-----------------------------------------------------------------------------
  Filter: IPv6
-----------------------------------------------------------------------------*/

#[test]
fn command_filter_ipv6() {
    Command::cargo_bin("awsipranges")
        .unwrap()
        .arg("--ipv6")
        .assert()
        .success();
}

/*-----------------------------------------------------------------------------
  Filter: Region
-----------------------------------------------------------------------------*/

#[test]
fn command_filter_region() {
    Command::cargo_bin("awsipranges")
        .unwrap()
        .arg("--region")
        .arg("us-east-1")
        .assert()
        .success();
}

/*-----------------------------------------------------------------------------
  Filter: Network Border Group
-----------------------------------------------------------------------------*/

#[test]
fn command_filter_network_border_group() {
    Command::cargo_bin("awsipranges")
        .unwrap()
        .arg("--network-border-group")
        .arg("us-east-1-atl-1")
        .assert()
        .success();
}

/*-----------------------------------------------------------------------------
  Filter: Service
-----------------------------------------------------------------------------*/

#[test]
fn command_filter_service() {
    Command::cargo_bin("awsipranges")
        .unwrap()
        .arg("--service")
        .arg("S3")
        .assert()
        .success();
}

/*-----------------------------------------------------------------------------
  Filter - IPv4, Region, Network Border Group, Service
-----------------------------------------------------------------------------*/

#[test]
fn command_filter_ipv4_region_network_border_group_service() {
    Command::cargo_bin("awsipranges")
        .unwrap()
        .arg("--ipv4")
        .arg("--region")
        .arg("us-east-1")
        .arg("--network-border-group")
        .arg("us-east-1-atl-1")
        .arg("--service")
        .arg("EC2")
        .assert()
        .success();
}

/*--------------------------------------------------------------------------------------
  Save to CSV
--------------------------------------------------------------------------------------*/

#[test]
fn command_save_to_csv() {
    Command::cargo_bin("awsipranges")
        .unwrap()
        .arg("--csv")
        .arg("./scratch/command_save_to_csv.csv")
        .assert()
        .success();
}
