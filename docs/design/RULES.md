# Architektur-Regeln für `src/`

> Verbindlich für neuen und geänderten Code. Pläne: [`lib-refactoring/`](./lib-refactoring/README.md).

## State-Zugriff über den `StateManager`

**Ziel:** Footprint einer Änderung von „weitem" sichtbar; Schreibzugriff so schmal
wie nötig; „verändern können" zieht „verändern" nach sich.

- **R1 — Trichotomie.** Domänen-Lesen: `state() -> &ProjectState`. Manager-Lesen
  (git-Baseline, Pfade): `mgr`-Methoden, nur an der Spitze. Schreiben: über die
  View-Konstruktoren `mgr.write_*()` (R7); `*_mut()` lebt auf der View, nicht am
  `StateManager`. Kein `&mut ProjectState`, kein `state_mut()`.

- **R2 — Lesen breit, Schreiben schmal.** Ein breiter `state()`-Getter; Schreiben
  nur feldweise. (Lesen kann keine Invariante verletzen, Schreiben schon.)

- **R3 — Wirbelsäule.** `mgr` hält nur die Orchestrierungs-Spitze (Pipeline +
  direkte Schritt-Helfer) und wird **nie eine Schicht tiefer** gereicht — nur
  schmale Borrows gehen runter.

- **R4 — Narrow early.** Orchestrator-Muster **Lesen → entscheiden → schreiben**:
  Lese-Abgeleitetes vorher als *owned* ziehen, dann schreiben.

- **R5 — Read-only ist ein eigener Typ.** `open()` = voller Lebenszyklus;
  `open_readonly()` = Lese-Handle (kein `finish`/`*_mut`/Auto-Commit). Kein
  `readonly`-Flag. State-Reads tiefer unten laufen über `&ProjectState` — **kein**
  `ReadState`-Typ (R7).

- **R6 — Validität zentral.** `finish()` prüft vor dem Commit, `Drop` warnt. Kein
  Per-Edit-Check.

- **R7 — State-Views statt Parameter-Explosion.** Wrapper über `&mut ProjectState`
  mit genau *einem* schreibbaren Feld; Typname = Footprint:
  `WriteLayoutState` / `WritePhotosState` / `WriteConfigState`. Reines Lesen →
  `&ProjectState`. Erzeugt nur an der Spitze (R3), als *ein* Parameter nach unten.
  Keine layout+photos-Kombi, bis eine Operation beide gleichzeitig mutabel braucht.

  ```rust
  struct WriteLayoutState<'a>(&'a mut ProjectState);
  // layout_mut() schreibt; layout()/photos()/config() lesen
  fn solve_multipage(s: &mut WriteLayoutState, …) -> …
  ```

  *Grenze:* `photos()` und `layout_mut()` nicht gleichzeitig live (beide borgen den
  View) → Reads vorher als owned ziehen (R4); interner Feld-Split nur als Ausnahme.

- **R8 — Einheitlicher Schreib-Flow.** Jeder schreibende Command kapselt
  `open → Ausführung → finish` über `run_write_command` in seiner Top-Level-Funktion.
  Die Closure ist die Orchestrierungs-Spitze (R3): sie liest/validiert über `state()`,
  erzeugt **eine** Write-View (`write_*()`, R7) und übergibt **diese** — nicht den
  `mgr` — an die eigentliche Schreib-Unterfunktion. So sieht man am Signaturtyp den
  Footprint, und der Commit-Flow ist über alle Commands gleich.

  ```rust
  pub fn foo(root: &Path, cfg: &FooConfig) -> Result<CommandOutput<FooResult>> {
      run_write_command(root, |mgr| {
          // lesen/entscheiden über mgr.state() (R4)
          let mut view = mgr.write_layout();
          let result = apply_foo(&mut view, cfg)?; // Unterfunktion bekommt nur die View
          Ok((commit_msg, result))
      })
  }
  fn apply_foo(s: &mut WriteLayoutState, cfg: &FooConfig) -> Result<FooResult> { … }
  ```
