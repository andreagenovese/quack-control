# ADR 0001: a control plane of its own

Status: accepted, 2026-10-01. Italian:
[0001-a-control-plane-of-its-own.it.md](0001-a-control-plane-of-its-own.it.md).

## Context

quack-nav's study of a map app
([docs/study/map-app.md](https://github.com/andreagenovese/quacknav/blob/main/docs/study/map-app.md))
asked what an app like a robot vacuum's would take — the live map, tap to
go, places, exploring, a chat with the duck's agent — reached from outside
the home over Pollen's channel. The user decided on 2026-10-01: **for now,
local management only**, a simple page in a browser on the home network;
the rest stays the plan. The same day, a second decision: the page is not
quack-nav's. It is a control plane for every daemon on the duck — quack-nav
now, quacksat and future ones later — in a repository of its own, designed
to meet Pollen's channel later.

## Decision

1. **A separate repository, `quack-control`**, and one small daemon on the
   duck. quack-nav keeps only its side, a **control contract** any client
   may use (quack-nav's `docs/control-contract.md`: the sockets,
   `nav.catalog`/`nav.call`, the map stream, `nav.knobs`, `nav.restart`).
   The navigation stays free of HTTP, of a page and of a second binary;
   the control plane can manage daemons that are not quack-nav's, and be
   released on its own rhythm.
2. **An adapter per kind of daemon** (`src/adapters/`): a trait with a
   name, a kind, a health check, a start for its background lanes, and a
   router for `/api/<service>/…`. The server does the HTTP, the token and
   the page. quack-nav's adapter speaks quack-navd's two sockets;
   quacksat's is a placeholder until quacksat exposes a control socket.
   A new daemon is a new adapter and a `kind` in the config.
3. **Local first, plain HTTP on the LAN**, with an optional token, a
   JSON-only rule against cross-site requests, and allowlists in each
   adapter: the map view's calls in the shapes it uses; every catalog tool
   in the advanced view, the acting ones only confirmed. No port is ever
   meant to face the internet.
4. **Minimal and synchronous**, like the daemons it manages: std threads,
   a hand-written HTTP/1.1 subset, server-sent events, one embedded HTML
   page with no build and no network dependency.

## The road to Pollen's channel

The study's options stay as written (§4 there):

- **Option A — `quack-linkd`**: an adapter of ours registered on Pollen's
  rendezvous as a producer of its own, with its own token, answering
  `peer {rpc}` envelopes. In this repository it becomes a second **face**
  of the same control plane: the same adapters and the same allowlists,
  reached by the rendezvous's lane instead of the LAN's HTTP. The page's
  calls are already JSON-RPC-shaped (`{name, args}` to `nav.call`), so
  the lane carries the same lines.
- **Option C — extension routes upstream**: if Pollen accepts a
  data-driven route table in `mediad` (a prefix, a socket), the face
  shrinks to a manifest and the app talks through Pollen's own session.
  quack-control would then register its own socket, and its adapters stay.
- Asking Pollen first (the study's §8) is still the first step for either.

## Consequences

- Two repositories to install on the duck; quack-control's installer
  assumes quack-nav's `robot` group and sockets.
- A client of the contract can lag: quack-control checks for what it needs
  (`at` in the places, `remember_place` with `x`/`y`, `nav.knobs`) and says
  "update quack-nav" instead of misbehaving.
- The page is the duck's controls on the home network: whoever reaches it
  can drive the duck unless a token is set — said in the README, logged at
  start.
- The deferred items ([../todo.md](../todo.md)) are this repository's to
  carry: remote access, the chat, the app, rooms, no-go zones, map
  management.
