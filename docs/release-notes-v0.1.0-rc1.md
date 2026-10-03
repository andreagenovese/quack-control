# quack-control v0.1.0-rc1 — release notes

2026-10-03. Italian copy: [release-notes-v0.1.0-rc1.it.md](release-notes-v0.1.0-rc1.it.md).
The full list of changes is in [CHANGELOG.md](../CHANGELOG.md).

## What this is

The **first release candidate** of quack-control: a local control plane
for the Pollen Robotics Microduck's daemons. One binary runs on the duck
beside the daemons it manages and serves **one page on the home network**
— open it from a phone or a laptop — that shows the map the duck built,
live, and drives it. It talks to each daemon over its own unix sockets,
through an adapter per kind: [quack-nav](https://github.com/andreagenovese/quacknav)'s
quack-navd and Pollen's mediad (the camera) now, quacksat later. Why a
repository of its own: [ADR 0001](adr/0001-a-control-plane-of-its-own.md).

**Tested against quack-nav's MuJoCo twin only, not on a physical duck.**
The twin runs quack-navd's real sockets and answers the camera's call the
way mediad does; the duck arrives in December 2026.

Independent project, not affiliated with Pollen Robotics or Hugging Face.

## What the page does

**Map** (the default view):

- the live map (walls, floor, unknown), the duck's pose and heading — red
  when it is lost or seated — its uncertainty (one sigma), the path it
  walked on its **last two runs**, the route of the job it runs, the goal,
  the drops and obstacles on its books, the named places;
- **tap the map** → **Go here** (asks first) or **Name** the point; a big
  **STOP** that stops whatever the duck is doing, its own motion included;
- **exploring**: Start, Stop, **Exploration complete** (asks first);
- **places by state** — usable, waiting for the duck to find itself, on
  another map, stale — with their distance, Go (only where it can go),
  Forget, and Teach the spot the duck stands on;
- **badges** for what the duck does of its own accord: resting (with its
  last check), may have been moved (untrusted: it finds itself before the
  next job), moving on its own and why (self-started motion), stopped by
  you; and the status line (mapping mode, localized or searching, the job
  and its outcome, how much of the house is mapped);
- **map rotation**: ⟲ ⟳ by 90°, a two-finger turn, ⊞ squares the walls to
  the screen, N back to the map as drawn — kept per map in the browser;
  pan, pinch, fit, follow the duck;
- **the camera**: a small window over the map with what the duck's head
  camera sees (mediad's `media.frame`, an upright JPEG of at most 640 px),
  any corner, **full screen** (the whole page on the iPhone), frame rate
  and age, "camera unavailable" and why. Frames are asked for only while
  the window is open and the page visible, at most five a second for all
  the pages together.

**Services**: each managed daemon and whether it answers.

**Advanced** (collapsed, behind a warning):

- **every quack-navd tool** in `nav.catalog`, with a form generated from
  its JSON Schema (or raw JSON) and the raw reply; a tool that acts asks
  for confirmation, and the server refuses it without;
- **the knobs editor**: quack-navd's `QK_*`/`MAPLOC_*` knobs grouped, with
  default, saved and running values; **Apply** is `nav.restart` (it stops
  what the duck is doing, saves the map, and the homecoming runs as at
  boot).

## Security

- **Plain HTTP, for the home network only.** Whoever reaches the port can
  open the page, and the page drives the duck: never forward it to the
  internet.
- **An optional token** (`QC_TOKEN` in `/etc/robot/quack-control.env`,
  root-only): every call and the live stream need it; the page asks for it
  once. It travels in clear like everything else here — it keeps out a
  guest's phone, not someone sniffing the Wi-Fi.
- Every call is a JSON `POST`, so another site cannot drive the duck
  through a browser at home; the map view reaches seven tools in the shapes
  it uses; the service runs as the unprivileged `quackctl`, in the `robot`
  group.
- **Port 8090**, because mediad's own console listens on 8080.

Details: [README, Security](../README.md#security).

## Install

Every release carries an install package,
`quack-control-0.1.0-rc1-aarch64-linux.tar.gz` with its `.sha256` — the
board's binary (aarch64, glibc ≥ 2.31), the systemd unit, the service
account, the example config, `install-on-duck.sh` and a step-by-step
`README-install.md` (Italian: `README-install.it.md`). The bare binary,
`quack-control-aarch64-linux`, is attached too, with its sha256.

Install quack-nav first, then follow
[Installing from a release](../README.md#installing-from-a-release):
download, `shasum -a 256 -c`, unpack, `./install-on-duck.sh --dry-run
microduck@<duck>` to see every command, then without `--dry-run`. Open
`http://<the duck's address>:8090/`.

## Compatibility

| quack-control | quack-nav | camera | quacksat |
|---|---|---|---|
| 0.1.0-rc1 | **v0.2.0-rc2 or later** | Pollen's mediad daemon-v0.14.4+ (`media.frame`), or the twin's `eye.py` | no control socket yet |

- **quack-nav v0.2.0-rc2 or later** for the full page: `nav.knobs`,
  `nav.restart`, places taught at a point, and the rest / untrusted /
  self-started fields. With an older quack-navd the page degrades — the
  map, go, stop and explore work; what it lacks answers "update quack-nav"
  and the badges stay off.
- **The camera** needs Pollen's mediad (daemon-v0.14.4 or later, which
  serves `media.frame`), or on the twin quack-nav's viewer
  (`scripts/twin/viewer/eye.py`, `VIEWER=on`).
- **quacksat** shows as not available until it has a control socket
  ([todo](todo.md)).

## Known limitations

From [docs/todo.md](todo.md):

- not yet on a physical duck; the camera window not yet tried on a real
  phone (iPhone Safari's overlay, Android Chrome's full screen) nor the
  cost of a frame → JPEG on the RK3566;
- the camera is polled JPEG snapshots, not video (WebRTC from mediad is
  for later);
- the page does not say who started the running job (the page, the voice,
  the homecoming), and cannot answer the explorer's "where are we?";
- home network only: no remote access yet (planned over Pollen's channel);
  no rooms, no-go zones or map management yet;
- no screenshot test in CI yet; 40 unit tests (HTTP, token, routes, every
  check on the calls, the camera).
