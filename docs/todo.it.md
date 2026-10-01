# Da fare

Ciò che il piano di controllo porta avanti. Le voci rimandate dalla
decisione del 2026-10-01 (prima la gestione locale) puntano allo studio di
quack-nav, [map-app.it.md](https://github.com/andreagenovese/quacknav/blob/main/docs/study/map-app.it.md), e restano il piano come scritto lì.
Inglese: [todo.md](todo.md).

## Rimandate dalla decisione del 2026-10-01

- [ ] 2026-10-01: l'accesso remoto sul canale di Pollen — `quack-linkd`,
      una seconda faccia di questo piano di controllo registrata sul
      rendezvous come producer a sé, con dietro gli stessi adattatori e le
      stesse liste ammesse (studio §1.2, §4 A; ADR 0001).
- [ ] 2026-10-01: la richiesta upstream di rotte di estensione — `mediad`
      che inoltra un prefisso registrato al nostro socket (studio §4 C).
- [ ] 2026-10-01: il token Hugging Face di quella faccia — un suo login
      device-flow, mai `/etc/robot/hf-token`, scope più stretti se HF lo
      consente (studio §5).
- [ ] 2026-10-01: le domande per Pollen — un producer di terzi, la rotta di
      estensione, CORS, il limite di dimensione di `POST /send`, gli scope
      (studio §8).
- [ ] 2026-10-01: quacksat nel piano di controllo — un socket di controllo
      (`/run/quacksat/control.sock`: `sat.status`, `sat.config`) perché il
      suo adattatore smetta di essere un segnaposto (`src/adapters/quacksat.rs`).
- [ ] 2026-10-01: la chat con quacksat — `chat.sock` con `chat.say`,
      risposte in streaming, `chat.subscribe`, il lock di turno condiviso
      con la parola di attivazione; un riquadro di conversazione nella
      pagina (studio §3, §7 fase 2).
- [ ] 2026-10-01: la voce dalla pagina o dall'app — il riconoscitore della
      piattaforma, o lo STT di quacksat con un upload Opus (studio §3, §8).
- [ ] 2026-10-01: l'app Tauri 2, prima desktop poi iOS/Android, con le
      notifiche (arrivata, caduta, chiede un nome) (studio §6, §7 fase 6).
- [ ] 2026-10-01: le stanze come aree — disegnare un poligono o accettarne
      uno suggerito; in quack-nav i luoghi diventano aree (studio §7 fase 3).
- [ ] 2026-10-01: zone vietate e muri virtuali — disegnati qui, uno strato
      di costo nel pianificatore e nell'esploratore di quack-nav (studio §7
      fase 4).
- [ ] 2026-10-01: la gestione delle mappe — elenca, rinomina, cancella,
      cambia, esporta; servono `map_rename`/`map_delete` in quack-nav
      (studio §7 fase 5).

## La pagina

- [ ] Mostrare chi ha avviato il lavoro in corso (la pagina, la voce, la
      homecoming) — quack-navd non lo dice ancora (studio §5 "Due guidatori").
- [ ] Rispondere dalla pagina al "dove siamo?" dell'esploratore
      (`nav.take_question` oggi è di quacksat; la pagina mostra solo
      `question_pending`).
- [ ] Provare sul gemello le chiamate interattive (go here, stop,
      esplorazione, teach, le manopole e Apply con `twin.sh restart-navd`);
      contro il gemello sono girate solo le letture (2026-10-01).
- [ ] 2026-10-01: video della telecamera — WebRTC da mediad (il suo
      flusso `webrtcsink`, H.264, e il signalling su 8443) invece degli
      snapshot JPEG interrogati: video fluido con una frazione della CPU,
      ma un client WebRTC nella pagina e una strada attraverso il
      signalling di mediad (e il suo modello senza autenticazione) da
      decidere; gli snapshot restano come ripiego.
- [ ] 2026-10-01: la porta 8080 è anche della console di mediad
      (`--web-port` di daemon-v0.15.0): scegliere per sempre la porta di
      quack-control sull'anatra (config d'esempio, script d'installazione,
      documentazione).
- [ ] Provare la finestra della telecamera su un telefono vero (il
      ripiego a sovrapposizione di Safari su iPhone, lo schermo intero di
      Chrome su Android) e sull'anatra (il costo di un fotogramma
      1280x720 → JPEG sul RK3566).
- [ ] Un test a screenshot nella CI (Chrome headless contro un quack-navd finto).
