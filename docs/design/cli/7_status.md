# `fotobuch status`

## CLI-Interface

```text
$ fotobuch status [PAGE]
```

Rein lesend — verändert nichts.

## Projektzustände

| Zustand | Bedeutung |
| ------- | --------- |
| `empty` | Fotos vorhanden, noch nie gebaut (Layout leer) |
| `clean` | Layout existiert, nichts geändert seit letztem Build |
| `modified` | Layout existiert, YAML seit letztem Build geändert |

## Kompakte Ansicht: `fotobuch status`

```text
Project: urlaub
85 photos in 6 groups (5 unplaced)

Layout: 12 pages, 7.1 photos/page avg
  4 pages modified since last build
    pages 2, 5: need rebuild (ratio mismatch in swapped photos)
    pages 3, 8: compatible swaps only (no rebuild needed)
```

## Detail-Ansicht: `fotobuch status <PAGE>`

```text
page 3, slot 2
  id:      2024-01-15_Urlaub/IMG_002.jpg
  ratio:   0.67
  group:   B
  placed:  x=155.0mm y=10.0mm w=90.3mm h=135.5mm
```

**Swap-Gruppen**: Fotos mit kompatiblem Seitenverhältnis (≤5% Abweichung) bekommen denselben Buchstaben (A, B, C, …). Berechnung on-the-fly — zeigt welche Fotos ohne Rebuild gegeneinander getauscht werden können.

## Konsistenzprüfungen

Warnungen werden ausgegeben für:

- **Orphaned Placements**: Foto in `layout`, aber nicht mehr in `photos`

Unplaced-Fotos sind ein normaler Zustand und werden ohne Warnung angezeigt.

## Verhalten ohne Git

Kein Git-Repo oder kein Build-Commit → keine Änderungserkennung, Status zeigt `empty` oder `clean`. Konsistenzprüfungen und Detail-Ansicht funktionieren unabhängig von Git.
