//! The HTTP side: the page, the token, and `/api/<service>/…` handed to
//! that service's adapter.
//!
//! | request | answer |
//! |---|---|
//! | `GET /` | the page (`src/page.html`, embedded) |
//! | `GET /api/info` | whether a token is needed — the one call that needs none |
//! | `GET /api/services` | each service: kind, available, why, features |
//! | `… /api/<service>/<path>` | the adapter's (see `src/adapters/`) |
//!
//! A POST must say `Content-Type: application/json`: a page on another
//! site can make a browser send a form here, but not JSON without asking
//! first, and nothing here says yes — so with a token or without, another
//! site cannot drive the duck through the browser of someone at home.

use std::io::{BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::Duration;

use serde_json::{Value, json};

use crate::adapters::quack_nav::hub::Item;
use crate::adapters::{Adapter, Reply};
use crate::http::{self, Request};

const PAGE: &str = include_str!("page.html");
/// Connections at once — a household's phones and laptops, and their
/// event streams, with room to spare.
const MAX_CONNECTIONS: usize = 48;
/// A quiet event stream gets a comment this often, so a phone or a proxy
/// does not decide it is dead.
const KEEPALIVE: Duration = Duration::from_secs(15);

pub struct Server {
    pub token: Option<String>,
    pub adapters: Vec<Arc<dyn Adapter>>,
}

impl Server {
    pub fn run(self: Arc<Self>, bind: &str) -> anyhow::Result<()> {
        let listener = TcpListener::bind(bind).map_err(|e| anyhow::anyhow!("cannot listen on {bind}: {e} (set `bind` in the config)"))?;
        tracing::info!(bind, token = self.token.is_some(), "serving the page");
        let open = Arc::new(AtomicUsize::new(0));
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            if open.load(Ordering::SeqCst) >= MAX_CONNECTIONS {
                let _ = http::respond_json(&mut stream, 503, &json!({"ok": false, "error": "too many connections"}));
                continue;
            }
            open.fetch_add(1, Ordering::SeqCst);
            let (server, open) = (self.clone(), open.clone());
            std::thread::spawn(move || {
                server.connection(stream);
                open.fetch_sub(1, Ordering::SeqCst);
            });
        }
        Ok(())
    }

    fn connection(&self, stream: TcpStream) {
        let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
        let _ = stream.set_write_timeout(Some(Duration::from_secs(20)));
        let _ = stream.set_nodelay(true);
        let Ok(mut out) = stream.try_clone() else { return };
        let request = match http::read_request(&mut BufReader::new(stream)) {
            Ok(request) => request,
            Err(Some(bad)) => {
                let _ = http::respond_json(&mut out, bad.0, &json!({"ok": false, "error": bad.1}));
                return;
            }
            Err(None) => return,
        };
        let _ = match self.handle(&request) {
            Reply::Json(status, body) => http::respond_json(&mut out, status, &body),
            Reply::Html(page) => http::respond(&mut out, 200, "text/html; charset=utf-8", page.as_bytes()),
            Reply::Bytes { status, content_type, headers, body } => http::respond_with(&mut out, status, content_type, &headers, &body),
            Reply::Events(now, rx) => http::start_events(&mut out).and_then(|()| stream_events(&mut out, now, rx)),
        };
    }

    /// One request, answered — everything but the writing.
    pub fn handle(&self, request: &Request) -> Reply {
        let path = request.path.as_str();
        if path == "/" || path == "/index.html" {
            return match request.method.as_str() {
                "GET" | "HEAD" => Reply::Html(PAGE),
                _ => Reply::Json(405, json!({"ok": false, "error": "GET the page"})),
            };
        }
        let Some(api) = path.strip_prefix("/api/") else {
            return Reply::Json(404, json!({"ok": false, "error": "no such page"}));
        };
        if api == "info" {
            return Reply::Json(200, json!({"ok": true, "token_required": self.token.is_some(), "version": env!("CARGO_PKG_VERSION")}));
        }
        if let Some(token) = &self.token
            && !request.token().is_some_and(|given| http::same_secret(given, token))
        {
            return Reply::Json(401, json!({"ok": false, "error": "this page needs the token (the `token` in the config, or QC_TOKEN)"}));
        }
        let body = match request.method.as_str() {
            "POST" => {
                let json_type = request.header("content-type").is_some_and(|t| t.trim().starts_with("application/json"));
                if !json_type {
                    return Reply::Json(415, json!({"ok": false, "error": "send JSON, with Content-Type: application/json"}));
                }
                if request.body.is_empty() {
                    Value::Null
                } else {
                    match serde_json::from_slice(&request.body) {
                        Ok(body) => body,
                        Err(e) => return Reply::Json(400, json!({"ok": false, "error": format!("the body is not JSON: {e}")})),
                    }
                }
            }
            "GET" => Value::Null,
            _ => return Reply::Json(405, json!({"ok": false, "error": "GET or POST"})),
        };
        if api == "services" {
            let listed: Vec<Value> = self
                .adapters
                .iter()
                .map(|a| {
                    let health = a.health();
                    json!({"name": a.name(), "kind": a.kind(), "available": health.available, "detail": health.detail, "features": a.features()})
                })
                .collect();
            return Reply::Json(200, json!({"ok": true, "services": listed}));
        }
        let (service, rest) = api.split_once('/').unwrap_or((api, ""));
        match self.adapters.iter().find(|a| a.name() == service) {
            Some(adapter) => adapter.route(&request.method, rest, &body),
            None => Reply::Json(404, json!({"ok": false, "error": format!("no service `{service}`")})),
        }
    }
}

/// Write what is known, then every event, until the page goes away. A
/// frame's grid goes only when it differs from the one this page has.
fn stream_events(out: &mut impl Write, now: Vec<Item>, rx: Receiver<Item>) -> std::io::Result<()> {
    let mut cells_sent: Option<u64> = None;
    let mut send = |out: &mut dyn Write, item: Item| -> std::io::Result<()> {
        match item {
            Item::Text(text) => out.write_all(text.as_bytes())?,
            Item::Frame(frame) => {
                let text = if cells_sent == Some(frame.cells_id) { &frame.light } else { &frame.full };
                out.write_all(text.as_bytes())?;
                cells_sent = Some(frame.cells_id);
            }
        }
        out.flush()
    };
    for item in now {
        send(out, item)?;
    }
    loop {
        match rx.recv_timeout(KEEPALIVE) {
            Ok(item) => send(out, item)?,
            Err(RecvTimeoutError::Timeout) => {
                out.write_all(b": keepalive\n\n")?;
                out.flush()?;
            }
            Err(RecvTimeoutError::Disconnected) => return Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::Health;

    struct Fake;

    impl Adapter for Fake {
        fn name(&self) -> &str {
            "nav"
        }
        fn kind(&self) -> &'static str {
            "fake"
        }
        fn start(&self) {}
        fn health(&self) -> Health {
            Health { available: true, detail: "fine".into() }
        }
        fn features(&self) -> Vec<&'static str> {
            vec!["map"]
        }
        fn route(&self, method: &str, path: &str, body: &Value) -> Reply {
            Reply::Json(200, json!({"method": method, "path": path, "body": body}))
        }
    }

    fn server(token: Option<&str>) -> Server {
        Server { token: token.map(str::to_owned), adapters: vec![Arc::new(Fake)] }
    }

    fn request(raw: &str) -> Request {
        http::read_request(&mut BufReader::new(raw.as_bytes())).unwrap()
    }

    fn status(reply: Reply) -> (u16, Value) {
        match reply {
            Reply::Json(s, v) => (s, v),
            Reply::Html(_) => (200, Value::Null),
            Reply::Events(..) => (200, json!("events")),
            Reply::Bytes { status, content_type, .. } => (status, json!(content_type)),
        }
    }

    #[test]
    fn the_token_guards_everything_but_the_page_and_the_info() {
        let s = server(Some("s3cret"));
        assert_eq!(status(s.handle(&request("GET / HTTP/1.1\r\n\r\n"))).0, 200);
        let (code, info) = status(s.handle(&request("GET /api/info HTTP/1.1\r\n\r\n")));
        assert_eq!((code, info["token_required"].clone()), (200, json!(true)));
        assert_eq!(status(s.handle(&request("GET /api/services HTTP/1.1\r\n\r\n"))).0, 401);
        assert_eq!(status(s.handle(&request("GET /api/services?token=wrong HTTP/1.1\r\n\r\n"))).0, 401);
        assert_eq!(status(s.handle(&request("GET /api/services?token=s3cret HTTP/1.1\r\n\r\n"))).0, 200);
        assert_eq!(status(s.handle(&request("GET /api/nav/events HTTP/1.1\r\nX-Token: s3cret\r\n\r\n"))).0, 200);
        // Without a token configured, nothing is asked.
        assert_eq!(status(server(None).handle(&request("GET /api/services HTTP/1.1\r\n\r\n"))).0, 200);
    }

    #[test]
    fn a_post_is_json_or_it_is_refused() {
        let s = server(None);
        let form = "POST /api/nav/call HTTP/1.1\r\nContent-Type: application/x-www-form-urlencoded\r\nContent-Length: 3\r\n\r\na=1";
        assert_eq!(status(s.handle(&request(form))).0, 415);
        let bad = "POST /api/nav/call HTTP/1.1\r\nContent-Type: application/json\r\nContent-Length: 3\r\n\r\n{x}";
        assert_eq!(status(s.handle(&request(bad))).0, 400);
        let good = "POST /api/nav/call HTTP/1.1\r\nContent-Type: application/json\r\nContent-Length: 8\r\n\r\n{\"a\": 1}";
        let (code, body) = status(s.handle(&request(good)));
        assert_eq!(code, 200);
        assert_eq!(body, json!({"method": "POST", "path": "call", "body": {"a": 1}}));
        assert_eq!(status(s.handle(&request("DELETE /api/nav/call HTTP/1.1\r\n\r\n"))).0, 405);
    }

    #[test]
    fn requests_go_to_the_service_they_name() {
        let s = server(None);
        let (code, services) = status(s.handle(&request("GET /api/services HTTP/1.1\r\n\r\n")));
        assert_eq!(code, 200);
        assert_eq!(services["services"][0], json!({"name": "nav", "kind": "fake", "available": true, "detail": "fine", "features": ["map"]}));
        assert_eq!(status(s.handle(&request("GET /api/other/events HTTP/1.1\r\n\r\n"))).0, 404);
        assert_eq!(status(s.handle(&request("GET /elsewhere HTTP/1.1\r\n\r\n"))).0, 404);
        assert_eq!(status(s.handle(&request("GET /api/nav/a/b HTTP/1.1\r\n\r\n"))).1["path"], "a/b");
    }

    #[test]
    fn the_camera_s_snapshot_is_a_jpeg_behind_the_token() {
        use crate::adapters::mediad::tests::{adapter, fake_mediad};
        let dir = tempfile::tempdir().unwrap();
        let (socket, _) = fake_mediad(dir.path(), 64, 32, 90);
        let s = Server { token: Some("s3cret".into()), adapters: vec![Arc::new(adapter(&socket))] };
        assert_eq!(status(s.handle(&request("GET /api/camera/snapshot HTTP/1.1\r\n\r\n"))).0, 401);
        let (code, kind) = status(s.handle(&request("GET /api/camera/snapshot?token=s3cret HTTP/1.1\r\n\r\n")));
        assert_eq!((code, kind), (200, json!("image/jpeg")));
        let (code, kind) = status(s.handle(&request("GET /api/camera/snapshot HTTP/1.1\r\nX-Token: s3cret\r\n\r\n")));
        assert_eq!((code, kind), (200, json!("image/jpeg")));
        let (_, services) = status(s.handle(&request("GET /api/services?token=s3cret HTTP/1.1\r\n\r\n")));
        assert_eq!(services["services"][0]["features"], json!(["camera"]));
        assert_eq!(services["services"][0]["available"], json!(true));
    }

    #[test]
    fn a_page_gets_a_frame_s_grid_once() {
        use crate::adapters::quack_nav::hub::Frame;
        let frame = |id| Item::Frame(Arc::new(Frame { cells_id: id, full: format!("F{id}|"), light: format!("L{id}|") }));
        let (tx, rx) = std::sync::mpsc::sync_channel(8);
        for id in [1, 1, 2, 2, 1] {
            tx.send(frame(id)).unwrap();
        }
        drop(tx);
        let mut out = Vec::new();
        stream_events(&mut out, vec![Item::Text(Arc::new("x|".into()))], rx).unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), "x|F1|L1|F2|L2|F1|");
    }
}
