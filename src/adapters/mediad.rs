//! mediad, Pollen's camera daemon: the duck's head camera as still frames.
//!
//! | request | what it does |
//! |---|---|
//! | `GET snapshot` | the newest frame as `image/jpeg`, upright, at most 640 px on its long edge |
//!
//! **Where a frame comes from.** `mediad` (microduck, `mediad/src/frame.rs`,
//! in daemon-v0.14.4 and daemon-v0.15.0) serves `media.frame` on a local
//! unix socket, `/run/mediad/media.sock`, mode 0660, group `robot` — the
//! group quack-control's unit already has for quack-navd. One JSON-RPC line
//! asks (`{"jsonrpc":"2.0","id":1,"method":"media.frame"}`); one line
//! answers with a header (`width`, `height`, `format` "UYVY", `bytes`,
//! `captured_at_unix_us`, `rotate`) and exactly `bytes` raw UYVY bytes
//! follow: 1280x720, 1.84 MB, at the default `720p30` rung. It is
//! deliberately not a `Call` of `duck_ipc_proto` (its binary tail must not
//! enter the control routing). mediad waits for the next capture to answer
//! (up to 500 ms) and copies a frame only when asked, so a camera nobody
//! watches costs nothing. `rotate` is the mount (90 on every duck, and on
//! the twin): the frame is turned here, because a JPEG has nowhere to say it.
//!
//! **What this adapter adds:** one fetch at a time, at most one every
//! [`MIN_INTERVAL`], shared by every open page (a frame younger than that
//! is served from memory); a failure is remembered for [`ERROR_HOLD`] so a
//! dead camera is not asked again by every poll; the frame is shrunk (box
//! filter) and encoded as a JPEG here, a few tens of kB instead of 1.8 MB.
//! On the twin, scripts/twin/viewer/eye.py answers the same call.

use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::Deserialize;
use serde_json::{Value, json};

use super::{Adapter, Health, Reply};
use crate::config::ServiceConfig;

/// Five frames a second at most, however many pages watch.
pub const MIN_INTERVAL: Duration = Duration::from_millis(200);
/// How long a failed fetch answers for itself before the camera is asked again.
pub const ERROR_HOLD: Duration = Duration::from_secs(1);
/// mediad itself gives up on a client after 5 s and on a capture after 0.5 s.
const IO_TIMEOUT: Duration = Duration::from_secs(3);
const HELLO_TIMEOUT: Duration = Duration::from_secs(1);
/// The longest edge of the picture the page gets.
pub const MAX_EDGE: usize = 640;
const JPEG_QUALITY: u8 = 70;
/// mediad's own bound on a header line.
const MAX_LINE: u64 = 4096;
/// And on a frame (`MediaFrameHeader::valid_uyvy`).
const MAX_BYTES: usize = 16 * 1024 * 1024;

/// One JPEG, as it is served.
#[derive(Clone)]
pub struct Snapshot {
    pub jpeg: Arc<Vec<u8>>,
    pub width: usize,
    pub height: usize,
    pub captured_unix_us: u64,
    fetched: Instant,
}

#[derive(Default)]
struct Cache {
    frame: Option<Snapshot>,
    error: Option<(Instant, String)>,
    frames: u64,
}

pub struct Mediad {
    name: String,
    socket: PathBuf,
    cache: Mutex<Cache>,
    /// Held while a frame is fetched: the others wait and take its result.
    fetching: Mutex<()>,
}

/// `MediaFrameHeader` (duck_ipc_proto, API 34–37).
#[derive(Debug, Deserialize)]
struct Header {
    width: u32,
    height: u32,
    format: String,
    bytes: usize,
    #[serde(default)]
    captured_at_unix_us: u64,
    #[serde(default)]
    rotate: u32,
}

impl Header {
    fn valid_uyvy(&self) -> bool {
        self.width > 0
            && self.height > 0
            && self.width.is_multiple_of(2)
            && self.format == "UYVY"
            && matches!(self.rotate, 0 | 90 | 180 | 270)
            && self.bytes <= MAX_BYTES
            && (self.width as usize).checked_mul(self.height as usize).and_then(|n| n.checked_mul(2)) == Some(self.bytes)
    }
}

impl Mediad {
    pub fn new(config: &ServiceConfig) -> Self {
        Self {
            name: config.name.clone(),
            socket: PathBuf::from(&config.media_socket),
            cache: Mutex::new(Cache::default()),
            fetching: Mutex::new(()),
        }
    }

    /// The newest frame, fetched only when the one in memory is older than
    /// [`MIN_INTERVAL`], and by one caller at a time.
    pub fn snapshot(&self) -> Result<Snapshot, String> {
        if let Some(answer) = self.cached() {
            return answer;
        }
        let _one = self.fetching.lock().unwrap_or_else(|e| e.into_inner());
        // Whoever held the lock before may have just fetched it.
        if let Some(answer) = self.cached() {
            return answer;
        }
        let result = fetch(&self.socket).and_then(|(header, pixels)| encode(&header, &pixels));
        let mut cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
        match result {
            Ok(frame) => {
                cache.frame = Some(frame.clone());
                cache.error = None;
                cache.frames += 1;
                Ok(frame)
            }
            Err(e) => {
                cache.error = Some((Instant::now(), e.clone()));
                Err(e)
            }
        }
    }

    fn cached(&self) -> Option<Result<Snapshot, String>> {
        let cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((at, e)) = &cache.error
            && at.elapsed() < ERROR_HOLD
        {
            return Some(Err(e.clone()));
        }
        cache.frame.as_ref().filter(|f| f.fetched.elapsed() < MIN_INTERVAL).map(|f| Ok(f.clone()))
    }

    fn reply(&self) -> Reply {
        match self.snapshot() {
            Ok(frame) => {
                let now_us = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_micros() as u64).unwrap_or(0);
                let age_ms = now_us.saturating_sub(frame.captured_unix_us) / 1000;
                Reply::Bytes {
                    status: 200,
                    content_type: "image/jpeg",
                    headers: vec![
                        ("X-Frame-Age-Ms", age_ms.to_string()),
                        ("X-Frame-Captured-Us", frame.captured_unix_us.to_string()),
                        ("X-Frame-Size", format!("{}x{}", frame.width, frame.height)),
                    ],
                    body: frame.jpeg,
                }
            }
            Err(e) => Reply::Json(503, json!({"ok": false, "error": format!("the camera gave no frame: {e}")})),
        }
    }
}

impl Adapter for Mediad {
    fn name(&self) -> &str {
        &self.name
    }

    fn kind(&self) -> &'static str {
        "mediad"
    }

    fn start(&self) {}

    fn health(&self) -> Health {
        let path = self.socket.display();
        if !self.socket.exists() {
            return Health {
                available: false,
                detail: format!("no camera: nothing listens at {path} (mediad is not running; on the twin, the viewer serves it)"),
            };
        }
        let api = match hello(&self.socket) {
            Ok(api) => api,
            Err(e) => return Health { available: false, detail: format!("{path} does not answer: {e}") },
        };
        let cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
        let last = match (&cache.error, &cache.frame) {
            (Some((at, e)), _) => format!("the last frame failed {:.0} s ago: {e}", at.elapsed().as_secs_f32()),
            (None, Some(f)) => format!(
                "{} frames served, the last {}x{} {:.0} s ago",
                cache.frames,
                f.width,
                f.height,
                f.fetched.elapsed().as_secs_f32()
            ),
            (None, None) => "no frame asked for yet".into(),
        };
        Health { available: true, detail: format!("mediad answers at {path} (API {api}); {last}") }
    }

    fn features(&self) -> Vec<&'static str> {
        vec!["camera"]
    }

    fn route(&self, method: &str, path: &str, _body: &Value) -> Reply {
        match (method, path) {
            ("GET", "snapshot") => self.reply(),
            (_, "snapshot") => Reply::Json(405, json!({"ok": false, "error": "GET the snapshot"})),
            _ => Reply::Json(404, json!({"ok": false, "error": format!("{} has no `{path}` (only `snapshot`)", self.name)})),
        }
    }
}

fn connect(socket: &Path, timeout: Duration) -> Result<UnixStream, String> {
    let stream = UnixStream::connect(socket).map_err(|e| format!("cannot reach {}: {e}", socket.display()))?;
    stream.set_read_timeout(Some(timeout)).map_err(|e| e.to_string())?;
    stream.set_write_timeout(Some(timeout)).map_err(|e| e.to_string())?;
    Ok(stream)
}

/// One JSON-RPC response line: its `result`, or its error as a message.
fn response(reader: &mut impl BufRead, id: u64) -> Result<Value, String> {
    let mut line = Vec::new();
    reader.take(MAX_LINE + 1).read_until(b'\n', &mut line).map_err(|e| format!("no answer: {e}"))?;
    if line.is_empty() {
        return Err("the camera closed the connection".into());
    }
    if line.len() as u64 > MAX_LINE || !line.ends_with(b"\n") {
        return Err("the camera's answer is not one line".into());
    }
    let reply: Value = serde_json::from_slice(&line).map_err(|e| format!("the camera's answer is not JSON: {e}"))?;
    if reply.get("id").and_then(Value::as_u64) != Some(id) {
        return Err("the camera answered another request".into());
    }
    if let Some(error) = reply.get("error") {
        return Err(error.get("message").and_then(Value::as_str).unwrap_or("an error").to_owned());
    }
    reply.get("result").cloned().ok_or_else(|| "the camera's answer has no result".into())
}

/// `hello`: the API version the camera's daemon speaks.
fn hello(socket: &Path) -> Result<u64, String> {
    let mut stream = connect(socket, HELLO_TIMEOUT)?;
    stream.write_all(b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"hello\"}\n").map_err(|e| e.to_string())?;
    let result = response(&mut BufReader::new(stream), 1)?;
    Ok(result.get("api_version").and_then(Value::as_u64).unwrap_or(0))
}

/// `media.frame`: the header and the raw UYVY bytes.
fn fetch(socket: &Path) -> Result<(Header, Vec<u8>), String> {
    let mut stream = connect(socket, IO_TIMEOUT)?;
    stream.write_all(b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"media.frame\"}\n").map_err(|e| e.to_string())?;
    let mut reader = BufReader::new(stream);
    let header: Header =
        serde_json::from_value(response(&mut reader, 1)?).map_err(|e| format!("the frame's header does not parse: {e}"))?;
    if !header.valid_uyvy() {
        return Err(format!("a frame this adapter cannot read: {header:?}"));
    }
    let mut pixels = vec![0; header.bytes];
    reader.read_exact(&mut pixels).map_err(|e| format!("the frame was cut short: {e}"))?;
    Ok((header, pixels))
}

fn encode(header: &Header, pixels: &[u8]) -> Result<Snapshot, String> {
    let (rgb, width, height) = rgb_from_uyvy(pixels, header.width as usize, header.height as usize, header.rotate, MAX_EDGE);
    let mut jpeg = Vec::with_capacity(64 * 1024);
    jpeg_encoder::Encoder::new(&mut jpeg, JPEG_QUALITY)
        .encode(&rgb, width as u16, height as u16, jpeg_encoder::ColorType::Rgb)
        .map_err(|e| format!("cannot encode the frame: {e}"))?;
    Ok(Snapshot { jpeg: Arc::new(jpeg), width, height, captured_unix_us: header.captured_at_unix_us, fetched: Instant::now() })
}

/// Packed UYVY (BT.601 limited range, mediad's `uyvy` crate's convention)
/// to RGB, shrunk by a whole factor so the long edge is at most `max_edge`
/// (each output pixel the mean of its block), turned `rotate` degrees
/// clockwise. Returns the pixels and the turned width and height.
pub fn rgb_from_uyvy(uyvy: &[u8], width: usize, height: usize, rotate: u32, max_edge: usize) -> (Vec<u8>, usize, usize) {
    let factor = width.max(height).div_ceil(max_edge.max(1)).max(1);
    let (sw, sh) = ((width / factor).max(1), (height / factor).max(1));
    let (ow, oh) = if matches!(rotate, 90 | 270) { (sh, sw) } else { (sw, sh) };
    let mut rgb = vec![0u8; ow * oh * 3];
    let n = (factor * factor) as i32;
    for sy in 0..sh {
        for sx in 0..sw {
            let (mut y, mut u, mut v) = (0i32, 0i32, 0i32);
            for by in 0..factor {
                let row = (sy * factor + by).min(height - 1) * width;
                for bx in 0..factor {
                    let x = (sx * factor + bx).min(width - 1);
                    let pair = (row + (x & !1)) * 2;
                    if pair + 3 >= uyvy.len() {
                        continue;
                    }
                    y += uyvy[pair + 1 + 2 * (x & 1)] as i32;
                    u += uyvy[pair] as i32;
                    v += uyvy[pair + 2] as i32;
                }
            }
            let (y, u, v) = (y / n - 16, u / n - 128, v / n - 128);
            let r = (298 * y + 409 * v + 128) >> 8;
            let g = (298 * y - 100 * u - 208 * v + 128) >> 8;
            let b = (298 * y + 516 * u + 128) >> 8;
            let (dx, dy) = match rotate {
                90 => (sh - 1 - sy, sx),
                180 => (sw - 1 - sx, sh - 1 - sy),
                270 => (sy, sw - 1 - sx),
                _ => (sx, sy),
            };
            let i = (dy * ow + dx) * 3;
            rgb[i] = r.clamp(0, 255) as u8;
            rgb[i + 1] = g.clamp(0, 255) as u8;
            rgb[i + 2] = b.clamp(0, 255) as u8;
        }
    }
    (rgb, ow, oh)
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::os::unix::net::UnixListener;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// A mediad that answers `hello` and `media.frame` with a `w`x`h`
    /// grey frame whose left half is bright, counting the frames it sends.
    pub fn fake_mediad(dir: &Path, w: u32, h: u32, rotate: u32) -> (PathBuf, Arc<AtomicUsize>) {
        let path = dir.join("media.sock");
        let listener = UnixListener::bind(&path).unwrap();
        let served = Arc::new(AtomicUsize::new(0));
        let count = served.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { continue };
                let mut out = stream.try_clone().unwrap();
                let mut reader = BufReader::new(stream);
                let mut line = String::new();
                while reader.read_line(&mut line).unwrap_or(0) > 0 {
                    let request: Value = serde_json::from_str(&line).unwrap();
                    line.clear();
                    let id = request["id"].clone();
                    if request["method"] == "hello" {
                        let reply = json!({"jsonrpc": "2.0", "id": id, "result": {"api_version": 37, "daemon_version": null, "revision": null}});
                        out.write_all(format!("{reply}\n").as_bytes()).unwrap();
                        continue;
                    }
                    let mut pixels = Vec::new();
                    for _ in 0..h {
                        for x in 0..w / 2 {
                            let luma = if x < w / 4 { 235 } else { 16 };
                            pixels.extend_from_slice(&[128, luma, 128, luma]);
                        }
                    }
                    let header = json!({"width": w, "height": h, "format": "UYVY", "bytes": pixels.len(),
                        "captured_at_unix_us": 1_700_000_000_000_000u64, "rotate": rotate});
                    let reply = json!({"jsonrpc": "2.0", "id": id, "result": header});
                    out.write_all(format!("{reply}\n").as_bytes()).unwrap();
                    out.write_all(&pixels).unwrap();
                    count.fetch_add(1, Ordering::SeqCst);
                    break;
                }
            }
        });
        (path, served)
    }

    pub fn adapter(socket: &Path) -> Mediad {
        let mut config = ServiceConfig::new("camera", "mediad");
        config.media_socket = socket.display().to_string();
        Mediad::new(&config)
    }

    #[test]
    fn a_frame_comes_back_as_an_upright_jpeg_of_the_page_s_size() {
        let dir = tempfile::tempdir().unwrap();
        let (socket, _) = fake_mediad(dir.path(), 1280, 720, 90);
        let frame = adapter(&socket).snapshot().unwrap();
        assert_eq!((frame.width, frame.height), (360, 640), "1280x720 halved, then a quarter turn");
        assert_eq!(&frame.jpeg[..2], &[0xFF, 0xD8]);
        assert_eq!(frame.captured_unix_us, 1_700_000_000_000_000);
        assert!(frame.jpeg.len() < 100_000, "{}", frame.jpeg.len());
    }

    #[test]
    fn pages_polling_together_cost_the_camera_one_frame_per_interval() {
        let dir = tempfile::tempdir().unwrap();
        let (socket, served) = fake_mediad(dir.path(), 64, 32, 0);
        let camera = Arc::new(adapter(&socket));
        let pages: Vec<_> = (0..6)
            .map(|_| {
                let camera = camera.clone();
                std::thread::spawn(move || camera.snapshot().unwrap().jpeg)
            })
            .collect();
        let first: Vec<_> = pages.into_iter().map(|p| p.join().unwrap()).collect();
        assert_eq!(served.load(Ordering::SeqCst), 1, "six pages at once, one fetch");
        assert!(first.iter().all(|j| Arc::ptr_eq(j, &first[0])));
        std::thread::sleep(MIN_INTERVAL + Duration::from_millis(30));
        camera.snapshot().unwrap();
        assert_eq!(served.load(Ordering::SeqCst), 2, "and a new one once the interval has passed");
    }

    #[test]
    fn a_missing_camera_is_a_503_said_once_per_hold() {
        let dir = tempfile::tempdir().unwrap();
        let camera = adapter(&dir.path().join("media.sock"));
        let health = camera.health();
        assert!(!health.available);
        assert!(health.detail.contains("nothing listens"), "{}", health.detail);
        let Reply::Json(status, body) = camera.route("GET", "snapshot", &Value::Null) else { panic!("not JSON") };
        assert_eq!(status, 503);
        assert!(body["error"].as_str().unwrap().contains("cannot reach"), "{body}");
        // The camera comes up: the failure still answers for ERROR_HOLD,
        // then the camera is asked again.
        let (_, served) = fake_mediad(dir.path(), 64, 32, 0);
        assert!(camera.snapshot().is_err());
        assert_eq!(served.load(Ordering::SeqCst), 0);
        std::thread::sleep(ERROR_HOLD + Duration::from_millis(50));
        assert!(camera.snapshot().is_ok());
        assert_eq!(served.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn the_reply_is_a_jpeg_with_its_age_and_size() {
        let dir = tempfile::tempdir().unwrap();
        let (socket, _) = fake_mediad(dir.path(), 64, 32, 0);
        let camera = adapter(&socket);
        assert!(camera.health().available);
        assert!(camera.health().detail.contains("API 37"));
        match camera.route("GET", "snapshot", &Value::Null) {
            Reply::Bytes { status, content_type, headers, body } => {
                assert_eq!((status, content_type), (200, "image/jpeg"));
                assert_eq!(&body[..2], &[0xFF, 0xD8]);
                let size = headers.iter().find(|(k, _)| *k == "X-Frame-Size").unwrap();
                assert_eq!(size.1, "64x32");
                assert!(headers.iter().any(|(k, _)| *k == "X-Frame-Age-Ms"));
            }
            _ => panic!("not an image"),
        }
        assert!(matches!(camera.route("POST", "snapshot", &Value::Null), Reply::Json(405, _)));
        assert!(matches!(camera.route("GET", "video", &Value::Null), Reply::Json(404, _)));
        assert_eq!(camera.features(), ["camera"]);
    }

    #[test]
    fn a_quarter_turn_puts_the_left_of_the_sensor_at_the_top() {
        // 4x2, the left two pixels bright.
        let uyvy = [128, 235, 128, 235, 128, 16, 128, 16].repeat(2);
        let (rgb, w, h) = rgb_from_uyvy(&uyvy, 4, 2, 90, 640);
        assert_eq!((w, h), (2, 4));
        let px = |x: usize, y: usize| rgb[(y * w + x) * 3];
        assert_eq!((px(0, 0), px(1, 1), px(0, 2), px(1, 3)), (255, 255, 0, 0));
        let (rgb, w, h) = rgb_from_uyvy(&uyvy, 4, 2, 0, 2);
        assert_eq!((w, h), (2, 1), "shrunk by two");
        assert_eq!((rgb[0], rgb[3]), (255, 0));
    }

    #[test]
    fn headers_that_do_not_add_up_are_refused() {
        let ok = Header { width: 4, height: 2, format: "UYVY".into(), bytes: 16, captured_at_unix_us: 0, rotate: 90 };
        assert!(ok.valid_uyvy());
        assert!(!Header { bytes: 15, ..ok }.valid_uyvy());
        let ok = Header { width: 4, height: 2, format: "NV12".into(), bytes: 16, captured_at_unix_us: 0, rotate: 0 };
        assert!(!ok.valid_uyvy());
    }
}
