//! One adapter per kind of daemon: what quack-control knows about it.
//!
//! The server does the HTTP, the token and the page; an adapter turns
//! `/api/<service>/<path>` into calls on its daemon's own sockets and says
//! whether the daemon answers. A new daemon plugs in with an [`Adapter`]
//! and a `kind` in [`build`].

use std::sync::Arc;

use serde_json::Value;

use crate::config::ServiceConfig;

pub mod quack_nav;
pub mod quacksat;

/// What an adapter answers a request with.
pub enum Reply {
    /// A status and a JSON body.
    Json(u16, Value),
    /// A page.
    Html(&'static str),
    /// Server-sent events: what is known now, then what follows.
    Events(Vec<quack_nav::hub::Item>, std::sync::mpsc::Receiver<quack_nav::hub::Item>),
}

/// Whether a daemon answers, in a word and a sentence.
#[derive(Debug, Clone, PartialEq)]
pub struct Health {
    pub available: bool,
    pub detail: String,
}

pub trait Adapter: Send + Sync {
    fn name(&self) -> &str;
    fn kind(&self) -> &'static str;
    /// Start its background lanes (streams, polls). Called once.
    fn start(&self);
    fn health(&self) -> Health;
    /// What the page may do with it, for the page to show (`["map", …]`).
    fn features(&self) -> Vec<&'static str>;
    /// One request under `/api/<name>/`: `path` is the rest, `body` the
    /// JSON a POST carried (null for a GET).
    fn route(&self, method: &str, path: &str, body: &Value) -> Reply;
}

/// The adapters the config asks for. An unknown kind is an error: a typo
/// should not leave a service silently unmanaged.
pub fn build(services: &[ServiceConfig]) -> anyhow::Result<Vec<Arc<dyn Adapter>>> {
    services
        .iter()
        .map(|s| -> anyhow::Result<Arc<dyn Adapter>> {
            match s.kind.as_str() {
                "quack-nav" => Ok(Arc::new(quack_nav::QuackNav::new(s))),
                "quacksat" => Ok(Arc::new(quacksat::Quacksat::new(s))),
                other => anyhow::bail!("service `{}`: no adapter for kind `{other}` (quack-nav, quacksat)", s.name),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_kind_has_its_adapter_and_typos_are_refused() {
        let services = [ServiceConfig::new("nav", "quack-nav"), ServiceConfig::new("sat", "quacksat")];
        let adapters = build(&services).unwrap();
        assert_eq!(adapters.iter().map(|a| a.kind()).collect::<Vec<_>>(), ["quack-nav", "quacksat"]);
        assert_eq!(adapters[0].name(), "nav");
        let e = build(&[ServiceConfig::new("x", "quack-navd")]).err().unwrap().to_string();
        assert!(e.contains("no adapter"), "{e}");
    }
}
