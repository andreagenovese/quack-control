# Changelog

All notable changes to quack-control. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow
[Semantic Versioning](https://semver.org/) (pre-1.0: a minor version may
break things). Italian copy: [CHANGELOG.it.md](CHANGELOG.it.md).

## [Unreleased]

## [0.1.0-rc1] - 2026-10-03

The first release candidate, tested against quack-nav's MuJoCo twin, not
yet on a duck; it needs quack-nav v0.2.0-rc2 or later for the full page.
Release notes: [docs/release-notes-v0.1.0-rc1.md](docs/release-notes-v0.1.0-rc1.md).

### Added

- **Release notes on the release.** A `v*` tag's GitHub release takes
  `docs/release-notes-<tag>.md` as its text, its relative links made
  absolute to the files at the tag (`scripts/ci/release_body.py`, as in
  quack-nav); a `-rc` tag is a prerelease.
- **An install package for every release.** CI packs
  `quack-control-<version>-aarch64-linux.tar.gz` (with its `.sha256`)
  beside the bare binary, and a `v*` tag attaches both: the binary, the
  unit, the service account, the example config, `install-on-duck.sh` and
  a step-by-step `README-install.md` (and `.it.md`), so the duck installs
  from a download (README, "Installing from a release").
  `scripts/package.sh <version> <binary> <outdir>` packs it locally.
  `install-on-duck.sh` runs from the package or from a checkout, takes
  `--dry-run`, and no longer fails on macOS's bash 3.2 when `SSH_OPTS` is
  empty.
- Badges for what the duck does of its own accord (quack-nav 2026-10-02):
  moving on its own and why, may have been moved, resting with the last
  check, stopped by the user; places by state (usable, waiting, on another
  map, stale), Go only where it can go.
- The map turns: ⟲ ⟳ by 90°, two fingers freely, ⊞ squares the walls to
  the screen (the angle whose wall projections are sharpest), N back to
  the map as drawn; kept per map in the browser.
- **The local control plane** (ADR 0001, 2026-10-01): one binary,
  `quack-control`, serving one page on the home network (default
  `0.0.0.0:8090`, not mediad's console's 8080; an optional token, plain HTTP for the LAN only), with an
  adapter per managed daemon.
- **quack-nav**: the live map, pose, uncertainty, the path walked on the
  last two runs (a run ends where the duck stood still 20 s), route, goal,
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
