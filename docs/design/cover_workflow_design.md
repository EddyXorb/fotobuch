# Cover Workflow

## Problem

Nach dem ersten `build` wird die Cover-Seite wie eine normale Seite behandelt. Es gibt keine Möglichkeit, gängige Cover-Layouts (ein Foto vorne, Panorama über den ganzen Umschlag) auszudrücken.

## Lösung: `mode` in der Cover-Config

Neues Feld `mode` in `config.book.cover`. Bei einem Modus ≠ `free` wird der GA-Solver umgangen und Slot-Positionen werden deterministisch berechnet.

### Verfügbare Modi

| Mode          | Slots    | Verhalten                                                   |
| ------------- | -------- | ----------------------------------------------------------- |
| `free`        | beliebig | GA-Solver (Standard)                                        |
| `front`       | 1        | Vorderseite, Aspect-Ratio erhalten, zentriert               |
| `front-full`  | 1        | Vorderseite komplett gefüllt                                |
| `back`        | 1        | Rückseite, Aspect-Ratio erhalten, zentriert                 |
| `back-full`   | 1        | Rückseite komplett gefüllt                                  |
| `spread`      | 1        | Ganzer Umschlag über Buchrücken, Aspect-Ratio erhalten      |
| `spread-full` | 1        | Ganzer Umschlag komplett gefüllt                            |
| `split`       | 2        | Slot 0 = Vorder-, Slot 1 = Rückseite, Aspect-Ratio erhalten |
| `split-full`  | 2        | Slot 0 = Vorder-, Slot 1 = Rückseite, komplett gefüllt      |

### Config

```yaml
config:
  book:
    cover:
      active: true
      mode: front
      spine_clearance_mm: 5.0    # Abstand Foto-Kante zu Buchrücken (split/front/back)
```

### Slot-Berechnung

- **Halbe Seitenbreite** = `(front_back_width_mm - spine_width) / 2`
- **Vorderseite**: rechte Hälfte (x ab Mitte + spine/2 + clearance)
- **Rückseite**: linke Hälfte (x von 0 bis Mitte - spine/2 - clearance)
- **Aspect-Ratio-Modi**: Foto wird maximal in die Zielfläche eingepasst und zentriert
- **Full-Modi**: Slot entspricht exakt der Zielfläche (Template übernimmt Crop)
- **`spread`/`spread-full`**: Buchrücken wird ignoriert, Foto geht darüber hinweg

### Workflow-Beispiele

```bash
# Ein Foto auf der Vorderseite
# config: cover.mode: front
fotobuch place cover.jpg --into 0
fotobuch rebuild --page 0

# Panorama über den ganzen Umschlag
# config: cover.mode: spread-full
fotobuch place panorama.jpg --into 0
fotobuch rebuild --page 0

# Vorder- und Rückseite separat
# config: cover.mode: split
fotobuch place front.jpg --into 0   # → Slot 0 = Vorderseite
fotobuch place back.jpg --into 0    # → Slot 1 = Rückseite
fotobuch rebuild --page 0
```
