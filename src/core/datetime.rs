use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{self, Deserialize, Deserializer, Serializer};

/*-------------------------------------------------------------------------------------------------
  DateTime Format
-------------------------------------------------------------------------------------------------*/

const AWS_IP_RANGES_DATETIME_FORMAT: &'static str = "%Y-%m-%d-%H-%M-%S";

pub fn serialize<S>(date: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let s = format!("{}", date.format(AWS_IP_RANGES_DATETIME_FORMAT));
    serializer.serialize_str(&s)
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    NaiveDateTime::parse_from_str(&s, AWS_IP_RANGES_DATETIME_FORMAT)
        .map(|naive_date_time| naive_date_time.and_utc())
        .map_err(serde::de::Error::custom)
}
