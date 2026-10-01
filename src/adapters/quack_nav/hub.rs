//! What the page shows, gathered once and fanned out to every browser.
//!
//! Two lanes in, both read-only: the map socket's `robot.map` stream
//! (`map.frame` a second, `map.pose` between frames) and a poll of
//! `robot.map_status` and `robot.list_places` on the nav socket. Out, a
//! server-sent event per item to each page that listens. The grid travels
//! only when it changed since that page last got it; the trail — the path
//! walked, in map coordinates — is kept here, so a page opened late sees
//! it too.

use std::collections::VecDeque;
use std::hash::{Hash, Hasher};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use super::bridge::{CallError, nav_call};
use crate::http::event;

/// A pose further than this from the trail's last point extends it.
const TRAIL_STEP_M: f64 = 0.05;
const TRAIL_MAX: usize = 4000;
/// The trail sent with a frame: every n-th point, at most this many.
const TRAIL_SENT: usize = 1500;
/// A frame whose submap count jumps by this many is another map (loaded,
/// adopted, wiped): the trail drawn on the old one means nothing on it
/// (the twin viewer's `SWAP_SUBMAPS`).
const SWAP_SUBMAPS: i64 = 20;
/// Poses between frames come at 20 Hz; a phone needs half of them.
const POSE_EVERY: Duration = Duration::from_millis(90);
const STATUS_EVERY: Duration = Duration::from_secs(1);
const PLACES_EVERY: Duration = Duration::from_secs(5);
const IDLE_EVERY: Duration = Duration::from_secs(15);
/// How long a status read may wait for the robot's lock.
const STATUS_TIMEOUT: Duration = Duration::from_secs(10);
/// Events a slow page may fall behind by before it misses some.
const QUEUE: usize = 64;

/// A map frame, ready to send: with its grid, and without it for a page
/// that already has this grid (`cells_id`).
pub struct Frame {
    pub cells_id: u64,
    pub full: String,
    pub light: String,
}

#[derive(Clone)]
pub enum Item {
    Frame(Arc<Frame>),
    /// A whole event, `event: …\ndata: …\n\n`.
    Text(Arc<String>),
}

#[derive(Default)]
struct Latest {
    frame: Option<Arc<Frame>>,
    status: Option<Arc<String>>,
    places: Option<Arc<String>>,
    link: Option<Arc<String>>,
}

#[derive(Default)]
struct Links {
    map: String,
    nav: String,
}

/// The fan-out: the newest of each item, and every page listening.
#[derive(Clone, Default)]
pub struct Hub {
    pages: Arc<Mutex<Vec<SyncSender<Item>>>>,
    latest: Arc<Mutex<Latest>>,
    links: Arc<Mutex<Links>>,
}

impl Hub {
    /// A new listener: what is known now, then everything that follows.
    pub fn subscribe(&self) -> (Vec<Item>, Receiver<Item>) {
        let (tx, rx) = mpsc::sync_channel(QUEUE);
        // Added before the snapshot is taken: nothing falls between.
        self.pages.lock().expect("hub poisoned").push(tx);
        let l = self.latest.lock().expect("hub poisoned");
        let now = [l.link.clone().map(Item::Text), l.frame.clone().map(Item::Frame), l.status.clone().map(Item::Text), l.places.clone().map(Item::Text)];
        (now.into_iter().flatten().collect(), rx)
    }

    pub fn listeners(&self) -> usize {
        self.pages.lock().expect("hub poisoned").len()
    }

    /// How the two lanes are doing: (map, nav), "" before the first try.
    pub fn links(&self) -> (String, String) {
        let links = self.links.lock().expect("hub poisoned");
        (links.map.clone(), links.nav.clone())
    }

    fn publish(&self, item: Item) {
        self.pages.lock().expect("hub poisoned").retain(|page| match page.try_send(item.clone()) {
            Ok(()) | Err(TrySendError::Full(_)) => true,
            Err(TrySendError::Disconnected(_)) => false,
        });
    }

    fn publish_text(&self, name: &str, data: &Value, keep: impl FnOnce(&mut Latest) -> &mut Option<Arc<String>>) {
        let text = Arc::new(event(name, &data.to_string()));
        *keep(&mut self.latest.lock().expect("hub poisoned")) = Some(text.clone());
        self.publish(Item::Text(text));
    }

    /// The state of both lanes, sent when either changes.
    fn link(&self, map: Option<String>, nav: Option<String>) {
        let data = {
            let mut links = self.links.lock().expect("hub poisoned");
            let before = (links.map.clone(), links.nav.clone());
            if let Some(map) = map {
                links.map = map;
            }
            if let Some(nav) = nav {
                links.nav = nav;
            }
            if before == (links.map.clone(), links.nav.clone()) {
                return;
            }
            json!({"map": links.map, "nav": links.nav})
        };
        self.publish_text("link", &data, |l| &mut l.link);
    }

    /// Read the map socket's stream forever, reconnecting.
    pub fn run_map(&self, socket: String) {
        let mut trail = Trail::default();
        let mut backoff = Duration::from_millis(500);
        loop {
            match self.map_stream(&socket, &mut trail) {
                Ok(()) => self.link(Some("the map stream ended; reconnecting".into()), None),
                Err(e) => self.link(Some(format!("no map: {e}")), None),
            }
            std::thread::sleep(backoff);
            backoff = (backoff * 2).min(Duration::from_secs(5));
        }
    }

    fn map_stream(&self, socket: &str, trail: &mut Trail) -> std::io::Result<()> {
        let stream = UnixStream::connect(socket).map_err(|e| std::io::Error::new(e.kind(), format!("{socket}: {e}")))?;
        stream.set_read_timeout(Some(Duration::from_secs(15)))?;
        let mut out = stream.try_clone()?;
        writeln!(out, r#"{{"jsonrpc":"2.0","id":1,"method":"robot.map","params":{{}}}}"#)?;
        let mut last_pose = Instant::now() - POSE_EVERY;
        for line in BufReader::new(stream).lines() {
            let line = line?;
            let Ok(message) = serde_json::from_str::<Value>(&line) else { continue };
            match message.get("method").and_then(Value::as_str) {
                Some("map.frame") => {
                    let Some(p) = message.get("params") else { continue };
                    trail.frame(p);
                    let frame = Arc::new(frame_item(p, trail));
                    self.latest.lock().expect("hub poisoned").frame = Some(frame.clone());
                    self.publish(Item::Frame(frame));
                    self.link(Some("streaming".into()), None);
                }
                Some("map.pose") => {
                    let Some(p) = message.get("params") else { continue };
                    trail.pose(p);
                    if last_pose.elapsed() >= POSE_EVERY {
                        last_pose = Instant::now();
                        self.publish(Item::Text(Arc::new(event("pose", &p.to_string()))));
                    }
                }
                _ => {
                    // The subscription's own answer.
                    if let Some(result) = message.get("result") {
                        let state = if result.get("enabled").and_then(Value::as_bool) == Some(false) {
                            "mapping is disabled on this duck".to_owned()
                        } else {
                            "subscribed; waiting for a frame".to_owned()
                        };
                        self.link(Some(state), None);
                    } else if let Some(error) = message.get("error") {
                        let why = error.get("message").and_then(Value::as_str).unwrap_or("refused");
                        return Err(std::io::Error::other(format!("robot.map refused: {why}")));
                    }
                }
            }
        }
        Ok(())
    }

    /// Ask quack-navd how the map and the jobs are doing, forever.
    /// Once a second while a page listens; with nobody watching, once in
    /// `IDLE_EVERY`, enough for the services view to say whether it answers.
    pub fn run_status(&self, socket: String) {
        let mut places_at: Option<Instant> = None;
        let mut asked: Option<Instant> = None;
        loop {
            let started = Instant::now();
            if self.listeners() == 0 && asked.is_some_and(|t| t.elapsed() < IDLE_EVERY) {
                std::thread::sleep(STATUS_EVERY);
                continue;
            }
            asked = Some(started);
            match nav_call(&socket, "robot.map_status", &json!({}), STATUS_TIMEOUT) {
                Ok(status) => {
                    self.publish_text("status", &json!({"ok": true, "status": status}), |l| &mut l.status);
                    self.link(None, Some("ok".into()));
                }
                Err(CallError::Refused(e)) => {
                    self.publish_text("status", &json!({"ok": false, "error": e}), |l| &mut l.status);
                    self.link(None, Some("ok".into()));
                }
                Err(e) => self.link(None, Some(e.text())),
            }
            if places_at.is_none_or(|t| t.elapsed() >= PLACES_EVERY) {
                self.refresh_places(&socket);
                places_at = Some(Instant::now());
            }
            std::thread::sleep(STATUS_EVERY.saturating_sub(started.elapsed()));
        }
    }

    /// The places, now — after a teach or a forget, the page should not
    /// wait five seconds to see it.
    pub fn refresh_places(&self, socket: &str) {
        match nav_call(socket, "robot.list_places", &json!({}), STATUS_TIMEOUT) {
            Ok(places) => self.publish_text("places", &json!({"ok": true, "places": places}), |l| &mut l.places),
            Err(e) => self.publish_text("places", &json!({"ok": false, "error": e.text()}), |l| &mut l.places),
        }
    }
}

/// The path walked, in map coordinates, from the trusted poses.
#[derive(Default)]
struct Trail {
    points: VecDeque<(f64, f64)>,
    submaps: Option<i64>,
}

impl Trail {
    fn frame(&mut self, p: &Value) {
        let submaps = p.get("n_submaps").and_then(Value::as_i64).unwrap_or(0);
        if self.submaps.is_some_and(|before| (submaps - before).abs() >= SWAP_SUBMAPS) {
            self.points.clear();
        }
        self.submaps = Some(submaps);
        self.pose(p);
    }

    fn pose(&mut self, p: &Value) {
        let tracking = p.get("tracking").and_then(Value::as_bool).unwrap_or(false);
        let seated = p.get("seated").and_then(Value::as_bool).unwrap_or(false);
        let (Some(x), Some(y)) = (p.get("x").and_then(Value::as_f64), p.get("y").and_then(Value::as_f64)) else {
            return;
        };
        if !tracking || seated {
            return;
        }
        if self.points.back().is_none_or(|&(lx, ly)| (x - lx).hypot(y - ly) >= TRAIL_STEP_M) {
            self.points.push_back((x, y));
            if self.points.len() > TRAIL_MAX {
                self.points.pop_front();
            }
        }
    }

    fn sent(&self) -> Value {
        let every = self.points.len().div_ceil(TRAIL_SENT).max(1);
        let mut out: Vec<Value> = self.points.iter().step_by(every).map(|&(x, y)| json!([round2(x), round2(y)])).collect();
        // The newest point always, so the trail reaches the duck.
        if let Some(&(x, y)) = self.points.back()
            && (self.points.len() - 1) % every != 0
        {
            out.push(json!([round2(x), round2(y)]));
        }
        Value::Array(out)
    }
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

/// A `map.frame`'s params as the page takes them: everything the daemon
/// sent, the trail, and the grid's identity.
fn frame_item(p: &Value, trail: &Trail) -> Frame {
    let cells = p.get("cells").and_then(Value::as_str).unwrap_or_default();
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    (cells, p.get("rows"), p.get("cols")).hash(&mut hasher);
    p.get("x_min").map(Value::to_string).hash(&mut hasher);
    p.get("y_min").map(Value::to_string).hash(&mut hasher);
    let cells_id = hasher.finish();
    let mut light = p.clone();
    if let Some(map) = light.as_object_mut() {
        map.remove("cells");
        map.insert("cells_id".into(), json!(cells_id.to_string()));
        map.insert("trail".into(), trail.sent());
    }
    let mut full = light.clone();
    full["cells"] = json!(cells);
    Frame { cells_id, full: event("frame", &full.to_string()), light: event("frame", &light.to_string()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(submaps: i64, x: f64, cells: &str) -> Value {
        json!({"seq": 1, "x": x, "y": 0.0, "yaw": 0.0, "tracking": true, "seated": false,
               "x_min": -1.0, "y_min": -1.0, "cell_m": 0.05, "rows": 1, "cols": 2,
               "cells": cells, "n_submaps": submaps})
    }

    #[test]
    fn the_trail_grows_by_steps_of_trusted_poses_and_resets_on_another_map() {
        let mut t = Trail::default();
        t.frame(&frame(10, 0.0, "AAE="));
        t.pose(&json!({"x": 0.01, "y": 0.0, "tracking": true}));
        t.pose(&json!({"x": 0.2, "y": 0.0, "tracking": false}));
        t.pose(&json!({"x": 0.3, "y": 0.0, "tracking": true, "seated": true}));
        t.pose(&json!({"x": 0.1, "y": 0.0, "tracking": true}));
        assert_eq!(t.points.len(), 2);
        t.frame(&frame(12, 0.2, "AAE="));
        assert_eq!(t.points.len(), 3);
        t.frame(&frame(40, 0.2, "AAE="));
        assert_eq!(t.points.len(), 1, "a swapped map starts a new trail");
    }

    #[test]
    fn a_long_trail_is_thinned_but_ends_at_the_duck() {
        let mut t = Trail::default();
        for i in 0..TRAIL_MAX + 10 {
            t.pose(&json!({"x": i as f64 * 0.1, "y": 0.0, "tracking": true}));
        }
        assert_eq!(t.points.len(), TRAIL_MAX);
        let sent = t.sent();
        let sent = sent.as_array().unwrap();
        assert!(sent.len() <= TRAIL_SENT + 1, "{}", sent.len());
        assert_eq!(sent.last().unwrap()[0], json!(round2((TRAIL_MAX + 9) as f64 * 0.1)));
    }

    #[test]
    fn a_frame_carries_its_grid_once_and_its_identity_always() {
        let t = Trail::default();
        let a = frame_item(&frame(1, 0.0, "AAE="), &t);
        let b = frame_item(&frame(1, 0.5, "AAE="), &t);
        let c = frame_item(&frame(1, 0.0, "AQE="), &t);
        assert_eq!(a.cells_id, b.cells_id, "the pose is not the grid");
        assert_ne!(a.cells_id, c.cells_id);
        assert!(a.full.contains("\"cells\":\"AAE=\""));
        assert!(!a.light.contains("\"cells\""));
        assert!(a.light.contains("cells_id"));
        assert!(a.full.starts_with("event: frame\ndata: {"));
    }

    #[test]
    fn a_page_gets_what_is_known_then_what_follows() {
        let hub = Hub::default();
        hub.publish_text("status", &json!({"ok": true}), |l| &mut l.status);
        let (now, rx) = hub.subscribe();
        assert_eq!(now.len(), 1);
        hub.link(Some("streaming".into()), None);
        hub.link(Some("streaming".into()), None);
        let Item::Text(text) = rx.try_recv().unwrap() else { panic!() };
        assert!(text.starts_with("event: link\n"), "{text}");
        assert!(rx.try_recv().is_err(), "an unchanged link is not sent again");
        drop(rx);
        hub.link(Some("gone".into()), None);
        assert_eq!(hub.listeners(), 0, "a page that left is dropped");
    }
}
