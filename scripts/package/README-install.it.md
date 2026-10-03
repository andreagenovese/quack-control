# Installare quack-control sull'anatra

Questo pacchetto installa `quack-control`, il piano di controllo locale
del Microduck (una pagina sulla rete di casa che mostra la mappa dal vivo
e guida l'anatra), sulla scheda dell'anatra (Radxa Zero 3, Linux aarch64,
Debian 13) dal tuo computer, via ssh. Non servono né una copia del
repository né una compilazione. Copia inglese:
[README-install.md](README-install.md).

Cosa serve:

- un'anatra con quack-nav già installato (quack-navd e il gruppo `robot`
  a cui appartengono i suoi socket);
- accesso ssh all'anatra con un account che ha `sudo` (`microduck`
  sull'immagine della scheda; le immagini più vecchie avevano `radxa`);
- sul tuo computer: `bash`, `ssh`, `scp`, `tar` e `shasum` o `sha256sum`
  (macOS e Linux li hanno tutti).

## 1. Scaricare e verificare

Da <https://github.com/andreagenovese/quack-control/releases>, il
pacchetto e il suo checksum. Questo pacchetto è `@VERSION@`; per uno più
nuovo si mette in `V` la sua versione (il tag senza la `v`):

```sh
V=@VERSION@
gh release download "v$V" --repo andreagenovese/quack-control \
    --pattern "quack-control-$V-aarch64-linux.tar.gz*"
# oppure, senza gh:
curl -LO "https://github.com/andreagenovese/quack-control/releases/download/v$V/quack-control-$V-aarch64-linux.tar.gz"
curl -LO "https://github.com/andreagenovese/quack-control/releases/download/v$V/quack-control-$V-aarch64-linux.tar.gz.sha256"

shasum -a 256 -c "quack-control-$V-aarch64-linux.tar.gz.sha256"   # oppure sha256sum -c
```

Deve stampare `OK`. Poi si scompatta:

```sh
tar xzf "quack-control-$V-aarch64-linux.tar.gz"
cd "quack-control-$V"
```

## 2. Installare

```sh
./install-on-duck.sh --dry-run microduck@192.168.1.42   # facoltativo: stampa cosa farebbe
./install-on-duck.sh microduck@192.168.1.42
```

Copia i file sull'anatra, poi lì con `sudo`:

| sull'anatra | da questo pacchetto |
|---|---|
| `/usr/local/bin/quack-control` | `bin/quack-control` |
| `/etc/systemd/system/quack-control.service` | `systemd/quack-control.service` |
| `/etc/sysusers.d/quack-control.conf` (utente `quackctl`) | `systemd/sysusers.d/quack-control.conf` |
| `/etc/robot/quack-control.toml` — **solo se non c'è** | `quack-control.example.toml` |
| `/etc/robot/quack-control.env` — **solo se non c'è** | creato con `QC_TOKEN` commentato, root, modo 600 |

Poi abilita e riavvia il servizio, stampando ogni comando, e stampa
l'indirizzo della pagina. `SSH_OPTS="-p 2222"` passa opzioni a ssh e scp.

## 3. La configurazione

`/etc/robot/quack-control.toml` (si modifica sull'anatra con
`sudo nano /etc/robot/quack-control.toml`):

| chiave | nell'esempio | che cos'è |
|---|---|---|
| `bind` | `0.0.0.0:8090` | dove ascolta la pagina: tutte le interfacce, porta 8090. HTTP semplice: mai inoltrarla su internet |
| `[[service]] kind = "quack-nav"`: `nav_socket`, `map_socket` | `/run/quack-nav/nav.sock`, `/run/quack-nav/map.sock` | i due socket di quack-navd (il suo `socket` e `[maploc] socket`) |
| `[[service]] kind = "mediad"`: `media_socket` | `/run/mediad/media.sock` | la telecamera della testa (mediad di Pollen) |
| `[[service]] kind = "quacksat"` | — | per ora un segnaposto |

**Il token** (consigliato): va in `/etc/robot/quack-control.env`, non
nella configurazione leggibile da tutti:

```sh
sudo sh -c 'echo "QC_TOKEN=$(tr -dc a-z0-9 </dev/urandom | head -c 32)" > /etc/robot/quack-control.env'
sudo cat /etc/robot/quack-control.env    # la pagina lo chiede una volta
```

Dopo ogni modifica:

```sh
sudo systemctl restart quack-control
```

## 4. Controllare

```sh
systemctl status quack-control
journalctl -u quack-control -f
```

Poi si apre `http://<indirizzo dell'anatra>:8090/` da un telefono o da un
computer sulla stessa rete.

## 5. Aggiornare

Si scarica il pacchetto più nuovo, si verifica, si scompatta e si lancia
il suo `./install-on-duck.sh` allo stesso modo. Il binario, la unit e
l'account vengono sostituiti; la configurazione e il file del token
restano.

## 6. Disinstallare

Sull'anatra:

```sh
sudo systemctl disable --now quack-control
sudo rm /usr/local/bin/quack-control /etc/systemd/system/quack-control.service /etc/sysusers.d/quack-control.conf
sudo systemctl daemon-reload
# tenuti di proposito: /etc/robot/quack-control.toml e .env — si tolgono
# (con `sudo userdel quackctl`) per ripartire da zero
```

Altro: il README del progetto, <https://github.com/andreagenovese/quack-control>.
