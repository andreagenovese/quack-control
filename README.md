# quack-control

A local control plane for the Pollen Robotics Microduck's daemons: **one
page in a browser on the home network** — a phone, a laptop — that shows
the map the duck built, live, and drives it. It runs on the duck beside the
daemons it manages and talks to each one over its own unix sockets, through
an adapter per kind: [quack-nav](https://github.com/andreagenovese/quacknav)
now, quacksat and others later. Italian: [README.it.md](README.it.md).

Independent project, not affiliated with Pollen Robotics or Hugging Face.
Status: 0.1.0, unreleased; tested against the MuJoCo twin (read-only so
far), not yet on a duck.

## What the page does

**Map** (the default view):

- the live map (walls, floor, unknown), the duck's pose and heading — red
  when it is lost or seated — and its uncertainty (one sigma), the path it
  walked, the route of the job it runs, the goal, the drops and obstacles
  on its books, the named places;
- pan and zoom (drag, pinch, wheel), fit, follow the duck;
- tap the map → **Go here** (asks first) or **Name** the point; a big
  **STOP** that stops whatever the duck is doing;
- places: the list with their distance, **Go**, **Forget**, and **Teach**
  the spot the duck stands on;
- exploring: **Start**, **Stop**, **Exploration complete** (asks first);
- the status line: mapping mode and map, localized or searching, the job
  (exploring, going somewhere, its outcome), how much of the house is
  mapped and in how many sessions, the daemon's own hint; a quack-navd
  that refuses says why, one that is unreachable says so, and the page
  reconnects by itself;
- **the camera**: a small window over the map (the camera button, top
  right), with what the duck's head camera sees — drag it by its title to
  any corner, close it, **full screen** (the browser's own where it has
  one; on the iPhone, whose Safari has it for videos only, the window
  covers the page). It shows the frame rate and how old the frame is, or
  "camera unavailable" and why; open/closed and the corner are remembered
  in the browser. Frames are asked for only while the window is open, the
  map view shown and the page visible — none at all otherwise.

**Services**: each managed daemon and whether it answers. quacksat shows
as not available: it has no control socket yet ([docs/todo.md](docs/todo.md)).

**Advanced** (collapsed, behind a warning):

- **All tools**: every tool quack-navd lists in `nav.catalog`, with a form
  built from its JSON Schema (or raw JSON), the raw reply shown; a tool
  that acts asks for confirmation, and the server refuses it without;
- **Knobs**: quack-navd's environment knobs (`QK_*`, `MAPLOC_*`) grouped,
  with their default, what the code says, the value saved and the value
  running; edit, reset one, reset all, and **Apply**, which restarts
  quack-navd so it reads them (it stops what the duck is doing, saves the
  map, and the homecoming runs as at boot).

## Running it

Rust 1.89 or newer.

```sh
cargo build --release
target/release/quack-control /etc/robot/quack-control.toml
```

`quack-control.example.toml`:

```toml
bind = "0.0.0.0:8090"
# token = "a-long-random-string"     # better: QC_TOKEN in the environment

[[service]]
name = "quack-nav"
kind = "quack-nav"
nav_socket = "/run/quack-nav/nav.sock"
map_socket = "/run/quack-nav/map.sock"

[[service]]
name = "camera"
kind = "mediad"
media_socket = "/run/mediad/media.sock"

[[service]]
name = "quacksat"
kind = "quacksat"
```

No file: these defaults. Then open `http://<the duck's address>:8090/`.

### Against the twin

quack-nav's MuJoCo twin serves quack-navd's sockets under its `STATE`
directory (`/tmp/quack-twin` by default). A config that points there, on
this machine only:

```toml
bind = "127.0.0.1:8090"

[[service]]
name = "quack-nav"
kind = "quack-nav"
nav_socket = "/tmp/quack-twin/nav.sock"
map_socket = "/tmp/quack-twin/map.sock"

[[service]]
name = "camera"
kind = "mediad"
media_socket = "/tmp/quack-twin/media.sock"
```

```sh
target/release/quack-control twin.toml
open http://127.0.0.1:8090/
# without a browser: the live stream, and the services
curl -N http://127.0.0.1:8090/api/quack-nav/events
curl http://127.0.0.1:8090/api/services
```

The camera on the twin: there is no mediad, so quack-nav's twin viewer
(`scripts/twin/viewer/eye.py`, `VIEWER=on`) renders the duck's head camera
in MuJoCo and answers mediad's call on `$STATE/media.sock` — the same
adapter for the twin and the duck. With `VIEWER=off`, no camera.

The knobs on the twin: quack-navd writes `$STATE/knobs.env`, and Apply
answers that it is not under systemd — `scripts/twin/twin.sh restart-navd`
in quack-nav restarts it with them.

## Security

**Plain HTTP, for the home network only.** Whoever reaches the port can
open the page, and the page drives the duck. Never forward it to the
internet; remote access is planned over Pollen's channel instead
([docs/todo.md](docs/todo.md)).

- **The token** (`token`, or `QC_TOKEN` — the unit reads
  `/etc/robot/quack-control.env`, root-only, mode 600): when set, every call
  and the live stream need it, as the `X-Token` header or `?token=`; the
  page asks for it once and keeps it in the browser. It travels in clear
  on the network, like everything else here: it keeps out a guest's phone,
  not someone sniffing the Wi-Fi.
- **Another site** cannot drive the duck through the browser of someone at
  home: every call is a `POST` with a JSON body, which a page from another
  origin cannot send without a preflight that quack-control never answers.
- **What reaches the duck**: the map view reaches seven tools in the
  shapes it uses (`robot.go_to` a point, a place or stop; explore start,
  stop, complete — never `fresh`; teach and forget a place; three reads).
  The advanced view reaches every catalog tool, and one that acts only with
  `"confirmed": true`; the knobs only through quack-navd's `nav.knobs`,
  which checks every value against its type.
- **On the duck** quack-control runs as `quackctl`, in the `robot` group
  (that is what quack-navd's sockets ask), unprivileged and fenced
  (`systemd/quack-control.service`).

## The camera

The `mediad` adapter (`src/adapters/mediad.rs`) reads Pollen's `mediad`
(microduck, daemon-v0.14.4 and daemon-v0.15.0): `media.frame` on its local
socket `/run/mediad/media.sock` (0660, group `robot`, which `quackctl` is
in). One JSON-RPC line asks; one line answers with a header (width,
height, `UYVY`, bytes, capture time, `rotate`) and the raw frame follows —
1280x720, 1.84 MB at the default rung. mediad copies a frame only when
asked and waits for the next capture to answer (at most 500 ms).

`GET /api/<service>/snapshot` answers `image/jpeg` (token-protected like
every call): the frame turned upright (`rotate`, 90 on every duck and on
the twin), shrunk to at most 640 px on its long edge, JPEG quality 70 —
360x640, 5–40 kB — with `X-Frame-Age-Ms`, `X-Frame-Captured-Us` and
`X-Frame-Size`. **One fetch at a time, at most five a second for all the
pages together**: a frame younger than 200 ms is served from memory, and a
failure answers for itself (503) for a second before the camera is asked
again. Video (WebRTC, mediad's own stream) is for later
([docs/todo.md](docs/todo.md)).

**Port 8090**: mediad's own console listens on `0.0.0.0:8090`
(`--web-port`, daemon-v0.15.0), so quack-control's default is 8090
(since 2026-10-01; it was 8080, and the second to start could not listen).

## Installing on the duck

After quack-nav (quack-navd and the `robot` group):

```sh
scripts/cross-build.sh                              # aarch64, glibc >= 2.31
scripts/install-on-duck.sh microduck@192.168.1.42   # binary, unit, account, config
```

| on the duck | from this repo |
|---|---|
| `/usr/local/bin/quack-control` | the cross-built binary (1.7 MB) |
| `/etc/systemd/system/quack-control.service` | `systemd/quack-control.service` |
| `/etc/sysusers.d/quack-control.conf` (user `quackctl`) | `systemd/sysusers.d/quack-control.conf` |
| `/etc/robot/quack-control.toml` | `quack-control.example.toml`, only when absent |
| `/etc/robot/quack-control.env` | created empty (`QC_TOKEN` commented), root, 600, only when absent |

`systemctl status quack-control`, `journalctl -u quack-control -f`.
Uninstalling: `sudo systemctl disable --now quack-control`, then remove the
binary, the unit and the sysusers file.

## How it is built

One binary, std threads, no async runtime and no HTTP crate: a few hundred
lines of HTTP/1.1 (`src/http.rs`), one thread per connection, server-sent
events for the live data and `POST` JSON for commands. Dependencies:
serde, serde_json, toml, tracing, anyhow, jpeg-encoder (pure Rust, for the
camera's frames). The page is one HTML file with
vanilla JavaScript and a canvas, embedded in the binary — no build step,
nothing loaded from the network, it works offline.

| file | what it does |
|---|---|
| `src/server.rs` | the routes, the token, the JSON-only rule, the event streams |
| `src/adapters/mod.rs` | the `Adapter` trait and the kinds |
| `src/adapters/quack_nav/` | quack-navd: the map stream and the status poll (`hub.rs`), the checks on every call (`bridge.rs`) |
| `src/adapters/mediad.rs` | the camera: `media.frame`, the shared cache, UYVY → upright JPEG |
| `src/adapters/quacksat.rs` | a placeholder, and what quacksat would need to expose |
| `src/page.html` | the page |

The quack-nav adapter uses quack-nav's **control contract**
(quack-nav's `docs/control-contract.md`): `nav.catalog`/`nav.call` on the
nav socket, `robot.map` on the map socket, `nav.knobs` and `nav.restart`.
Why a repository of its own, the adapter model and the road to Pollen's
channel: [docs/adr/0001-a-control-plane-of-its-own.md](docs/adr/0001-a-control-plane-of-its-own.md).

```sh
cargo test --release      # 39 tests: HTTP, token, routes, every check on the calls, the camera
```

## License

Apache-2.0 ([LICENSE](LICENSE), [NOTICE](NOTICE)).
