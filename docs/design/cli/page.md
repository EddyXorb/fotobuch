# `page`, `place`, `unplace` — Kommando-Referenz

## Übersicht

```
# Layout-Ebene
fotobuch place    <PATTERN>...  [--filter <PATTERN>]  [--into <PAGE>]
fotobuch unplace  <PAGE:SLOT_EXPR>

# Seiten-Operationen
fotobuch page move    <SRC=PAGES_EXPR|PAGE:SLOT_EXPR>  to   <DST=PAGE|PAGE+>
fotobuch page move    <SRC=PAGES_EXPR|PAGE:SLOT_EXPR>  out
fotobuch page split   <PAGE:SLOT>
fotobuch page combine <PAGES_EXPR>
fotobuch page swap    <PAGE:SLOT_EXPR>  <PAGE:SLOT_EXPR>
```

**Adressierungs-Syntax:**

```
PAGE         # genau eine seite:  3
PAGES_EXPR   # eine oder mehrere: 3  |  3,5  |  3..5

SLOT_EXPR:
  2          # einzelner slot
  2,7        # mehrere slots
  2..5       # slot-range
  2..5,7     # kombiniert

Spezial-Ziele (nur bei page move to):
  4+         # neue seite nach 4
```

-----

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

-----

### `unplace`

Entfernt Fotos aus dem Layout, lässt sie aber im Projekt (`photos`). Gegenstück zu `place`.
Fotos werden dadurch "unplaced" und tauchen wieder in `fotobuch status` als unplatziert auf.

```
fotobuch unplace 3:2          # slot 2 auf seite 3
fotobuch unplace 3:2,7        # slots 2 und 7 auf seite 3
fotobuch unplace 3:2..5       # slots 2 bis 5 auf seite 3
fotobuch unplace 3:2..5,7     # kombiniert
```

Seiten ohne verbleibende Fotos werden nicht automatisch gelöscht — dafür `page combine`
oder manuelles Löschen via `page move SRC out`.

Alternativ: `page move 3:2 out` ist äquivalent zu `unplace 3:2`. Für ganze Seiten
(`page move 3 out`) wird die Seite direkt gelöscht — das geht mit `unplace` nicht.

-----

### `page move`

Zwei Varianten: Verschieben (`to`) und Unplatzieren (`out`).

#### Variante 1: Verschieben (`to`)

Verschiebt Fotos von der Quelle auf eine Zielseite. Der Solver verteilt die Fotos
auf der Zielseite neu (impliziter Rebuild der Zielseite).

```
fotobuch page move 3:2 to 5           # slot 2 von seite 3 auf seite 5
fotobuch page move 3:1..3,7 to 5      # slots 1-3 und 7 von seite 3 auf seite 5
fotobuch page move 3 to 5             # alle fotos von seite 3 auf seite 5
fotobuch page move 3,4 to 5           # alle fotos von seiten 3 und 4 auf seite 5
fotobuch page move 3..5 to 2          # alle fotos von seiten 3-5 auf seite 2
fotobuch page move 3:2 to 4+          # slot 2 von seite 3 auf neue seite nach 4
```

Die Quellseite wird nach dem Verschieben nicht automatisch gelöscht, auch wenn sie leer ist.

#### Variante 2: Unplace (`out`)

`out` bedeutet: Fotos werden aus dem Layout entfernt (unplaced), aber nicht
aus dem Projekt gelöscht.

```
fotobuch page move 3 out              # seite 3 wird gelöscht, fotos werden unplaced
fotobuch page move 3,4 out            # seiten 3 und 4 werden gelöscht
fotobuch page move 3:2 out            # nur slot 2 auf seite 3 wird unplaced, seite bleibt
fotobuch page move 3:1..3 out         # slots 1-3 werden unplaced, seite bleibt
```

Unterschied je nach Quelle:

- `Src::Pages` (`3`, `3,4`, `3..5`): Die gesamten Seiten werden **gelöscht**
- `Src::Slots` (`3:2`, `3:1..3`): Nur die Slots werden entfernt, **Seite bleibt** (ggf. leer)

-----

### `page split`

Shortcut für `page move PAGE:SLOT.. to PAGE+`. Teilt eine Seite an einem gegebenen Slot:
alle Fotos ab diesem Slot (inklusive) wandern auf eine neu eingefügte Seite direkt danach.

```
fotobuch page split 3:4     # fotos ab slot 4 gehen auf neue seite 4, alte seite 4 wird 5
```

-----

### `page combine`

Shortcut: verschiebt alle Fotos der angegebenen Seiten auf die erste Seite der Angabe,
löscht danach die leeren Seiten. Alle nachfolgenden Seitennummern rücken entsprechend vor.

```
fotobuch page combine 3,5       # fotos von seite 5 auf seite 3, seite 5 wird gelöscht
fotobuch page combine 3..5      # fotos von 4 und 5 auf seite 3, seiten 4 und 5 werden gelöscht
```

-----

### `page swap`

Tauscht Fotos zwischen zwei Adressen. Unterstützt Slots, einzelne Seiten und Seitenbereiche/-listen.

```
fotobuch page swap 3:2 5:6          # einzelslot-swap
fotobuch page swap 3:1..3 5:2..4    # slot-range-swap
fotobuch page swap 3 5              # ganze seiten
fotobuch page swap 3..6 8..11       # seitenbereich-swap (paarweise, gleiche anzahl)
fotobuch page swap 3,5 7,9          # seitenlisten-swap
```

-----

## Parser-Design

### Grundprinzip

Der Parser ist in drei Schichten aufgeteilt, analog zu klassischen Compiler-Frontends:
**Lexer → Parser → Validator**. Jede Schicht hat eine klar definierte Aufgabe und gibt
einen typisierten Output weiter.

### Schicht 1: Lexer

Zerlegt den Raw-String in Token. Keine Semantik, nur Klassifikation:

```rust
#[derive(Debug, PartialEq)]
enum Token {
    Number(u32),   // "3"
    Comma,         // ","
    Range,         // ".."
    Colon,         // ":"
    To,            // "to"
    Out,           // "out"
    Plus,          // "+"
}
```

Der Lexer ist eine einfache Zustandsmaschine über `chars()`. Whitespace wird ignoriert.
Alphabetische Sequenzen werden als Keywords erkannt (`to`, `out`); unbekannte Keywords
sind ein Fehler.

### Schicht 2: Parser

Baut aus den Token einen typisierten AST. Die Grammatik ist kontextfrei und eindeutig:

```
pages_expr  = page ("," page)*
            | page ".." page

page        = NUMBER

slot_expr   = slot_item ("," slot_item)*

slot_item   = NUMBER ".." NUMBER
            | NUMBER

src         = pages_expr
            | page ":" slot_expr

dst_move    = page
            | page "+"

dst_swap    = pages_expr
            | page ":" slot_expr

move_cmd    = src "to" dst_move
            | src "out"
```

Daraus entstehen typisierte Structs:

```rust
enum Src {
    Pages(PagesExpr),
    Slots { page: u32, slots: SlotExpr },
}

enum DstMove {
    Page(u32),
    NewPageAfter(u32),
    Unplace,
}

enum DstSwap {
    Pages(PagesExpr),
    Slots { page: u32, slots: SlotExpr },
}

enum PageMoveCmd {
    Move { src: Src, dst: DstMove },
    Swap { left: Src, right: DstSwap },
}
```

### Schicht 3: Validator

Prüft semantische Constraints die der Parser nicht ausdrücken kann:

- Seitennummern existieren im aktuellen Projekt
- Slot-Nummern existieren auf den angegebenen Seiten
- Bei `page swap` (Seiten): beide Seitenmengen sind verschieden und überschneidungsfrei, gleiche Anzahl
- Bei `page combine`: mindestens zwei Seiten angegeben
- Bei `page split`: Slot ist nicht der erste (wäre ein No-Op)

Der Validator arbeitet gegen den geladenen `ProjectState` und gibt strukturierte Fehler zurück:

```rust
enum ValidationError {
    PageNotFound(u32),
    SlotNotFound { page: u32, slot: u32 },
    SwapSamePage(u32),
    SwapCountMismatch { left: usize, right: usize },
    SwapRangesOverlap,
    CombineSinglePage(u32),
    SplitAtFirstSlot(u32),
}
```

### Fehlerbehandlung

Fehler werden mit Kontext ausgegeben, nicht nur mit einer Fehlernummer:

```
$ fotobuch page move 3:2 to 99
error: page 99 does not exist (project has 12 pages)

$ fotobuch page move 3:15 to 5
error: slot 15 does not exist on page 3 (page has 7 slots)

$ fotobuch page split 3:1
error: cannot split at first slot (would leave page 3 empty)
```

### Clap-Integration

Die `page`-Subkommandos werden in Clap als Subcommand definiert. `page move` nimmt alle
Tokens als `Vec<String>` entgegen und der CLI-Parser fügt sie zu einem String zusammen:

```rust
// cli.rs
#[derive(Subcommand)]
enum PageCommand {
    Move {
        /// e.g. "3:1..3,7 to 5" or "3:2 ~ 5:6" or "3 out"
        #[arg(num_args = 1..)]
        args: Vec<String>,
    },
    Split { address: String },
    Combine { pages: String },
    Swap { left: String, right: String },
}
```

-----

## Aufteilung lib vs. cli

### Grundprinzip

```
cli.rs                    # String → typisierte Structs (nur Syntax)
lib/commands/page.rs      # Validierung + Ausführung (Semantik)
```

Fehler aus beiden Schichten werden in `cli.rs` für die Konsole formatiert.

### Was in die lib gehört

Die lib exportiert alle Typen und die Ausführungslogik:

```rust
// lib/commands/page.rs

// Typen (pub, von cli.rs verwendet)
pub enum Src { ... }
pub enum DstMove { ... }
pub enum DstSwap { ... }
pub enum PageMoveCmd { ... }
pub struct PagesExpr { ... }
pub struct SlotExpr { ... }

// Validierungsfehler (pub, von cli.rs formatiert)
pub enum ValidationError {
    PageNotFound(u32),
    SlotNotFound { page: u32, slot: u32 },
    SwapSamePage(u32),
    CombineSinglePage(u32),
    SplitAtFirstSlot(u32),
}

// Ausführung
pub fn execute_move(
    project_root: &Path,
    cmd: PageMoveCmd,
) -> Result<PageMoveResult, PageMoveError> { ... }

pub fn execute_split(
    project_root: &Path,
    page: u32,
    slot: u32,
) -> Result<PageMoveResult, PageMoveError> { ... }

pub fn execute_combine(
    project_root: &Path,
    pages: PagesExpr,
) -> Result<PageMoveResult, PageMoveError> { ... }

pub fn execute_swap(
    project_root: &Path,
    left: Src,
    right: DstSwap,
) -> Result<PageMoveResult, PageMoveError> { ... }
```

`PageMoveError` ist ein Wrapper über `ValidationError` und interne Fehler
(IO, YAML-Serialisierung etc.):

```rust
pub enum PageMoveError {
    Validation(ValidationError),
    Io(std::io::Error),
    Project(ProjectError),
}
```

### Was in cli.rs bleibt

```rust
// cli.rs

/// Syntaktisches Parsing: Raw-String → PageMoveCmd.
/// Kein Zugriff auf ProjectState, nur Zeichenketten-Analyse.
fn parse_move_cmd(args: &[String]) -> Result<PageMoveCmd, ParseError> { ... }

/// Fehlerformatierung für die Konsole.
fn format_page_move_error(err: &PageMoveError) -> String { ... }

fn handle_page_move(args: &[String]) -> Result<()> {
    let cmd = parse_move_cmd(args).map_err(|e| /* format + exit */)?;
    let result = fotobuch::commands::page::execute_move(&project_root, cmd)?;
    print_page_move_result(&result);
    Ok(())
}
```

`ParseError` bleibt in `cli.rs` — er beschreibt Syntaxfehler in der Benutzereingabe,
die die lib nichts angehen:

```rust
// cli.rs
enum ParseError {
    UnexpectedToken { got: String, expected: &'static str },
    MissingOperator,
    MissingDestination,
    InvalidNumber(String),
    UnknownKeyword(String),
}
```

### Datenfluss

```
cli.rs
  Vec<String> (raw args)
    → parse_move_cmd()          # Syntax-Check, kein I/O
        → PageMoveCmd
    → lib::commands::page::execute_move(project_root, cmd)
        → ProjectState laden
        → validate()            # Semantik-Check gegen ProjectState
        → ausführen             # YAML mutieren
        → YAML schreiben
        → PageMoveResult
    → print_page_move_result()
```
