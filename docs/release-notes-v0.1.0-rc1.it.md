# quack-control v0.1.0-rc1 — note di rilascio

2026-10-03. Copia inglese: [release-notes-v0.1.0-rc1.md](release-notes-v0.1.0-rc1.md).
L'elenco completo delle modifiche è in [CHANGELOG.it.md](../CHANGELOG.it.md).

## Cos'è

La **prima release candidate** di quack-control: un piano di controllo
locale per i demoni del Microduck di Pollen Robotics. Un binario gira
sull'anatra accanto ai demoni che gestisce e serve **una pagina sulla rete
di casa** — la si apre da un telefono o da un portatile — che mostra dal
vivo la mappa costruita dall'anatra e la guida. Parla con ciascun demone
sui suoi socket unix, con un adattatore per tipo: quack-navd di
[quack-nav](https://github.com/andreagenovese/quacknav) e mediad di Pollen
(la telecamera) ora, quacksat poi. Perché un repository a sé:
[ADR 0001](adr/0001-a-control-plane-of-its-own.it.md).

**Provato solo contro il gemello MuJoCo di quack-nav, non su un'anatra
vera.** Il gemello serve i veri socket di quack-navd e risponde alla
chiamata della telecamera come fa mediad; l'anatra arriva a dicembre 2026.

Progetto indipendente, nessuna affiliazione con Pollen Robotics o Hugging
Face.

## Che cosa fa la pagina

**Mappa** (la vista iniziale):

- la mappa dal vivo (muri, pavimento, ignoto), la posa e la direzione
  dell'anatra — rossa quando è persa o seduta — la sua incertezza (una
  sigma), il percorso fatto nelle **ultime due corse**, la rotta del lavoro
  in corso, la meta, i dislivelli e gli ostacoli nei suoi registri, i
  luoghi con un nome;
- **tocca la mappa** → **Go here** (chiede prima) o **Name** per dare un
  nome al punto; un grande **STOP** che ferma qualunque cosa l'anatra stia
  facendo, anche quello che ha iniziato da sola;
- **esplorazione**: Start, Stop, **Exploration complete** (chiede prima);
- **i luoghi per stato** — utilizzabile, in attesa che l'anatra si
  ritrovi, su un'altra mappa, scaduto — con la distanza, Go (solo dove può
  andare), Forget, e Teach per il punto dove l'anatra si trova;
- **badge** per quello che l'anatra fa di sua iniziativa: a riposo (con
  l'ultimo controllo), forse spostata (posa non affidabile: si ritrova
  prima del prossimo lavoro), si muove da sola e perché (movimento avviato
  da sé), fermata da te; e la riga di stato (modalità di mappatura,
  localizzata o in ricerca, il lavoro e il suo esito, quanta casa è
  mappata);
- **rotazione della mappa**: ⟲ ⟳ di 90°, rotazione con due dita, ⊞ mette i
  muri in squadra con lo schermo, N torna alla mappa com'è disegnata —
  salvata per mappa nel browser; sposta, pizzica, adatta, segui l'anatra;
- **la telecamera**: una piccola finestra sopra la mappa con quello che
  vede la telecamera sulla testa dell'anatra (`media.frame` di mediad, un
  JPEG dritto di al più 640 px), in qualunque angolo, a **schermo intero**
  (sull'iPhone copre la pagina), fotogrammi al secondo ed età, "camera
  unavailable" e perché. I fotogrammi si chiedono solo con la finestra
  aperta e la pagina visibile, al più cinque al secondo per tutte le pagine
  insieme.

**Services**: ogni demone gestito e se risponde.

**Advanced** (chiusa, dietro un avviso):

- **ogni strumento di quack-navd** in `nav.catalog`, con un modulo generato
  dal suo JSON Schema (o JSON grezzo) e la risposta grezza; uno strumento
  che agisce chiede conferma, e il server lo rifiuta senza;
- **l'editor delle manopole**: le manopole `QK_*`/`MAPLOC_*` di quack-navd
  raggruppate, con valore di default, salvato e in uso; **Apply** è
  `nav.restart` (ferma quello che l'anatra sta facendo, salva la mappa, e
  il ritorno a casa riparte come all'avvio).

## Sicurezza

- **HTTP in chiaro, solo per la rete di casa.** Chi raggiunge la porta può
  aprire la pagina, e la pagina guida l'anatra: mai inoltrarla su
  internet.
- **Un token facoltativo** (`QC_TOKEN` in `/etc/robot/quack-control.env`,
  solo root): ogni chiamata e il flusso dal vivo lo richiedono; la pagina
  lo chiede una volta. Viaggia in chiaro come tutto il resto — tiene fuori
  il telefono di un ospite, non chi intercetta il Wi-Fi.
- Ogni chiamata è un `POST` JSON, quindi un altro sito non può guidare
  l'anatra attraverso un browser di casa; la vista mappa raggiunge sette
  strumenti nelle forme che usa; il servizio gira come l'utente non
  privilegiato `quackctl`, nel gruppo `robot`.
- **Porta 8090**, perché la console di mediad ascolta sulla 8080.

Dettagli: [README, Sicurezza](../README.it.md#sicurezza).

## Installazione

Ogni release porta un pacchetto d'installazione,
`quack-control-0.1.0-rc1-aarch64-linux.tar.gz` con il suo `.sha256` — il
binario per la scheda (aarch64, glibc ≥ 2.31), la unit systemd, l'account
di servizio, la configurazione d'esempio, `install-on-duck.sh` e un
`README-install.it.md` passo per passo (inglese: `README-install.md`). È
allegato anche il binario nudo, `quack-control-aarch64-linux`, con il suo
sha256.

Prima si installa quack-nav, poi si segue
[Installare da una release](../README.it.md#installare-da-una-release):
scarica, `shasum -a 256 -c`, scompatta, `./install-on-duck.sh --dry-run
microduck@<anatra>` per vedere ogni comando, poi senza `--dry-run`. Si
apre `http://<indirizzo dell'anatra>:8090/`.

## Compatibilità

| quack-control | quack-nav | telecamera | quacksat |
|---|---|---|---|
| 0.1.0-rc1 | **v0.2.0-rc2 o successiva** | mediad di Pollen daemon-v0.14.4+ (`media.frame`), o `eye.py` del gemello | ancora nessun socket di controllo |

- **quack-nav v0.2.0-rc2 o successiva** per la pagina completa:
  `nav.knobs`, `nav.restart`, i luoghi insegnati in un punto, e i campi di
  riposo / posa non affidabile / movimento avviato da sé. Con un quack-navd
  più vecchio la pagina si riduce — mappa, go, stop ed esplorazione
  funzionano; quello che manca risponde "update quack-nav" e i badge
  restano spenti.
- **La telecamera** richiede mediad di Pollen (daemon-v0.14.4 o
  successivo, che serve `media.frame`), o sul gemello il viewer di
  quack-nav (`scripts/twin/viewer/eye.py`, `VIEWER=on`).
- **quacksat** risulta non disponibile finché non ha un socket di
  controllo ([todo](todo.it.md)).

## Limiti noti

Da [docs/todo.it.md](todo.it.md):

- non ancora su un'anatra vera; la finestra della telecamera non ancora
  provata su un telefono vero (il ripiego di Safari su iPhone, lo schermo
  intero di Chrome su Android) né il costo di un fotogramma → JPEG
  sull'RK3566;
- la telecamera sono istantanee JPEG interrogate, non video (WebRTC da
  mediad è per dopo);
- la pagina non dice chi ha avviato il lavoro in corso (la pagina, la voce,
  il ritorno a casa), e non può rispondere al "dove siamo?"
  dell'esploratore;
- solo rete di casa: ancora nessun accesso remoto (previsto attraverso il
  canale di Pollen); ancora niente stanze, zone vietate o gestione delle
  mappe;
- ancora nessun test con screenshot in CI; 40 test unitari (HTTP, token,
  route, ogni controllo sulle chiamate, la telecamera).
