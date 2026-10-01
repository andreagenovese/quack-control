# Changelog

Tutte le modifiche rilevanti a quack-control. Il formato segue
[Keep a Changelog](https://keepachangelog.com/it-IT/1.1.0/); le versioni
seguono il [Semantic Versioning](https://semver.org/lang/it/) (prima della
1.0: una versione minore può rompere qualcosa). Copia inglese:
[CHANGELOG.md](CHANGELOG.md).

## [Unreleased]

### Aggiunto

- **Il piano di controllo locale** (ADR 0001, 2026-10-01): un binario,
  `quack-control`, che serve una pagina sulla rete di casa (default
  `0.0.0.0:8080`, un token facoltativo, HTTP in chiaro solo per la LAN),
  con un adattatore per ogni demone gestito.
- **quack-nav**: mappa, posa, incertezza, scia, rotta, meta, dislivelli e
  luoghi dal vivo; tocca per andare o per dare un nome a un punto, stop,
  luoghi, avvio / stop / completamento dell'esplorazione, la riga di stato
  — attraverso il contratto di controllo di quack-nav, con le chiamate
  della vista mappa controllate una per una. Una vista avanzata con ogni
  strumento del catalogo (un modulo dal suo JSON Schema, conferma per
  quelli che agiscono) e l'editor delle manopole (`nav.knobs`, Apply =
  `nav.restart`).
- **quacksat**: un segnaposto che dice di non essere disponibile, e che
  cosa quacksat dovrebbe esporre.
- La vista Services; una unit systemd (utente `quackctl` in `robot`),
  `scripts/install-on-duck.sh`, `scripts/cross-build.sh` (aarch64, glibc
  >= 2.31), un workflow di CI.
