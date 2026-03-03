#set document(
  title: "Photobook Layout Solver - CSP Model",
  author: "Photobook Solver Team",
  date: datetime.today()
)

#set page(
  paper: "a4",
  margin: (x: 2.5cm, y: 2cm),
  numbering: "1",
)

#set text(
  font: "Linux Libertine",
  size: 11pt,
  lang: "de"
)

#set heading(numbering: "1.1")
#show link: underline

#align(center)[
  #text(20pt, weight: "bold")[
    Photobook Layout Solver
  ]
  
  #v(0.5em)
  #text(16pt)[
    Constraint Satisfaction Problem (CSP) Model
  ]
  
  #v(0.5em)
  #text(12pt)[
    OR-Tools CP-SAT Implementation
  ]
  
  #v(1em)
  #datetime.today().display("[day].[month].[year]")
]

#v(2em)

= Einführung

Dieses Dokument beschreibt die mathematische Formulierung des Photobook Layout Solvers als Constraint Satisfaction Problem (CSP). Die Implementierung verwendet Google OR-Tools CP-SAT Solver.

== Problemstellung

Gegeben:
- Eine geordnete Menge von Fotos $P = {p_1, p_2, ..., p_n}$ mit Metadaten
- Eine Menge von Seiten $S = {s_1, s_2, ..., s_m}$ mit fester Geometrie
- Optimierungsziele für Layout-Qualität
- *Sortierung der Fotos:* Primär nach Gruppe (lexikalisch), sekundär nach Timestamp

Gesucht:
- Zuweisung: Welches Foto auf welche Seite? ($p_i arrow.r.double s_j$)
- Platzierung: Position und Größe jedes Fotos auf der Seite

Constraints:
- *Hard:* Müssen erfüllt sein (chronologische Ordnung, keine Überlappung)
- *Soft:* Sollen optimiert werden (Aspect-Ratio, Gruppen-Kohäsion, Flächen-Gewichte)



= Notation

== Mengen

#let P = $cal(P)$
#let G = $cal(G)$
#let S = $cal(S)$

- $P = {p_1, p_2, ..., p_n}$, Menge aller Fotos (sortiert: 1. Gruppe, 2. Timestamp)
- $G = {g_1, g_2, ..., g_k}$, Menge aller Gruppen (lexikalisch sortiert)
- $S = {s_1, s_2, ..., s_m}$, Menge aller Seiten
- $P_g subset.eq P$, Menge aller Fotos in Gruppe $g in G$
- $P_s subset.eq P$, Menge aller Fotos auf Seite $s in S$

== Parameter

#table(
  columns: (auto, auto, 1fr),
  align: (center, center, left),
  [*Symbol*], [*Typ*], [*Beschreibung*],
  
  [$n$], [$bb(N)$], [Anzahl Fotos],
  [$m$], [$bb(N)$], [Anzahl Seiten (variabel)],
  [$k$], [$bb(N)$], [Anzahl Gruppen],
  [$W$], [$bb(R)^+$], [Seitenbreite (mm)],
  [$H$], [$bb(R)^+$], [Seitenhöhe (mm)],
  [$M$], [$bb(R)^+$], [Rand (margin, mm)],
  [$G$], [$bb(R)^+$], [Gap zwischen Fotos (mm)],
  [$B$], [$bb(R)^+$], [Bleed: Überhang über Papierrand (mm, default: 3)],
  [$T$], [$bb(R)^+$], [Threshold für Bleed-Aktivierung (mm, default: 5)],
  [$p_i$], [Photo], [Foto $i in {1,...,n}$],
  [$w_i$], [$bb(N)$], [Breite von Foto $p_i$ (Pixel)],
  [$h_i$], [$bb(N)$], [Höhe von Foto $p_i$ (Pixel)],
  [$t_i$], [DateTime], [Timestamp von $p_i$],
  [$g_i$], [String], [Gruppe von $p_i$, mit $g_i <= g_(i+1)$ (lex.)],
  [$alpha_i$], [$bb(R)^+$], [Area-weight von $p_i$ (default: 1.0)],
  [$r_i$], [$bb(R)^+$], [Aspect-Ratio: $r_i = w_i / h_i$],
  [$delta_"max"$], [$bb(R)^+$], [Max. Aspect-Ratio Abweichung (default: 0.2)],
  [$epsilon$], [$bb(R)^+$], [Toleranz für Area-Weight (default: 0.25)],
  [$tau$], [$bb(N)$], [Timeout in Sekunden (default: 30)],
  [$m_"target"$], [$bb(N)$], [Ziel-Anzahl Seiten (optional)],
)

*Sortierung:* Die Fotos sind sortiert nach:
1. *Primär:* Gruppe $g_i$ (lexikalisch), d.h. $g_i <= g_(i+1)$ (lexikographisch)
2. *Sekundär:* Timestamp $t_i$ innerhalb jeder Gruppe, d.h. $g_i = g_j ==> t_i <= t_j$ für $i < j$

== Entscheidungsvariablen

#table(
  columns: (auto, auto, 1fr),
  align: (center, center, left),
  [*Variable*], [*Domäne*], [*Bedeutung*],
  
  [$"page"_i$], [${1, ..., m}$], [Seite auf der Foto $p_i$ liegt],
  [$x_i$], [${M, ..., W-M}$], [X-Koordinate von $p_i$ (mm, diskret)],
  [$y_i$], [${M, ..., H-M}$], [Y-Koordinate von $p_i$ (mm, diskret)],
  [$w'_i$], [${1, ..., W-2M}$], [Platzierte Breite von $p_i$ (mm)],
  [$h'_i$], [${1, ..., H-2M}$], [Platzierte Höhe von $p_i$ (mm)],
)

*Diskretisierung:* Alle Koordinaten und Dimensionen sind ganzzahlig in Millimetern.

= Hard Constraints

Hard Constraints müssen zwingend erfüllt sein, damit eine Lösung gültig ist.

== Sortierungs-Ordnung

Die Fotos sind bereits sortiert nach:
1. *Gruppe* (lexikalisch): $g_i <= g_(i+1)$
2. *Timestamp* (innerhalb Gruppe): $g_i = g_j and i < j ==> t_i <= t_j$

Diese Sortierung muss im Layout erhalten bleiben:

$ forall i in {1,...,n-1}: quad "page"_i <= "page"_(i+1) $

Fotos können auf derselben Seite sein oder Foto $p_i$ muss auf einer früheren Seite als $p_(i+1)$ liegen.

*Beispiel:* Gruppe `2024-01-15_Urlaub` kommt vor `2024-02-20_Geburtstag`, und innerhalb jeder Gruppe sind die Fotos chronologisch geordnet.

== Keine Überlappung mit Gap

Fotos auf derselben Seite dürfen sich nicht überlappen und müssen einen Mindestabstand $G$ einhalten.

*Modellierung:* Verwende erweiterte Rechtecke mit Gap-Padding für NoOverlap2D.

Definiere das erweiterte Rechteck für Foto $p_i$:
$ R_i^"gap" = [x_i - G/2, x_i + w'_i + G/2] times [y_i - G/2, y_i + h'_i + G/2] $

Constraint:
$ forall i, j in {1,...,n}, i != j: quad "page"_i = "page"_j ==> R_i^"gap" inter R_j^"gap" = emptyset $

*Implementierung:* Der *NoOverlap2D* Constraint von OR-Tools CP-SAT wird auf die erweiterten Rechtecke $R_i^"gap"$ angewendet. Dadurch wird garantiert, dass der Abstand zwischen zwei Foto-Kanten mindestens $G$ beträgt.

*Bemerkung:* Das Gap $G/2$ wird gleichmäßig auf beide Seiten verteilt, sodass der tatsächliche Abstand zwischen zwei Fotos genau $G$ ist.

== Seiten-Bounds und Margin

Fotos müssen innerhalb der physikalischen Seite (mit Bleed) liegen:

$ forall i in {1,...,n}: cases(
  -B <= x_i <= W + B - w'_i,
  -B <= y_i <= H + B - h'_i
) $

Zusätzlich gilt: Fotos ohne Bleed müssen innerhalb des Margins $M$ bleiben:

$ forall i in {1,...,n}: cases(
  (x_i != -B) ==> (x_i >= M),
  (y_i != -B) ==> (y_i >= M),
  (x_i + w'_i != W + B) ==> (x_i + w'_i <= W - M),
  (y_i + h'_i != H + B) ==> (y_i + h'_i <= H - M)
) $

*Interpretation:* Fotos können entweder Bleed haben (Position $= -B$ bzw. Ende $= W+B$) ODER sie müssen im designierten Bereich $[M, W-M]$ bleiben. Der "verbotene Bereich" $(-B, M)$ ist nur durch Bleed bei $-B$ erreichbar.

== Bleed für Druck-Ränder

Fotos, die näher als $T$ mm am Papierrand sind, müssen um $B$ mm über den Papierrand hinausragen (wichtig für professionellen Druck).

*Wichtig:* Die Distanz zum Papierrand bezieht sich auf das *tatsächliche Foto* $(x_i, y_i, w'_i, h'_i)$, *nicht* auf die Gap-erweiterten Rechtecke $R_i^"gap"$ aus dem NoOverlap-Constraint.

Definiere für jedes Foto $p_i$ die Distanz des tatsächlichen Fotos zum Papierrand:
$ d_i^"left" = x_i, quad d_i^"right" = W - (x_i + w'_i) $
$ d_i^"top" = y_i, quad d_i^"bottom" = H - (y_i + h'_i) $

Constraint: Für jeden Rand, der näher als $T$ ist, wird Bleed erzwungen:

$ forall i in {1,...,n}: cases(
  d_i^"left" < T ==> x_i = -B,
  d_i^"right" < T ==> x_i + w'_i = W + B,
  d_i^"top" < T ==> y_i = -B,
  d_i^"bottom" < T ==> y_i + h'_i = H + B
) $

*Bemerkungen:* 
- Jeder Rand wird unabhängig geprüft
- Fotos können an mehreren Rändern gleichzeitig Bleed haben (z.B. Eckfotos)
- Das Gap $G$ beeinflusst den Bleed nicht - nur die Foto-Position zählt

== Gruppen-Angrenzung

Verschiedene Gruppen dürfen auf einer Seite nur gemischt werden, wenn sie chronologisch angrenzend sind.

Seien $G_A, G_B in G$ zwei verschiedene chronologisch aufeinanderfolgende Gruppen mit $max{t_i | p_i in P_(G_A)} <= min{t_j | p_j in P_(G_B)}$.

Falls beide Gruppen auf derselben Seite $s$ vorkommen:
$ max{x_i | p_i in P_(G_A) inter P_s} < min{x_j | p_j in P_(G_B) inter P_s} $

Der rechteste Startpunkt von Gruppe $G_A$ muss strikt links vom linkesten Startpunkt von Gruppe $G_B$ sein.

= Soft Constraints (Zielfunktion)

Soft Constraints werden als Penalty-Terme in der Zielfunktion berücksichtigt.

== Aspect-Ratio Abweichung

*Herausforderung:* OR-Tools CP-SAT arbeitet nur mit Integer-Werten, Divisionen wie $w'_i / h'_i$ sind problematisch.

*Lösung:* Verwende Produktformulierung statt Division.

Die Abweichung zwischen platziertem und originalem Aspect-Ratio:
$ |w'_i / h'_i - w_i / h_i| $

Äquivalent (da $h'_i, h_i > 0$):
$ |w'_i / h'_i - w_i / h_i| = (| w'_i dot h_i - h'_i dot w_i |) / (h'_i dot h_i) $

Da der Nenner immer positiv ist, minimieren wir äquivalent:
$ "dev"_"aspect"(i) = |w'_i dot h_i - h'_i dot w_i| $

*Vorteil:* Nur Integer-Multiplikationen, keine Division, exakt lösbar.

Hard Constraint für maximale Abweichung $delta_"max"$:
$ forall i: quad |w'_i dot h_i - h'_i dot w_i| <= delta_"max" dot w_i dot h'_i $

Die totale Aspect-Ratio Abweichung:
$ D_"aspect" = sum_(i=1)^n "dev"_"aspect"(i) = sum_(i=1)^n |w'_i dot h_i - h'_i dot w_i| $

== Area-Weight Erfüllung

Für jede Seite $s in S$ mit Fotos $P_s subset P$ definiere:

Nutzbare Fläche der Seite:
$ A_"usable" = (W - 2M) dot (H - 2M) $

Totales Gewicht auf Seite $s$:
$ alpha_"total"(s) = sum_(p_i in P_s) alpha_i $

Ziel-Fläche für Foto $p_i$ auf Seite $s$:
$ A_i^"target" = A_"usable" dot alpha_i / alpha_"total"(s) $

Tatsächliche Fläche:
$ A_i^"actual" = w'_i dot h'_i $

Abweichung mit Toleranz $epsilon$:
$ "dev"_"area"(i) = cases(
  0 & "falls" quad (1-epsilon) dot A_i^"target" <= A_i^"actual" <= (1+epsilon) dot A_i^"target",
  |A_i^"actual" - A_i^"target"| & "sonst"
) $

Totale Area-Weight Abweichung:
$ D_"area" = sum_(i=1)^n "dev"_"area"(i) $

== Gruppen-Kohäsion

Für jede Gruppe $g in G$ definiere die Anzahl der verschiedenen Seiten, auf denen Gruppe $g$ vorkommt:

$ N_"pages"(g) = |{s in S | P_g inter P_s != emptyset}| $

Anzahl der Gruppen-Splits:
$ "splits"(g) = N_"pages"(g) - 1 $

Totale Gruppen-Splits:
$ D_"group" = sum_(g in G) "splits"(g) $

Eine Gruppe auf genau einer Seite hat $"splits"(g) = 0$ (optimal).

== Seitenzahl-Ziel

Falls eine Ziel-Seitenzahl $m_"target"$ vorgegeben ist, definiere die tatsächlich verwendete Anzahl Seiten:

$ m_"used" = max{"page"_i | i in {1,...,n}} $

Abweichung:
$ D_"pages" = |m_"used" - m_"target"| $

Falls kein Ziel vorgegeben: $D_"pages" = 0$.

= Zielfunktion

Die Zielfunktion kombiniert alle Soft-Constraint Penalties gewichtet:

$ "minimize" quad Z = w_"aspect" dot D_"aspect" + w_"area" dot D_"area" + w_"group" dot D_"group" + w_"pages" dot D_"pages" $

mit Gewichten:
$ w_"aspect", w_"area", w_"group", w_"pages" in bb(R)^+ $

#v(1em)

*Standard-Gewichtung:*

#table(
  columns: (auto, auto, 1fr),
  align: (center, center, left),
  [*Gewicht*], [*Wert*], [*Bedeutung*],
  
  [$w_"aspect"$], [1.0], [Mittel - Balance zwischen Foto-Form und Platzierung],
  [$w_"area"$], [10.0], [Hoch - Starke Bestrafung bei Area-Weight Verletzung],
  [$w_"group"$], [2.0], [Mittel-Hoch - Bevorzuge zusammenhängende Gruppen],
  [$w_"pages"$], [0.5], [Niedrig - Seitenzahl ist sekundär],
)

*Bemerkung:* Alle Gewichte sind über die API konfigurierbar und können projektspezifisch angepasst werden.



= Solver-Konfiguration

== CP-SAT Parameter

Der OR-Tools CP-SAT Solver wird mit folgenden Parametern konfiguriert:

- *Timeout:* $tau$ Sekunden (default: 30s), mindestens bis zur ersten Lösung
- *Parallelisierung:* Mehrere Worker-Threads
- *Logging:* Optional für Debugging

== Lösungsstatus

Der Solver kann folgende Stati zurückgeben:

- *OPTIMAL:* Beste Lösung gefunden, alle Constraints erfüllt, Zielfunktion minimal
- *FEASIBLE:* Gültige Lösung gefunden (bei Timeout), aber möglicherweise nicht optimal
- *INFEASIBLE:* Keine Lösung existiert - Constraints sind zu restriktiv
- *UNKNOWN:* Timeout vor erster Lösung - keine gültige Lösung gefunden

== Behandlung von INFEASIBLE

Falls keine Lösung existiert, können folgende Parameter gelockert werden:

1. Erhöhe $delta_"max"$ (mehr Aspect-Ratio Abweichung erlauben)
2. Reduziere Unterschiede in $alpha_i$ (Area-Weights angleichen)
3. Erhöhe $m_"target"$ oder entferne Seitenzahl-Vorgabe
4. Reduziere $G$ (kleinerer Gap zwischen Fotos)



= Erweiterungen

== Iteratives Verfahren (skalierbar)

Für große Probleminstanzen ($n > 200$) kann ein iteratives Verfahren verwendet werden:

1. Initiale Schätzung der Seitenzuweisung basierend auf $m_"target"$ und $n$
2. Für jedes Seiten-Paar $(s, s+1)$:
   - Fixiere $"page"_i$ für alle $i$ mit $"page"_i in.not {s, s+1}$
   - Optimiere nur Fotos mit $"page"_i in {s, s+1}$
   - Übernehme gefundene Positionen
3. Wiederhole bis Konvergenz oder max. Iterationen

*Vorteil:* Reduktion der Problemgröße von $O(n^2)$ auf $O(n)$ bei schrittweiser Optimierung.



= Referenzen

- OR-Tools Documentation: #link("https://developers.google.com/optimization")
- CP-SAT Solver Guide: #link("https://developers.google.com/optimization/cp/cp_solver")
- NoOverlap2D Constraint: #link("https://developers.google.com/optimization/cp/channeling#no_overlap_2d")

#v(2em)

#align(center)[
  #text(10pt, style: "italic")[
    Dieses Dokument wird iterativ erweitert während der Implementierung.
  ]
]

