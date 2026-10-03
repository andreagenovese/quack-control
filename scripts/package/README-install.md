# Installing quack-control on the duck

This package installs `quack-control`, the Microduck's local control
plane (a page on the home network that shows the live map and drives the
duck), on the duck's board (Radxa Zero 3, aarch64 Linux, Debian 13) from
your own computer, over ssh. No checkout of the repository and no build
are needed. Italian copy: [README-install.it.md](README-install.it.md).

What you need:

- a duck with quack-nav installed first (quack-navd and the `robot`
  group its sockets belong to) — v0.2.0-rc2 or later for the full page;
  an older one works with less and says "update quack-nav";
- ssh access to the duck with an account that has `sudo` (`microduck` on
  the board image; older images had `radxa`);
- on your computer: `bash`, `ssh`, `scp`, `tar`, and `shasum` or
  `sha256sum` (macOS and Linux have them all).

## 1. Download and verify

From <https://github.com/andreagenovese/quack-control/releases>, the
package and its checksum. This package is `@VERSION@`; for a newer one set
`V` to its version (the tag without its `v`):

```sh
V=@VERSION@
gh release download "v$V" --repo andreagenovese/quack-control \
    --pattern "quack-control-$V-aarch64-linux.tar.gz*"
# or, without gh:
curl -LO "https://github.com/andreagenovese/quack-control/releases/download/v$V/quack-control-$V-aarch64-linux.tar.gz"
curl -LO "https://github.com/andreagenovese/quack-control/releases/download/v$V/quack-control-$V-aarch64-linux.tar.gz.sha256"

shasum -a 256 -c "quack-control-$V-aarch64-linux.tar.gz.sha256"   # or sha256sum -c
```

It must print `OK`. Then unpack:

```sh
tar xzf "quack-control-$V-aarch64-linux.tar.gz"
cd "quack-control-$V"
```

## 2. Install

```sh
./install-on-duck.sh --dry-run microduck@192.168.1.42   # optional: print what it would do
./install-on-duck.sh microduck@192.168.1.42
```

It copies the files to the duck, then with `sudo` there:

| on the duck | from this package |
|---|---|
| `/usr/local/bin/quack-control` | `bin/quack-control` |
| `/etc/systemd/system/quack-control.service` | `systemd/quack-control.service` |
| `/etc/sysusers.d/quack-control.conf` (user `quackctl`) | `systemd/sysusers.d/quack-control.conf` |
| `/etc/robot/quack-control.toml` — **only when there is none** | `quack-control.example.toml` |
| `/etc/robot/quack-control.env` — **only when there is none** | created with `QC_TOKEN` commented out, root, mode 600 |

Then it enables and restarts the service, printing every command, and
prints the page's address. `SSH_OPTS="-p 2222"` passes options to ssh and
scp.

## 3. The config

`/etc/robot/quack-control.toml` (edit on the duck with
`sudo nano /etc/robot/quack-control.toml`):

| key | in the example | what it is |
|---|---|---|
| `bind` | `0.0.0.0:8090` | where the page listens: every interface, port 8090. Plain HTTP: never forward it to the internet |
| `[[service]] kind = "quack-nav"`: `nav_socket`, `map_socket` | `/run/quack-nav/nav.sock`, `/run/quack-nav/map.sock` | quack-navd's two sockets (its `socket` and `[maploc] socket`) |
| `[[service]] kind = "mediad"`: `media_socket` | `/run/mediad/media.sock` | the head camera (Pollen's mediad) |
| `[[service]] kind = "quacksat"` | — | a placeholder for now |

**The token** (recommended): put it in `/etc/robot/quack-control.env`, not
in the world-readable config:

```sh
sudo sh -c 'echo "QC_TOKEN=$(tr -dc a-z0-9 </dev/urandom | head -c 32)" > /etc/robot/quack-control.env'
sudo cat /etc/robot/quack-control.env    # the page asks for it once
```

After any edit:

```sh
sudo systemctl restart quack-control
```

## 4. Check

```sh
systemctl status quack-control
journalctl -u quack-control -f
```

Then open `http://<the duck's address>:8090/` from a phone or a computer
on the same network.

## 5. Upgrade

Download the newer package, verify it, unpack it, and run its
`./install-on-duck.sh` the same way. The binary, the unit and the account
are replaced; your config and token file are kept.

## 6. Uninstall

On the duck:

```sh
sudo systemctl disable --now quack-control
sudo rm /usr/local/bin/quack-control /etc/systemd/system/quack-control.service /etc/sysusers.d/quack-control.conf
sudo systemctl daemon-reload
# kept on purpose: /etc/robot/quack-control.toml and .env — remove them
# (and `sudo userdel quackctl`) for a clean slate
```

More: the project's README, <https://github.com/andreagenovese/quack-control>.
