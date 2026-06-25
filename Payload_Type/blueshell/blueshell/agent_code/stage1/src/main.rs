use stage1::{config::AgentConfig, runtime::Agent};

fn main() {
    let config = AgentConfig::stamped();
    if let Ok(mut agent) = Agent::new(config) {
        let _ = agent.run();
    }
}
