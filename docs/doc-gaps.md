# Dokumentationslücken – Vorschläge

Themen, die im Handbuch fehlen oder nur flüchtig erwähnt werden.

---

## Hoch priorisiert

### 1. Vault-Struktur und Branch-Modell
Das Buch erwähnt „Vault" und mehrere Projekte, erklärt aber nie, dass jedes Projekt auf einem eigenen `fotobuch/<name>`-Branch lebt und `project switch` intern ein `git checkout` ist. Ein kurzes Kapitel „Wie mehrere Projekte zusammen funktionieren" würde den Workflow klar machen.

### 2. Manual-Modus-Workflow (`page mode` + `page pos`)
`page mode` und `page pos` sind in der Kommandotabelle aufgeführt, aber es gibt keine praktische Anleitung: wann man auf Manual umschaltet, wie man `page pos` mit `--by`/`--at`/`--scale` nutzt und wie man wieder in Auto wechselt. Der GUI-Manual-Modus (gelber Griff) ist ebenfalls nur kurz erwähnt.

### 3. Solver-Troubleshooting-Guide
Das Konfigurations-Kapitel hat einen kurzen Tipp, aber kein dediziertes "Was tun, wenn das Layout schlecht ist?"-Kapitel. Typische Fragen: schlechte Seitenverteilung → `weight_split` erhöhen; Laufzeit zu lang → `search_timeout` reduzieren oder `enable_local_search: false`; zu viele/wenige Seiten → `page_target` / `page_max` anpassen.

---

## Mittel priorisiert

### 4. XMP-Filter-Anleitung
`--filter-xmp` wird im Quickstart kurz gezeigt, aber es gibt keine Erklärung, welche XMP-Felder sinnvoll sind (z. B. `Rating`, `Label`, `DateCreated`, Kamera-Modell) und wie man den Regex dazu schreibt. Ein paar praxisnahe Beispiele würden hier helfen.

### 5. `remove --unplaced` und `rebuild --range` / `--flex`
Diese Flags existieren in der CLI (und im generierten Referenz-Dokument), sind aber in `commands.md` und den Konzepten nicht erwähnt:
- `remove --unplaced` – entfernt alle nicht platzierten Fotos auf einmal
- `rebuild --range-start/--range-end` – rebuild für einen Seitenbereich
- `rebuild --flex N` – erlaubt dem Solver, die Seitenanzahl um ±N zu variieren

### 6. `fotobuch init`-Befehl (Alias für `project new`)
`init` ist ein Alias für `project new`, erscheint im generierten Referenz-Kapitel, aber nicht in `commands.md` oder den Konzepten.

---

## Niedrig priorisiert

### 7. Fotogewichte – praktischer Leitfaden
Gewichte werden in Konzepten, CLI-Quickstart und Konfiguration verstreut erwähnt. Ein zentrales kurzes „Wie nutze ich Gewichte sinnvoll?"-Kapitel (wann `--weight` beim `add`, wann nachträgliches `page weight`, Faustregeln) würde den Einstieg erleichtern.

### 8. Anhang (Appendix) – Einrichtungsbeispiel
Der Anhang ist in Konfiguration und Template beschrieben, aber ein konkretes Einrichtungsbeispiel mit Ausgabe (wie das finale PDF mit Index aussieht) fehlt.

### 9. `place --into-new-page-at`
Das Flag `place --into-new-page-at <POS>` erscheint im generierten Referenz-Kapitel, wird aber nirgendwo erklärt.

### 10. Headless-/CI-Nutzung
Kein Hinweis darauf, dass `fotobuch` ohne GUI auf einem Server genutzt werden kann (z. B. für automatisierte Builds). Ein kurzes „Headless-Workflow"-Kapitel würde Nutzer ansprechen, die automatisierte Pipelines bauen wollen.
