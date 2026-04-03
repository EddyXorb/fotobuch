# Claude

- moderne Codingstandards sind Pflicht
- jeder Teilschritt ist ein conventional commit
- unittests für jedes neue feature sind pflicht

## Vor jedem Commit 

- cargo build ausführen und alle warnings beheben
- **keine Co-Authored-By trailers in Commit-Nachrichten** — verwende nur normales Conventional Commit Format
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


## General Workflow

- read the file rust.instructions.md in the same folder as this one for general workflow instructions and respect these. When contradicting instructions are given in this file, the instructions in this current CLAUDE-file take precedence, NOT the rust.instructions.md file.
