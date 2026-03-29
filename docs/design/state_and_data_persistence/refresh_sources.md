# `StateManager::refresh_sources`

**Status: Noch nicht implementiert.**

## Problem

Die drei Build-Pfade (`incremental_build`, `multipage_build`, `rebuild`) rufen `ensure_previews` unabhängig auf. Keiner aktualisiert `ProjectState` wenn eine Quelldatei auf Disk ersetzt wird — zurück bleiben stale Metadaten (falsches Aspect-Ratio, alter Hash).

Betroffene Felder von `PhotoFile`: `hash`, `width_px`, `height_px`, `timestamp`. `area_weight` ist nutzerdefiniert und darf nicht überschrieben werden.

## Lösung

`StateManager::refresh_sources`:
1. Veraltete Previews neu erzeugen (bestehende Logik)
2. Für alle neu erzeugten Previews: Datei-abgeleitete Metadaten neu einlesen
3. `self.state` Photo-Einträge in-place aktualisieren
4. Kombiniertes `SourceRefreshResult` zurückgeben

`PreviewCacheResult` wird um `regenerated_ids: Vec<String>` erweitert — IDs deren Previews neu erzeugt wurden (= Quellen die sich geändert haben).

Alle drei Build-Pfade ersetzen ihren `ensure_previews`-Aufruf durch einen einzigen `mgr.refresh_sources()?`-Aufruf.
