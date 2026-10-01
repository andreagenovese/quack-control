//! quack-nav: quack-navd's two sockets, as its control contract describes
//! them (quack-nav's `docs/control-contract.md`).
//!
//! | request | what it does |
//! |---|---|
//! | `GET events` | the live map, the pose, the status, the places, as server-sent events |
//! | `POST call` `{name, args}` | one of the map view's calls ([`bridge::vet`]) |
//! | `GET catalog` | `nav.catalog`: every tool, with its JSON Schema |
//! | `POST advanced` `{name, args, confirmed}` | any catalog tool ([`bridge::vet_advanced`]) |
//! | `GET knobs`, `POST knobs` `{set}` / `{reset_all}` | `nav.knobs` |
//! | `POST restart` `{confirmed: true}` | `nav.restart` |

use std::time::Duration;

use serde_json::{Value, json};

use super::{Adapter, Health, Reply};
use crate::config::ServiceConfig;

pub mod bridge;
pub mod hub;

use bridge::{CallError, nav_call, rpc};

/// A call may queue behind another holding the robot: `explore complete`
/// waits up to two minutes for the session to save.
const CALL_TIMEOUT: Duration = Duration::from_secs(150);
const QUICK: Duration = Duration::from_secs(10);

pub struct QuackNav {
    name: String,
    nav_socket: String,
    map_socket: String,
    hub: hub::Hub,
}

impl QuackNav {
    pub fn new(config: &ServiceConfig) -> Self {
        Self {
            name: config.name.clone(),
            nav_socket: config.nav_socket.clone(),
            map_socket: config.map_socket.clone(),
            hub: hub::Hub::default(),
        }
    }

    fn call(&self, body: &Value) -> Reply {
        let name = body.get("name").and_then(Value::as_str).unwrap_or_default();
        let args = body.get("args").cloned().unwrap_or(Value::Null);
        let args = match bridge::vet(name, &args) {
            Ok(args) => args,
            Err(e) => return refused(403, &e),
        };
        // A point is taught only by a quack-navd that knows how: an older
        // one would ignore x and y and name where the duck stands.
        if name == "robot.remember_place" && args.get("x").is_some() && !self.teaches_points() {
            return refused(409, "this quack-navd cannot name a point on the map yet (it predates `remember_place` with x and y): update quack-nav");
        }
        let reply = answer(nav_call(&self.nav_socket, name, &args, CALL_TIMEOUT));
        if matches!(name, "robot.remember_place" | "robot.forget_place") {
            self.hub.refresh_places(&self.nav_socket);
        }
        reply
    }

    fn catalog(&self) -> Result<Value, CallError> {
        rpc(&self.nav_socket, &json!({"jsonrpc": "2.0", "id": 1, "method": "nav.catalog", "params": {}}), QUICK)
    }

    fn teaches_points(&self) -> bool {
        self.catalog().is_ok_and(|tools| {
            tools.as_array().is_some_and(|tools| {
                tools.iter().any(|t| t["name"] == "robot.remember_place" && t["parameters"]["properties"].get("x").is_some())
            })
        })
    }

    fn advanced(&self, body: &Value) -> Reply {
        let catalog = match self.catalog() {
            Ok(catalog) => bridge::catalog_names(&catalog),
            Err(e) => return answer(Err(e)),
        };
        let name = body.get("name").and_then(Value::as_str).unwrap_or_default();
        let confirmed = body.get("confirmed").and_then(Value::as_bool) == Some(true);
        match bridge::vet_advanced(name, body.get("args").unwrap_or(&Value::Null), &catalog, confirmed) {
            Ok(args) => {
                let reply = answer(nav_call(&self.nav_socket, name, &args, CALL_TIMEOUT));
                self.hub.refresh_places(&self.nav_socket);
                reply
            }
            Err(e) => refused(403, &e),
        }
    }

    fn method(&self, method: &str, params: Value) -> Reply {
        answer(rpc(&self.nav_socket, &json!({"jsonrpc": "2.0", "id": 1, "method": method, "params": params}), QUICK))
    }
}

/// A daemon's answer as the page takes it: `{ok, result}` or `{ok, error}`.
fn answer(result: Result<Value, CallError>) -> Reply {
    match result {
        Ok(result) => Reply::Json(200, json!({"ok": true, "result": result})),
        // A refusal is quack-navd's answer, not a failure of the bridge.
        Err(CallError::Refused(e)) if e.starts_with("unknown method") => {
            Reply::Json(501, json!({"ok": false, "error": format!("this quack-navd is older than this page: {e}; update quack-nav")}))
        }
        Err(CallError::Refused(e)) => Reply::Json(200, json!({"ok": false, "error": e})),
        Err(e) => Reply::Json(502, json!({"ok": false, "error": e.text()})),
    }
}

fn refused(status: u16, why: &str) -> Reply {
    Reply::Json(status, json!({"ok": false, "error": why}))
}

impl Adapter for QuackNav {
    fn name(&self) -> &str {
        &self.name
    }

    fn kind(&self) -> &'static str {
        "quack-nav"
    }

    fn start(&self) {
        let (hub, socket) = (self.hub.clone(), self.map_socket.clone());
        std::thread::Builder::new().name("nav-map".into()).spawn(move || hub.run_map(socket)).expect("a thread");
        let (hub, socket) = (self.hub.clone(), self.nav_socket.clone());
        std::thread::Builder::new().name("nav-status".into()).spawn(move || hub.run_status(socket)).expect("a thread");
    }

    fn health(&self) -> Health {
        let (map, nav) = self.hub.links();
        let nav = if nav.is_empty() { "not asked yet".to_owned() } else { nav };
        let map = if map.is_empty() { "not connected yet".to_owned() } else { map };
        Health { available: nav == "ok", detail: format!("nav: {nav} · map: {map}") }
    }

    fn features(&self) -> Vec<&'static str> {
        vec!["map", "places", "explore", "advanced", "knobs"]
    }

    fn route(&self, method: &str, path: &str, body: &Value) -> Reply {
        let confirmed = body.get("confirmed").and_then(Value::as_bool) == Some(true);
        match (method, path) {
            ("GET", "events") => {
                let (now, rx) = self.hub.subscribe();
                Reply::Events(now, rx)
            }
            ("POST", "call") => self.call(body),
            ("GET", "catalog") => answer(self.catalog()),
            ("POST", "advanced") => self.advanced(body),
            ("GET", "knobs") => self.method("nav.knobs", json!({})),
            ("POST", "knobs") => match bridge::vet_knobs(body) {
                Ok(params) => self.method("nav.knobs", params),
                Err(e) => refused(400, &e),
            },
            ("POST", "restart") if confirmed => self.method("nav.restart", json!({})),
            ("POST", "restart") => refused(403, "a restart stops what the duck is doing: send {\"confirmed\": true}"),
            (_, "events" | "call" | "catalog" | "advanced" | "knobs" | "restart") => refused(405, "not with this method"),
            _ => refused(404, "no such request"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Write};

    /// A nav socket that answers every line with `answer(request)`.
    fn fake_navd(answer: fn(&Value) -> Value) -> (tempfile::TempDir, String) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nav.sock").to_str().unwrap().to_owned();
        let listener = std::os::unix::net::UnixListener::bind(&path).unwrap();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let stream = stream.unwrap();
                let mut out = stream.try_clone().unwrap();
                for line in BufReader::new(stream).lines() {
                    let request: Value = serde_json::from_str(&line.unwrap()).unwrap();
                    writeln!(out, "{}", answer(&request)).unwrap();
                }
            }
        });
        (dir, path)
    }

    fn navd(request: &Value) -> Value {
        let id = request["id"].clone();
        match request["method"].as_str().unwrap() {
            "nav.catalog" => json!({"jsonrpc": "2.0", "id": id, "result": [
                {"name": "robot.remember_place", "parameters": {"properties": {"name": {}}}},
                {"name": "robot.map_wipe", "parameters": {}}]}),
            "nav.call" => json!({"jsonrpc": "2.0", "id": id, "result": {"echo": request["params"]}}),
            other => json!({"jsonrpc": "2.0", "id": id, "error": {"code": -32601, "message": format!("unknown method `{other}`")}}),
        }
    }

    fn adapter(socket: &str) -> QuackNav {
        let mut config = ServiceConfig::new("nav", "quack-nav");
        config.nav_socket = socket.into();
        config.map_socket = format!("{socket}.map");
        QuackNav::new(&config)
    }

    fn json_of(reply: Reply) -> (u16, Value) {
        match reply {
            Reply::Json(status, body) => (status, body),
            _ => panic!("not JSON"),
        }
    }

    #[test]
    fn the_map_view_s_calls_reach_quack_navd_and_others_do_not() {
        let (_dir, socket) = fake_navd(navd);
        let nav = adapter(&socket);
        let (status, body) = json_of(nav.route("POST", "call", &json!({"name": "robot.go_to", "args": {"x": 1, "y": 2}})));
        assert_eq!(status, 200, "{body}");
        assert_eq!(body["result"]["echo"], json!({"name": "robot.go_to", "args": {"x": 1.0, "y": 2.0}}));
        let (status, _) = json_of(nav.route("POST", "call", &json!({"name": "robot.map_wipe", "args": {}})));
        assert_eq!(status, 403);
        let (status, _) = json_of(nav.route("GET", "call", &Value::Null));
        assert_eq!(status, 405);
        let (status, _) = json_of(nav.route("GET", "nope", &Value::Null));
        assert_eq!(status, 404);
    }

    #[test]
    fn a_point_is_not_taught_by_a_quack_navd_that_would_teach_the_duck_s_spot() {
        let (_dir, socket) = fake_navd(navd);
        let nav = adapter(&socket);
        let (status, body) = json_of(nav.route("POST", "call", &json!({"name": "robot.remember_place", "args": {"name": "a", "x": 1, "y": 1}})));
        assert_eq!(status, 409, "{body}");
        let (status, _) = json_of(nav.route("POST", "call", &json!({"name": "robot.remember_place", "args": {"name": "a"}})));
        assert_eq!(status, 200);
    }

    #[test]
    fn the_advanced_door_asks_before_acting_and_an_old_daemon_says_so() {
        let (_dir, socket) = fake_navd(navd);
        let nav = adapter(&socket);
        let (status, _) = json_of(nav.route("POST", "advanced", &json!({"name": "robot.map_wipe"})));
        assert_eq!(status, 403);
        let (status, _) = json_of(nav.route("POST", "advanced", &json!({"name": "robot.map_wipe", "confirmed": true})));
        assert_eq!(status, 200);
        let (status, body) = json_of(nav.route("GET", "knobs", &Value::Null));
        assert_eq!(status, 501, "{body}");
        assert!(body["error"].as_str().unwrap().contains("update quack-nav"));
        let (status, _) = json_of(nav.route("POST", "restart", &json!({})));
        assert_eq!(status, 403);
    }

    #[test]
    fn a_silent_daemon_is_unreachable_and_unavailable() {
        let dir = tempfile::tempdir().unwrap();
        let nav = adapter(dir.path().join("none.sock").to_str().unwrap());
        let (status, body) = json_of(nav.route("POST", "call", &json!({"name": "robot.map_status"})));
        assert_eq!(status, 502);
        assert!(body["error"].as_str().unwrap().contains("unreachable"));
        assert!(!nav.health().available);
    }
}
