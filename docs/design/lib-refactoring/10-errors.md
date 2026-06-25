# Umsetzungsplan 10 — Fehlermeldungen: CLI-Begriffe aus der Lib verbannen

> Konkretisiert Abschnitt 10 des [Hauptdokuments](./README.md).
> Stand: gegen den aktuellen Code geprüft.

## Worum es geht

Laufzeitfehler der Lib nennen konkrete CLI-Kommandos/Flags als String-Literale.
Zwei Probleme: für Nicht-CLI-Konsumenten (GUI) ist der Hinweis sinnlos, und die
Flag-Namen driften unbemerkt von den clap-Definitionen weg. **Regel:** die Lib
beschreibt nur die *Bedingung* (getippter Fehler), die Oberfläche ergänzt die
*Abhilfe*.

## Was offen ist (verifiziert)

CLI-Strings in Lib-Laufzeitfehlern (`anyhow::bail!`/`anyhow!`, Display, `println!`):

- „Run `fotobuch build` first": `build/build_layout.rs:46,76`, `build/plan.rs:111`,
  `place.rs:103`.
- „`page mode {} a`": `build/build_layout.rs:57–62`.
- „`rebuild --page 0`": `build/build_layout.rs:113` (Warnung), `:118` (Fehler).
- „`fotobuch build release --force`": `build/plan.rs:120–124`.
- „`page mode {p} m`": `page/types.rs:180` (in `ValidationError`-Display — der Typ
  ist schon getippt, nur der Text nennt das Kommando).
- „`fotobuch project list`": `project/switch.rs:34–36`.
- Next-Steps `WELCOME_MESSAGE` mit `fotobuch add/build/place/…`:
  `project/new.rs:65–93`, gedruckt via `println!` in der Lib (`:197`).

Heute nutzt die Lib durchweg `anyhow::bail!`; getippte Enums existieren nur
punktuell (`page/types.rs`, Solver). Die CLI ist eine **eigene Kiste** (`cli/`,
clap-Derive in `cli/cli.rs`).

## Designentscheidung

Zweischichtig (Details + Drift-Mechanismen im [README §10](./README.md#10)):

1. **Lib** gibt getippte Fehler zurück (`thiserror`-Enum je Command), deren
   `#[error(...)]`-Text nur den Sachverhalt nennt — kein Kommando, kein Flag.
2. **CLI** übersetzt Variante → Hinweis (`hint(&err)`), Flag-Namen werden zur
   Laufzeit aus `Cli::command()` gelesen (clap bleibt einzige Namensquelle).
   - Gegen *fehlenden* Hinweis: exhaustives `match` ohne `_`-Arm → Compilezeit.
   - Gegen *falschen* Namen: `long(&cmd, sub_path, arg_id)` liest aus clap; ein
     Test prüft, dass jeder Lookup auflöst.

`project/new.rs`-Next-Steps und `switch.rs`-Hinweise sind reine Oberflächentexte →
ganz in die CLI verschieben (kein Lib-`println!`).

## Commit-Portionen

**N1 · `feat(commands): typed build errors`**
`BuildError` (`thiserror`) für die build-Pfad-Bedingungen einführen: `NoLayout`,
`LayoutDirty { pages }`, `PageIsManual { idx }`, `CoverExcluded`, `PagesWithRelease`.
`bail!`-Stellen in `build/build_layout.rs` und `build/plan.rs` darauf umstellen;
`#[error]`-Texte CLI-frei. `place.rs:103` nutzt `BuildError::NoLayout` mit.
*Verify:* Lib-Tests prüfen Varianten (kein Substring-Match auf „fotobuch").

**N2 · `refactor(page): strip CLI hint from ValidationError`**
`page/types.rs:180`-Text auf reine Bedingung kürzen („page {p} is not in manual
mode"). Der `page mode … m`-Hinweis wandert in die CLI (N5).

**N3 · `feat(project): typed switch error`**
`switch.rs` gibt `ProjectError::NotFound { name }` statt `anyhow!` mit
`project list`-Text.

**N4 · `refactor(project): move welcome/next-steps text to CLI`**
`WELCOME_MESSAGE` + `println!` aus `project/new.rs` entfernen; die Lib gibt nur
das Ergebnis (angelegtes Projekt) zurück. Die CLI druckt die Next-Steps.

**N5 · `feat(cli): error hints from clap`**
In `cli/`: `hint(&err) -> String` je Lib-Fehler-Enum, exhaustives `match` ohne
`_`. `long(cmd, sub_path, arg_id)`-Helfer liest `--flag` aus `Cli::command()`.
Test `hint_lookups_resolve` prüft alle benutzten Lookups. CLI-Fehlerausgabe hängt
den Hinweis an (`error: <bedingung>` / `hint: <abhilfe>`).
*Verify:* Test schlägt fehl, wenn ein Flag umbenannt/entfernt wird.

## Reihenfolge & Risiko

N1–N4 (Lib) zuerst, je für sich durch Tests gedeckt; N5 (CLI) baut darauf auf und
schließt die `match`-Vollständigkeit ab. Mittel — die typed-error-Umstellung
berührt Rückgabetypen mehrerer Commands; Aufrufer in `cli/` im selben Schritt
nachziehen. Doc-Comments (`//! \`fotobuch …\``) bleiben unverändert (Doku, kein
Laufzeit-String).
