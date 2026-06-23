# Umsetzungsplan 8 — `input`-Modul

> Konkretisiert Abschnitt 8 des [Hauptdokuments](./README.md).
> Stand: gegen den aktuellen Code geprüft.

## Worum es geht

Kleines, rein strukturelles Aufräumen: zwei gleichnamige `metadata.rs`, ein
fehlplatzierter Export und eine Kommentar-Leiche. Keine Logikänderung.

## Was offen ist (verifiziert)

- **Zwei `metadata.rs`, nur über den Pfad unterscheidbar:**
  - `input/metadata.rs` (91 Z.) = `compute_partial_hash` (Duplikat-Hashing),
    genutzt von `commands/add/deduplication.rs`.
  - `input/scan/metadata.rs` (97 Z.) = `enrich_photo_metadata` (EXIF/Orientierung).
- **`enrich_photo_metadata` als Scan-API exportiert** (`scan.rs:15 pub use`), wird
  aber auch außerhalb des Scanners genutzt (`cache/preview_cache.rs:4`) — es ist
  ein **Enricher**, kein Scanner-Output.
- **Kommentar-Leiche** `// pub mod grouper; // Will be added in Commit 5`
  (`input.rs:6`).

## Commit-Portionen

**L1 · `refactor(input): rename metadata.rs to hashing.rs`**
`input/metadata.rs` → `input/hashing.rs` (Inhalt: `compute_partial_hash`). Import
in `commands/add/deduplication.rs` nachziehen. Reine Umbenennung.
*Verify:* kompiliert; Tests grün.

**L2 · `refactor(input): rehome the photo enricher out of scan`**
`enrich_photo_metadata` aus `input/scan/metadata.rs` auf Input-Ebene ziehen
(`input/exif.rs`, parallel zu `input/xmp.rs`; alternativ rollenbasiert
`input/enrich.rs`). `scan.rs`-Re-Export entfernen; `scanner.rs` und
`cache/preview_cache.rs` importieren aus dem neuen Pfad. `input/scan/metadata.rs`
entfällt.
*Verify:* kein Consumer hängt mehr an `scan::enrich_photo_metadata`; Tests grün.

**L3 · `chore(input): remove stale grouper comment`**
`input.rs:6` löschen.

## Reihenfolge & Risiko

Sehr gering — alles mechanische Umbenennung/Verschiebung, durch bestehende
Input-/Cache-Tests gedeckt. Alle Dateien bleiben unter R9 (300 Z.).
