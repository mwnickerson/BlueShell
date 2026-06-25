use stage1::{config::AgentConfig, runtime::Agent};

fn main() {
    let config = AgentConfig::stamped();
    stage1::diagnostic!(
        "starting transport={} endpoint={} uri={} interval_ms={} jitter={}",
        config.transport,
        config.endpoint,
        config.uri,
        config.interval_ms,
        config.jitter_pct
    );
    match Agent::new(config) {
        Ok(mut agent) => {
            if let Err(error) = agent.run() {
                stage1::diagnostic!("agent stopped: {error:?}");
            }
        }
        Err(error) => {
            stage1::diagnostic!("initialization failed: {error:?}");
        }
    }
}
