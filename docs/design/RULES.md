# Architektur-Regeln für `src/`

> Projektweite Design-Regeln, die aus dem Lib-Refactoring hervorgegangen sind.
> Verbindlich für neuen und geänderten Code. Die konkreten Umsetzungspläne liegen
> in [`lib-refactoring/`](./lib-refactoring/README.md).

## State-Zugriff über den `StateManager`

**Ziel.** Von „weitem" erkennbar machen, *was* am State verändert wird und *in
welchem Umfang*; verändernden Zugriff so schmal wie nötig schneiden; und „etwas
verändern können" soll möglichst „etwas verändern" nach sich ziehen (kein breiter,
ungenutzter Zugriff).

### R1 — Trichotomie des Zugriffs

Das Risiko ist asymmetrisch: Lesen kann keine Invariante verletzen, Schreiben
schon; Lifecycle/Pfade/git gehören nur dem Manager. Daraus drei Zugriffsarten:

| Zugriff | Accessor | erlaubt wo |
|---|---|---|
| **Domänen-Lesen** (aus `ProjectState`) | `mgr.state() -> &ProjectState` | überall, wohin durchgereicht |
| **Manager-Lesen** (git-Baseline, Pfade) | `mgr.…()` → *owned* Ergebnis | nur oberste Schicht |
| **Domänen-Schreiben** | schmale `mgr.layout_mut()` / `photos_mut()` / `config_mut()` | nur oberste Schicht, schmal nach unten gereicht |

`&mut ProjectState` und ein Sammel-`state_mut()` existieren **nicht**.

### R2 — Lesen breit, Schreiben schmal

- Genau **ein** breiter Lese-Getter `state() -> &ProjectState`. Keine drei
  Einzel-Read-Getter; Lesen ist risikolos.
- Schreiben **nur feldweise** über benannte `*_mut()`-Accessoren. Der Methodenname
  ist der sichtbare Footprint.

### R3 — Wirbelsäulen-Regel

`mgr` darf nur die **Orchestrierungs-Wirbelsäule** eines Commands halten: die
Pipeline-Funktion plus ihre direkten Schritt-Helfer (z. B. `refresh_cache` /
`build_layout` / `finish`). Diese Spitze macht alle `state()`-Reads und
`*_mut()`-Griffe selbst und gibt **nur schmale Borrows** eine Schicht tiefer.
`mgr` wird **nie eine Schicht tiefer** durchgereicht.

### R4 — Narrow early

Die Verengung passiert **oben** im Orchestrator, nicht im Leaf. Muster pro
Command: **Lesen → entscheiden → schreiben**.

```rust
let plan = plan_xyz(mgr.state(), config)?;   // READ: &ProjectState → owned Plan, Borrow endet
let affected = apply_xyz(mgr.layout_mut(), &plan);  // WRITE: Footprint sichtbar ganz oben
mgr.finish("…")?;
```

Wer den (kleinen) Orchestrator liest, sieht den vollständigen Footprint; die
Leaf-Helfer (`apply_xyz(layout: &mut Vec<LayoutPage>, …)`) *bestätigen* ihn nur.

Gleichzeitiges Lesen-A-und-Schreiben-B löst sich über R4 fast immer auf: das
Lese-Abgeleitete vorher als *owned* ziehen (Indizes/Pläne), dann schreiben. Nur
falls ein Lesewert sich nachweislich nicht als owned ziehen lässt, ist ein einzelner
maßgeschneiderter Split-Accessor die **dokumentierte Ausnahme**.

### R5 — Read-only-Zugriff ist ein eigener Typ

- `StateManager::open()` = voller Lebenszyklus (Auto-Commit beim Öffnen,
  `finish`, Drop-Warnung).
- `StateManager::open_readonly()` liefert einen **eigenen schmalen Lese-Handle**
  (kein `finish`, kein `*_mut`, kein Auto-Commit, keine Drop-Warnung). Read-only
  ist damit *im Typ* sichtbar und vom Compiler erzwungen.
- Die Lese-Oberfläche lebt einmal in einem geteilten Lese-Kern, den `StateManager`
  enthält. Kein `readonly`-Flag (würde Mutation aufrufbar lassen).

### R6 — Validität an einer Stelle

`finish()` validiert autoritativ vor dem Commit; `Drop` warnt bei nicht
committeten Änderungen. Ein blockierender Check pro Einzel-Edit ist nicht nötig.
