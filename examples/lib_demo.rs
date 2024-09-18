use awsipranges::ipnetwork::IpNetwork;
use awsipranges::Result;

fn main() -> Result<()> {
    // Get the AWS IP Ranges
    let aws_ip_ranges = awsipranges::get_ranges()?;

    // Find the longest match prefix for an IP Address
    let ip_address: IpNetwork = "3.141.102.225".parse().unwrap();
    let prefix = aws_ip_ranges.get_longest_match_prefix(&ip_address);
    println!("{:?}", prefix);

    // Search for IP Prefixes
    let search_prefixes: Vec<IpNetwork> = vec![
        "3.141.102.225".parse().unwrap(),
        "44.192.140.65".parse().unwrap(),
    ];
    let search_results = aws_ip_ranges.search(&search_prefixes);
    for aws_ip_prefix in search_results.aws_ip_ranges.prefixes().values() {
        println!("{:?}", aws_ip_prefix);
    }

    // Filter the AWS IP Ranges
    let filtered_ranges = aws_ip_ranges
        .filter_builder()
        .ipv4()
        .regions(["us-west-2"])?
        .services(["S3"])?
        .filter();
    for aws_ip_prefix in filtered_ranges.prefixes().values() {
        println!("{:?}", aws_ip_prefix);
    }

    Ok(())
}
