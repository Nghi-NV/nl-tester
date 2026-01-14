/// Application configuration
pub struct Config {
    /// Default timeout for element waiting (ms)
    pub default_timeout_ms: u64,

    /// Default retry count for failed commands
    pub default_retry_count: u32,

    /// Delay between retries (ms)
    pub retry_delay_ms: u64,

    /// Continue on failure flag
    pub continue_on_failure: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_timeout_ms: 5000,
            default_retry_count: 3,
            retry_delay_ms: 1000,
            continue_on_failure: false,
        }
    }
}
