use std::time::Duration;

const STAMP: &str = include_str!(concat!(env!("OUT_DIR"), "/build_config.json"));

#[derive(Clone, Debug, serde::Deserialize)]
pub struct AgentConfig {
    pub payload_uuid: String,
    pub key_b64: String,
    pub transport: String,
    pub endpoint: String,
    pub uri: String,
    pub interval_ms: u64,
    pub jitter_pct: u8,
}

impl AgentConfig {
    pub fn stamped() -> Self {
        serde_json::from_str(STAMP).unwrap_or_else(|_| Self {
            payload_uuid: String::new(),
            key_b64: String::new(),
            transport: String::new(),
            endpoint: String::new(),
            uri: String::new(),
            interval_ms: 5_000,
            jitter_pct: 0,
        })
    }

    pub fn sleep_duration(&self) -> Duration {
        let spread = self
            .interval_ms
            .saturating_mul(self.jitter_pct.min(100) as u64)
            / 100;
        let offset = if spread == 0 {
            0
        } else {
            rand::random::<u64>() % (spread * 2 + 1)
        };
        Duration::from_millis(
            self.interval_ms
                .saturating_sub(spread)
                .saturating_add(offset),
        )
    }
}
