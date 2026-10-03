# Changelog

Tutte le modifiche rilevanti a quack-control. Il formato segue
[Keep a Changelog](https://keepachangelog.com/it-IT/1.1.0/); le versioni
seguono il [Semantic Versioning](https://semver.org/lang/it/) (prima della
1.0: una versione minore può rompere qualcosa). Copia inglese:
[CHANGELOG.md](CHANGELOG.md).

## [Unreleased]

### Modificato

- Il percorso fatto mostra solo le ultime due corse (una corsa finisce
  dove la papera resta ferma 20 s), non tutti i percorsi da quando
  quack-control è partito.

### Aggiunto

- **Un pacchetto d'installazione per ogni release.** La CI impacchetta
  `quack-control-<versione>-aarch64-linux.tar.gz` (con il suo `.sha256`)
  accanto al binario nudo, e un tag `v*` li allega entrambi: il binario,
  la unit, l'account di servizio, la configurazione d'esempio,
  `install-on-duck.sh` e un `README-install.it.md` passo per passo (e
  `.md`), così l'anatra si installa da un download (README, "Installare da
  una release"). `scripts/package.sh <versione> <binario> <cartella>` lo
  impacchetta in locale. `install-on-duck.sh` gira dal pacchetto o da una
  copia del repository, accetta `--dry-run` e non fallisce più con il bash
  3.2 di macOS quando `SSH_OPTS` è vuoto.
- Badge per quello che l'anatra fa di sua iniziativa (quack-nav
  2026-10-02): si muove da sola e perché, forse spostata, a riposo con
  l'ultimo controllo, fermata dall'utente; i luoghi per stato
  (utilizzabile, in attesa, su un'altra mappa, scaduto), Go solo dove può
  andare.
- La mappa ruota: ⟲ ⟳ di 90°, due dita liberamente, ⊞ mette i muri in
  squadra con lo schermo (l'angolo con le proiezioni dei muri più nette),
  N torna alla mappa com'è disegnata; salvato per mappa nel browser.
- **Il piano di controllo locale** (ADR 0001, 2026-10-01): un binario,
  `quack-control`, che serve una pagina sulla rete di casa (default
  `0.0.0.0:8090`, non l'8080 della console di mediad; un token facoltativo, HTTP in chiaro solo per la LAN),
  con un adattatore per ogni demone gestito.
- **quack-nav**: mappa, posa, incertezza, scia, rotta, meta, dislivelli e
  luoghi dal vivo; tocca per andare o per dare un nome a un punto, stop,
  luoghi, avvio / stop / completamento dell'esplorazione, la riga di stato
  — attraverso il contratto di controllo di quack-nav, con le chiamate
  della vista mappa controllate una per una. Una vista avanzata con ogni
  strumento del catalogo (un modulo dal suo JSON Schema, conferma per
  quelli che agiscono) e l'editor delle manopole (`nav.knobs`, Apply =
  `nav.restart`).
- **La telecamera** (adattatore `mediad`): una piccola finestra sopra la
  mappa con la telecamera sulla testa dell'anatra — in qualunque angolo,
  richiudibile, a schermo intero (con un ripiego dove manca lo schermo
  intero per gli elementi, come sull'iPhone), fotogrammi al secondo ed
  età, "camera unavailable" e perché. Interrogata solo con la finestra
  aperta e la pagina visibile. `GET /api/<servizio>/snapshot` serve un
  JPEG dritto (al più 640 px) dal `media.frame` di mediad
  (`/run/mediad/media.sock`), una lettura alla volta e al più cinque al
  secondo per tutte le pagine insieme. Sul gemello risponde alla stessa
  chiamata il viewer di quack-nav su `$STATE/media.sock`. Nel config di
  default.
- **quacksat**: un segnaposto che dice di non essere disponibile, e che
  cosa quacksat dovrebbe esporre.
- La vista Services; una unit systemd (utente `quackctl` in `robot`),
  `scripts/install-on-duck.sh`, `scripts/cross-build.sh` (aarch64, glibc
  >= 2.31), un workflow di CI.
