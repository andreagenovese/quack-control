# To do

What the control plane carries next. The items deferred by the decision of
2026-10-01 (local management first) point at quack-nav's study,
[map-app.md](https://github.com/andreagenovese/quacknav/blob/main/docs/study/map-app.md), and stay the plan as written there. Italian:
[todo.it.md](todo.it.md).

## Deferred by the decision of 2026-10-01

- [ ] 2026-10-01: remote access over Pollen's channel — `quack-linkd`, a
      second face of this control plane registered on the rendezvous as a
      producer of its own, the same adapters and allowlists behind it
      (study §1.2, §4 A; ADR 0001).
- [ ] 2026-10-01: the extension-route ask upstream — `mediad` forwarding
      a registered prefix to our socket (study §4 C).
- [ ] 2026-10-01: the Hugging Face token for that face — its own
      device-flow login, never `/etc/robot/hf-token`, narrower scopes if
      HF allows (study §5).
- [ ] 2026-10-01: the questions for Pollen — a third-party producer, the
      extension route, CORS, the `POST /send` size limit, scopes (study §8).
- [ ] 2026-10-01: quacksat in the control plane — a control socket
      (`/run/quacksat/control.sock`: `sat.status`, `sat.config`) so its
      adapter can stop being a placeholder (`src/adapters/quacksat.rs`).
- [ ] 2026-10-01: the chat with quacksat — `chat.sock` with `chat.say`,
      streamed replies, `chat.subscribe`, the turn lock shared with the
      wake word; a conversation pane in the page (study §3, §7 phase 2).
- [ ] 2026-10-01: voice from the page or the app — the platform's
      recogniser, or quacksat's STT over an Opus upload (study §3, §8).
- [ ] 2026-10-01: the Tauri 2 app, desktop then iOS/Android, with
      notifications (arrived, fell, needs a name) (study §6, §7 phase 6).
- [ ] 2026-10-01: rooms as areas — draw a polygon or accept a suggested
      one; places become areas in quack-nav (study §7 phase 3).
- [ ] 2026-10-01: no-go zones and virtual walls — drawn here, a cost layer
      in quack-nav's planner and explorer (study §7 phase 4).
- [ ] 2026-10-01: map management — list, rename, delete, switch, export;
      needs `map_rename`/`map_delete` in quack-nav (study §7 phase 5).

## The page

- [ ] Show who started the running job (the page, the voice, the
      homecoming) — quack-navd does not say yet (study §5 "Two drivers").
- [ ] Answer the explorer's "where are we?" from the page (`nav.take_question`
      is quacksat's today; the page shows `question_pending` only).
- [ ] Test the interactive calls on the twin (go here, stop, explore,
      teach, the knobs and Apply with `twin.sh restart-navd`); only the
      reads have been run against it (2026-10-01).
- [ ] 2026-10-01: camera video — WebRTC from mediad (its `webrtcsink`
      stream, H.264, and the signalling on 8443) instead of polled JPEG
      snapshots: smooth video at a fraction of the CPU, but a WebRTC
      client in the page and a path through mediad's signalling (and its
      no-auth model) to decide on; the snapshots stay as the fallback.
- [ ] 2026-10-01: port 8080 is mediad's console's too (daemon-v0.15.0
      `--web-port`): pick quack-control's port on the duck for good (the
      example config, the install script, the docs).
- [ ] Try the camera window on a real phone (iPhone Safari's overlay
      fallback, Android Chrome's full screen) and on the duck (the cost of
      a 1280x720 frame → JPEG on the RK3566).
- [ ] A screenshot test in CI (headless Chrome against a fake quack-navd).
