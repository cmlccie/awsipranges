use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{self, Deserialize, Deserializer, Serializer};

/*-------------------------------------------------------------------------------------------------
  DateTime Format
-------------------------------------------------------------------------------------------------*/

const AWS_IP_RANGES_DATETIME_FORMAT: &str = "%Y-%m-%d-%H-%M-%S";

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

/*-------------------------------------------------------------------------------------------------
  Unit Tests
-------------------------------------------------------------------------------------------------*/

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use serde::Serialize;
    use serde_json::{json, Value};

    #[derive(Serialize)]
    struct TestDateTime {
        #[serde(with = "super")]
        datetime: DateTime<Utc>,
    }

    #[test]
    fn test_serialize() {
        let test_datetime = TestDateTime {
            datetime: Utc.with_ymd_and_hms(2022, 1, 1, 0, 0, 0).unwrap(),
        };

        let serialized_value: Value = serde_json::to_value(test_datetime).unwrap();
        let expected_value: Value = json!({"datetime": "2022-01-01-00-00-00"});

        assert_eq!(serialized_value, expected_value);
    }

    #[test]
    fn test_deserialize() {
        let test_datetime_json = r#"{"datetime": "2022-01-01-00-00-00"}"#;

        let deserialized_value: Value = serde_json::from_str(test_datetime_json).unwrap();
        let expected_value: Value = json!({"datetime": "2022-01-01-00-00-00"});

        assert_eq!(deserialized_value, expected_value);
    }
}
