use assert_cmd::Command;
use std::path::PathBuf;

/*-------------------------------------------------------------------------------------------------
  awsipranges Binary Tests
-------------------------------------------------------------------------------------------------*/

/*--------------------------------------------------------------------------------------
  Test Helpers
--------------------------------------------------------------------------------------*/

/// A small, real subset of the AWS IP Ranges JSON.
fn fixture() -> PathBuf {
    [
        env!("CARGO_MANIFEST_DIR"),
        "tests",
        "fixtures",
        "ip-ranges.json",
    ]
    .iter()
    .collect()
}

/// An `awsipranges` command that reads the fixture offline, isolated from the caller's
/// environment.
fn awsipranges() -> Command {
    let mut command = Command::cargo_bin("awsipranges").unwrap();
    command
        .env_clear()
        .env("AWSIPRANGES_CACHE_FILE", fixture())
        .arg("--offline");
    command
}

/// Run a command and return its exit code, stdout, and stderr.
fn run(command: &mut Command) -> (i32, String, String) {
    let output = command.output().unwrap();
    (
        output.status.code().unwrap(),
        String::from_utf8(output.stdout).unwrap(),
        String::from_utf8(output.stderr).unwrap(),
    )
}

/// Lines of output, for exact comparisons.
fn lines(output: &str) -> Vec<&str> {
    output.lines().collect()
}

/*--------------------------------------------------------------------------------------
  No Arguments and Version
--------------------------------------------------------------------------------------*/

#[test]
fn command_no_args_displays_all_prefixes() {
    let (code, stdout, _) = run(&mut awsipranges());
    assert_eq!(code, 0);
    assert!(stdout.contains("44.192.0.0/11"));
    assert!(stdout.contains("2600:1f1a:4000::/36"));
    assert!(stdout.contains("17  AWS IP Prefixes"), "{stdout}");
}

#[test]
fn command_version() {
    let (code, stdout, _) = run(Command::cargo_bin("awsipranges").unwrap().arg("--version"));
    assert_eq!(code, 0);
    assert!(stdout.starts_with("awsipranges "));
}

/*--------------------------------------------------------------------------------------
  Output Formats
--------------------------------------------------------------------------------------*/

#[test]
fn command_output_cidr() {
    let (code, stdout, _) = run(awsipranges().args(["--output", "cidr", "44.192.140.65"]));
    assert_eq!(code, 0);
    assert_eq!(lines(&stdout), ["44.192.0.0/11", "44.192.140.64/28"]);
}

#[test]
fn command_output_netmask() {
    let (code, stdout, _) = run(awsipranges().args(["--output", "netmask", "44.192.140.65"]));
    assert_eq!(code, 0);
    assert_eq!(
        lines(&stdout),
        ["44.192.0.0 255.224.0.0", "44.192.140.64 255.255.255.240"]
    );
}

#[test]
fn command_output_json() {
    let (code, stdout, _) = run(awsipranges().args(["--output", "json", "44.192.140.65"]));
    assert_eq!(code, 0);

    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["sync_token"], "1790249826");
    assert_eq!(json["create_date"], "2026-09-24T11:37:06Z");
    assert_eq!(
        json["prefixes"],
        serde_json::json!([
            {
                "prefix": "44.192.0.0/11",
                "region": "us-east-1",
                "network_border_group": "us-east-1",
                "services": ["AMAZON", "EC2"],
                "matches": [{"search": "44.192.140.65", "addresses": ["44.192.140.65"]}],
            },
            {
                "prefix": "44.192.140.64/28",
                "region": "us-east-1",
                "network_border_group": "us-east-1",
                "services": ["S3"],
                "matches": [{"search": "44.192.140.65", "addresses": ["44.192.140.65"]}],
            },
        ])
    );
}

#[test]
fn command_output_json_without_search_has_no_matches() {
    let (code, stdout, _) = run(awsipranges().args(["--output", "json", "--region", "eu-west-1"]));
    assert_eq!(code, 0);

    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let prefixes = json["prefixes"].as_array().unwrap();
    assert_eq!(prefixes.len(), 2);
    assert!(
        prefixes
            .iter()
            .all(|prefix| prefix.get("matches").is_none())
    );
}

#[test]
fn command_output_json_matches_each_search_to_its_prefixes() {
    let (code, stdout, _) = run(awsipranges().args([
        "--output",
        "json",
        "44.192.140.65",
        "16.12.96.0/22",
        "2600:1f1a:4000::1",
    ]));
    assert_eq!(code, 0);

    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let matches: Vec<(String, String)> = json["prefixes"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|prefix| {
            prefix["matches"].as_array().unwrap().iter().map(|m| {
                (
                    prefix["prefix"].as_str().unwrap().to_string(),
                    m["search"].as_str().unwrap().to_string(),
                )
            })
        })
        .collect();
    let expected = [
        ("16.12.96.0/21", "16.12.96.0/22"),
        ("44.192.0.0/11", "44.192.140.65"),
        ("44.192.140.64/28", "44.192.140.65"),
        ("2600:1f1a:4000::/36", "2600:1f1a:4000::1"),
    ];
    assert_eq!(
        matches,
        expected.map(|(prefix, search)| (prefix.to_string(), search.to_string()))
    );
}

#[test]
fn command_output_table_shows_matches_when_searching() {
    let (code, stdout, _) = run(awsipranges().args(["44.192.140.65", "16.12.96.1"]));
    assert_eq!(code, 0);
    assert!(stdout.contains("Matches"), "{stdout}");
    assert!(stdout.contains("16.12.96.1"), "{stdout}");

    let (code, stdout, _) = run(awsipranges().args(["--region", "eu-west-1"]));
    assert_eq!(code, 0);
    assert!(!stdout.contains("Matches"), "{stdout}");
}

#[test]
fn command_output_regions() {
    let (code, stdout, _) = run(awsipranges().args(["--output", "regions"]));
    assert_eq!(code, 0);
    assert_eq!(
        lines(&stdout),
        ["GLOBAL", "ca-west-1", "eu-west-1", "us-east-1", "us-west-2"]
    );
}

#[test]
fn command_output_network_border_groups() {
    let (code, stdout, _) = run(awsipranges().args(["--output", "network-border-groups"]));
    assert_eq!(code, 0);
    assert!(lines(&stdout).contains(&"us-east-1-atl-1"));
}

#[test]
fn command_output_services() {
    let (code, stdout, _) = run(awsipranges().args(["--output", "services"]));
    assert_eq!(code, 0);
    assert_eq!(lines(&stdout), ["AMAZON", "CLOUDFRONT", "EC2", "S3"]);
}

/*--------------------------------------------------------------------------------------
  Search
--------------------------------------------------------------------------------------*/

#[test]
fn command_search_ipv6_address() {
    let (code, stdout, _) = run(awsipranges().args([
        "--output",
        "cidr",
        "2600:1f1a:4000:a03a:54b4:19e6:f50c:9d01",
    ]));
    assert_eq!(code, 0);
    assert_eq!(lines(&stdout), ["2600:1f1a:4000::/36"]);
}

#[test]
fn command_search_not_found_exits_1() {
    let (code, stdout, stderr) = run(awsipranges().arg("1.1.1.1"));
    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert!(stderr.contains("No AWS IP Prefixes match"));
}

#[test]
fn command_search_not_found_quiet_is_silent() {
    let (code, stdout, stderr) = run(awsipranges().args(["--quiet", "1.1.1.1"]));
    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert!(stderr.is_empty(), "{stderr}");
}

#[test]
fn command_search_broad_prefix_does_not_panic() {
    let (code, _, stderr) = run(awsipranges().arg("0.0.0.0/0"));
    assert_eq!(code, 1, "{stderr}");
}

#[test]
fn command_search_invalid_input_exits_2() {
    for (value, message) in [
        ("1.2.3", "not a valid IP address, CIDR, or hostname"),
        (
            "44.192.140.65/33",
            "not a valid IP address, CIDR, or hostname",
        ),
        (
            "not a hostname",
            "not a valid IP address, CIDR, or hostname",
        ),
        ("https://example.com/", "looks like a URL"),
    ] {
        let (code, stdout, stderr) = run(awsipranges().arg(value));
        assert_eq!(code, 2, "{value}");
        assert!(stdout.is_empty(), "{value}");
        assert!(stderr.contains(message), "{value}: {stderr}");
    }
}

/*--------------------------------------------------------------------------------------
  Search: Hostnames
--------------------------------------------------------------------------------------*/

#[test]
fn command_search_hostname_resolves_addresses() {
    // localhost resolves from the hosts file, without DNS, to non-AWS loopback addresses
    let (code, stdout, stderr) = run(awsipranges().arg("localhost"));
    assert_eq!(code, 1, "{stderr}");
    assert!(stdout.is_empty());
    assert!(stderr.contains("localhost resolves to "), "{stderr}");
}

#[test]
fn command_search_unresolvable_hostname_exits_2() {
    // The .invalid top-level domain never resolves (RFC 6761)
    let (code, stdout, stderr) = run(awsipranges().arg("awsipranges-test.invalid"));
    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert!(
        stderr.contains("error: failed to resolve awsipranges-test.invalid"),
        "{stderr}"
    );
}

#[test]
fn command_search_partial_resolution_failure_shows_results_and_exits_2() {
    let (code, stdout, stderr) = run(awsipranges().args([
        "--output",
        "cidr",
        "44.192.140.65",
        "awsipranges-test.invalid",
    ]));
    assert_eq!(code, 2);
    assert_eq!(lines(&stdout), ["44.192.0.0/11", "44.192.140.64/28"]);
    assert!(stderr.contains("failed to resolve awsipranges-test.invalid"));
}

/*--------------------------------------------------------------------------------------
  Filter
--------------------------------------------------------------------------------------*/

#[test]
fn command_filter_ipv4_region_service() {
    let (code, stdout, _) = run(awsipranges().args([
        "--ipv4",
        "--region",
        "us-west-2",
        "--service",
        "s3",
        "--output",
        "cidr",
    ]));
    assert_eq!(code, 0);
    assert_eq!(
        lines(&stdout),
        ["16.12.88.0/21", "16.12.96.0/21", "16.12.104.0/21"]
    );
}

#[test]
fn command_filter_ipv6() {
    let (code, stdout, _) = run(awsipranges().args(["--ipv6", "--output", "cidr"]));
    assert_eq!(code, 0);
    assert_eq!(lines(&stdout).len(), 5);
    assert!(lines(&stdout).iter().all(|prefix| prefix.contains(':')));
}

#[test]
fn command_filter_network_border_group() {
    let (code, stdout, _) = run(awsipranges().args([
        "--network-border-group",
        "US-EAST-1-ATL-1",
        "--output",
        "cidr",
    ]));
    assert_eq!(code, 0);
    assert_eq!(
        lines(&stdout),
        ["15.181.80.0/20", "15.181.247.0/24", "15.220.233.0/24"]
    );
}

#[test]
fn command_filter_global_region() {
    let (code, stdout, _) = run(awsipranges().args(["--region", "global", "--output", "cidr"]));
    assert_eq!(code, 0);
    assert_eq!(lines(&stdout), ["23.228.249.0/24", "120.52.22.96/27"]);
}

#[test]
fn command_filter_unknown_region_exits_2_with_hint() {
    let (code, _, stderr) = run(awsipranges().args(["--region", "nowhere"]));
    assert_eq!(code, 2);
    assert!(
        stderr.contains("error: unknown region: nowhere"),
        "{stderr}"
    );
    assert!(stderr.contains("hint:"), "{stderr}");
}

/*--------------------------------------------------------------------------------------
  Cache
--------------------------------------------------------------------------------------*/

#[test]
fn command_offline_without_cache_exits_2() {
    let (code, _, stderr) = run(Command::cargo_bin("awsipranges")
        .unwrap()
        .env_clear()
        .env("AWSIPRANGES_CACHE_FILE", "/nonexistent/ip-ranges.json")
        .arg("--offline"));
    assert_eq!(code, 2);
    assert!(stderr.contains("failed to read the AWS IP Ranges cache file"));
}

#[test]
fn command_refresh_conflicts_with_offline() {
    let (code, _, _) = run(awsipranges().arg("--refresh"));
    assert_eq!(code, 2);
}

/// Smoke test against the real AWS IP Ranges URL.
#[test]
fn command_live_download() {
    let cache_dir = tempfile::tempdir().unwrap();
    let cache_file = cache_dir.path().join("ip-ranges.json");
    let (code, stdout, stderr) = run(Command::cargo_bin("awsipranges")
        .unwrap()
        .env("AWSIPRANGES_CACHE_FILE", &cache_file)
        .args(["--output", "services"]));
    assert_eq!(code, 0, "{stderr}");
    assert!(lines(&stdout).contains(&"EC2"));
    assert!(cache_file.exists());
}

/*--------------------------------------------------------------------------------------
  Save to CSV
--------------------------------------------------------------------------------------*/

#[test]
fn command_save_to_csv() {
    let output_dir = tempfile::tempdir().unwrap();
    let csv_file = output_dir.path().join("prefixes.csv");
    let (code, _, _) = run(awsipranges()
        .args(["--output", "cidr", "44.192.140.65", "--csv"])
        .arg(&csv_file));
    assert_eq!(code, 0);
    assert_eq!(
        lines(&std::fs::read_to_string(csv_file).unwrap()),
        [
            "AWS IP Prefix,Region,Network Border Group,Services,Matches",
            "44.192.0.0/11,us-east-1,us-east-1,\"AMAZON, EC2\",44.192.140.65",
            "44.192.140.64/28,us-east-1,us-east-1,S3,44.192.140.65",
        ]
    );
}

/*--------------------------------------------------------------------------------------
  Shell Completions
--------------------------------------------------------------------------------------*/

#[test]
fn command_completions() {
    for shell in ["bash", "zsh", "fish", "powershell", "elvish"] {
        let (code, stdout, _) = run(Command::cargo_bin("awsipranges")
            .unwrap()
            .args(["--completions", shell]));
        assert_eq!(code, 0, "{shell}");
        assert!(stdout.contains("awsipranges"), "{shell}");
    }
}
