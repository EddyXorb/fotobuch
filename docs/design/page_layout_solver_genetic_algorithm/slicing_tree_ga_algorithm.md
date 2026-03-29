# Slicing-Tree Photo Layout mit Genetic Algorithm

Basierend auf: Fan, "Photo Layout with a Fast Evaluation Method and Genetic Algorithm" (IEEE ICMEW 2012)

## Problemstellung

Gegeben: Rechteckiger Canvas (W×H), N Fotos mit Aspect-Ratios $a_i = w_i/h_i$, gewünschte relative Größen $t_i$, fester Abstand β zwischen Fotos.

Gesucht: Tight-Packing-Layout ohne Überlappung, bei dem Aspect-Ratios erhalten bleiben.

## Slicing Structure

Das Layout wird als vollständiger Binärbaum dargestellt:

- **Blattknoten:** Jeweils ein Foto
- **Innere Knoten:** `V` (vertikaler Schnitt, Kinder nebeneinander) oder `H` (horizontaler Schnitt, Kinder übereinander)

Bei N Fotos hat der Baum N Blätter und N−1 innere Knoten. Arena-basiert in einem `Vec<Node>` (O(1) Clone per memcpy). Root hat Sentinel-Parent.

Folgende Eigenschaften sind strukturell garantiert (kein Fitness-Term nötig):
- Alignment entlang Schnittlinien
- Uniform gaps (β konstant)
- Keine Überlappung

## Affiner Layout-Solver (O(N))

Für β=0 lässt sich jeder Knoten durch sein Aspect-Ratio `a = w/h` beschreiben. Bei β>0 wird diese Beziehung **affin**: `w = α·h + γ`. Pro Knoten wird ein Paar (α, γ) propagiert.

### Bottom-up: Koeffizienten berechnen

**Blatt** mit Aspect-Ratio $a_i$: `α = aᵢ, γ = 0`

**V-Knoten** (gleiche Höhe, Breiten addieren sich):
```
α_V = αl + αr
γ_V = γl + γr + β
```

**H-Knoten** (gleiche Breite, Höhen addieren sich — Invertierung nötig da w = α·h + γ nach h aufgelöst werden muss):
```
α_H = αl·αr / (αl + αr)
γ_H = (γl/αl + γr/αr − β) · αl·αr / (αl + αr)
```

Invariante: α > 0 für alle Knoten (per Induktion beweisbar). Damit ist die Invertierung immer zulässig.

### Top-down: Dimensionen zuweisen

**Root** bekommt Canvas (W, H): Höhe H voll ausnutzen; falls w_root > W, stattdessen Breite W voll ausnutzen.

**V-Knoten** gibt Kindern gleiche Höhe: `wₗ = αl·h + γl`, `wᵣ = αr·h + γr`

**H-Knoten** gibt Kindern gleiche Breite: `hₗ = (w − γl)/αl`, `hᵣ = (w − γr)/αr`

**Positionen:** V-Knoten: Kind r beginnt bei x + wl + β. H-Knoten: Kind r beginnt bei y + hl + β.

## Kostenfunktion

```
C = w_size·C1 + w_coverage·C2 + w_bary·C_bary
```

Standardgewichte: `w_size=0.2, w_coverage=1.0, w_bary=0.0`

### C1 – Größenverteilung

$$C_1 = \sum_{i=0}^{N-1} k_i \cdot (s_i - t_i)^2$$

- $s_i = (w_i \cdot h_i) / S$ — normalisierte Fläche (S = Canvas-Fläche)
- $t_i$ — normalisierte Wunschgröße (`area_weight_i / Σ weights`)
- $k_i = 50$ falls $s_i/t_i < 0.5$ (Undersized-Penalty), sonst $k_i = 1$

### C2 – Canvas-Abdeckung

$$C_2 = 1 - \sum_{i=0}^{N-1} s_i$$

### C_bary – Baryzentrum-Zentrierung

Flächengewichteter Schwerpunkt, normalisierter quadratischer Abstand zum Canvas-Zentrum:

```
C_bary = ((bx − W/2) / W)² + ((by − H/2) / H)²
```

Liegt in [0, 0.25], unabhängig von Canvas-Größe.

## Genetic Algorithm

### Initialisierung

Zufälligen Baum erzeugen (N−1 mal ein Blatt durch einen inneren Knoten mit zwei Kindern ersetzen). Mit `enforce_order=true` (Standard): Fotos per DFS-Preorder zuweisen statt Fisher-Yates. Siehe [in_page_ordering_improvement.md](in_page_ordering_improvement.md).

### Mutation

- `enforce_order=true`: Cut-Flip (V↔H) auf einem zufälligen inneren Knoten. Kein Reassign nötig (DFS-Reihenfolge ändert sich nicht).
- `enforce_order=false`: Zwei zufällige Blätter tauschen.

### Crossover

Zwei kompatible Teilbäume (gleiche Blattanzahl ≥ 3, nicht Root) finden und Topologien tauschen. Labels bleiben im jeweiligen Baum. Bei `enforce_order=true`: danach `assign_photos_by_dfs()` auf beiden Kindern aufrufen. Siehe [crossover_implementation.md](crossover_implementation.md).

### Island Model

Mehrere unabhängige Populationen auf separaten Threads. Migration alle M Generationen: k beste Individuen → zufällige Nachbar-Island.

Vorteile: Natürlich parallel (kein Locking während Evolution), mehr Diversität, bessere Konvergenz als eine einzelne große Population.

### Komplexitäten

| Operation                                | Komplexität  |
| ---------------------------------------- | ------------ |
| Layout Solver (ein Baum)                 | O(N)         |
| GA gesamt (P Population, G Generationen) | O(P × G × N) |
