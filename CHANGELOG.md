# Changelog

All notable changes to quack-control. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow
[Semantic Versioning](https://semver.org/) (pre-1.0: a minor version may
break things). Italian copy: [CHANGELOG.it.md](CHANGELOG.it.md).

## [Unreleased]

### Changed

- The path walked shows the last two runs only (a run ends where the duck
  stood still 20 s), not every path since quack-control started.

### Added

- The map turns: ⟲ ⟳ by 90°, two fingers freely, ⊞ squares the walls to
  the screen (the angle whose wall projections are sharpest), N back to
  the map as drawn; kept per map in the browser.
- **The local control plane** (ADR 0001, 2026-10-01): one binary,
  `quack-control`, serving one page on the home network (default
  `0.0.0.0:8090`, not mediad's console's 8080; an optional token, plain HTTP for the LAN only), with an
  adapter per managed daemon.
- **quack-nav**: the live map, pose, uncertainty, trail, route, goal,
  drops and places; tap to go or to name a point, stop, places, explore
  start / stop / complete, the status line — through quack-nav's control
  contract, the map view's calls checked one by one. An advanced view
  with every catalog tool (a form from its JSON Schema, confirmation for
  the ones that act) and the knobs editor (`nav.knobs`, Apply =
  `nav.restart`).
- **The camera** (`mediad` adapter): a small window over the map with
  the duck's head camera — any corner, collapsible, full screen (with a
  fallback where element full screen is missing, as on the iPhone), frame
  rate and age, "camera unavailable" and why. Polled only while the window
  is open and the page visible. `GET /api/<service>/snapshot` serves an
  upright JPEG (at most 640 px) from mediad's `media.frame`
  (`/run/mediad/media.sock`), one fetch at a time and at most five a
  second for all the pages together. On the twin, quack-nav's viewer
  answers the same call on `$STATE/media.sock`. In the default config.
- **quacksat**: a placeholder that says it is not available, and what
  quacksat would need to expose.
- The Services view; a systemd unit (user `quackctl` in `robot`),
  `scripts/install-on-duck.sh`, `scripts/cross-build.sh` (aarch64, glibc
  >= 2.31), a CI workflow.
