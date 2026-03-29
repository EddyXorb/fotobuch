# Genpool-Diversität: Duplikat-Eliminierung

**Status: Noch nicht implementiert** (kein `canonicalize()`, kein `deduplicate()` im Code).

## Problem

Elitismus + Tournament-Selection ohne Diversitätskontrolle führt zu schneller Konvergenz: dominante Individuen übernehmen die Population. Mit `enforce_order=true` ist der Suchraum kleiner (nur Topologie + Cuts), was das Problem verschärft.

## Ansatz

Nach Selection, vor Crossover/Mutation: exakte Duplikate erkennen und durch frische Random-Bäume ersetzen.

### Kanonische Arena-Form

Derived `PartialEq`/`Hash` auf `Vec<Node>` ist nur korrekt wenn alle Arenas in derselben Reihenfolge aufgebaut sind. `random_tree()` baut in Einfügereihenfolge (nicht DFS-Preorder), daher ist `canonicalize()` nötig: Arena in DFS-Preorder umbauen. `rebuild_with_graft()` (Crossover) liefert bereits Pre-Order; Mutation (Cut-Flip) ändert nichts an der Arena-Reihenfolge.

### Zeitpunkt der Deduplizierung

Nach `build_next_population()` (Elite + Offspring zusammengeführt), vor dem Sortieren. So bleiben Eliten erhalten, aber Kopien davon werden durch frisches Material ersetzt.

## Warum nur exakte Duplikate?

- Kein Schwellwert-Tuning, kein Distanzmaß
- Entfernt nur echte Redundanz
- `HashSet`-Insert ist O(1) amortisiert, Hash über ~3 KB Arena

"Ähnliche" Bäume (z.B. gleiche Topologie, ein Cut anders) sind wertvolle Nachbarn im Suchraum und bleiben erhalten.
