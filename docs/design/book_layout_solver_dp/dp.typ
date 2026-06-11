#set document(
  title: "Fotobuch Seitenzuordnung — DP-Formulierung",
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
  #text(14pt)[Exakte Lösung per dynamischer Programmierung]

  #v(0.8em)
  #datetime.today().display("[day].[month].[year]")
]

#v(2em)

= Motivation

Die bisherige MIP-Formulierung (siehe `book_layout_solver_mip/page_assignment_mip.typ`)
skaliert schlecht: Bei $n approx 1000$ Bildern und $b_"max" approx 100$ Seiten-Slots
entstehen $cal(O)(k dot b_"max")$ Integer-Variablen mit Big-M-Kopplungen und schwacher
LP-Relaxierung. Dieses Dokument zeigt, dass das identische Problem — gleiche zulässige
Menge, gleiche Zielfunktion — als *Sequence-Partitioning-Problem* exakt in
Polynomialzeit per dynamischer Programmierung (DP) lösbar ist.

= Dynamische Programmierung in Kürze

Dynamische Programmierung löst ein Optimierungsproblem, indem es in *Teilprobleme*
zerlegt wird, deren optimale Lösungen wiederverwendbar sind. Zwei Voraussetzungen:

+ *Optimale Substruktur:* Die optimale Gesamtlösung enthält optimale Teillösungen.
  Beginnt in einer optimalen Aufteilung die letzte Seite bei Bild $i$, so müssen die
  Bilder $1..i$ darin selbst optimal aufgeteilt sein — andernfalls ließe sich die
  Gesamtlösung durch Austausch verbessern (Austauschargument).
+ *Überlappende Teilprobleme:* Derselbe Zwischenzustand („erste $i$ Bilder auf $m$
  Seiten") tritt in vielen Gesamtlösungen auf, wird aber nur *einmal* gelöst und
  tabelliert (Memoisierung).

Der exponentiell große Suchraum (alle Cut-Kombinationen, $cal(O)(2^n)$) kollabiert,
weil der Zustand $(i, m)$ alles zusammenfasst, was für die restliche Entscheidung
relevant ist: _Wie_ die ersten $i$ Bilder intern geschnitten wurden, ist für die
folgenden Seiten irrelevant — keine Nebenbedingung und kein Zielterm koppelt über
eine Seitengrenze hinweg (@sec:zerlegung). Die Rekursion über die „letzte
Entscheidung" (Größe der letzten Seite) heißt *Bellman-Rekursion*. Äquivalente
Sichtweise: kürzester Pfad in einem azyklischen Graphen (DAG), dessen Knoten die
Zustände $(i, m)$ sind und dessen Kanten „eine Seite anhängen" bedeuten.

= Mengen, Parameter, Notation

Parameter wie in der MIP-Formulierung:

#table(
  columns: (auto, auto, 1fr),
  align: (center, center, left),
  stroke: 0.5pt,

  [*Symbol*], [*Typ*], [*Bedeutung*],

  [$n$], [$bb(N)$], [Gesamtanzahl Bilder],
  [$k$], [$bb(N)$], [Anzahl Gruppen],
  [$|G_l|$], [$bb(N)$], [Anzahl Bilder in Gruppe $l in {1, ..., k}$],
  [$s$], [$bb(N)$], [Ziel-Seitenanzahl],
  [$b_"min", b_"max"$], [$bb(N)$], [Min./Max. erlaubte Seitenanzahl],
  [$p_"min", p_"max"$], [$bb(N)$], [Min./Max. Bilder pro Seite],
  [$g_"min"$], [$bb(N)$], [Min. Bilder einer Gruppe auf einer Seite bei Spaltung],
  [$g_"max"$], [$bb(N)$], [Max. verschiedene Gruppen pro Seite],
  [$overline(n)$], [$bb(R)^+$], [Ziel-Bildanzahl pro Seite, $overline(n) = n \/ s$],
  [$w_1, w_2, w_3$], [$bb(R)^+$], [Gewichte der Zielfunktion],
)

Die Bilder $0, ..., n-1$ sind fix geordnet (chronologisch, Gruppen lexikalisch
sortiert und zusammenhängend). Zusätzliche Notation:

#table(
  columns: (auto, 1fr),
  align: (center, left),
  stroke: 0.5pt,

  [$gamma(i)$], [Gruppenindex von Bild $i$ (0-basiert); monoton steigend in $i$],
  [$"start"_l, "end"_l$], [Bildindex-Bereich von Gruppe $l$: $G_l = ["start"_l, "end"_l)$],
)

= Reduktion auf Schnittpunkte <sec:zerlegung>

== Lösungsraum

Da Seiten zusammenhängende Abschnitte der fixen Bildfolge sind, ist jede Lösung
vollständig durch einen *Cut-Vektor* beschrieben:

$ 0 = c_0 < c_1 < ... < c_m = n, quad b_"min" <= m <= b_"max" $

Seite $j in {0, ..., m-1}$ (0-basiert) enthält die Bilder $[c_j, c_(j+1))$.

*Äquivalenz zum MIP:* Die Abbildung ist eine Bijektion zwischen zulässigen
MIP-Lösungen und zulässigen Cut-Vektoren: Aus Cuts folgen die kumulierten
Gruppenvariablen $g_(l,j) = max(0, min(|G_l|, c_j - "start"_l))$; umgekehrt
$c_j = sum_l g_(l,j)$. Monotonie, sequentielle Ordnung und Randbedingungen des MIP
erzwingen genau diese Struktur; alle übrigen Nebenbedingungen und Zielterme
übersetzen sich 1:1 (siehe unten). Das DP-Optimum ist daher *identisch* mit dem
MIP-Optimum — keine Heuristik, keine Approximation.

== Zulässigkeit einer Seite

Eine Seite $[a, b)$ ist zulässig, $F(a,b) = 1$, genau dann wenn:

+ *Seitengröße:* $p_"min" <= b - a <= p_"max"$
+ *Max. Gruppen pro Seite:* Da Gruppen zusammenhängend sind, ist die Anzahl
  berührter Gruppen $gamma(b-1) - gamma(a) + 1 <= g_"max"$
+ *Spaltungsregel:* Für jede berührte Gruppe $l$ mit Anteil
  $t_l = min(b, "end"_l) - max(a, "start"_l)$ gilt: Ist die Gruppe nicht
  vollständig auf der Seite ($t_l < |G_l|$), dann muss $|G_l| >= g_"min"$
  (Gruppe spaltbar) *und* $t_l >= g_"min"$ (Fragment groß genug) gelten.

Nur die erste ($gamma(a)$) und letzte ($gamma(b-1)$) berührte Gruppe können
unvollständig sein — mittlere Gruppen liegen stets ganz im Intervall. Alle drei
Prüfungen sind daher in $cal(O)(1)$ möglich.

== Kostenzerlegung

Die MIP-Zielfunktion $Z = w_1 D_"even" + w_2 D_"split" + w_3 D_"pages"$ zerfällt:

*Seitenkosten* (entspricht $D_"even"$, da $overline(n)$ konstant ist):

$ c_"page" (a, b) = w_1 dot |(b - a) - overline(n)| $

*Cut-Kosten* (entspricht $D_"split"$): Eine Gruppe, die $p$ Seiten berührt, hat genau
$p - 1$ innere Cuts strikt in ihrem Inneren; jeder innere Cut liegt strikt im Inneren
höchstens einer Gruppe. Also ist $D_"split"$ die Anzahl innerer Cuts, die *nicht* auf
einer Gruppengrenze liegen:

$ kappa(c) = cases(
  w_2 quad &"falls" 0 < c < n "und" gamma(c-1) = gamma(c),
  0 quad &"sonst"
) $

*Seitenzahlkosten* (entspricht $D_"pages"$): $w_3 dot |m - s|$, hängt nur von der
Seitenanzahl $m$ ab.

Damit:

$ Z(c_0, ..., c_m) = sum_(j=0)^(m-1) c_"page" (c_j, c_(j+1)) + sum_(j=1)^(m-1) kappa(c_j) + w_3 dot |m - s| $

= Bellman-Rekursion

== Zustand und Rekursion

$D(i, m)$ = minimale Kosten (Seiten- plus Cut-Kosten), die Bilder $[0, i)$ auf genau
$m$ zulässige Seiten zu verteilen.

*Basis:*

$ D(0, 0) = 0, quad D(i, 0) = infinity quad forall i > 0, quad D(0, m) = infinity quad forall m > 0 $

*Rekursion* über die Größe $p$ der letzten Seite (der Cut an deren Anfang wird beim
Anhängen der Seite bezahlt; $kappa(0) = 0$):

$ D(i, m) = min_(p in P(i)) {D(i - p, m - 1) + kappa(i - p) + c_"page" (i - p, i)} $

mit $P(i) = {p in [p_"min", min(p_"max", i)] : F(i - p, i) = 1}$;
leere Menge $arrow.r.double$ $infinity$.

*Lösung:*

$ Z^* = min_(b_"min" <= m <= b_"max") {D(n, m) + w_3 dot |m - s|} $

Ist $Z^* = infinity$, ist die Instanz unzulässig (im MIP: infeasible). Der optimale
Cut-Vektor wird per *Backtracking* rekonstruiert: Zu jedem Zustand wird die
argmin-Seitengröße gespeichert und von $(n, m^*)$ rückwärts abgelaufen.

== Korrektheit

Optimale Substruktur: Sei $(c_0, ..., c_m)$ optimal für $(i, m) = (c_m, m)$ mit
letzter Seite $[c_(m-1), c_m)$. Wäre $(c_0, ..., c_(m-1))$ nicht optimal für
$(c_(m-1), m-1)$, könnte das Präfix durch ein billigeres ersetzt werden, ohne
Zulässigkeit oder Kosten der letzten Seite zu ändern (keine Kopplung über die
Seitengrenze) — Widerspruch. Die Rekursion enumeriert alle zulässigen letzten
Seiten vollständig, also gilt Gleichheit.

== Komplexität

#table(
  columns: (auto, 1fr),
  align: (left, left),
  stroke: 0.5pt,

  [*Zeit*], [$cal(O)(n dot b_"max" dot (p_"max" - p_"min" + 1))$ — Zustände mal Übergänge, je $cal(O)(1)$],
  [*Speicher*], [$cal(O)(n dot b_"max")$ für Werte- und Backtracking-Tabelle],
)

Beispiel $n = 1000$, $b_"max" = 100$, $p_"max" - p_"min" + 1 = 20$: ca. $2 dot 10^6$
Übergänge — Laufzeit im Millisekundenbereich, unabhängig von der Gruppenanzahl $k$.

Optionales Pruning (konstanter Faktor): Zustand $(i, m)$ ist nur erreichbar für
$m dot p_"min" <= i <= m dot p_"max"$.

= Eigenschaften gegenüber dem MIP

#table(
  columns: (auto, 1fr, 1fr),
  align: (left, left, left),
  stroke: 0.5pt,

  [], [*MIP (HiGHS)*], [*DP*],

  [Optimalität], [bis `mip_rel_gap`, Timeout-abhängig], [exakt, beweisbar],
  [Laufzeit $n = 1000$], [Minuten bis Stunden], [Millisekunden],
  [Determinismus], [nein (Threads, Timeout)], [ja],
  [Instanz-Splitting], [nötig (`max_photos_for_split`)], [entfällt],
  [Warm-Start-Hint], [nötig], [entfällt],
)

*Grenzen:* Die DP setzt voraus, dass Zulässigkeit und Kosten pro Seite lokal sind.
Künftige Nebenbedingungen, die Seiten *koppeln* (z. B. „Gruppe X und Y nie auf
benachbarten Seiten"), erfordern eine Zustandserweiterung oder einen anderen Ansatz.
