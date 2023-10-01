/*-------------------------------------------------------------------------------------------------
  Blocking Modules
-------------------------------------------------------------------------------------------------*/

mod json;

/*-------------------------------------------------------------------------------------------------
  Primary Interface
-------------------------------------------------------------------------------------------------*/

use crate::blocking::json::get_json;
use crate::core::awsipranges::AwsIpRanges;
use crate::core::errors::Result;

pub fn get_ranges() -> Result<Box<AwsIpRanges>> {
    let json = get_json()?;
    AwsIpRanges::from_json(&json)
}
