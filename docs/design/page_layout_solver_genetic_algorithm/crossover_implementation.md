# Crossover: Topologie-Tausch

Crossover tauscht Teilbaum-**Topologien** zwischen zwei Eltern. Die Blatt-Labels (photo_idx) bleiben im jeweiligen Original-Baum; nur Struktur und Schnittrichtungen werden übertragen.

```
Eltern:   Baum A, Baum B  (gleiche Fotos, unterschiedliche Struktur)
Ergebnis: Baum A', Baum B' (neue Strukturen, gleiche Foto-Zuordnungen)
```

## Algorithmus

1. **Blattanzahl berechnen** — bottom-up Pass, jeder Knoten bekommt die Anzahl Blätter in seinem Teilbaum.

2. **Kompatible Paare finden** — Zwei Teilbäume sind kompatibel wenn: beide innere Knoten, gleiche Blattanzahl ≥ 3, keiner ist Root. Gruppierung nach Blattanzahl (HashMap) → O(N) amortisiert.

3. **Topologie extrahieren** — Pre-Order-Traversal des ausgewählten Teilbaums: Struktur (V/H/Leaf) und Blatt-Labels separat sammeln.

4. **Teilbaum einsetzen** — Zielbaum neu aufbauen (kein In-Place): Original-Knoten kopieren, am Ziel-Index die neue Topologie einsetzen, Labels aus dem **Zielbaum** zuweisen. Neuaufbau statt In-Place weil der neue Teilbaum andere Anzahl innerer Knoten haben kann. Nebeneffekt: Arena danach in Pre-Order.

5. **Beide Kinder fertigen** — Topologie A → Baum B, Topologie B → Baum A.

6. **DFS-Reassign** — Bei `enforce_order=true`: `assign_photos_by_dfs()` auf beiden Kindern aufrufen.

## Invarianten nach Crossover

- Genau N Blätter, N−1 innere Knoten
- Jedes `photo_idx` kommt genau einmal vor
- Alle Indizes gültig, Parent-Referenzen konsistent

## Komplexität

| Operation               | Komplexität                   |
| ----------------------- | ----------------------------- |
| Blattanzahl berechnen   | O(N)                          |
| Kompatible Paare finden | O(N) amortisiert              |
| Topologie extrahieren   | O(k), k = Blätter im Teilbaum |
| Neuaufbau               | O(N)                          |
| **Gesamt**              | **O(N)**                      |

## Label-Reihenfolge

Blatt-Labels werden in Pre-Order aus dem alten Teilbaum gelesen und in derselben Reihenfolge auf die neue Topologie verteilt. Bei `enforce_order=true` ist das irrelevant — `assign_photos_by_dfs()` überschreibt sie ohnehin deterministisch.
