//! `/etc/robot/quack-control.toml`: where the page listens, the token, and
//! the services it manages — one `[[service]]` each, with the adapter its
//! `kind` names.

use anyhow::Context;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    /// Every interface, port 8090 (mediad's console has 8080), by default: the home network.
    pub bind: String,
    /// When set, every call and the live streams need it (the `X-Token`
    /// header, or `?token=`); the page itself loads without it.
    /// `QC_TOKEN` in the environment overrides it, so it can stay out of
    /// a world-readable file.
    pub token: Option<String>,
    #[serde(rename = "service")]
    pub services: Vec<ServiceConfig>,
}

/// One managed daemon. The fields a kind does not use are ignored by it.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceConfig {
    /// Its name in the page and in the URLs (`/api/<name>/…`).
    pub name: String,
    /// `quack-nav`, `mediad` or `quacksat` (see `src/adapters/`).
    pub kind: String,
    /// quack-nav: its nav socket (quack-navd's `socket`).
    #[serde(default = "nav_socket")]
    pub nav_socket: String,
    /// quack-nav: its map socket (quack-navd's `[maploc] socket`; robotd's
    /// own when quack-navd does not host the mapper).
    #[serde(default = "map_socket")]
    pub map_socket: String,
    /// mediad: its local frame socket (`media.frame`; on the twin,
    /// `$STATE/media.sock`).
    #[serde(default = "media_socket")]
    pub media_socket: String,
}

fn nav_socket() -> String {
    "/run/quack-nav/nav.sock".into()
}

fn map_socket() -> String {
    "/run/quack-nav/map.sock".into()
}

/// duck_ipc_proto's `socket::MEDIA` (daemon-v0.14.4 and later).
fn media_socket() -> String {
    "/run/mediad/media.sock".into()
}

impl ServiceConfig {
    pub fn new(name: &str, kind: &str) -> Self {
        Self {
            name: name.into(),
            kind: kind.into(),
            nav_socket: nav_socket(),
            map_socket: map_socket(),
            media_socket: media_socket(),
        }
    }
}

impl Default for Config {
    /// quack-nav, the camera (mediad) and quacksat at their installed paths.
    fn default() -> Self {
        Self {
            bind: "0.0.0.0:8090".into(),
            token: None,
            services: vec![
                ServiceConfig::new("quack-nav", "quack-nav"),
                ServiceConfig::new("camera", "mediad"),
                ServiceConfig::new("quacksat", "quacksat"),
            ],
        }
    }
}

impl Config {
    /// Read the file, or take the defaults when there is none, then the
    /// token from `QC_TOKEN` when it is set.
    pub fn load(path: &str) -> anyhow::Result<Self> {
        let mut config = match std::fs::read_to_string(path) {
            Ok(text) => Self::parse(&text).with_context(|| format!("the config file {path} does not parse"))?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                tracing::info!(path, "no config file; the defaults it is");
                Self::default()
            }
            Err(e) => return Err(e).with_context(|| format!("cannot read the config file {path}")),
        };
        if let Ok(token) = std::env::var("QC_TOKEN") {
            config.token = Some(token);
        }
        config.token = config.token.filter(|t| !t.trim().is_empty());
        Ok(config)
    }

    pub fn parse(text: &str) -> anyhow::Result<Self> {
        let config: Self = toml::from_str(text)?;
        for (i, s) in config.services.iter().enumerate() {
            anyhow::ensure!(
                !s.name.is_empty() && s.name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
                "service `{}`: a name is letters, digits, '-' and '_'",
                s.name
            );
            anyhow::ensure!(
                !config.services[..i].iter().any(|t| t.name == s.name),
                "two services are named `{}`",
                s.name
            );
        }
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_lists_services_with_their_defaults() {
        let c = Config::parse(
            "bind = \"127.0.0.1:9000\"\n[[service]]\nname = \"nav\"\nkind = \"quack-nav\"\nnav_socket = \"/tmp/n.sock\"\n",
        )
        .unwrap();
        assert_eq!(c.bind, "127.0.0.1:9000");
        assert_eq!(c.token, None);
        assert_eq!(c.services.len(), 1);
        assert_eq!(c.services[0].nav_socket, "/tmp/n.sock");
        assert_eq!(c.services[0].map_socket, "/run/quack-nav/map.sock");
    }

    #[test]
    fn no_file_means_every_daemon_at_its_path() {
        let c = Config::load("/nonexistent/quack-control.toml").unwrap();
        let kinds: Vec<&str> = c.services.iter().map(|s| s.kind.as_str()).collect();
        assert_eq!(kinds, ["quack-nav", "mediad", "quacksat"]);
        assert_eq!(c.services[1].media_socket, "/run/mediad/media.sock");
        assert_eq!(c.bind, "0.0.0.0:8090");
    }

    #[test]
    fn names_must_be_url_safe_and_unique() {
        assert!(Config::parse("[[service]]\nname = \"a b\"\nkind = \"quack-nav\"\n").is_err());
        assert!(Config::parse("[[service]]\nname = \"a\"\nkind = \"x\"\n[[service]]\nname = \"a\"\nkind = \"y\"\n").is_err());
        assert!(Config::parse("typo = 1\n").is_err());
    }
}
