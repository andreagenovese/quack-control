# quack-control

Un piano di controllo locale per i demoni del Microduck di Pollen Robotics:
**una pagina in un browser sulla rete di casa** — un telefono, un portatile
— che mostra dal vivo la mappa costruita dall'anatra e la guida. Gira
sull'anatra accanto ai demoni che gestisce e parla con ciascuno sui suoi
socket unix, con un adattatore per tipo:
[quack-nav](https://github.com/andreagenovese/quacknav) ora, quacksat e
altri poi. Inglese: [README.md](README.md).

Progetto indipendente, nessuna affiliazione con Pollen Robotics o Hugging
Face. Stato: 0.1.0, non rilasciato; provato contro il gemello MuJoCo (per
ora in sola lettura), non ancora su un'anatra.

## Che cosa fa la pagina

**Mappa** (la vista iniziale):

- la mappa dal vivo (muri, pavimento, ignoto), la posa e la direzione
  dell'anatra — rossa quando è persa o seduta — e la sua incertezza (una
  sigma), il percorso fatto, la rotta del lavoro in corso, la meta, i
  dislivelli e gli ostacoli nei suoi registri, i luoghi con un nome;
- sposta e ingrandisci (trascina, pizzica, rotella), adatta, segui
  l'anatra;
- tocca la mappa → **Go here** (chiede prima) o **Name** per dare un nome
  al punto; un grande **STOP** che ferma qualunque cosa stia facendo;
- i luoghi: l'elenco con la distanza, **Go**, **Forget**, e **Teach** per
  il punto in cui sta l'anatra;
- l'esplorazione: **Start**, **Stop**, **Exploration complete** (chiede
  prima);
- la riga di stato: modo di mappatura e mappa, localizzata o in ricerca, il
  lavoro (esplora, va da qualche parte, com'è finito), quanta casa è
  mappata e in quante sessioni, il suggerimento del demone; un quack-navd
  che rifiuta dice perché, uno irraggiungibile lo dice, e la pagina si
  riconnette da sola.

**Services**: ogni demone gestito e se risponde. quacksat risulta non
disponibile: non ha ancora un socket di controllo ([docs/todo.it.md](docs/todo.it.md)).

**Advanced** (chiusa, dietro un avviso):

- **All tools**: ogni strumento che quack-navd elenca in `nav.catalog`,
  con un modulo costruito dal suo JSON Schema (o JSON grezzo), e la
  risposta grezza; uno strumento che agisce chiede conferma, e il server
  lo rifiuta senza;
- **Knobs**: le manopole d'ambiente di quack-navd (`QK_*`, `MAPLOC_*`)
  raggruppate, con il default, cosa dice il codice, il valore salvato e
  quello in uso; modifica, ripristina una, ripristina tutte, e **Apply**,
  che riavvia quack-navd perché le legga (ferma ciò che l'anatra sta
  facendo, salva la mappa, e la homecoming gira come all'avvio).

## Farlo girare

Rust 1.89 o più recente.

```sh
cargo build --release
target/release/quack-control /etc/robot/quack-control.toml
```

`quack-control.example.toml`:

```toml
bind = "0.0.0.0:8080"
# token = "una-stringa-lunga-e-casuale"   # meglio: QC_TOKEN nell'ambiente

[[service]]
name = "quack-nav"
kind = "quack-nav"
nav_socket = "/run/quack-nav/nav.sock"
map_socket = "/run/quack-nav/map.sock"

[[service]]
name = "quacksat"
kind = "quacksat"
```

Senza file: questi default. Poi si apre `http://<indirizzo dell'anatra>:8080/`.

### Contro il gemello

Il gemello MuJoCo di quack-nav serve i socket di quack-navd sotto la sua
directory `STATE` (`/tmp/quack-twin` di default). Un config che punta lì,
solo su questa macchina:

```toml
bind = "127.0.0.1:8080"

[[service]]
name = "quack-nav"
kind = "quack-nav"
nav_socket = "/tmp/quack-twin/nav.sock"
map_socket = "/tmp/quack-twin/map.sock"
```

```sh
target/release/quack-control twin.toml
open http://127.0.0.1:8080/
# senza browser: il flusso dal vivo, e i servizi
curl -N http://127.0.0.1:8080/api/quack-nav/events
curl http://127.0.0.1:8080/api/services
```

Le manopole sul gemello: quack-navd scrive `$STATE/knobs.env`, e Apply
risponde che non è sotto systemd — `scripts/twin/twin.sh restart-navd` in
quack-nav lo riavvia con quelle.

## Sicurezza

**HTTP in chiaro, solo per la rete di casa.** Chi raggiunge la porta può
aprire la pagina, e la pagina guida l'anatra. Non inoltrarla mai su
internet; l'accesso remoto è previsto invece sul canale di Pollen
([docs/todo.it.md](docs/todo.it.md)).

- **Il token** (`token`, o `QC_TOKEN` — l'unit legge
  `/etc/robot/quack-control.env`, solo root, modo 600): se impostato, ogni
  chiamata e il flusso dal vivo lo richiedono, come header `X-Token` o
  `?token=`; la pagina lo chiede una volta e lo tiene nel browser. Viaggia
  in chiaro sulla rete, come tutto il resto qui: tiene fuori il telefono di
  un ospite, non chi ascolta il Wi-Fi.
- **Un altro sito** non può guidare l'anatra attraverso il browser di chi
  è in casa: ogni chiamata è un `POST` con corpo JSON, che una pagina di
  un'altra origine non può mandare senza un preflight a cui quack-control
  non risponde mai.
- **Che cosa arriva all'anatra**: la vista mappa raggiunge sette strumenti
  nelle forme che usa (`robot.go_to` un punto, un luogo o stop; avvio,
  stop e completamento dell'esplorazione — mai `fresh`; insegnare e
  dimenticare un luogo; tre letture). La vista avanzata raggiunge ogni
  strumento del catalogo, e uno che agisce solo con `"confirmed": true`; le
  manopole solo attraverso `nav.knobs` di quack-navd, che controlla ogni
  valore contro il suo tipo.
- **Sull'anatra** quack-control gira come `quackctl`, nel gruppo `robot`
  (è ciò che chiedono i socket di quack-navd), senza privilegi e recintato
  (`systemd/quack-control.service`).

## Installarlo sull'anatra

Dopo quack-nav (quack-navd e il gruppo `robot`):

```sh
scripts/cross-build.sh                              # aarch64, glibc >= 2.31
scripts/install-on-duck.sh microduck@192.168.1.42   # binario, unit, account, config
```

| sull'anatra | da questo repo |
|---|---|
| `/usr/local/bin/quack-control` | il binario cross-compilato (1,5 MB) |
| `/etc/systemd/system/quack-control.service` | `systemd/quack-control.service` |
| `/etc/sysusers.d/quack-control.conf` (utente `quackctl`) | `systemd/sysusers.d/quack-control.conf` |
| `/etc/robot/quack-control.toml` | `quack-control.example.toml`, solo se manca |
| `/etc/robot/quack-control.env` | creato vuoto (`QC_TOKEN` commentato), root, 600, solo se manca |

`systemctl status quack-control`, `journalctl -u quack-control -f`.
Disinstallare: `sudo systemctl disable --now quack-control`, poi togliere
il binario, l'unit e il file sysusers.

## Com'è fatto

Un binario, thread std, niente runtime async e niente crate HTTP: qualche
centinaio di righe di HTTP/1.1 (`src/http.rs`), un thread per
connessione, server-sent events per i dati dal vivo e `POST` JSON per i
comandi. Dipendenze: serde, serde_json, toml, tracing, anyhow. La pagina è
un file HTML con JavaScript semplice e un canvas, incluso nel binario —
nessuna build, niente caricato dalla rete, funziona offline.

| file | che cosa fa |
|---|---|
| `src/server.rs` | le rotte, il token, la regola solo-JSON, i flussi di eventi |
| `src/adapters/mod.rs` | il trait `Adapter` e i tipi |
| `src/adapters/quack_nav/` | quack-navd: il flusso della mappa e l'interrogazione dello stato (`hub.rs`), i controlli su ogni chiamata (`bridge.rs`) |
| `src/adapters/quacksat.rs` | un segnaposto, e che cosa quacksat dovrebbe esporre |
| `src/page.html` | la pagina |

L'adattatore quack-nav usa il **contratto di controllo** di quack-nav
(`docs/control-contract.it.md` di quack-nav): `nav.catalog`/`nav.call` sul
socket nav, `robot.map` sul socket della mappa, `nav.knobs` e
`nav.restart`. Perché un repository a sé, il modello ad adattatori e la
strada verso il canale di Pollen:
[docs/adr/0001-a-control-plane-of-its-own.it.md](docs/adr/0001-a-control-plane-of-its-own.it.md).

```sh
cargo test --release      # 32 test: HTTP, token, rotte, ogni controllo sulle chiamate
```

## Licenza

Apache-2.0 ([LICENSE](LICENSE), [NOTICE](NOTICE)).
