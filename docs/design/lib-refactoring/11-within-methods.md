# Umsetzungsplan 11 — Innerhalb von Methoden

> Konkretisiert Abschnitt 11 des [Hauptdokuments](./README.md).
> Stand: gegen den aktuellen Code geprüft.

## Worum es geht

Restpunkt aus „lange Methoden mit Frühreturns über mehrere Abstraktionsebenen".
Die früheren build-Punkte sind mit Abschnitt 1 erledigt; offen war nur noch
`execute_move_to`.

## Befund: `execute_move_to` — bereits durch #50 entschärft

Die ursprüngliche Funktion (`move_cmd.rs:33–248`, 6 Frühreturns quer durch
Validierung/Dispatch/Mutation) existiert so nicht mehr. Mit der Slot-Move-Familie
(Plan 7, #50) ist `move_cmd.rs` in Submodule geteilt:

- `move_cmd.rs` (`execute_move_to`, jetzt ~25 Z.): nur noch Dispatch — Manual-Ziel
  und Same-Page-No-Op früh raus, sonst `run_write_command` + Delegation an
  `standard::apply_move` (RULES R8).
- `move_cmd/standard.rs::apply_move`: Orchestrierung nach Muster
  **Lesen → entscheiden → schreiben** (RULES R4); Slot-Indizes werden vor jeder
  Einfügung als *owned* gezogen, dann mutiert. Mutation in `move_*_to_page`-Helfer.
- `move_cmd/manual.rs`: Manual-Pfad isoliert.

Damit ist die Vermischung der Abstraktionsebenen aufgelöst: Dispatch, Orchestrierung
und Mutation liegen getrennt, jede Funktion unter R9 (300 Z.).

## Was bleibt

Kein eigenständiger Umsetzungsschritt nötig. Was an Methoden-Hygiene noch offen
ist, hängt an größeren Komplexen und wird dort geführt:

- `place.rs` (Orchestrierung + chronologischer Algorithmus vermischt) → Plan 7 K6
  (`page_placement.rs` herauslösen).
- Manual-Ziel beim Move braucht einen Slot → §13.1 (bereits umgesetzt, siehe
  Test `move_slot_into_manual_page_creates_positioned_slot`).

## Reihenfolge & Risiko

Entfällt — der Befund ist abgegolten. Künftige „lange Methode"-Funde nach demselben
Muster behandeln: erst nach Abstraktionsebene aufteilen (Dispatch/Orchestrierung/
Rechnung), Reads vor Writes als owned ziehen (R4), Mutation in einen Helfer.
