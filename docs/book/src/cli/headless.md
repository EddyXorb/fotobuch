# Headless / CI Usage

The `fotobuch` binary has no GUI dependencies and runs in any headless environment — CI servers, Docker containers, SSH sessions.

---

## Minimal build script

```bash
#!/bin/bash
set -e

fotobuch project new MyBook --width 297 --height 210 --quiet
cd MyBook
fotobuch add /mnt/photos/2024-Summer
fotobuch build
fotobuch build release
# MyBook_final.pdf is ready
```
---

## Docker example

```dockerfile
FROM debian:bookworm-slim
COPY fotobuch /usr/local/bin/fotobuch
WORKDIR /vault
ENTRYPOINT ["fotobuch"]
```

Mount the vault as a volume:

```bash
docker run --rm -v /data/vault:/vault fotobuch build release
```
