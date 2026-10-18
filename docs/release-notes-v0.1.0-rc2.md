# quack-control v0.1.0-rc2 — release notes

2026-10-18. Italian copy: [release-notes-v0.1.0-rc2.it.md](release-notes-v0.1.0-rc2.it.md).
The full list of changes is in [CHANGELOG.md](../CHANGELOG.md); the previous
candidate's notes are in [release-notes-v0.1.0-rc1.md](release-notes-v0.1.0-rc1.md).

## What this is

The **second release candidate** of quack-control, the local control plane
for the Pollen Robotics Microduck's daemons. **Tested against quack-nav's
MuJoCo twin only, not on a physical duck**; the duck arrives in December
2026.

Independent project, not affiliated with Pollen Robotics or Hugging Face.

## What changed since rc1

- **The house in 3D (experimental).** A **3D** tab draws a house built off
  board with the duck's own camera colours: turn, move, zoom, isometric or
  from above, cut at a height so the rooms show, the duck's way through the
  house, and the duck live on it when the house was built on the map it is
  on. The house is quack-nav's `scripts/map3d` work (v0.3.0-rc2): it
  records beside an exploring duck, builds on a Mac or a PC, and
  `send.sh microduck@<duck> out/house3d.bin` puts the file at
  `/var/lib/quack-control/house3d.bin`, which the page reads again when it
  changes (↻). Nothing is built on the duck. Without the file the tab says
  how to make one. Details: [README, The house in 3D](../README.md#the-house-in-3d).
- **The band on the map.** With quack-nav v0.3.0-rc2 the map also shows
  what the depth sensor found in the way at the duck's height (brown) and
  table tops or bed edges with floor under them (tan): what the navigation
  goes round as walls. Sent with the grid, only when it changes.
- CI: actions on Node 24.

## Install

As rc1: the package `quack-control-0.1.0-rc2-aarch64-linux.tar.gz` with its
`.sha256`, then [Installing from a release](../README.md#installing-from-a-release)
(`./install-on-duck.sh --dry-run microduck@<duck>` first). An existing
config is kept; the `house` service is in the default services, so a
config written by rc1 needs it added to `/etc/robot/quack-control.toml`
to show the 3D tab:

```toml
[[service]]
name = "house"
kind = "house3d"
house_file = "/var/lib/quack-control/house3d.bin"
```

## Compatibility

| quack-control | quack-nav | camera | quacksat |
|---|---|---|---|
| 0.1.0-rc2 | **v0.2.0-rc2 or later**; the 3D house from v0.3.0-rc2's `scripts/map3d` | Pollen's mediad daemon-v0.14.4+, or the twin's `eye.py` | no control socket yet |

## Known limitations

As rc1's ([release-notes-v0.1.0-rc1.md](release-notes-v0.1.0-rc1.md#known-limitations)),
and:

- the 3D house is a preview: its colours on the twin are synthetic, and
  the camera's axes and intrinsics are to be checked on the duck;
- the band is drawn as quack-nav reads it; along some walls it shows as a
  brown rim, which the navigation reads as wall anyway.
