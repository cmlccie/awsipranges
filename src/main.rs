fn main() {
    let aws_ip_ranges = awsipranges::AwsIpRanges::new();

    println!("sync_token:    {}", aws_ip_ranges.sync_token);
    println!("creation_date: {}", aws_ip_ranges.create_date);
    println!("");
    println!(
        "First {:?}",
        aws_ip_ranges.ipv4_prefixes.iter().next().unwrap()
    );
    println!(
        "First {:?}",
        aws_ip_ranges.ipv6_prefixes.iter().next().unwrap()
    );
    println!("");
    println!("Regions {:?}", aws_ip_ranges.regions);
    println!("");
    println!(
        "Network Border Groups {:?}",
        aws_ip_ranges.network_border_groups
    );
    println!("");
    println!("Services {:?}", aws_ip_ranges.services);
}
