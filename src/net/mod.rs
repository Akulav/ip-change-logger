//! Everything that talks to the network.

pub mod feeds;
pub mod intel;
pub mod public_ip;

use std::time::Duration;

/// Shared HTTP agent with sane timeouts and an identifying user agent.
pub fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(12)))
        .user_agent(concat!(
            "IPTrail/",
            env!("CARGO_PKG_VERSION"),
            " (+https://github.com/Akulav/ip-change-logger)"
        ))
        .build()
        .into()
}
