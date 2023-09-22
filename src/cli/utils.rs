/*-------------------------------------------------------------------------------------------------
  Utility Functions
-------------------------------------------------------------------------------------------------*/

pub fn to_lowercase<const COUNT: usize>(value: &str, exceptions: [&str; COUNT]) -> String {
    let lower = value.to_lowercase();
    let upper = value.to_uppercase();

    if exceptions.contains(&upper.as_str()) {
        upper
    } else {
        lower
    }
}
