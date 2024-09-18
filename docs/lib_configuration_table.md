| Environment Variable               | Default Value                                    | Configuration Method                  |
| ---------------------------------- | ------------------------------------------------ | ------------------------------------- |
| `AWSIPRANGES_URL`                  | `https://ip-ranges.amazonaws.com/ip-ranges.json` | [ClientBuilder::url]                  |
| `AWSIPRANGES_CACHE_FILE`           | `${HOME}/.aws/ip-ranges.json`                    | [ClientBuilder::cache_file]           |
| `AWSIPRANGES_CACHE_TIME`           | `86400` seconds (24 hours)                       | [ClientBuilder::cache_time]           |
| `AWSIPRANGES_RETRY_COUNT`          | `4`                                              | [ClientBuilder::retry_count]          |
| `AWSIPRANGES_RETRY_INITIAL_DELAY`  | `200` milliseconds                               | [ClientBuilder::retry_initial_delay]  |
| `AWSIPRANGES_RETRY_BACKOFF_FACTOR` | `2`                                              | [ClientBuilder::retry_backoff_factor] |
| `AWSIPRANGES_RETRY_TIMEOUT`        | `5000` milliseconds (5 seconds)                  | [ClientBuilder::retry_timeout]        |
