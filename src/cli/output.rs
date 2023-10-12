use awsipranges::AwsIpRanges;
use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::presets::{NOTHING, UTF8_FULL};
use comfy_table::*;

/*-------------------------------------------------------------------------------------------------
  Output Functions
-------------------------------------------------------------------------------------------------*/

/*--------------------------------------------------------------------------------------
  Prefix Table
--------------------------------------------------------------------------------------*/

pub fn prefix_table(aws_ip_ranges: &AwsIpRanges) {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic);

    table.set_header(vec![
        Cell::new("IP Prefix")
            .add_attribute(Attribute::Bold)
            .fg(Color::Green),
        Cell::new("Region")
            .add_attribute(Attribute::Bold)
            .fg(Color::Green),
        Cell::new("Network Border Group")
            .add_attribute(Attribute::Bold)
            .fg(Color::Green),
        Cell::new("Services")
            .add_attribute(Attribute::Bold)
            .fg(Color::Green),
    ]);

    for prefix in aws_ip_ranges.prefixes().values() {
        let mut sorted_services = prefix
            .services
            .iter()
            .map(|service| service.to_string())
            .collect::<Vec<String>>();
        sorted_services.sort();
        let services = sorted_services.join(", ");

        table.add_row(vec![
            Cell::new(prefix.prefix).add_attribute(Attribute::Bold),
            Cell::new(&prefix.region),
            Cell::new(&prefix.network_border_group),
            Cell::new(services),
        ]);
    }

    // Right-align the IP Prefix column
    let column = table.column_mut(0).expect("The first column exists");
    column.set_cell_alignment(CellAlignment::Right);

    println!("{table}");

    // Print prefix-table summary
    let aws_ip_prefix_count = aws_ip_ranges.prefixes().len();
    let aws_region_count = aws_ip_ranges.regions().len();

    let mut summary_table = Table::new();
    summary_table
        .load_preset(NOTHING)
        .set_content_arrangement(ContentArrangement::Dynamic);

    summary_table.add_row(vec![
        Cell::new(aws_ip_prefix_count),
        Cell::new("AWS IP Prefixes"),
    ]);
    summary_table.add_row(vec![Cell::new(aws_region_count), Cell::new("AWS Regions")]);

    let summary_numbers_column = summary_table
        .column_mut(0)
        .expect("The first column exists");
    summary_numbers_column.set_cell_alignment(CellAlignment::Right);

    println!("{summary_table}");
}

/*--------------------------------------------------------------------------------------
  Prefixes In CIDR Format
--------------------------------------------------------------------------------------*/

pub fn prefixes_in_cidr_format(aws_ip_ranges: &AwsIpRanges) {
    for aws_ip_prefix in aws_ip_ranges.prefixes().values() {
        println!("{}", aws_ip_prefix.prefix);
    }
}

/*--------------------------------------------------------------------------------------
  Prefixes In Netmask Format
--------------------------------------------------------------------------------------*/

pub fn prefixes_in_netmask_format(aws_ip_ranges: &AwsIpRanges) {
    for aws_ip_prefix in aws_ip_ranges.prefixes().values() {
        println!(
            "{} {}",
            aws_ip_prefix.prefix.network(),
            aws_ip_prefix.prefix.mask()
        );
    }
}

/*--------------------------------------------------------------------------------------
  Regions
--------------------------------------------------------------------------------------*/

pub fn regions(aws_ip_ranges: &AwsIpRanges) {
    for region in aws_ip_ranges.regions().iter() {
        println!("{region}");
    }
}

/*--------------------------------------------------------------------------------------
  Network Border Groups
--------------------------------------------------------------------------------------*/

pub fn network_border_groups(aws_ip_ranges: &AwsIpRanges) {
    for network_border_group in aws_ip_ranges.network_border_groups().iter() {
        println!("{network_border_group}");
    }
}

/*--------------------------------------------------------------------------------------
  Services
--------------------------------------------------------------------------------------*/

pub fn services(aws_ip_ranges: &AwsIpRanges) {
    for service in aws_ip_ranges.services().iter() {
        println!("{service}");
    }
}
