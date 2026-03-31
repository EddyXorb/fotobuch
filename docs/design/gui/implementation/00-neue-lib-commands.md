# Phase 0: Neue Lib-Commands — Übersicht

> **Hinweis**: GUI-Kommentare in diesen Plänen sind rein informativ und sollen
> in diesem Implementierungsplan **nicht** umgesetzt werden. Es geht hier nur um
> die Lib- und CLI-Änderungen.

## Reihenfolge

Die Schritte bauen aufeinander auf:

| # | Was | Datei | Branch |
|---|-----|-------|--------|
| 0 | `CommandOutput<T>` + `render_pages()` | `00-3-command-output.md` | `feat/command-output` |
| 1 | `page mode` (führt `PageMode` ein) | `00-1-page-mode.md` | `feat/page-mode` |
| 2 | `page pos` (braucht `PageMode`) | `00-0-page-pos.md` | `feat/page-pos` |
| 3 | `config set` (unabhängig) | `00-2-config-set.md` | `feat/config-set` |

`CommandOutput<T>` muss **zuerst** umgesetzt werden — alle neuen und bestehenden
Commands bauen darauf auf. `page mode` vor `page pos`, weil `pos` den Manual-Mode
voraussetzt. `config set` ist unabhängig und kann parallel.
