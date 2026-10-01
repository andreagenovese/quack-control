//! `quack-control`: a local control plane for the Microduck's daemons.
//!
//! One page on the home network — the live map, tap to go, places,
//! exploring, every tool and the knobs — served by one small process on
//! the duck that talks to each daemon over its own unix sockets, through
//! an adapter per kind (`src/adapters/`): quack-nav now, quacksat and
//! others later. Plain HTTP for the home network only; an optional token.
//!
//!     quack-control [/etc/robot/quack-control.toml]

mod adapters;
mod config;
mod http;
mod server;

use std::sync::Arc;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .init();
    let path = std::env::args().nth(1).unwrap_or_else(|| "/etc/robot/quack-control.toml".into());
    let config = config::Config::load(&path)?;
    let adapters = adapters::build(&config.services)?;
    for adapter in &adapters {
        tracing::info!(service = adapter.name(), kind = adapter.kind(), "managing");
        adapter.start();
    }
    if config.token.is_none() {
        tracing::warn!("no token: anyone on this network can open the page and drive the duck (set `token`, or QC_TOKEN)");
    }
    let server = Arc::new(server::Server { token: config.token.clone(), adapters });
    server.run(&config.bind)
}
