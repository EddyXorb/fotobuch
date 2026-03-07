# Slicing-Tree Photo Layout mit Genetic Algorithm

Basierend auf: Fan, "Photo Layout with a Fast Evaluation Method and Genetic Algorithm" (IEEE ICMEW 2012)

## Problemstellung

Gegeben: Rechteckiger Canvas (W×H), N Fotos mit Aspect-Ratios $a_i = w_i/h_i$, gewünschte relative Größen $t_i$, fester Abstand β zwischen Fotos.

Gesucht: Tight-Packing-Layout ohne Überlappung, bei dem Aspect-Ratios erhalten bleiben.

## Kernidee: Slicing Structure

Das Layout wird als **vollständiger Binärbaum** dargestellt:

- **Blattknoten (L):** Jeweils ein Foto mit Label $p_i$
- **Innere Knoten (I):** Label `V` (vertikaler Schnitt) oder `H` (horizontaler Schnitt)
- `V`-Knoten: Kinder werden **nebeneinander** (links/rechts) platziert
- `H`-Knoten: Kinder werden **übereinander** (oben/unten) platziert

Bei N Fotos hat der Baum N Blätter und N−1 innere Knoten.

## Fast Layout Solver (O(N))

Zwei Durchläufe über den Baum:

### Pass 1: Aspect-Ratio berechnen (bottom-up)

Für jeden inneren Knoten mit Kindern mit Aspect-Ratios $a_1$, $a_2$:

```
V-Knoten:  a_parent = a1 + a2
H-Knoten:  1/a_parent = 1/a1 + 1/a2   →   a_parent = (a1 * a2) / (a1 + a2)
```

Blattknoten: $a_i$ direkt aus Foto-Metadaten.

### Pass 2: Dimensionen zuweisen (top-down)

Root-Knoten bekommt Canvas-Dimensionen:

```
if W < a_root * H:
    w_root = W
else:
    w_root = a_root * H
h_root = w_root / a_root
```

Für jeden inneren Knoten mit Breite $w$ und Höhe $h$, wobei die Kinder Aspect-Ratios $a_1$, $a_2$ haben:

```
V-Knoten (nebeneinander, gleiche Höhe):
    Kind 1: w1 = a1 * h,   h1 = h
    Kind 2: w2 = a2 * h,   h2 = h

H-Knoten (übereinander, gleiche Breite):
    Kind 1: w1 = w,   h1 = w / a1
    Kind 2: w2 = w,   h2 = w / a2
```

**Hinweis:** β wird beim Fast Solver ignoriert (β≡0). Nur für die finale Lösung wird β exakt eingerechnet (siehe Abschnitt β-Korrektur).

### Positionen berechnen

Ebenfalls top-down: Root startet bei (0,0). Dann:

```
V-Knoten an Position (x, y):
    Kind 1: (x, y)
    Kind 2: (x + w1, y)

H-Knoten an Position (x, y):
    Kind 1: (x, y)
    Kind 2: (x, y + h1)
```

## β-Korrektur für die finale Lösung

Bei β>0 verbrauchen die Gaps absolute Breite/Höhe. Die einfache Rekursion bricht, weil das effektive Aspect-Ratio jedes Knotens von den konkreten Dimensionen abhängt, nicht nur von den Kind-Ratios.

### Warum β die Rekursion kaputt macht

Bei β=0 gilt für einen V-Knoten: `w_parent = w1 + w2 = (a1 + a2) · h`. Das Aspect-Ratio ist unabhängig von der absoluten Größe.

Bei β>0: `w_parent = w1 + w2 + β = (a1 + a2) · h + β`. Das Ratio `w_parent / h` hängt jetzt von `h` ab — es ist nicht mehr konstant.

### Ansatz 1: Lineares Gleichungssystem (Atkins, O(N³))

Für jeden Knoten eine Unbekannte (z.B. die Höhe $h_k$ für V-Knoten, Breite $w_k$ für H-Knoten). Dann:

**Blatt $i$:** $w_i = a_i \cdot h_i$

**V-Knoten $k$ mit Kindern $l$, $r$:**
- Gleiche Höhe: $h_l = h_k$, $h_r = h_k$
- Breite: $w_k = w_l + w_r + \beta$

**H-Knoten $k$ mit Kindern $l$, $r$:**
- Gleiche Breite: $w_l = w_k$, $w_r = w_k$
- Höhe: $h_k = h_l + h_r + \beta$

**Randbedingung am Root:** $w_{root} = W$ oder $h_{root} = H$ (je nachdem welche Dimension bindet).

Durch Einsetzen entsteht ein lineares Gleichungssystem in ~N Unbekannten, lösbar per LU-Zerlegung oder SVD.

Rust-Libraries: `nalgebra` oder `faer`.

### Ansatz 2: Affine Rekursion (O(N))

**Idee:** Bei β=0 beschreibt jeder Knoten seine Geometrie mit einem Skalar: dem Aspect-Ratio `a = w/h`. Bei β>0 wird diese Beziehung **affin**: `w = α·h + γ`. Statt eines Skalars propagiert man ein Paar (α, γ) durch den Baum.

**Konvention:** Jeder Knoten speichert (α, γ) mit der Bedeutung: `w_node = α · h_node + γ`.

#### Bottom-up: Koeffizienten (α, γ) berechnen

**Blatt** mit Aspect-Ratio $a_i$:

```
α = aᵢ
γ = 0
```

**V-Knoten** (Kinder l, r nebeneinander — gleiche Höhe, Breiten addieren sich):

V erzwingt $h_l = h_r = h_V$. Kinder liefern $w_l = \alpha_l \cdot h_V + \gamma_l$ und $w_r = \alpha_r \cdot h_V + \gamma_r$.

```
w_V = w_l + w_r + β
    = (αl + αr) · h_V + (γl + γr + β)
```

Ergebnis:

```
α_V = αl + αr
γ_V = γl + γr + β
```

**H-Knoten** (Kinder l, r übereinander — gleiche Breite, Höhen addieren sich):

H erzwingt $w_l = w_r = w_H$. Die Kind-Gleichungen `w = α·h + γ` müssen nach h aufgelöst werden:

```
h_l = (w_H − γl) / αl
h_r = (w_H − γr) / αr
```

(Invertierung ist zulässig, da α > 0 für alle Knoten — siehe Beweis.)

Einsetzen in $h_H = h_l + h_r + \beta$:

```
h_H = w_H/αl − γl/αl + w_H/αr − γr/αr + β
    = w_H · (1/αl + 1/αr) + (−γl/αl − γr/αr + β)
```

Umstellen nach $w_H$, mit $S = 1/\alpha_l + 1/\alpha_r$:

```
w_H = h_H / S − (−γl/αl − γr/αr + β) / S
    = h_H / S + (γl/αl + γr/αr − β) / S
```

Ergebnis:

```
α_H = 1/S = αl·αr / (αl + αr)
γ_H = (γl/αl + γr/αr − β) · αl·αr / (αl + αr)
```

**Warum der H-Fall eine Invertierung braucht:** Alle Knoten speichern einheitlich `w = α·h + γ`. Ein H-Knoten gibt seinen Kindern aber eine **Breite** vor (nicht Höhe). Deshalb muss er die Kind-Gleichungen nach h auflösen, um deren Höhen zu bestimmen und aufzuaddieren. Das passiert bei **jedem** H-Knoten automatisch, unabhängig davon ob die Kinder Blätter, V-Knoten oder H-Knoten sind — die (α, γ)-Koeffizienten kapseln den gesamten Teilbaum.

#### Top-down: Dimensionen zuweisen

**Root:** Wir haben $w_{root} = \alpha_{root} \cdot h_{root} + \gamma_{root}$ und den Canvas (W, H).

```
// Versuch 1: Höhe voll ausnutzen
h_root = H
w_root = α_root · H + γ_root
if w_root > W:
    // Versuch 2: Breite voll ausnutzen
    w_root = W
    h_root = (W − γ_root) / α_root
```

**V-Knoten** mit bekanntem $(w_V, h_V)$ — Kinder bekommen gleiche Höhe:

```
h_l = h_V
w_l = αl · h_V + γl

h_r = h_V
w_r = αr · h_V + γr
```

**H-Knoten** mit bekanntem $(w_H, h_H)$ — Kinder bekommen gleiche Breite:

```
w_l = w_H
h_l = (w_H − γl) / αl

w_r = w_H
h_r = (w_H − γr) / αr
```

#### Positionen berechnen (ebenfalls top-down)

Identisch zum β=0-Fall, nur mit β-Offset:

```
V-Knoten an Position (x, y):
    Kind l: (x, y)
    Kind r: (x + w_l + β, y)

H-Knoten an Position (x, y):
    Kind l: (x, y)
    Kind r: (x, y + h_l + β)
```

#### Zusammenfassung: β=0 als Spezialfall

| | β=0 | β>0 |
|---|---|---|
| Pro Knoten gespeichert | a (Skalar) | (α, γ) (Paar) |
| Blatt | a = aᵢ | α=aᵢ, γ=0 |
| V bottom-up | a = a₁+a₂ | α=α₁+α₂, γ=γ₁+γ₂+β |
| H bottom-up | 1/a = 1/a₁+1/a₂ | Invertierung + Addition (s.o.) |
| Top-down | w = a·h | w = α·h + γ |
| Komplexität | O(N) | O(N) |

**⚠️ Unklar:** Warum weder Atkins (O(N³)) noch Fan diesen O(N)-Ansatz nutzen. Möglicherweise Probleme mit numerischer Instabilität bei tiefen Bäumen. **Vor Verwendung gegen Ansatz 1 an Testfällen verifizieren.**

### Korrektheitsbeweis (Induktion)

**Behauptung:** Für jeden Teilbaum mit Wurzel k gilt: $w_k = \alpha_k \cdot h_k + \gamma_k$ mit berechenbaren Konstanten $\alpha_k > 0$, $\gamma_k$.

**Basis:** Blatt mit Aspect-Ratio $a_i$: $w_i = a_i \cdot h_i + 0$. Affin, $\alpha_i = a_i > 0$. ✓

**Schritt V-Knoten** mit Kindern l, r (nach IV affin mit $\alpha_l, \gamma_l, \alpha_r, \gamma_r$):

V erzwingt $h_l = h_r = h_k$, also:

$$w_k = w_l + w_r + \beta = (\alpha_l + \alpha_r) \cdot h_k + (\gamma_l + \gamma_r + \beta)$$

Affin, $\alpha_k = \alpha_l + \alpha_r > 0$. ✓

**Schritt H-Knoten** mit Kindern l, r (nach IV affin):

H erzwingt $w_l = w_r = w_k$. Invertieren (möglich, da $\alpha > 0$):

$$h_l = (w_k - \gamma_l) / \alpha_l, \quad h_r = (w_k - \gamma_r) / \alpha_r$$

Einsetzen in $h_k = h_l + h_r + \beta$:

$$h_k = w_k \cdot (1/\alpha_l + 1/\alpha_r) - \gamma_l/\alpha_l - \gamma_r/\alpha_r + \beta$$

Umstellen nach $w_k$:

$$w_k = \frac{\alpha_l \alpha_r}{\alpha_l + \alpha_r} \cdot h_k + \frac{\alpha_l \alpha_r}{\alpha_l + \alpha_r} \cdot (\gamma_l/\alpha_l + \gamma_r/\alpha_r - \beta)$$

Affin, $\alpha_k = \alpha_l \alpha_r / (\alpha_l + \alpha_r) > 0$. ✓

**Kern des Arguments:** Ein V-Knoten hat genau eine freie Variable (h), ein H-Knoten genau eine (w). Die jeweils andere Dimension aller Kinder wird durch diese eine Variable determiniert. Es kommen nie mehrere unabhängige Unbekannte rein — egal wie tief der Baum ist.

## Kostenfunktion

```
C = w1·C1 + w2·C2 + w_bary·C_bary + w_order·C_order
```

### C1 – Größenverteilung (aus Paper)

$$C_1 = \sum_{i=0}^{N-1} k_i \cdot (s_i - t_i)^2$$

- $s_i = (w_i \cdot h_i) / S$ — normalisierte Fläche des Fotos im Layout (S = Canvas-Fläche)
- $t_i$ — normalisierte Wunschgröße (Summe aller $t_i$ = 1)
- $k_i = 5$ falls $s_i/t_i < 0.5$ (zu kleine Fotos werden stärker bestraft), sonst $k_i = 1$

### C2 – Canvas-Abdeckung (aus Paper)

$$C_2 = 1 - \sum_{i=0}^{N-1} s_i$$

Misst den Leerraum. Idealwert: 0.

### C_bary – Baryzentrum-Zentrierung (neu)

Flächengewichteter Schwerpunkt aller Fotos:

```
cx_i = x_i + w_i/2          // Mittelpunkt Foto i
cy_i = y_i + h_i/2
A_i  = w_i · h_i            // Fläche Foto i

bx = Σ(A_i · cx_i) / Σ(A_i)   // Schwerpunkt
by = Σ(A_i · cy_i) / Σ(A_i)
```

Penalty als normalisierter quadratischer Abstand zum Canvas-Zentrum:

```
C_bary = ((bx − W/2) / W)² + ((by − H/2) / H)²
```

Normalisierung durch W, H damit der Term unabhängig von der Canvas-Größe in [0, 0.25] liegt.

### C_order – Chronologische Leseordnung (neu)

Fotos sind nach Gruppe und Timestamp sortiert (Index i < j → Foto i kommt vor j). Auf einer Seite soll diese Ordnung einer Leserichtung (links-oben → rechts-unten) entsprechen.

Normalisierte Leseposition jedes Fotos:

```
score_i = x_i / W + y_i / H
```

Penalty für jede Inversion aufeinanderfolgender Fotos auf derselben Seite:

```
C_order = Σ max(0, score_i − score_{i+1})   für alle i wo page_i = page_{i+1}
```

Korrekte Ordnung: Penalty = 0. Nur Inversionen werden bestraft, nicht der absolute Abstand.

### Gewichte

| Gewicht | Vorschlag | Begründung |
|---|---|---|
| $w_1$ | 1.0 | Größenverteilung (Paper-Default) |
| $w_2$ | 0.15 | Canvas-Abdeckung (Paper-Default, dort λ) |
| $w_{bary}$ | 0.5 | Zentrierung — sekundär, da gute Abdeckung bereits zentriert |
| $w_{order}$ | 0.3 | Leseordnung — weich, da nicht immer erreichbar |

Gewichte sind projektspezifisch anzupassen. C1 und C2 haben andere Wertebereiche als die neuen Terme — ggf. nach ersten Tests kalibrieren.

### Was der Slicing-Tree bereits garantiert

Folgende Ziele aus dem CSP-Modell brauchen keinen Kostenfunktions-Term:

- **Alignment:** Kanten fluchten immer entlang der Schnittlinien — strukturell garantiert
- **Gap-Uniformität:** β ist konstant an jeder Schnittstelle — strukturell garantiert
- **Keine Überlappung:** Durch Baumstruktur ausgeschlossen

## Genetic Algorithm

### Initialisierung: Zufällige Bäume erzeugen

1. Erzeuge N−1 innere Knoten: Starte mit Root (zufällig V/H). Wähle wiederholt einen bestehenden I-Knoten mit <2 Kindern, füge neuen I-Knoten als Kind hinzu (zufällig V/H).
2. Fülle alle offenen Kind-Slots mit Blattknoten. Weise jedem Blatt ein zufällig gezogenes (ohne Zurücklegen) Foto-Label zu.

**Populationsgröße:** 100 × N.

### Mutation

Wähle zufällig zwei Knoten **gleichen Typs** (beide I oder beide L) mit **unterschiedlichen Labels** im selben Baum. Tausche deren Labels. Baumstruktur bleibt unverändert.

### Crossover

1. Finde in Baum A alle Teilbäume und deren Blattanzahlen.
2. Finde in Baum B alle Teilbäume und deren Blattanzahlen.
3. Finde Paare $(st_A, st_B)$ mit **gleicher Blattanzahl ≥ 3**.
4. Falls kein Paar existiert: Abbruch. Sonst: Wähle zufälliges Paar und tausche die Teilbäume. Blatt-Labels bleiben im jeweiligen Original-Baum und werden auf neue Knoten verteilt.

### Selektion & Evolution

Standard-GA-Loop:

1. Erzeuge initiale Population
2. Bewerte alle Individuen (Fast Layout Solver → Kostenfunktion)
3. Selektion der Besten
4. Erzeuge nächste Generation durch Mutation und Crossover
5. Wiederhole bis Abbruchkriterium (z.B. 40 Generationen oder $C_1 < 0.02$)

### Parallelisierung: Island Model

Statt einer großen Population mehrere unabhängige Populationen ("Islands") auf separaten Threads. Alle paar Generationen migrieren die besten Individuen zwischen Islands.

```
Konfiguration (Beispiel für N=25):
    Islands:              8 (= Anzahl CPU-Kerne)
    Population pro Island: 300 (statt 2500 gesamt)
    Migration alle:       5 Generationen
    Migranten pro Runde:  2 beste Individuen → zufällige Nachbar-Island
```

**Ablauf:**

```
1. Jeder Thread erzeugt eigene Population mit eigenem RNG
2. Jeder Thread läuft unabhängig: Evaluate → Select → Mutate/Crossover
3. Alle M Generationen: Synchronisationspunkt
   - Jede Island schickt ihre k besten Individuen an eine Nachbar-Island
   - Empfangene Individuen ersetzen die schlechtesten der Ziel-Island
4. Weiter bis globales Abbruchkriterium (beste Fitness über alle Islands)
```

**Vorteile gegenüber einer einzelnen großen Population:**

- Natürlich parallel — kein Locking während der Evolution, nur kurze Sync-Punkte bei Migration
- Mehr Diversität — verschiedene Islands explorieren verschiedene Regionen des Suchraums
- Weniger Crossover-Overhead pro Island (Teilbaum-Matching ist O(N²) pro Paar)
- Konvergiert typischerweise zu besseren Lösungen als eine gleich große Einzelpopulation

## Zusammenfassung der Komplexitäten

| Operation | Komplexität |
|---|---|
| Layout Solver β=0 (ein Baum) | O(N) |
| Layout Solver β>0 affin (ein Baum) | O(N) |
| Layout Solver β>0 via Gleichungssystem | O(N³) |
| GA gesamt (P=Population, G=Generationen) | O(P × G × N) |

## Implementierungshinweise für Rust

### Baumstruktur: Arena im Vec

Knoten in einem `Vec` statt als Heap-allozierte `Box<Node>`-Bäume. Ermöglicht O(1) Deep-Copy per `Vec::clone` (memcpy) — kritisch, da der GA ständig Bäume kopiert.

```rust
#[derive(Clone, Copy)]
enum Cut { V, H }

#[derive(Clone, Copy)]
enum Node {
    Leaf { photo_idx: u16, parent: u16 },
    Internal { cut: Cut, left: u16, right: u16, parent: u16 },
}

#[derive(Clone)]
struct SlicingTree {
    nodes: Vec<Node>,  // Root ist nodes[0]
}
```

Bei N Fotos: 2N−1 Knoten, ~16 Bytes pro Knoten → ~3 KB bei N=100. Passt komplett in L1-Cache. Root hat `parent: u16::MAX` als Sentinel.

**Kein Heap-Indexierung (2i+1, 2i+2):** Die Bäume sind *voll* (0 oder 2 Kinder) aber nicht *vollständig* (beliebige Struktur). Stattdessen explizite `left`/`right`-Indizes in den Vec.

**Traversierung:** Rekursiv über Indizes statt linearer Iteration. Bei <200 Knoten im L1-Cache kein Performance-Unterschied zu topologisch sortierter Iteration.

### Baum-Aufbau

Starte mit einem Blatt. N−1 mal ein zufälliges Blatt durch einen inneren Knoten mit zwei neuen Blättern ersetzen. Ein separater `leaves`-Vec trackt die verfügbaren Blätter.

```rust
fn random_tree(n: usize, rng: &mut impl Rng) -> SlicingTree {
    let mut nodes = vec![Node::Leaf { photo_idx: 0, parent: u16::MAX }];
    let mut leaves: Vec<u16> = vec![0];

    for _ in 0..n - 1 {
        let leaf_pos = rng.gen_range(0..leaves.len());
        let leaf_idx = leaves[leaf_pos];

        // Alten parent merken, bevor der Knoten überschrieben wird
        let old_parent = match nodes[leaf_idx as usize] {
            Node::Leaf { parent, .. } => parent,
            _ => unreachable!(),
        };

        // Zwei neue Blätter anhängen
        let left = nodes.len() as u16;
        let right = left + 1;
        nodes.push(Node::Leaf { photo_idx: 0, parent: leaf_idx });
        nodes.push(Node::Leaf { photo_idx: 0, parent: leaf_idx });

        // Altes Blatt wird in-place zum inneren Knoten
        let cut = if rng.gen_bool(0.5) { Cut::V } else { Cut::H };
        nodes[leaf_idx as usize] = Node::Internal {
            cut, left, right, parent: old_parent,
        };

        // leaves aktualisieren: O(1) per swap_remove
        leaves.swap_remove(leaf_pos);
        leaves.push(left);
        leaves.push(right);
    }

    // Foto-Labels zufällig auf Blätter verteilen
    let mut photos: Vec<u16> = (0..n as u16).collect();
    photos.shuffle(rng);
    let mut photo_iter = photos.into_iter();
    for node in &mut nodes {
        if let Node::Leaf { photo_idx, .. } = node {
            *photo_idx = photo_iter.next().unwrap();
        }
    }

    SlicingTree { nodes }
}
```

Die Knoten liegen nicht in Traversierungsreihenfolge im Vec — das ist bei der Problemgröße irrelevant.

### Sonstige Hinweise

- **Aspect-Ratios als f64** — kein Integer-Constraint nötig, da kein CP-SAT-Solver verwendet wird
- **β direkt im GA:** Affiner Ansatz hat gleiche O(N)-Komplexität wie β=0-Solver, kann also direkt für die Fitness-Bewertung verwendet werden statt nur am Ende
- **Parallelisierung:** Fitness-Evaluation der Population ist trivial parallelisierbar (rayon)
- **Frühes Stoppen:** Konvergenz-Check auf Plateau der Kostenfunktion
