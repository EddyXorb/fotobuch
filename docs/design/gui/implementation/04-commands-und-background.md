# Phase 3: Commands + Background-Pipeline

**Ziel**: GUI-Aktionen führen Lib-Commands aus, alles non-blocking.

## 3.1 — Command-Dispatch

- User-Aktion → `task_tx.send(RunCommand(...))` → Background führt aus
- Background sendet `CommandDone` → UI updatet `project_state` + `derived`
- Background rendert dirty pages → UI swappt Texturen

## 3.2 — Swap/Move (gleiche Seite)

- Drag-Start auf Slot → DragState
- Drop auf anderen Slot → Background: `commands::page::swap()`
- Ratio-Feedback: grün (gleiche Ratio) / rot (unterschiedlich)
- M-Taste gehalten: Move statt Swap

## 3.3 — Blur-Effekt + Undo/Redo

- Dirty page → Blur über alter Textur + Spinner
- Neue Textur fertig → Blur entfernen
- Ctrl+Z/Y → Background: `commands::undo()`/`redo()` → alle Seiten dirty
