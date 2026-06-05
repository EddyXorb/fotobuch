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

## Useful flags for scripts

| Flag           | Command       | Effect                                          |
| -------------- | ------------- | ----------------------------------------------- |
| `--quiet`      | `project new` | Suppress the welcome banner                     |
| `--dry` / `-d` | `add`         | Preview what would be added, no changes written |

---

## Exit codes

- `0` — success
- non-zero — error; message on `stderr`

---

## Dependencies

- Typst (PDF renderer) is bundled — no separate install.
- A writable filesystem for `.fotobuch/` cache.
- No `git` binary required — fotobuch uses libgit2 directly for all git operations.

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
