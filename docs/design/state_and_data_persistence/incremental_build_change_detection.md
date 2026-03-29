# Incremental Build Change Detection: `compute_outdated_pages()`

Bestimmt welche Seiten bei einer Zustandsänderung neu berechnet werden müssen. Eine Seite ist unverändert wenn:

- Keine Foto-Metadaten geändert (Aspect-Ratio, area_weight)
- Slot-Struktur stimmt mit dem vorherigen Zustand überein
- Jeder Slot hat das korrekte Aspect-Ratio für das entsprechende Foto

## Algorithmus

### Phase 1: Referenz-Maps aufbauen

```
HashMap<PhotoId, (AspectRatio, AreaWeight)>   // schneller Metadata-Lookup
HashMap<BTreeSet<PhotoId>, Vec<usize>>         // Seiten-Identität (Index in reference.layout)
HashSet<PhotoId>                               // Fotos mit geänderter Metadata
```

`BTreeSet` als Seiten-Schlüssel ermöglicht seitenordnungsunabhängiges Matching (Page-Reordering).

### Phase 2: Jede neue Seite auswerten

Für jede Seite in `new.layout`:

1. **Foto-Mutation prüfen**: Ist ein Foto in `changed_photos` → **OUTDATED**
2. **Passende alte Seite finden**: `BTreeSet<PhotoId>` in `page_hashes` nachschlagen; kein Kandidat → **OUTDATED**; für jeden Kandidaten: exakten Slot-Vergleich (x, y, width, height)
3. **Slot-Anzahl validieren**: `slots.len() != photos.len()` → **OUTDATED**
4. **Aspect-Ratios validieren**: `|slot_ar - photo_ar| > THRESHOLD` für einen Slot → **OUTDATED**
5. Alle Checks bestanden → **UNCHANGED**

## Key Design Decisions

- **BTreeSet als Seiten-Identität**: Seiten-Reordering (z.B. nach `page move`) wird korrekt erkannt
- **Kandidaten-Liste**: Unterstützt doppelte Foto-Sets in der Referenz
- **Exakter Slot-Vergleich**: Erkennt jede strukturelle Mutation (auch Re-Solving)
- **Index-Kopplung**: `slots[i]` entspricht immer `photos[i]`
