//! quacksat, the voice satellite: a placeholder until it has something to
//! talk to.
//!
//! quacksat (at ca9b782) binds no socket of its own: it is a client of
//! robotd's and quack-navd's sockets and of its LLM/STT/TTS endpoints, its
//! settings are `/etc/robot/quacksat.toml` (the backend — `none`, `agent`,
//! `direct`, `wyoming` —, the audio devices, the wake word, `[nav]`) and
//! its one environment knob `QUACKSAT_MODELS_DIR`. Its unit creates
//! `/run/quacksat/` (`RuntimeDirectory=`), which says it is installed and
//! nothing more. What it would need to expose, on a socket there, for this
//! adapter to become real (docs/todo.md):
//!
//! - `sat.status`: the backend and whether it is connected, the wake word
//!   (listening, muted), the audio devices found, the nav lane up or down,
//!   the last turn's time and outcome;
//! - `sat.config`: the config's sections as JSON, and a way to set the few
//!   that are safe to change live (the wake threshold, the backend's URL),
//!   with a restart like quack-nav's `nav.restart` for the rest;
//! - later, the chat (quack-nav's `docs/study/map-app.md` §3):
//!   `/run/quacksat/chat.sock` with `chat.say`, streamed `chat.delta` /
//!   `chat.tool` / `chat.done`, `chat.subscribe`, and the turn lock shared
//!   with the wake word.

use std::path::Path;

use serde_json::{Value, json};

use super::{Adapter, Health, Reply};
use crate::config::ServiceConfig;

const RUNTIME_DIR: &str = "/run/quacksat";

pub struct Quacksat {
    name: String,
}

impl Quacksat {
    pub fn new(config: &ServiceConfig) -> Self {
        Self { name: config.name.clone() }
    }
}

impl Adapter for Quacksat {
    fn name(&self) -> &str {
        &self.name
    }

    fn kind(&self) -> &'static str {
        "quacksat"
    }

    fn start(&self) {}

    fn health(&self) -> Health {
        let detail = if Path::new(RUNTIME_DIR).is_dir() {
            "installed, but quacksat exposes no control socket yet: nothing to manage from here (see docs/todo.md)"
        } else {
            "not available: quacksat exposes no control socket yet, and it does not seem to run here"
        };
        Health { available: false, detail: detail.into() }
    }

    fn features(&self) -> Vec<&'static str> {
        Vec::new()
    }

    fn route(&self, _method: &str, _path: &str, _body: &Value) -> Reply {
        Reply::Json(501, json!({"ok": false, "error": "quacksat is not available from quack-control yet: it exposes no control socket (see docs/todo.md)"}))
    }
}
