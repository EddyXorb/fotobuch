#set document(
  title: "Fotobuch Seitenzuordnung — MIP-Formulierung",
  date: datetime.today()
)

#set page(
  paper: "a4",
  margin: (x: 2.5cm, y: 2cm),
  numbering: "1",
)

#set text(
  size: 11pt,
  lang: "de"
)

#set heading(numbering: "1.1")

#align(center)[
  #text(20pt, weight: "bold")[Fotobuch Seitenzuordnung]

  #v(0.3em)
  #text(14pt)[MIP-Formulierung]

  #v(0.8em)
  #datetime.today().display("[day].[month].[year]")
]

#v(2em)

= Mengen, Parameter, Variablen

== Mengen

#table(
  columns: (auto, 1fr),
  align: (center, left),
  stroke: 0.5pt,

  [$K = {1, ..., k}$], [Gruppen (lexikalisch sortiert); $k$ = Anzahl Gruppen],
  [$J = {1, ..., b_"max"}$], [Seiten-Slots; $b_"max"$ = max. erlaubte Seitenanzahl],
  [$K^- = {l in K : |G_l| < g_"min"}$], [Nicht spaltbare Gruppen (zu klein für $g_"min"$)],
)

== Parameter

#table(
  columns: (auto, auto, 1fr),
  align: (center, center, left),
  stroke: 0.5pt,

  [*Symbol*], [*Typ*], [*Bedeutung*],

  [$n$], [$bb(N)$], [Gesamtanzahl Bilder, $n = sum_l |G_l|$],
  [$k$], [$bb(N)$], [Anzahl Gruppen],
  [$|G_l|$], [$bb(N)$], [Anzahl Bilder in Gruppe $l$],
  [$s$], [$bb(N)$], [Ziel-Seitenanzahl],
  [$b_"min", b_"max"$], [$bb(N)$], [Min./Max. erlaubte Seitenanzahl],
  [$p_"min", p_"max"$], [$bb(N)$], [Min./Max. Bilder pro aktiver Seite],
  [$g_"min"$], [$bb(N)$], [Min. Bilder einer Gruppe auf einer Seite bei Spaltung],
  [$g_"max"$], [$bb(N)$], [Max. verschiedene Gruppen pro Seite],
  [$overline(n)$], [$bb(R)^+$], [Ziel-Bildanzahl pro Seite, $overline(n) = n \/ s$],
  [$w_1, w_2, w_3$], [$bb(R)^+$], [Gewichte der Zielfunktion],
  [$M$], [$bb(R)^+$], [Big-M-Konstante: $M = max(overline(n), p_"max")$],
)

*Vorbedingung:* $p_"min" >= g_"min"$.

== Entscheidungsvariablen

#table(
  columns: (auto, auto, 1fr),
  align: (center, center, left),
  stroke: 0.5pt,

  [*Variable*], [*Domäne*], [*Bedeutung*],

  [$g_(l,j)$], [${0, ..., |G_l|}$], [Kumulierte Bilder aus Gruppe $l$ auf Seiten $1..j$; $forall l in K, j in J$],
  [$b_(l,j)$], [${0, 1}$], [Gruppe $l$ hat Bilder auf Seite $j$; $forall l in K, j in J$],
  [$w_(l,j)$], [${0, 1}$], [Gruppe $l$ liegt vollständig auf Seite $j$: $n_(l,j) = |G_l|$; $forall l in K, j in J$],
  [$a_j$], [${0, 1}$], [Seite $j$ ist aktiv; $forall j in J$],
  [$d_j$], [$bb(R)^+_0$], [Abweichung der Seitengröße $j$ von $overline(n)$; $forall j in J$],
  [$d_s$], [$bb(R)^+_0$], [Abweichung der Seitenanzahl von $s$],
)

*Abgeleitete Größen* (Notation, keine eigenen Variablen):

$ n_(l,j) = g_(l,j) - g_(l,j-1) quad "(Bilder von Gruppe" l "auf Seite" j")" $

#pagebreak()

= Nebenbedingungen

== Randbedingungen

$ g_(l,0) = 0 quad forall l in K $
$ g_(l,b_"max") = |G_l| quad forall l in K $

== Monotonie

$ g_(l,j) >= g_(l,j-1) quad forall l in K, forall j in J $

== Seitenaktivität

Aktive Seiten sind zusammenhängend am Anfang:
$ a_j >= a_(j+1) quad forall j in {1, ..., b_"max" - 1} $

Seitenanzahl im erlaubten Bereich:
$ b_"min" <= sum_(j in J) a_j <= b_"max" $

== Seitengröße

$ p_"min" dot a_j <= sum_(l in K) n_(l,j) <= p_"max" dot a_j quad forall j in J $

== Linking $b_(l,j)$

Präsenz genau dann, wenn Bilder vorhanden:
$ n_(l,j) >= b_(l,j) quad forall l in K, forall j in J $
$ n_(l,j) <= |G_l| dot b_(l,j) quad forall l in K, forall j in J $

== Sequentielle Ordnung

Gruppe $l$ darf erst auf Seite $j$ Bilder haben, wenn alle früheren Gruppen bis einschließlich Seite $j$ vollständig kumuliert sind:

$ g_(l-1, j) >= |G_(l-1)| dot b_(l,j) quad forall l >= 2, forall j in J $

== Max. Gruppen pro Seite

$ sum_(l in K) b_(l,j) <= g_"max" quad forall j in J $

== Min. Bilder bei Gruppenspaltung

Linking für "Gruppe vollständig auf einer Seite":

$ n_(l,j) >= |G_l| dot w_(l,j) quad forall l in K, forall j in J $
$ n_(l,j) <= |G_l| - 1 + w_(l,j) quad forall l in K, forall j in J $

Für spaltbare Gruppen ($|G_l| >= g_"min"$):
$ n_(l,j) >= g_"min" dot (b_(l,j) - w_(l,j)) quad forall l in K without K^-, forall j in J $

Für nicht spaltbare Gruppen ($|G_l| < g_"min"$):
$ n_(l,j) = |G_l| dot b_(l,j) quad forall l in K^-, forall j in J $

#pagebreak()

= Zielfunktion

$ "minimize" quad Z = w_1 dot D_"even" + w_2 dot D_"split" + w_3 dot D_"pages" $

== Term 1: Gleichmäßige Verteilung ($D_"even"$)

Abweichung der Seitengröße von der Zielgröße $overline(n)$:

Inaktive Seiten ($a_j = 0$) sollen keinen Beitrag leisten. Big-M-Relaxierung der unteren Schranken mit $M = max(overline(n), p_"max")$:

$ d_j >= sum_(l in K) n_(l,j) - overline(n) - M (1 - a_j) quad forall j in J $
$ d_j >= overline(n) - sum_(l in K) n_(l,j) - M (1 - a_j) quad forall j in J $
$ d_j >= 0 quad forall j in J $

Für aktive Seiten ($a_j = 1$) reduziert sich das auf die Standard-Absolutbetrag-Linearisierung. Für inaktive Seiten werden beide Untergrenzen $<= 0$, sodass $d_j = 0$ optimal ist.

$ D_"even" = sum_(j in J) d_j $

== Term 2: Gruppen-Splits ($D_"split"$)

Anzahl der Seiten pro Gruppe minus 1, summiert über alle Gruppen:

$ D_"split" = sum_(l in K) lr((sum_(j in J) b_(l,j) - 1)) $

Da $sum_(j) b_(l,j) >= 1$ für alle $l$ (jede Gruppe muss vorkommen), ist $D_"split" >= 0$.

== Term 3: Seitenzahl-Abweichung ($D_"pages"$)

Linearisierung des Absolutbetrags über Hilfsvariable $d_s >= 0$:

$ d_s >= sum_(j in J) a_j - s $
$ d_s >= s - sum_(j in J) a_j $

$ D_"pages" = d_s $
