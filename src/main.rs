use awsipranges;

fn main() {
    let ip_ranges = awsipranges::get_ip_ranges();

    println!("sync_token:    {}", ip_ranges.sync_token);
    println!("creation_date: {}", ip_ranges.create_date);
    println!("prefixes:      {}", ip_ranges.prefixes.len());
    println!("ipv6_prefixes: {}", ip_ranges.ipv6_prefixes.len());
    println!("");
    println!("First {:?}", ip_ranges.prefixes.first().unwrap());
    println!("First {:?}", ip_ranges.ipv6_prefixes.first().unwrap());
}
