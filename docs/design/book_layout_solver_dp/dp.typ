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

= Dynamische Programmierung in Kürze <sec:dp>

Dynamische Programmierung (DP) zerlegt ein mehrstufiges Optimierungsproblem in eine
Folge einfacherer Teilprobleme und löst es per *Bellman-Gleichung* (vgl.
#link("https://en.wikipedia.org/wiki/Bellman_equation")[Wikipedia: _Bellman equation_]).
Die zentralen Bezeichner:

#table(
  columns: (auto, 1fr),
  align: (left, left),
  stroke: 0.5pt,

  [*Bezeichner*], [*Bedeutung*],

  [Zustand $x$], [Alle Informationen über die aktuelle Situation, die für die restliche Entscheidung nötig sind],
  [Aktion $a$], [Die im Zustand gewählte Entscheidung (Kontrollvariable)],
  [Aktionsmenge $Gamma(x)$], [Menge der im Zustand $x$ zulässigen Aktionen, $a in Gamma(x)$],
  [Übergang $T(x, a)$], [Folgezustand nach Aktion $a$: $x' = T(x, a)$],
  [Ertrag $F(x, a)$], [Sofortiger Beitrag der Aktion $a$ im Zustand $x$ (hier: Kosten)],
  [Diskontfaktor $beta$], [Gewichtung künftiger gegenüber gegenwärtigen Erträgen, $0 < beta <= 1$],
  [Wertfunktion $V(x)$], [Bester erreichbarer Zielwert ab Zustand $x$],
  [Policy-Funktion $a(x)$], [Regel, die jedem Zustand die optimale Aktion zuordnet],
)

Zwei Voraussetzungen machen den Ansatz anwendbar:

+ *Optimale Substruktur* (Bellmans Optimalitätsprinzip): Eine optimale Strategie hat
  die Eigenschaft, dass — unabhängig von Anfangszustand und erster Entscheidung — die
  verbleibenden Entscheidungen wieder eine optimale Strategie für den resultierenden
  Folgezustand bilden.
+ *Überlappende Teilprobleme:* Derselbe Zustand $x$ tritt in vielen Gesamtlösungen
  auf, wird aber nur *einmal* gelöst und tabelliert (Memoisierung).

Bellmans Prinzip führt auf die rekursive Definition der Wertfunktion, die
*Bellman-Gleichung*. In der hier benötigten Minimierungsform (zeitunabhängig, da der
Folgezustand direkt eingesetzt wird):

$ V(x) = min_(a in Gamma(x)) {F(x, a) + beta dot V(T(x, a))} $

Sie wird per *Induktion* gelöst: Die Wertfunktion an den Terminalzuständen ist direkt
bekannt; von dort propagieren die Werte über die Bellman-Gleichung bis zur Wurzel
$V(x_0)$. Das jeweilige Argmin liefert die Policy-Funktion $a(x)$. Äquivalent ist dies
ein kürzester Pfad in einem azyklischen Graphen, dessen Knoten die Zustände und dessen
Kanten die Aktionen sind.

= Mengen, Parameter, Notation

Parameter und Mengen:

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

= DP-Modell

== Zustand und Bellman-Notation

Wir bilden das Problem direkt auf die Bellman-Notation aus @sec:dp ab. Der Zustand ist
das Tupel $x_t = (i, m)$ mit den Komponenten

$ I(x_t) = i quad ("platzierte Bilder"), quad P(x_t) = m quad ("belegte Seiten") $

Wie auf der Wikipediaseite lassen wir den Zeitindex weg und schreiben kurz $x$. Eine
*Periode* entspricht dem Anhängen einer Seite; die Seiten werden von der Wurzel
$x_0 = (0, 0)$ aus *vorwärts* angehängt, bis alle Bilder platziert sind. Die
Bestandteile des Modells:

#table(
  columns: (auto, auto, 1fr),
  align: (center, center, left),
  stroke: 0.5pt,

  [*Bellman*], [*Hier*], [*Bedeutung*],

  [Zustand $x$], [$(i, m)$], [Erste $i = I(x)$ Bilder auf $m = P(x)$ Seiten verteilt],
  [Aktion $a$], [$p$], [Bildanzahl der nächsten (am Ende) angehängten Seite],
  [Aktionsmenge $Gamma(x)$], [@sec:aktionen], [Zulässige nächste Seitengrößen],
  [Übergang $T(x, a)$], [$(I(x) + a, P(x) + 1)$], [Nächste Seite angehängt],
  [Ertrag $F(x, a)$], [@sec:aktionen], [Kosten der angehängten Seite (es wird minimiert)],
  [Diskontfaktor $beta$], [$1$], [Endlicher Horizont, keine Abzinsung],
  [Wertfunktion $V(x)$], [], [Minimale Restkosten, um die Bilder $I(x), ..., n-1$ zu platzieren],
)

Die Wurzel ist $x_0 = (0, 0)$ (kein Bild platziert, keine Seite belegt); Terminalzustände
sind $(n, m)$, sobald alle $n$ Bilder verteilt sind.

== Lösungsraum: Schnittpunkte

Da Seiten zusammenhängende Abschnitte der fixen Bildfolge sind, ist jede Lösung
vollständig durch einen *Cut-Vektor* beschrieben:

$ 0 = c_0 < c_1 < ... < c_m = n, quad b_"min" <= m <= b_"max" $

Seite $j in {0, ..., m-1}$ (0-basiert) enthält die Bilder $[c_j, c_(j+1))$. Eine Aktion
$a = p$ im Zustand $x$ entspricht dem Cut $c = I(x)$ und hängt die Seite
$[I(x), I(x) + p)$ an. _Wie_ die ersten $I(x)$ Bilder intern geschnitten sind, ist für
die folgenden Seiten irrelevant — keine Nebenbedingung und kein Zielterm koppelt über
eine Seitengrenze hinweg. Genau deshalb ist $x = (i, m)$ ein hinreichender Zustand.

== Zulässigkeit einer Seite

Eine Seite über dem Bildintervall $[u, v)$ ist zulässig, $phi(u, v) = 1$, genau dann
wenn:

+ *Seitengröße:* $p_"min" <= v - u <= p_"max"$
+ *Max. Gruppen pro Seite:* Da Gruppen zusammenhängend sind, ist die Anzahl berührter
  Gruppen $gamma(v-1) - gamma(u) + 1 <= g_"max"$
+ *Spaltungsregel:* Für jede berührte Gruppe $l$ mit Anteil
  $t_l = min(v, "end"_l) - max(u, "start"_l)$ gilt: Ist die Gruppe nicht vollständig
  auf der Seite ($t_l < |G_l|$), dann muss $|G_l| >= g_"min"$ (Gruppe spaltbar) *und*
  $t_l >= g_"min"$ (Fragment groß genug) gelten.

Nur die erste ($gamma(u)$) und letzte ($gamma(v-1)$) berührte Gruppe können
unvollständig sein — mittlere Gruppen liegen stets ganz im Intervall. Alle drei
Prüfungen sind daher in $cal(O)(1)$ möglich.

== Kostenzerlegung

Die Zielfunktion setzt sich aus drei lokalen Termen zusammen:

*Seitenkosten* — Abweichung von der Ziel-Bildanzahl pro Seite:

$ c_"page" (u, v) = w_1 dot |(v - u) - overline(n)| $

*Cut-Kosten* — Strafe für das Spalten einer Gruppe. Jeder innere Cut liegt strikt im
Inneren höchstens einer Gruppe; tut er das, zerschneidet er sie. Die Cut-Kosten zählen
daher die inneren Cuts, die *nicht* auf einer Gruppengrenze liegen:

$ kappa(c) = cases(
  w_2 quad &"falls" 0 < c < n "und" gamma(c-1) = gamma(c),
  0 quad &"sonst"
) $

*Seitenzahlkosten* — Abweichung von der Ziel-Seitenanzahl: $w_3 dot |m - s|$; hängt
nur von $m = P(x)$ ab und wird als Terminalkosten behandelt.

Für einen vollständigen Cut-Vektor ergibt sich die Gesamtzielfunktion:

$ Z(c_0, ..., c_m) = sum_(j=0)^(m-1) c_"page" (c_j, c_(j+1)) + sum_(j=1)^(m-1) kappa(c_j) + w_3 dot |m - s| $

== Aktionsmenge und Ertrag <sec:aktionen>

Die im Zustand $x$ zulässigen Aktionen — die möglichen Größen der nächsten Seite:

$ Gamma(x) = {p in bb(N) : p_"min" <= p <= min(p_"max", n - I(x)) " und " phi(I(x), I(x) + p) = 1} $

Der Kandidatenbereich ist nach oben durch $p_"max"$ *und* durch die nur $n - I(x)$ noch
zu platzierenden Bilder beschränkt (die nächste Seite $[I(x), I(x) + p)$ erfordert
$I(x) + p <= n$) — daher das Minimum. Der Filter $phi$ wählt daraus die zulässigen
Größen. Da die Spaltungsregel nicht monoton in $p$ ist, darf $Gamma(x)$ *Lücken
enthalten* und ist im Allgemeinen nicht zusammenhängend.

Der Ertrag einer Aktion ist die Summe aus Seiten- und Cut-Kosten der angehängten Seite
(der Cut an ihrem Anfang wird beim Anhängen bezahlt, $kappa(0) = 0$):

$ F(x, a) = c_"page" (I(x), I(x) + a) + kappa(I(x)) $

== Bellman-Gleichung

*Terminalkosten:* Sind alle Bilder platziert ($I(x) = n$), entstehen keine weiteren
Seiten; es bleibt die Abweichung von der Ziel-Seitenanzahl. Für unzulässige Endzustände
(falsche Seitenzahl) bzw. Sackgassen ist die Wertfunktion $infinity$:

$ V((n, m)) = cases(
  w_3 dot |m - s| quad &"falls" b_"min" <= m <= b_"max",
  infinity quad &"sonst"
) $

*Bellman-Gleichung* (Minimierungsform, $beta = 1$) für nicht-terminale Zustände
($I(x) < n$):

$ V(x) &= min_(a in Gamma(x)) {F(x, a) + V(T(x, a))} \
      &= min_(a in Gamma(x)) {c_"page" (I(x), I(x) + a) + kappa(I(x)) + V((I(x) + a, P(x) + 1))} $

Leere Aktionsmenge $Gamma(x) = nothing arrow.r.double V(x) = infinity$ (Sackgasse).

*Optimaler Zielwert:* der Wert an der Wurzel,

$ Z^* = V((0, 0)) $

Ist $Z^* = infinity$, ist die Instanz unzulässig. Die zugehörige *Policy-Funktion*
$a(x)$ — welche nächste Seitengröße in jedem Zustand optimal ist — liefert per
*Backtracking* den optimalen Cut-Vektor: zu jedem Zustand wird das Argmin gespeichert
und von $(0, 0)$ vorwärts abgelaufen.

== Korrektheit <sec:korrektheit>

Bellmans *Optimalitätsprinzip*: Sei eine optimale Aufteilung für den Zustand
$x = (i, m)$ gegeben, deren erste Entscheidung die nächste Seite $[i, i + p)$ wählt. Der
verbleibende Plan muss optimal für den Folgezustand $T(x, p) = (i + p, m + 1)$ sein —
andernfalls ließe er sich durch einen billigeren ersetzen, ohne Zulässigkeit oder
Ertrag $F(x, p)$ der angehängten Seite zu ändern (keine Kopplung über die Seitengrenze)
— Widerspruch. Da die Bellman-Gleichung über $Gamma(x)$ alle zulässigen ersten
Entscheidungen vollständig enumeriert, gilt Gleichheit.

== Komplexität <sec:komplexitaet>

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

= Eigenschaften und Grenzen

Die DP liefert das *exakte* Optimum (beweisbar, @sec:korrektheit) und ist
*deterministisch* — dieselbe Eingabe ergibt stets dieselbe Lösung. Für $n = 1000$,
$b_"max" = 100$ liegt die Laufzeit im Millisekundenbereich (@sec:komplexitaet).

*Grenzen:* Die DP setzt voraus, dass Zulässigkeit und Kosten pro Seite lokal sind.
Künftige Nebenbedingungen, die Seiten *koppeln* (z. B. "Gruppe X und Y nie auf
benachbarten Seiten"), erfordern eine Zustandserweiterung oder einen anderen Ansatz.
