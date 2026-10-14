//! The house in 3D, with the camera's own colours: a file made off board
//! (quack-nav's scripts/map3d/build.py, `house3d.bin`) from what the duck
//! recorded while exploring, copied to the duck, and served to the page.
//!
//! | request | what it does |
//! |---|---|
//! | `GET info` | the file's header (`points`, `voxel_m`, `map`, `frames`, `built`, `path`) and its size |
//! | `GET model` | the file itself, `application/octet-stream`, for the page to draw |
//!
//! **The file.** `QH3D`, a u16 version (1), a u16 header length, the
//! header (JSON), padded to a multiple of 4; then 12 bytes a point,
//! little-endian: i16 x, y, z in millimetres on the map's frame, u8 r, g,
//! b, i8 normal x, y, z (times 127). A house at 2 cm is a few hundred kB.
//!
//! Nothing here builds or changes it: the duck's board is too small for
//! the depth model (and needs to keep its cores for walking). The file is
//! read once and kept until it changes on disk (its size or its time).

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use serde_json::{Value, json};

use super::{Adapter, Health, Reply};
use crate::config::ServiceConfig;

pub const MAGIC: &[u8; 4] = b"QH3D";
pub const VERSION: u16 = 1;
pub const POINT_BYTES: usize = 12;
/// quack-control lives under 64 MB (its unit's MemoryMax); a house at 2 cm
/// is well under 5.
const MAX_BYTES: u64 = 16 * 1024 * 1024;

/// The file as read, with what identifies that reading.
struct Loaded {
    stamp: (u64, Option<SystemTime>),
    bytes: Arc<Vec<u8>>,
    header: Value,
}

pub struct House3d {
    name: String,
    file: PathBuf,
    loaded: Mutex<Option<Loaded>>,
}

/// The header of a file, or why it is not one.
pub fn parse(bytes: &[u8]) -> Result<Value, String> {
    if bytes.len() < 8 || &bytes[..4] != MAGIC {
        return Err("not a house3d file (no QH3D at its start)".into());
    }
    let version = u16::from_le_bytes([bytes[4], bytes[5]]);
    if version != VERSION {
        return Err(format!("house3d version {version}; this quack-control reads {VERSION}"));
    }
    let len = u16::from_le_bytes([bytes[6], bytes[7]]) as usize;
    if !(8 + len).is_multiple_of(4) || bytes.len() < 8 + len {
        return Err("a house3d header that does not fit".into());
    }
    let header: Value = serde_json::from_slice(&bytes[8..8 + len]).map_err(|e| format!("a house3d header that is not JSON: {e}"))?;
    let points = header["points"].as_u64().ok_or("a house3d header without `points`")? as usize;
    if bytes.len() - 8 - len != points * POINT_BYTES {
        return Err(format!("{points} points promised, {} bytes of them", bytes.len() - 8 - len));
    }
    Ok(header)
}

impl House3d {
    pub fn new(config: &ServiceConfig) -> Self {
        Self { name: config.name.clone(), file: PathBuf::from(&config.house_file), loaded: Mutex::new(None) }
    }

    /// The file, read again only when it changed on disk.
    fn load(&self) -> Result<(Arc<Vec<u8>>, Value), String> {
        let path = self.file.display();
        let meta = std::fs::metadata(&self.file).map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => {
                format!("no 3D house yet: nothing at {path} (build one with quack-nav's scripts/map3d and copy its house3d.bin there)")
            }
            _ => format!("cannot read {path}: {e}"),
        })?;
        if meta.len() > MAX_BYTES {
            return Err(format!("{path} is {} MB, more than the {} MB this serves", meta.len() >> 20, MAX_BYTES >> 20));
        }
        let stamp = (meta.len(), meta.modified().ok());
        let mut loaded = self.loaded.lock().unwrap();
        if let Some(l) = loaded.as_ref().filter(|l| l.stamp == stamp) {
            return Ok((l.bytes.clone(), l.header.clone()));
        }
        let bytes = std::fs::read(&self.file).map_err(|e| format!("cannot read {path}: {e}"))?;
        let header = parse(&bytes).map_err(|e| format!("{path}: {e}"))?;
        let bytes = Arc::new(bytes);
        *loaded = Some(Loaded { stamp, bytes: bytes.clone(), header: header.clone() });
        Ok((bytes, header))
    }
}

impl Adapter for House3d {
    fn name(&self) -> &str {
        &self.name
    }

    fn kind(&self) -> &'static str {
        "house3d"
    }

    fn start(&self) {}

    fn health(&self) -> Health {
        match self.load() {
            Ok((_, h)) => Health {
                available: true,
                detail: format!(
                    "{} points at {} cm, {} frames, built {}",
                    h["points"],
                    h["voxel_m"].as_f64().map_or("?".into(), |v| format!("{:.0}", v * 100.0)),
                    h["frames"],
                    h["built"].as_str().unwrap_or("?")
                ),
            },
            Err(e) => Health { available: false, detail: e },
        }
    }

    fn features(&self) -> Vec<&'static str> {
        vec!["house3d"]
    }

    fn route(&self, method: &str, path: &str, _body: &Value) -> Reply {
        match (method, path) {
            ("GET", "info") => match self.load() {
                Ok((bytes, header)) => Reply::Json(200, json!({"ok": true, "header": header, "bytes": bytes.len()})),
                Err(e) => Reply::Json(404, json!({"ok": false, "error": e})),
            },
            ("GET", "model") => match self.load() {
                Ok((bytes, _)) => {
                    Reply::Bytes { status: 200, content_type: "application/octet-stream", headers: vec![("Cache-Control", "no-cache".into())], body: bytes }
                }
                Err(e) => Reply::Json(404, json!({"ok": false, "error": e})),
            },
            (_, "info" | "model") => Reply::Json(405, json!({"ok": false, "error": "GET it"})),
            _ => Reply::Json(404, json!({"ok": false, "error": format!("{} has no `{path}` (`info`, `model`)", self.name)})),
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    /// A file of `points` points, as build.py writes one.
    pub fn file(points: usize) -> Vec<u8> {
        let mut head = json!({"points": points, "voxel_m": 0.02, "map": "house", "frames": 3, "built": "2026-10-14T12:00:00", "path": [[0.0, 0.0]]})
            .to_string()
            .into_bytes();
        while !(8 + head.len()).is_multiple_of(4) {
            head.push(b' ');
        }
        let mut out = MAGIC.to_vec();
        out.extend_from_slice(&VERSION.to_le_bytes());
        out.extend_from_slice(&(head.len() as u16).to_le_bytes());
        out.extend_from_slice(&head);
        for i in 0..points {
            out.extend_from_slice(&(i as i16 * 20).to_le_bytes());
            out.extend_from_slice(&[0, 0, 0, 0, 200, 100, 50, 0, 0, 127]);
        }
        out
    }

    fn adapter(dir: &std::path::Path) -> House3d {
        let mut config = ServiceConfig::new("house", "house3d");
        config.house_file = dir.join("house3d.bin").display().to_string();
        House3d::new(&config)
    }

    #[test]
    fn a_house_is_served_with_its_header() {
        let dir = tempfile::tempdir().unwrap();
        let a = adapter(dir.path());
        assert!(!a.health().available);
        assert!(a.health().detail.contains("no 3D house yet"), "{}", a.health().detail);
        std::fs::write(dir.path().join("house3d.bin"), file(5)).unwrap();
        assert!(a.health().available, "{}", a.health().detail);
        match a.route("GET", "info", &Value::Null) {
            Reply::Json(200, v) => assert_eq!((v["header"]["points"].as_u64(), v["header"]["map"].as_str()), (Some(5), Some("house"))),
            _ => panic!("info"),
        }
        match a.route("GET", "model", &Value::Null) {
            Reply::Bytes { status: 200, body, .. } => assert_eq!(&body[..4], b"QH3D"),
            _ => panic!("model"),
        }
    }

    #[test]
    fn a_new_file_is_read_again() {
        let dir = tempfile::tempdir().unwrap();
        let a = adapter(dir.path());
        std::fs::write(dir.path().join("house3d.bin"), file(5)).unwrap();
        assert!(a.health().detail.starts_with("5 points"));
        std::fs::write(dir.path().join("house3d.bin"), file(7)).unwrap();
        assert!(a.health().detail.starts_with("7 points"), "{}", a.health().detail);
    }

    #[test]
    fn a_damaged_file_says_so() {
        assert!(parse(b"nope").is_err());
        let mut short = file(5);
        short.truncate(short.len() - 3);
        assert!(parse(&short).unwrap_err().contains("5 points promised"));
        let mut v2 = file(1);
        v2[4] = 2;
        assert!(parse(&v2).unwrap_err().contains("version 2"));
    }
}
