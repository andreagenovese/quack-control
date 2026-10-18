# quack-control v0.1.0-rc2 — note di rilascio

2026-10-18. Copia inglese: [release-notes-v0.1.0-rc2.md](release-notes-v0.1.0-rc2.md).
L'elenco completo delle modifiche è in [CHANGELOG.it.md](../CHANGELOG.it.md);
le note della candidata precedente sono in [release-notes-v0.1.0-rc1.it.md](release-notes-v0.1.0-rc1.it.md).

## Cos'è

La **seconda release candidate** di quack-control, il piano di controllo
locale per i demoni del Microduck di Pollen Robotics. **Provata solo contro
il gemello MuJoCo di quack-nav, non su un'anatra fisica**; l'anatra arriva
a dicembre 2026.

Progetto indipendente, nessuna affiliazione con Pollen Robotics o Hugging
Face.

## Cosa è cambiato dalla rc1

- **La casa in 3D (sperimentale).** Una scheda **3D** disegna una casa
  costruita fuori dall'anatra con i colori della sua camera: girare,
  spostare, zoom, isometrica o dall'alto, taglio in altezza per vedere le
  stanze, il percorso dell'anatra nella casa, e l'anatra dal vivo quando la
  casa è stata costruita sulla mappa su cui si trova. La casa viene da
  `scripts/map3d` di quack-nav (v0.3.0-rc2): registra accanto a un'anatra
  che esplora, costruisce su un Mac o un PC, e `send.sh microduck@<anatra>
  out/house3d.bin` mette il file in `/var/lib/quack-control/house3d.bin`,
  che la pagina rilegge quando cambia (↻). Sull'anatra non si costruisce
  niente. Senza il file la scheda dice come farne uno. Dettagli:
  [README, La casa in 3D](../README.it.md#la-casa-in-3d).
- **Lo strato sulla mappa.** Con quack-nav v0.3.0-rc2 la mappa mostra anche
  ciò che il sensore di profondità ha trovato d'ingombro all'altezza
  dell'anatra (marrone) e i piani di tavoli o i bordi dei letti con il
  pavimento sotto (beige): ciò che la navigazione aggira come muri. Viaggia
  con la griglia, solo quando cambia.
- CI: azioni su Node 24.

## Installazione

Come la rc1: il pacchetto `quack-control-0.1.0-rc2-aarch64-linux.tar.gz`
con il suo `.sha256`, poi [Installare da una release](../README.it.md#installare-da-una-release)
(prima `./install-on-duck.sh --dry-run microduck@<anatra>`). Una
configurazione esistente si tiene; il servizio `house` è tra i servizi di
default, quindi a una configurazione scritta dalla rc1 va aggiunto in
`/etc/robot/quack-control.toml` per vedere la scheda 3D:

```toml
[[service]]
name = "house"
kind = "house3d"
house_file = "/var/lib/quack-control/house3d.bin"
```

## Compatibilità

| quack-control | quack-nav | camera | quacksat |
|---|---|---|---|
| 0.1.0-rc2 | **v0.2.0-rc2 o successivo**; la casa 3D da `scripts/map3d` della v0.3.0-rc2 | mediad di Pollen daemon-v0.14.4+, o `eye.py` del gemello | ancora nessun socket di controllo |

## Limiti noti

Quelli della rc1 ([release-notes-v0.1.0-rc1.it.md](release-notes-v0.1.0-rc1.it.md)),
e:

- la casa 3D è un'anteprima: sul gemello i suoi colori sono sintetici, e
  gli assi e gli intrinseci della camera vanno verificati sull'anatra;
- lo strato si disegna come lo legge quack-nav; lungo alcuni muri si vede
  come un bordo marrone, che la navigazione legge comunque come muro.
