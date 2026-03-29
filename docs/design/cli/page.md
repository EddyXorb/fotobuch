# `page`, `place`, `unplace` — Kommando-Referenz

## Übersicht

```
# Layout-Ebene
fotobuch place    [--filter <PATTERN>]  [--into <PAGE>]
fotobuch unplace  <PAGE:SLOT_EXPR>

# Seiten-Operationen
fotobuch page move    <SRC=PAGES_EXPR|PAGE:SLOT_EXPR>  to   <DST=PAGE|PAGE+>
fotobuch page move    <SRC=PAGES_EXPR|PAGE:SLOT_EXPR>  out
fotobuch page split   <PAGE:SLOT>
fotobuch page combine <PAGES_EXPR>
fotobuch page swap    <PAGE:SLOT_EXPR>  <PAGE:SLOT_EXPR>
fotobuch page info    <PAGES_EXPR|PAGE:SLOT_EXPR>  [--weights|--ids|--pixels]
fotobuch page weight  <PAGE:SLOT_EXPR|PAGE>  <WEIGHT>
```

**Adressierungs-Syntax:**

```
PAGE         # genau eine Seite:  3
PAGES_EXPR   # eine oder mehrere: 3  |  3,5  |  3..5

SLOT_EXPR:
  2          # einzelner Slot
  2,7        # mehrere Slots
  2..5       # Slot-Range
  2..5,7     # kombiniert

Spezial-Ziele (nur bei page move to):
  4+         # neue Seite nach 4
```

---

## Detaillierte Beschreibung

### `place`

Weist bisher unplatzierten Fotos (vorhanden in `photos`, aber nicht in `layout`) Seiten zu.
Die Zuweisung erfolgt chronologisch — ein Foto landet auf der Seite, deren bereits platzierte
Fotos zeitlich am nächsten liegen.

```
fotobuch place "Urlaub"             # alle unplatzierten Fotos der Gruppe "Urlaub"
fotobuch place --filter "Urlaub"    # gleiches Ergebnis via Filter
fotobuch place --into 5             # alle unplatzierten Fotos auf Seite 5
fotobuch place                      # alle unplatzierten Fotos
```

`place` verändert nur `layout[].photos`, nie `layout[].slots`. Ein nachfolgendes `build`
rechnet die betroffenen Seiten neu.

---

### `unplace`

Entfernt Fotos aus dem Layout, lässt sie aber im Projekt (`photos`). Gegenstück zu `place`.
Fotos werden dadurch "unplaced" und tauchen wieder in `fotobuch status` als unplatziert auf.

```
fotobuch unplace 3:2          # Slot 2 auf Seite 3
fotobuch unplace 3:2,7        # Slots 2 und 7 auf Seite 3
fotobuch unplace 3:2..5       # Slots 2 bis 5 auf Seite 3
fotobuch unplace 3:2..5,7     # kombiniert
```

Wird der letzte Slot einer Seite entfernt, wird die Seite automatisch gelöscht.

`page move 3:2 out` ist äquivalent zu `unplace 3:2`. `page move 3 out` löscht die gesamte
Seite ohne Slot-Angabe.

---

### `page move`

#### Variante 1: Verschieben (`to`)

Verschiebt Fotos von der Quelle auf eine Zielseite. Die Zielseite wird implizit neu gelayoutet.

```
fotobuch page move 3:2 to 5           # Slot 2 von Seite 3 auf Seite 5
fotobuch page move 3:1..3,7 to 5      # Slots 1-3 und 7 von Seite 3 auf Seite 5
fotobuch page move 3 to 5             # alle Fotos von Seite 3 auf Seite 5
fotobuch page move 3,4 to 5           # alle Fotos von Seiten 3 und 4 auf Seite 5
fotobuch page move 3..5 to 2          # alle Fotos von Seiten 3-5 auf Seite 2
fotobuch page move 3:2 to 4+          # Slot 2 von Seite 3 auf neue Seite nach 4
```

Wird die Quellseite durch das Verschieben leer, wird sie automatisch gelöscht.

#### Variante 2: Unplace (`out`)

Fotos werden aus dem Layout entfernt (unplaced), aber nicht aus dem Projekt gelöscht.

```
fotobuch page move 3 out              # Seite 3 wird gelöscht, Fotos werden unplaced
fotobuch page move 3,4 out            # Seiten 3 und 4 werden gelöscht
fotobuch page move 3:2 out            # Slot 2 wird unplaced; Seite bleibt, außer sie wird leer
fotobuch page move 3:1..3 out         # Slots 1-3 werden unplaced
```

`Src::Pages` löscht die Seite immer. `Src::Slots` entfernt die Fotos — wird die Seite dadurch
leer, wird sie ebenfalls gelöscht.

---

### `page split`

Shortcut für `page move PAGE:SLOT.. to PAGE+`. Teilt eine Seite an einem gegebenen Slot:
alle Fotos ab diesem Slot (inklusive) wandern auf eine neu eingefügte Seite direkt danach.

```
fotobuch page split 3:4     # Fotos ab Slot 4 gehen auf neue Seite 4, alte Seite 4 wird 5
```

---

### `page combine`

Shortcut: verschiebt alle Fotos der angegebenen Seiten auf die erste Seite der Angabe,
löscht danach die leeren Seiten.

```
fotobuch page combine 3,5       # Fotos von Seite 5 auf Seite 3, Seite 5 wird gelöscht
fotobuch page combine 3..5      # Fotos von 4 und 5 auf Seite 3, Seiten 4 und 5 werden gelöscht
```

---

### `page swap`

Tauscht Inhalte zwischen zwei Adressen. Zwei Varianten mit unterschiedlicher Semantik:

#### Variante 1: Seiten-Swap (Pages × Pages) — Block-Transposition

Zwei nicht überlappende Seitenblöcke tauschen ihre Position in der Seitenfolge.
Seiten zwischen den Blöcken bleiben in ihrer relativen Reihenfolge erhalten.
Nur einzelne Seiten oder Ranges erlaubt (keine Komma-Listen).

```
fotobuch page swap 3 5              # Seiten 3 und 5 tauschen Position
fotobuch page swap 1..2 5..9        # Block [1,2] und Block [5..9] tauschen Position
```

Beispiel `swap 1..2 5..9` bei 10 Seiten:

```
vorher:  [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
nachher: [5, 6, 7, 8, 9, 3, 4, 1, 2, 10]
```

Fehler: Überlappende Seitennummern, Komma-Liste als Operand.

#### Variante 2: Slot-Swap — Blockweise Ersetzung

Die Foto-Blöcke werden gegenseitig an der Position des jeweils anderen eingefügt.
Nicht betroffene Fotos beider Seiten bleiben unverändert. Unterschiedliche Blockgrößen erlaubt.
Nur einzelne Slots oder Ranges erlaubt (keine Komma-Listen).

```
fotobuch page swap 3:2 5:6          # Foto in Slot 2 (Seite 3) ↔ Foto in Slot 6 (Seite 5)
fotobuch page swap 3:2..4 5:6..9    # Block [Slots 2-4] ↔ Block [Slots 6-9]
fotobuch page swap 3:2..10 5        # Slots 2-10 von Seite 3 ↔ alle Fotos von Seite 5
fotobuch page swap 1:3..5 1:7..9    # innerhalb derselben Seite: Slots 3-5 ↔ Slots 7-9
```

Algorithmus: Beide Foto-Blöcke werden vorab gesichert. Der linke Block wird entfernt und der
rechte an seiner Position eingefügt; dann der rechte Block entfernt und der linke an seiner
Position eingefügt. Einfügeposition ist der kleinste Index des jeweiligen Ursprungsblocks.

Erlaubt: Swap innerhalb derselben Seite. Fehler: Überlappende Slot-Nummern, Komma-Liste als Operand.

---

### `page info`

Gibt Informationen über platzierte Fotos aus. Bei genau einem aufgelösten Slot vertikale
Ansicht, ab zwei Slots Tabellenansicht.

```
fotobuch page info 3:2              # einzelner Slot → vertikale Ansicht
fotobuch page info 3:1..3,7         # mehrere Slots → Tabellenansicht
fotobuch page info 3                # ganze Seite → Tabellenansicht
fotobuch page info 3 --weights      # nur Weights aller Slots
fotobuch page info 3 --ids          # nur IDs aller Slots
fotobuch page info 3 --pixels       # nur Pixeldimensionen aller Slots
```

**Vertikale Ansicht** (ein Slot):

```
page 3, slot 2
  id:      2024-01-15_Urlaub/IMG_002.jpg
  source:  /home/user/Fotos/2024-01-15_Urlaub/IMG_002.jpg
  pixels:  4000x6000
  ratio:   0.67
  weight:  2.0
  placed:  x=155.0mm y=10.0mm w=90.3mm h=135.5mm
```

**Tabellenansicht** (mehrere Slots):

```
page 3  (4/7 slots shown)
  slot  ratio  weight  pixels     placed                    id
  1     1.50   1.0     6000x4000  10.0, 10.0, 135.5x90.3   2024-01-15_Urlaub/IMG_001.jpg
  2     0.67   2.0     4000x6000  155.0, 10.0, 90.3x135.5  2024-01-15_Urlaub/IMG_002.jpg
```

**`--weights` Ausgabe** (maschinenlesbar, direkt als Input für `page weight` verwendbar):

```
3:1=1.0
3:2=2.0
```

---

### `page weight`

Setzt das `area_weight` eines oder mehrerer Slots. Ohne Slot-Angabe gilt das Gewicht für
alle Slots der Seite.

```
fotobuch page weight 3:2 2.0        # einzelner Slot
fotobuch page weight 3:1..3,7 2.0   # mehrere Slots, gleiches Gewicht
fotobuch page weight 3 2.0          # alle Slots der Seite
```

---

## Parser-Design

Der Parser ist in drei Schichten aufgeteilt: **Lexer → Parser → Validator**.

**Lexer**: Zerlegt den Raw-String in Token (Zahlen, `,`, `..`, `:`, `to`, `out`, `+`). Whitespace wird ignoriert. Unbekannte Keywords sind ein Fehler.

**Parser**: Baut aus den Token einen typisierten AST. Die Grammatik ist kontextfrei und eindeutig. Ergebnis sind typisierte Structs (`Src`, `DstMove`, `DstSwap`, `PageMoveCmd`).

**Validator**: Prüft semantische Constraints gegen den geladenen `ProjectState`:
- Seitennummern existieren im Projekt
- Slot-Nummern existieren auf den angegebenen Seiten
- Bei Seiten-Swap: beide Blöcke sind überschneidungsfrei
- Bei `page combine`: mindestens zwei Seiten angegeben
- Bei `page split`: Slot ist nicht der erste (wäre ein No-Op)

Fehler werden mit Kontext ausgegeben:

```
$ fotobuch page move 3:2 to 99
error: page 99 does not exist (project has 12 pages)

$ fotobuch page move 3:15 to 5
error: slot 15 does not exist on page 3 (page has 7 slots)
```

## Aufteilung lib vs. cli

Syntaktisches Parsing (String → typisierte Structs) liegt in `cli.rs`. Validierung und Ausführung (Semantik, YAML-Mutation) liegt in `lib/commands/page.rs`. Fehler aus beiden Schichten werden in `cli.rs` für die Konsole formatiert.
