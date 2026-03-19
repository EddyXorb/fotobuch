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
fotobuch page info    <PAGES_EXPR|PAGE:SLOT_EXPR>  [--weights|--ids|--pixels]
fotobuch page weight  <PAGE:SLOT_EXPR|PAGE>  <WEIGHT>
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

Wird die letzte Seite einer Seite entfernt, wird die Seite automatisch gelöscht.

Alternativ: `page move 3:2 out` ist äquivalent zu `unplace 3:2`. `page move 3 out`
löscht die gesamte Seite ohne Slot-Angabe.

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

Wird die Quellseite durch das Verschieben leer, wird sie automatisch gelöscht.

#### Variante 2: Unplace (`out`)

`out` bedeutet: Fotos werden aus dem Layout entfernt (unplaced), aber nicht
aus dem Projekt gelöscht.

```
fotobuch page move 3 out              # seite 3 wird gelöscht, fotos werden unplaced
fotobuch page move 3,4 out            # seiten 3 und 4 werden gelöscht
fotobuch page move 3:2 out            # slot 2 wird unplaced; seite bleibt, außer sie wird leer
fotobuch page move 3:1..3 out         # slots 1-3 werden unplaced; seite bleibt, außer sie wird leer
```

`Src::Pages` löscht die Seite immer. `Src::Slots` entfernt die Fotos — wird die Seite dadurch
leer, wird sie ebenfalls gelöscht.

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

Tauscht Inhalte zwischen zwei Adressen. Es gibt zwei Varianten mit unterschiedlicher Semantik:

#### Variante 1: Seiten-Swap (Pages × Pages) — Block-Transposition

Zwei nicht überlappende Seitenblöcke tauschen ihre Position in der Seitenfolge.
Seiten zwischen den Blöcken bleiben in ihrer relativen Reihenfolge erhalten.
Nur einzelne Seiten oder Ranges erlaubt (keine Komma-Listen).

```
fotobuch page swap 3 5              # seiten 3 und 5 tauschen position
fotobuch page swap 1..2 5..9        # block [1,2] und block [5..9] tauschen position
```

Beispiel: `swap 1..2 5..9` bei 10 seiten:

```
vorher:  [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
nachher: [5, 6, 7, 8, 9, 3, 4, 1, 2, 10]
```

Fehler: Überlappende Seitennummern, Komma-Liste als Operand.

#### Variante 2: Slot-Swap — Blockweise Ersetzung an derselben Position

Die foto-blöcke werden gegenseitig an der position des jeweils anderen eingefügt.
Nicht betroffene fotos beider seiten bleiben unverändert. Unterschiedliche blockgrößen erlaubt.
Nur einzelne Slots oder Ranges erlaubt (keine Komma-Listen).

```
fotobuch page swap 3:2 5:6          # foto in slot 2 (seite 3) ↔ foto in slot 6 (seite 5)
fotobuch page swap 3:2..4 5:6..9    # block [slots 2-4] ↔ block [slots 6-9]
fotobuch page swap 3:2..10 5        # slots 2-10 von seite 3 ↔ alle fotos von seite 5
fotobuch page swap 1:3..5 1:7..9    # innerhalb derselben seite: slots 3-5 ↔ slots 7-9
```

Algorithmus:

1. Beide foto-blöcke werden vorab gespeichert (snapshot).
2. Linker block wird aus Seite L entfernt; rechter block wird an position `min(L-indices)` eingefügt.
3. Rechter block wird aus Seite R entfernt; linker block wird an position `min(R-indices)` eingefügt.

Beispiel `swap 3:2..3 5:4..6` (seite 3: [a,b,c,d], seite 5: [p,q,r,s,t,u]):

```
links  = slots 2..3 von seite 3 = [b, c]          einfügepos: 1 (0-basiert)
rechts = slots 4..6 von seite 5 = [s, t, u]       einfügepos: 3 (0-basiert)

seite 3 nachher: [a, s, t, u, d]    # [b,c] entfernt, [s,t,u] bei index 1 eingefügt
seite 5 nachher: [p, q, r, b, c]    # [s,t,u] entfernt, [b,c] bei index 3 eingefügt
```

Erlaubt: Swap innerhalb derselben Seite (`swap 1:3 1:7`). Fehler: Überlappende Slot-Nummern, Komma-Liste als Operand.

-----

### `page info`

Gibt Informationen über platzierte Fotos aus. Bei genau einem aufgelösten Slot vertikale
Ansicht, ab zwei Slots Tabellenansicht. Die Entscheidung erfolgt nach dem Auflösen der
`SLOT_EXPR` — `3:2..2` ergibt einen Slot und zeigt die vertikale Ansicht.

```
fotobuch page info 3:2              # einzelner slot → vertikale ansicht
fotobuch page info 3:1..3,7         # mehrere slots → tabellenansicht
fotobuch page info 3                # ganze seite → tabellenansicht
fotobuch page info 3 --weights      # nur weights aller slots
fotobuch page info 3 --ids          # nur ids aller slots
fotobuch page info 3 --pixels       # nur pixeldimensionen aller slots
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
  slot  id                                      pixels     ratio  weight  placed
  1     2024-01-15_Urlaub/IMG_001.jpg           6000x4000  1.50   1.0     10.0, 10.0, 135.5x90.3
  2     2024-01-15_Urlaub/IMG_002.jpg           4000x6000  0.67   2.0     155.0, 10.0, 90.3x135.5
  3     2024-01-15_Urlaub/IMG_003.jpg           6000x4000  1.50   1.0     10.0, 110.0, 135.5x90.3
  7     2024-01-15_Urlaub/IMG_007.jpg           6000x4000  1.50   1.5     155.0, 110.0, 135.5x90.3
```

**`--weights` Ausgabe** (maschinenlesbar, direkt als Input für `page weight` verwendbar):

```
3:1=1.0
3:2=2.0
3:3=1.0
3:7=1.5
```

-----

### `page weight`

Setzt das `area_weight` eines oder mehrerer Slots. Ohne Slot-Angabe wird das Gewicht
für alle Slots der Seite gesetzt.

```
fotobuch page weight 3:2 2.0        # einzelner slot
fotobuch page weight 3:1..3,7 2.0   # mehrere slots, gleiches gewicht
fotobuch page weight 3 2.0          # alle slots der seite
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
- Bei `page swap` (Seiten): beide Blöcke sind überschneidungsfrei
- Bei `page swap`: kein Operand ist eine Komma-Liste
- Bei `page swap` (Slots, gleiche Seite): Slot-Ranges überlappen nicht
- Bei `page combine`: mindestens zwei Seiten angegeben
- Bei `page split`: Slot ist nicht der erste (wäre ein No-Op)

Der Validator arbeitet gegen den geladenen `ProjectState` und gibt strukturierte Fehler zurück:

```rust
enum ValidationError {
    PageNotFound(u32),
    SlotNotFound { page: u32, slot: u32 },
    SwapRangesOverlap,
    SwapNonContiguous,
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
    Info {
        address: String,
        #[arg(long)] weights: bool,
        #[arg(long)] ids: bool,
        #[arg(long)] pixels: bool,
    },
    Weight { address: String, weight: f64 },
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
    SwapRangesOverlap,
    SwapNonContiguous,
    CombineSinglePage(u32),
    SplitAtFirstSlot(u32),
    WeightOutOfRange(f64),
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
) -> Result<PageMoveResult, PageError> { ... }

pub fn execute_info(
    project_root: &Path,
    address: InfoAddress,
    filter: InfoFilter,
) -> Result<PageInfoResult, PageError> { ... }

pub fn execute_weight(
    project_root: &Path,
    address: WeightAddress,
    weight: f64,
) -> Result<(), PageError> { ... }
```

`InfoAddress` und `InfoFilter` kapseln die aufgelöste Adresse und die Flag-Auswahl:

```rust
pub enum InfoAddress {
    Pages(PagesExpr),
    Slots { page: u32, slots: SlotExpr },
}

pub struct InfoFilter {
    pub weights: bool,
    pub ids: bool,
    pub pixels: bool,
}

/// Wenn alle flags false: alle felder ausgeben.
impl InfoFilter {
    pub fn all() -> Self { ... }
    pub fn is_all(&self) -> bool { !self.weights && !self.ids && !self.pixels }
}

pub enum WeightAddress {
    Page(u32),
    Slots { page: u32, slots: SlotExpr },
}
```

`PageError` ist ein Wrapper über `ValidationError` und interne Fehler
(IO, YAML-Serialisierung etc.):

```rust
pub enum PageError {
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
fn parse_info_address(s: &str) -> Result<InfoAddress, ParseError> { ... }
fn parse_weight_address(s: &str) -> Result<WeightAddress, ParseError> { ... }

/// Fehlerformatierung für die Konsole.
fn format_page_error(err: &PageError) -> String { ... }

fn handle_page_move(args: &[String]) -> Result<()> {
    let cmd = parse_move_cmd(args).map_err(|e| /* format + exit */)?;
    let result = fotobuch::commands::page::execute_move(&project_root, cmd)?;
    print_page_move_result(&result);
    Ok(())
}

fn handle_page_info(address: &str, filter: InfoFilter) -> Result<()> {
    let addr = parse_info_address(address)?;
    let result = fotobuch::commands::page::execute_info(&project_root, addr, filter)?;
    print_page_info_result(&result);
    Ok(())
}

fn handle_page_weight(address: &str, weight: f64) -> Result<()> {
    let addr = parse_weight_address(address)?;
    fotobuch::commands::page::execute_weight(&project_root, addr, weight)?;
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

  String (address)
    → parse_info_address()           # Syntax-Check
        → InfoAddress
    → lib::commands::page::execute_info(project_root, addr, filter)
        → ProjectState laden
        → validate()
        → PageInfoResult             # nur lesend, kein YAML schreiben
    → print_page_info_result()       # vertikale oder tabellenansicht

  String (address) + f64 (weight)
    → parse_weight_address()         # Syntax-Check
        → WeightAddress
    → lib::commands::page::execute_weight(project_root, addr, weight)
        → ProjectState laden
        → validate()
        → area_weight mutieren
        → YAML schreiben
```
