use awsipranges::{AwsIpRanges, Result};
use std::path::PathBuf;
use std::rc::Rc;

/*-------------------------------------------------------------------------------------------------
  Save AWS IP Ranges to CSV File
-------------------------------------------------------------------------------------------------*/

pub fn save(aws_ip_ranges: &AwsIpRanges, path: &PathBuf) -> Result<()> {
    let mut writer = csv::Writer::from_path(path)?;

    // Write header
    writer.serialize(&[
        "AWS IP Prefix",
        "Region",
        "Network Border Group",
        "Services",
    ])?;

    // Write prefix records
    for aws_ip_prefix in aws_ip_ranges.prefixes().values() {
        let record = (
            &aws_ip_prefix.prefix,
            aws_ip_prefix.region.clone(),
            aws_ip_prefix.network_border_group.clone(),
            aws_ip_prefix
                .services
                .iter()
                .map(|service| service.clone())
                .collect::<Vec<Rc<str>>>()
                .join(", "),
        );
        writer.serialize(record)?;
    }

    writer.flush()?;

    Ok(())
}
