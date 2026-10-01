# ADR 0001: un piano di controllo a sé

Stato: accettata, 2026-10-01. Inglese:
[0001-a-control-plane-of-its-own.md](0001-a-control-plane-of-its-own.md).

## Contesto

Lo studio di quack-nav su un'app della mappa
([docs/study/map-app.it.md](https://github.com/andreagenovese/quacknav/blob/main/docs/study/map-app.it.md))
chiedeva che cosa servisse a un'app come quella di un robot aspirapolvere
— la mappa dal vivo, tocca-e-vai, luoghi, esplorazione, una chat con
l'agente dell'anatra — raggiunta da fuori casa sul canale di Pollen.
L'utente ha deciso il 2026-10-01: **per ora solo la gestione locale**, una
pagina semplice in un browser sulla rete di casa; il resto resta il piano.
Lo stesso giorno, una seconda decisione: la pagina non è di quack-nav. È un
piano di controllo per ogni demone sull'anatra — quack-nav ora, quacksat e
i futuri poi — in un repository a sé, fatto per incontrare più avanti il
canale di Pollen.

## Decisione

1. **Un repository separato, `quack-control`**, e un piccolo demone
   sull'anatra. quack-nav tiene solo la sua parte, un **contratto di
   controllo** che qualsiasi client può usare (`docs/control-contract.it.md`
   di quack-nav: i socket, `nav.catalog`/`nav.call`, il flusso della mappa,
   `nav.knobs`, `nav.restart`). La navigazione resta senza HTTP, senza
   pagina e senza un secondo binario; il piano di controllo può gestire
   demoni che non sono di quack-nav, ed essere rilasciato con il suo ritmo.
2. **Un adattatore per tipo di demone** (`src/adapters/`): un trait con un
   nome, un tipo, un controllo di salute, un avvio per le sue corsie in
   background, e un instradatore per `/api/<servizio>/…`. Il server fa
   l'HTTP, il token e la pagina. L'adattatore di quack-nav parla i due
   socket di quack-navd; quello di quacksat è un segnaposto finché quacksat
   non espone un socket di controllo. Un nuovo demone è un nuovo
   adattatore e un `kind` nel config.
3. **Prima il locale, HTTP in chiaro sulla LAN**, con un token
   facoltativo, una regola solo-JSON contro le richieste da altri siti, e
   liste ammesse in ogni adattatore: le chiamate della vista mappa nelle
   forme che usa; ogni strumento del catalogo nella vista avanzata, quelli
   che agiscono solo confermati. Nessuna porta è pensata per stare su
   internet.
4. **Minimo e sincrono**, come i demoni che gestisce: thread std, un
   sottoinsieme di HTTP/1.1 scritto a mano, server-sent events, una pagina
   HTML inclusa senza build e senza dipendenze dalla rete.

## La strada verso il canale di Pollen

Le opzioni dello studio restano come scritte (§4 lì):

- **Opzione A — `quack-linkd`**: un nostro adattatore registrato sul
  rendezvous di Pollen come producer a sé, con il suo token, che risponde
  alle buste `peer {rpc}`. In questo repository diventa una seconda
  **faccia** dello stesso piano di controllo: gli stessi adattatori e le
  stesse liste ammesse, raggiunti dalla corsia del rendezvous invece che
  dall'HTTP della LAN. Le chiamate della pagina hanno già la forma JSON-RPC
  (`{name, args}` verso `nav.call`), quindi la corsia porta le stesse righe.
- **Opzione C — rotte di estensione upstream**: se Pollen accetta in
  `mediad` una tabella di rotte guidata dai dati (un prefisso, un socket),
  la faccia si riduce a un manifesto e l'app parla attraverso la sessione
  di Pollen. quack-control registrerebbe allora un suo socket, e i suoi
  adattatori restano.
- Chiedere prima a Pollen (§8 dello studio) resta il primo passo per
  entrambe.

## Conseguenze

- Due repository da installare sull'anatra; l'installatore di
  quack-control presuppone il gruppo `robot` e i socket di quack-nav.
- Un client del contratto può restare indietro: quack-control controlla
  ciò che gli serve (`at` nei luoghi, `remember_place` con `x`/`y`,
  `nav.knobs`) e dice "aggiorna quack-nav" invece di comportarsi male.
- La pagina sono i comandi dell'anatra sulla rete di casa: chi la
  raggiunge può guidare l'anatra se non c'è un token — detto nel README,
  scritto nel log all'avvio.
- Le voci rimandate ([../todo.it.md](../todo.it.md)) sono di questo
  repository: l'accesso remoto, la chat, l'app, le stanze, le zone vietate,
  la gestione delle mappe.
