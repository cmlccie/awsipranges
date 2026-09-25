use crate::cli::search::{self, Match, Search};
use awsipranges::{AwsIpPrefix, AwsIpRanges};
use chrono::{DateTime, Utc};
use comfy_table::presets::{NOTHING, UTF8_FULL};
use comfy_table::*;
use serde::Serialize;
use std::io::{self, Write};

/*-------------------------------------------------------------------------------------------------
  Output Functions
-------------------------------------------------------------------------------------------------*/

/*--------------------------------------------------------------------------------------
  Prefix Table
--------------------------------------------------------------------------------------*/

pub fn prefix_table(
    out: &mut impl Write,
    aws_ip_ranges: &AwsIpRanges,
    searches: &[Search],
) -> io::Result<()> {
    // Prefix Table
    let mut prefix_table = Table::new();
    prefix_table
        .load_style(UTF8_FULL.with_rounded_corners())
        .set_content_arrangement(ContentArrangement::Dynamic);

    // Show which searches matched each prefix when searching
    let searching = !searches.is_empty();
    let header = [
        "IP Prefix",
        "Region",
        "Network Border Group",
        "Services",
        "Matches",
    ];
    prefix_table.set_header(
        header
            .iter()
            .take(if searching { 5 } else { 4 })
            .map(|title| {
                Cell::new(title)
                    .add_attribute(Attribute::Bold)
                    .fg(Color::Green)
            }),
    );

    for prefix in aws_ip_ranges.prefixes().values() {
        let mut sorted_services = prefix
            .services
            .iter()
            .map(|service| service.to_string())
            .collect::<Vec<String>>();
        sorted_services.sort();
        let services = sorted_services.join(", ");

        let mut row = vec![
            Cell::new(prefix.prefix).add_attribute(Attribute::Bold),
            Cell::new(&prefix.region),
            Cell::new(&prefix.network_border_group),
            Cell::new(services),
        ];
        if searching {
            let matches: Vec<String> = search::matches(prefix, searches)
                .iter()
                .map(Match::to_lines)
                .collect();
            row.push(Cell::new(matches.join("\n")));
        }
        prefix_table.add_row(row);
    }

    // Right-align the IP Prefix column
    let column = prefix_table.column_mut(0).expect("The first column exists");
    column.set_cell_alignment(CellAlignment::Right);

    writeln!(out, "{prefix_table}")?;

    // Prefix Table Summary
    let aws_ip_prefix_count = aws_ip_ranges.prefixes().len();
    let aws_region_count = aws_ip_ranges.regions().len();
    let sync_token = aws_ip_ranges.sync_token();
    let create_date = aws_ip_ranges.create_date();

    let mut summary_table = Table::new();
    summary_table.load_style(NOTHING);

    summary_table.add_row(vec![
        Cell::new(aws_ip_prefix_count),
        Cell::new(if aws_ip_prefix_count == 1 {
            "AWS IP Prefix"
        } else {
            "AWS IP Prefixes"
        }),
        Cell::new("Data File Created")
            .set_alignment(CellAlignment::Right)
            .fg(Color::DarkGrey),
        Cell::new(create_date).fg(Color::DarkGrey),
    ]);
    summary_table.add_row(vec![
        Cell::new(aws_region_count),
        Cell::new(if aws_region_count == 1 {
            "AWS Region"
        } else {
            "AWS Regions"
        }),
        Cell::new("Sync Token")
            .set_alignment(CellAlignment::Right)
            .fg(Color::DarkGrey),
        Cell::new(sync_token).fg(Color::DarkGrey),
    ]);

    let summary_numbers_column = summary_table
        .column_mut(0)
        .expect("The first column exists");
    summary_numbers_column.set_cell_alignment(CellAlignment::Right);

    writeln!(out, "{summary_table}")
}

/*--------------------------------------------------------------------------------------
  JSON
--------------------------------------------------------------------------------------*/

#[derive(Serialize)]
struct JsonOutput<'a> {
    sync_token: &'a str,
    create_date: &'a DateTime<Utc>,
    prefixes: Vec<JsonPrefix<'a>>,
}

#[derive(Serialize)]
struct JsonPrefix<'a> {
    #[serde(flatten)]
    prefix: &'a AwsIpPrefix,

    /// The searches that matched the prefix; present only when searching.
    #[serde(skip_serializing_if = "Option::is_none")]
    matches: Option<Vec<Match>>,
}

pub fn json(
    out: &mut impl Write,
    aws_ip_ranges: &AwsIpRanges,
    searches: &[Search],
) -> io::Result<()> {
    let json_output = JsonOutput {
        sync_token: aws_ip_ranges.sync_token(),
        create_date: aws_ip_ranges.create_date(),
        prefixes: aws_ip_ranges
            .prefixes()
            .values()
            .map(|prefix| JsonPrefix {
                prefix,
                matches: (!searches.is_empty()).then(|| search::matches(prefix, searches)),
            })
            .collect(),
    };
    serde_json::to_writer_pretty(&mut *out, &json_output)?;
    writeln!(out)
}

/*--------------------------------------------------------------------------------------
  Prefixes In CIDR Format
--------------------------------------------------------------------------------------*/

pub fn prefixes_in_cidr_format(
    out: &mut impl Write,
    aws_ip_ranges: &AwsIpRanges,
    _searches: &[Search],
) -> io::Result<()> {
    aws_ip_ranges
        .prefixes()
        .values()
        .try_for_each(|aws_ip_prefix| writeln!(out, "{}", aws_ip_prefix.prefix))
}

/*--------------------------------------------------------------------------------------
  Prefixes In Netmask Format
--------------------------------------------------------------------------------------*/

pub fn prefixes_in_netmask_format(
    out: &mut impl Write,
    aws_ip_ranges: &AwsIpRanges,
    _searches: &[Search],
) -> io::Result<()> {
    aws_ip_ranges
        .prefixes()
        .values()
        .try_for_each(|aws_ip_prefix| {
            writeln!(
                out,
                "{} {}",
                aws_ip_prefix.prefix.network(),
                aws_ip_prefix.prefix.mask()
            )
        })
}

/*--------------------------------------------------------------------------------------
  Regions, Network Border Groups, and Services
--------------------------------------------------------------------------------------*/

pub fn regions(
    out: &mut impl Write,
    aws_ip_ranges: &AwsIpRanges,
    _searches: &[Search],
) -> io::Result<()> {
    lines(out, aws_ip_ranges.regions())
}

pub fn network_border_groups(
    out: &mut impl Write,
    aws_ip_ranges: &AwsIpRanges,
    _searches: &[Search],
) -> io::Result<()> {
    lines(out, aws_ip_ranges.network_border_groups())
}

pub fn services(
    out: &mut impl Write,
    aws_ip_ranges: &AwsIpRanges,
    _searches: &[Search],
) -> io::Result<()> {
    lines(out, aws_ip_ranges.services())
}

fn lines<T: std::fmt::Display>(
    out: &mut impl Write,
    values: impl IntoIterator<Item = T>,
) -> io::Result<()> {
    values
        .into_iter()
        .try_for_each(|value| writeln!(out, "{value}"))
}
