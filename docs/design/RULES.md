# Architektur-Regeln für `src/`

> Projektweite Design-Regeln aus dem Lib-Refactoring.
> Verbindlich für neuen und geänderten Code. Konkrete Umsetzungspläne liegen
> in [`lib-refactoring/`](./lib-refactoring/README.md).

## State-Zugriff über den `StateManager`

**Ziel.** Auf Anhieb erkennbar machen, *was* am State verändert wird und *in
welchem Umfang*; verändernden Zugriff so eng wie nötig halten; und Schreibrecht
soll unmittelbar zum Schreiben führen (kein breiter, ungenutzter Zugriff).

### R1 — Trichotomie des Zugriffs

Das Risiko ist asymmetrisch: Lesen kann keine Invariante verletzen, Schreiben
schon; Lebenszyklus, Pfade und Git-Verwaltung gehören allein dem Manager.
Daraus ergeben sich drei Zugriffsarten:

| Zugriff | Accessor | gültig in |
|---|---|---|
| **Domänen-Lesen** (aus `ProjectState`) | `mgr.state() -> &ProjectState` | überall, wohin weitergereicht |
| **Manager-Lesen** (Git-Baseline, Pfade) | `mgr.…()` → *owned* Ergebnis | nur oberste Schicht |
| **Domänen-Schreiben** | `mgr.write_layout()` / `write_photos()` / `write_config()` | nur oberste Schicht, als schmaler Borrow weitergereicht |

`&mut ProjectState` und ein Sammel-`state_mut()` existieren **nicht**.

### R2 — Lesen breit, Schreiben schmal

- Genau **ein** breiter Lese-Getter `state() -> &ProjectState`. Keine drei
  getrennten Lesezugriffe; Lesen ist risikolos.
- Schreiben **nur über benannte View-Typen** (`WriteLayoutState` usw.). Der
  Typname ist der sichtbare Footprint.

### R3 — Wirbelsäulenregel

`mgr` hält ausschließlich die **Orchestrierungs-Wirbelsäule** eines Commands:
die Pipeline-Funktion und ihre direkten Schritt-Helfer (z. B. `refresh_cache`,
`build_layout`, `finish`). Diese Schicht führt alle Lesezugriffe und
Write-View-Konstruktoren selbst durch und reicht **nur schmale Borrows** eine
Ebene tiefer. `mgr` wird **niemals tiefer weitergereicht**.

### R4 — Erst lesen, dann schreiben

Die Verengung geschieht **oben** im Orchestrator, nicht im Blattknoten.
Muster pro Command: **Lesen → Entscheiden → Schreiben**.

```rust
let plan = plan_xyz(mgr.state(), config)?;        // Lesen: &ProjectState → owned Plan
{
    let mut wls = mgr.write_layout();              // Schreiben: Footprint oben sichtbar
    apply_xyz(wls.layout_mut(), &plan);
}
mgr.finish("…")?;
```

Wer den (kurzen) Orchestrator liest, sieht den vollständigen Footprint; die
Blatt-Helfer (`apply_xyz(layout: &mut Vec<LayoutPage>, …)`) bestätigen ihn nur.

Gleichzeitiges Lesen-A-und-Schreiben-B löst sich über R4 fast immer auf: den
Lesewert zuerst als *owned* Wert ziehen (Indizes, Pläne), dann schreiben. Nur
wenn ein Lesewert sich nachweislich nicht als owned Wert ziehen lässt, ist ein
einzelner maßgeschneiderter Split-Accessor die **dokumentierte Ausnahme**.

### R5 — Read-only-Zugriff ist ein eigener Typ

- `StateManager::open()` = voller Lebenszyklus (Auto-Commit beim Öffnen,
  `finish`, Drop-Warnung).
- `StateManager::open_readonly()` liefert einen **eigenen schmalen Lese-Handle**
  (kein `finish`, kein Write-View, kein Auto-Commit, keine Drop-Warnung). Der
  eingeschränkte Zugriff ist damit *im Typ* sichtbar und wird vom Compiler erzwungen.
- Die Lese-Oberfläche lebt einmal in einem gemeinsamen Lesekern, den `StateManager`
  enthält. Kein `readonly`-Flag (das ließe Schreibzugriffe weiterhin aufrufbar).

### R6 — Validität an einer Stelle

`finish()` validiert autoritativ vor dem Commit; `Drop` warnt bei nicht
committeten Änderungen. Ein blockierender Check pro Einzel-Bearbeitung ist nicht nötig.
