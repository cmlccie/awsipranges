use crate::cli::search::{self, Search};
use awsipranges::AwsIpRanges;
use std::path::Path;
use std::rc::Rc;

/*-------------------------------------------------------------------------------------------------
  Save AWS IP Ranges to CSV File
-------------------------------------------------------------------------------------------------*/

pub fn save(aws_ip_ranges: &AwsIpRanges, searches: &[Search], path: &Path) -> csv::Result<()> {
    let mut writer = csv::Writer::from_path(path)?;
    let searching = !searches.is_empty();

    // Write header; the Matches column lists the searches that matched each prefix
    let header = [
        "AWS IP Prefix",
        "Region",
        "Network Border Group",
        "Services",
        "Matches",
    ];
    writer.write_record(&header[..if searching { 5 } else { 4 }])?;

    // Write prefix records
    for aws_ip_prefix in aws_ip_ranges.prefixes().values() {
        let mut record = vec![
            aws_ip_prefix.prefix.to_string(),
            aws_ip_prefix.region.to_string(),
            aws_ip_prefix.network_border_group.to_string(),
            aws_ip_prefix
                .services
                .iter()
                .cloned()
                .collect::<Vec<Rc<str>>>()
                .join(", "),
        ];
        if searching {
            let matches: Vec<String> = search::matches(aws_ip_prefix, searches)
                .iter()
                .map(ToString::to_string)
                .collect();
            record.push(matches.join("; "));
        }
        writer.write_record(&record)?;
    }

    writer.flush()?;

    Ok(())
}
