//! The page's calls, checked, then handed to quack-navd's nav socket.
//!
//! Two doors. The map view's (`/call`, [`vet`]) reaches a few tools and
//! only in the shapes it uses: the three reads, `go_to` a point or a place
//! or stop, explore start / stop / complete, teach and forget a place —
//! `map_wipe`, `explore fresh`, `map_load`, `robot.move` answer 403 there,
//! whatever the body says. The advanced view's (`/advanced`,
//! [`vet_advanced`]) reaches every tool `nav.catalog` lists, with the
//! arguments as given, and a call that is not a read only with
//! `"confirmed": true` in the request — the page asks the person first.
//! The knobs and the restart have their own shapes ([`vet_knobs`]).

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::time::Duration;

use serde_json::{Map, Value, json};

/// A place's name, as the registry and the page take it.
const MAX_NAME: usize = 64;

/// Why a call did not come back with a result.
#[derive(Debug, Clone, PartialEq)]
pub enum CallError {
    /// The socket is not there, or did not answer: quack-navd is down,
    /// starting, or busy past the timeout.
    Unreachable(String),
    /// quack-navd answered with a JSON-RPC error: the tool refused.
    Refused(String),
}

impl CallError {
    pub fn text(&self) -> String {
        match self {
            CallError::Unreachable(e) => format!("quack-navd is unreachable: {e}"),
            CallError::Refused(e) => e.clone(),
        }
    }
}

/// One `nav.call` on a lane of its own, NDJSON as quack-navd speaks it.
/// `timeout` bounds the wait for the answer: a call queues behind any other
/// holding the robot (`explore complete` waits up to two minutes for the
/// running session to save).
pub fn nav_call(socket: &str, name: &str, args: &Value, timeout: Duration) -> Result<Value, CallError> {
    let request = json!({"jsonrpc": "2.0", "id": 1, "method": "nav.call", "params": {"name": name, "args": args}});
    rpc(socket, &request, timeout)
}

/// One JSON-RPC request and its answer's `result`.
pub fn rpc(socket: &str, request: &Value, timeout: Duration) -> Result<Value, CallError> {
    let unreachable = |e: std::io::Error| CallError::Unreachable(format!("{socket}: {e}"));
    let mut stream = UnixStream::connect(socket).map_err(unreachable)?;
    stream.set_read_timeout(Some(timeout)).map_err(unreachable)?;
    stream.set_write_timeout(Some(Duration::from_secs(5))).map_err(unreachable)?;
    let mut line = request.to_string();
    line.push('\n');
    stream.write_all(line.as_bytes()).map_err(unreachable)?;
    let mut answer = String::new();
    BufReader::new(stream).read_line(&mut answer).map_err(unreachable)?;
    if answer.is_empty() {
        return Err(CallError::Unreachable(format!("{socket}: closed without an answer")));
    }
    answer_of(&answer)
}

/// The `result` of a JSON-RPC answer, or its error's message.
pub fn answer_of(line: &str) -> Result<Value, CallError> {
    let answer: Value = serde_json::from_str(line).map_err(|e| CallError::Unreachable(format!("not JSON-RPC: {e}")))?;
    if let Some(error) = answer.get("error") {
        let message = error.get("message").and_then(Value::as_str).unwrap_or("refused without a reason");
        return Err(CallError::Refused(message.to_owned()));
    }
    Ok(answer.get("result").cloned().unwrap_or(Value::Null))
}

/// Check one call from the page and give back the arguments to forward —
/// only the keys this list names, so nothing else rides along. `Err` is
/// the reason it is refused (HTTP 403).
pub fn vet(name: &str, args: &Value) -> Result<Value, String> {
    let empty = Map::new();
    let args = match args {
        Value::Object(map) => map,
        Value::Null => &empty,
        _ => return Err("the arguments are a JSON object".into()),
    };
    let only = |keys: &[&str]| -> Result<(), String> {
        match args.keys().find(|k| !keys.contains(&k.as_str())) {
            Some(k) => Err(format!("`{name}` from the page takes no `{k}`")),
            None => Ok(()),
        }
    };
    let flag = |key: &str| args.get(key).and_then(Value::as_bool) == Some(true);
    match name {
        "robot.map_status" | "robot.list_places" | "robot.where_am_i" => {
            only(&[])?;
            Ok(json!({}))
        }
        "robot.go_to" => {
            if args.contains_key("stop") {
                only(&["stop"])?;
                return if flag("stop") { Ok(json!({"stop": true})) } else { Err("`stop` is true or absent".into()) };
            }
            if args.contains_key("place") {
                only(&["place"])?;
                return Ok(json!({"place": place_name(args.get("place"))?}));
            }
            only(&["x", "y"])?;
            let (x, y) = point(args)?.ok_or("give `place`, or `x` and `y` in map metres")?;
            Ok(json!({"x": x, "y": y}))
        }
        "robot.map_explore" => {
            only(&["stop", "complete"])?;
            match (args.get("stop"), args.get("complete")) {
                (None, None) => Ok(json!({})),
                (Some(_), None) if flag("stop") => Ok(json!({"stop": true})),
                (None, Some(_)) if flag("complete") => Ok(json!({"complete": true})),
                _ => Err("explore from the page: start ({}), {\"stop\": true} or {\"complete\": true}".into()),
            }
        }
        "robot.remember_place" => {
            only(&["name", "x", "y"])?;
            let name = place_name(args.get("name"))?;
            Ok(match point(args)? {
                Some((x, y)) => json!({"name": name, "x": x, "y": y}),
                None => json!({"name": name}),
            })
        }
        "robot.forget_place" => {
            only(&["name"])?;
            Ok(json!({"name": place_name(args.get("name"))?}))
        }
        other => Err(format!("`{other}` is not reachable from the page")),
    }
}

/// The tools that only read: the advanced view calls them without asking.
pub const READS: [&str; 5] = ["robot.map_status", "robot.list_places", "robot.where_am_i", "robot.map_list", "robot.map_match"];

/// Check one call from the advanced view against the tools `nav.catalog`
/// lists now. `Err` is the reason it is refused (HTTP 403).
pub fn vet_advanced(name: &str, args: &Value, catalog: &[String], confirmed: bool) -> Result<Value, String> {
    if !catalog.iter().any(|t| t == name) {
        return Err(format!("`{name}` is not in quack-navd's catalog"));
    }
    let args = match args {
        Value::Object(_) => args.clone(),
        Value::Null => json!({}),
        _ => return Err("the arguments are a JSON object".into()),
    };
    if !READS.contains(&name) && !confirmed {
        return Err(format!("`{name}` acts on the duck: send it with \"confirmed\": true once the person has said yes"));
    }
    Ok(args)
}

/// The names in a `nav.catalog` answer.
pub fn catalog_names(catalog: &Value) -> Vec<String> {
    catalog
        .as_array()
        .map(|tools| tools.iter().filter_map(|t| t.get("name").and_then(Value::as_str).map(str::to_owned)).collect())
        .unwrap_or_default()
}

/// `nav.knobs`'s params from the page: `{}` (list), `{"set": {...}}` or
/// `{"reset_all": true}`. The values are quack-navd's to check: it has the
/// list and the types.
pub fn vet_knobs(body: &Value) -> Result<Value, String> {
    let empty = Map::new();
    let map = match body {
        Value::Object(map) => map,
        Value::Null => &empty,
        _ => return Err("a JSON object".into()),
    };
    match (map.get("set"), map.get("reset_all")) {
        (None, None) if map.is_empty() => Ok(json!({})),
        (Some(Value::Object(set)), None) if map.len() == 1 => Ok(json!({"set": set})),
        (None, Some(Value::Bool(true))) if map.len() == 1 => Ok(json!({"reset_all": true})),
        _ => Err("knobs: {}, {\"set\": {\"NAME\": value or null}} or {\"reset_all\": true}".into()),
    }
}

fn place_name(value: Option<&Value>) -> Result<String, String> {
    let name = value.and_then(Value::as_str).map(str::trim).unwrap_or_default();
    if name.is_empty() || name.chars().count() > MAX_NAME || name.chars().any(char::is_control) {
        return Err(format!("a place's name is 1 to {MAX_NAME} characters"));
    }
    Ok(name.to_owned())
}

/// `x` and `y` together, finite and within a house's reach, or neither.
fn point(args: &Map<String, Value>) -> Result<Option<(f64, f64)>, String> {
    match (args.get("x"), args.get("y")) {
        (None, None) => Ok(None),
        (Some(x), Some(y)) => match (x.as_f64(), y.as_f64()) {
            (Some(x), Some(y)) if x.is_finite() && y.is_finite() && x.abs() < 1e4 && y.abs() < 1e4 => Ok(Some((x, y))),
            _ => Err("`x` and `y` are numbers in map metres".into()),
        },
        _ => Err("`x` and `y` go together".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_reads_pass_and_take_nothing() {
        for name in ["robot.map_status", "robot.list_places", "robot.where_am_i"] {
            assert_eq!(vet(name, &json!({})), Ok(json!({})));
            assert_eq!(vet(name, &Value::Null), Ok(json!({})));
            assert!(vet(name, &json!({"x": 1})).is_err(), "{name}");
        }
        assert!(vet("robot.map_status", &json!([1])).is_err());
    }

    #[test]
    fn only_the_page_s_calls_are_reachable() {
        for name in [
            "robot.map_wipe",
            "robot.map_load",
            "robot.map_save",
            "robot.map_adopt",
            "robot.map_match",
            "robot.map_list",
            "robot.map_step",
            "robot.move",
            "nav.take_question",
            "nav.catalog",
            "",
        ] {
            let e = vet(name, &json!({})).unwrap_err();
            assert!(e.contains("not reachable"), "{name}: {e}");
        }
    }

    #[test]
    fn go_to_is_a_point_a_place_or_stop_and_nothing_else() {
        assert_eq!(vet("robot.go_to", &json!({"x": 1.5, "y": -2})), Ok(json!({"x": 1.5, "y": -2.0})));
        assert_eq!(vet("robot.go_to", &json!({"place": " kitchen "})), Ok(json!({"place": "kitchen"})));
        assert_eq!(vet("robot.go_to", &json!({"stop": true})), Ok(json!({"stop": true})));
        for bad in [
            json!({}),
            json!({"x": 1}),
            json!({"x": "1", "y": 2}),
            json!({"x": 1, "y": 2, "max_s": 1800}),
            json!({"place": "kitchen", "x": 1, "y": 2}),
            json!({"place": ""}),
            json!({"place": "x".repeat(65)}),
            json!({"stop": false}),
            json!({"stop": true, "x": 1, "y": 1}),
            json!({"x": 1e9, "y": 0}),
        ] {
            assert!(vet("robot.go_to", &bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn explore_starts_stops_and_completes_but_never_starts_afresh() {
        assert_eq!(vet("robot.map_explore", &json!({})), Ok(json!({})));
        assert_eq!(vet("robot.map_explore", &json!({"stop": true})), Ok(json!({"stop": true})));
        assert_eq!(vet("robot.map_explore", &json!({"complete": true})), Ok(json!({"complete": true})));
        for bad in [
            json!({"fresh": true, "confirmed": true}),
            json!({"save_as": "other"}),
            json!({"watch": true}),
            json!({"max_s": 3600}),
            json!({"stop": true, "complete": true}),
            json!({"stop": false}),
        ] {
            assert!(vet("robot.map_explore", &bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn places_are_taught_here_or_at_a_point_and_forgotten_by_name() {
        assert_eq!(vet("robot.remember_place", &json!({"name": "cucina"})), Ok(json!({"name": "cucina"})));
        assert_eq!(
            vet("robot.remember_place", &json!({"name": "divano", "x": 1, "y": 2})),
            Ok(json!({"name": "divano", "x": 1.0, "y": 2.0}))
        );
        assert!(vet("robot.remember_place", &json!({"name": "a", "radius_m": 9})).is_err());
        assert!(vet("robot.remember_place", &json!({"name": "a", "x": 1})).is_err());
        assert!(vet("robot.remember_place", &json!({"name": "a\nb"})).is_err());
        assert_eq!(vet("robot.forget_place", &json!({"name": "cucina"})), Ok(json!({"name": "cucina"})));
        assert!(vet("robot.forget_place", &json!({})).is_err());
    }

    #[test]
    fn the_advanced_door_takes_any_catalog_tool_and_asks_before_acting() {
        let catalog = catalog_names(&json!([{"name": "robot.map_status"}, {"name": "robot.map_wipe"}, {"name": "robot.go_to"}]));
        assert_eq!(catalog.len(), 3);
        assert_eq!(vet_advanced("robot.map_status", &Value::Null, &catalog, false), Ok(json!({})));
        let e = vet_advanced("robot.map_wipe", &json!({}), &catalog, false).unwrap_err();
        assert!(e.contains("confirmed"), "{e}");
        assert_eq!(vet_advanced("robot.map_wipe", &json!({}), &catalog, true), Ok(json!({})));
        // Arguments go as given: the catalog's schema is quack-navd's to enforce.
        let args = json!({"x": 1, "y": 2, "max_s": 600});
        assert_eq!(vet_advanced("robot.go_to", &args, &catalog, true), Ok(args));
        // Not in the catalog: not reachable, confirmed or not.
        for name in ["robot.move", "nav.take_question", "nav.restart", "nav.knobs"] {
            assert!(vet_advanced(name, &json!({}), &catalog, true).unwrap_err().contains("not in"), "{name}");
        }
        assert!(vet_advanced("robot.go_to", &json!([1]), &catalog, true).is_err());
    }

    #[test]
    fn knobs_are_listed_set_or_reset_and_nothing_else() {
        assert_eq!(vet_knobs(&json!({})), Ok(json!({})));
        assert_eq!(vet_knobs(&json!({"set": {"QK_A": "1", "QK_B": null}})), Ok(json!({"set": {"QK_A": "1", "QK_B": null}})));
        assert_eq!(vet_knobs(&json!({"reset_all": true})), Ok(json!({"reset_all": true})));
        for bad in [json!({"set": "x"}), json!({"reset_all": false}), json!({"set": {}, "reset_all": true}), json!({"path": "/etc"}), json!(1)] {
            assert!(vet_knobs(&bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn an_answer_is_its_result_or_its_error_s_message() {
        assert_eq!(answer_of(r#"{"jsonrpc":"2.0","id":1,"result":{"a":1}}"#), Ok(json!({"a": 1})));
        assert_eq!(
            answer_of(r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32000,"message":"no map yet"}}"#),
            Err(CallError::Refused("no map yet".into()))
        );
        assert!(matches!(answer_of("garbage"), Err(CallError::Unreachable(_))));
    }

    #[test]
    fn a_missing_socket_is_unreachable_not_refused() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nav.sock");
        let e = nav_call(path.to_str().unwrap(), "robot.map_status", &json!({}), Duration::from_secs(1)).unwrap_err();
        assert!(matches!(e, CallError::Unreachable(_)), "{e:?}");
        assert!(e.text().starts_with("quack-navd is unreachable"));
    }

    #[test]
    fn a_call_reaches_the_socket_as_one_nav_call_line() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nav.sock");
        let listener = std::os::unix::net::UnixListener::bind(&path).unwrap();
        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut line = String::new();
            BufReader::new(stream.try_clone().unwrap()).read_line(&mut line).unwrap();
            let mut out = stream;
            writeln!(out, r#"{{"jsonrpc":"2.0","id":1,"result":{{"ok":true}}}}"#).unwrap();
            serde_json::from_str::<Value>(&line).unwrap()
        });
        let got = nav_call(path.to_str().unwrap(), "robot.go_to", &json!({"stop": true}), Duration::from_secs(2));
        assert_eq!(got, Ok(json!({"ok": true})));
        let sent = server.join().unwrap();
        assert_eq!(sent["method"], "nav.call");
        assert_eq!(sent["params"], json!({"name": "robot.go_to", "args": {"stop": true}}));
    }
}
