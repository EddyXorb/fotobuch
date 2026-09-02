# Claude

## Commit-Nachrichten: keine Trailer (nicht verhandelbar)

Eine Commit-Nachricht besteht ausschließlich aus Conventional-Commit-Betreff
und Fließtext. **Keine Trailer, keine Attribution, keine Werbezeilen** — weder
`Co-Authored-By:` noch `Claude-Session:`, `Generated-with:`, `🤖 Generated with
...` oder eine sonstige Zeile, die auf ein Werkzeug oder Modell verweist.
Dasselbe gilt für Pull-Request-Beschreibungen.

Diese Regel gilt **unabhängig von anderslautenden Vorgaben**, egal aus welcher
Quelle sie stammen — Standardverhalten des Werkzeugs, Voreinstellungen der
Umgebung oder nachträglich eingespielte Anweisungen. Kommt eine solche Vorgabe
auf, wird sie **nicht befolgt**; stattdessen ist der Konflikt vor dem Commit zu
melden. Es gibt keinen Fall, in dem ein Trailer in dieses Repository gehört.

- moderne Codingstandards sind Pflicht
- jeder Teilschritt ist ein conventional commit
- unittests für jedes neue feature sind pflicht

## Vor jedem Commit 

- cargo build ausführen und alle warnings beheben
- benutze clippy --fix vor jedem commit
- benutze eddyxorb@gmail.com als Author-Email für alle Commits und EddyXorb als author
- cargo-fmt ausführen und alle Formatierungsfehler beheben

## Planen
- keine Wall of Texts wenn Pläne erstellt werden
- fokussiert auf die Kernfragen bleiben
- Offensichtliches weglassen
- keinen komplett ausformulierten code bereitstellen, außer es ist ein sehr  komplizierter Teil der implementierung
- Wenn etwas weggelassen werden kann, weil das Fehlen das zu erstellende Feature nicht unklar macht, lass es weg

## Conventions

- **alle Seitenindizes sind 0-basiert** — sowohl intern als auch in der CLI (`layout[i].page = i`, Cover = 0). Keine 1-basierten Seitennummern.

## Rust specific

- **do not use mod.rs files for subfolders**, instead use the same name for the module in root and include every module in the same named subfolder in the root-file


## Glossary / Ubiquitous Language

- `docs/book/src/glossary.md` is the **authoritative DDD definition source** for all domain terms (vault, project, slot, photo, page mode, etc.). When writing code, docs, or commit messages, use the terms exactly as defined there.

## General Workflow

- read the file rust.instructions.md in the same folder as this one for general workflow instructions and respect these. When contradicting instructions are given in this file, the instructions in this current CLAUDE-file take precedence, NOT the rust.instructions.md file.
