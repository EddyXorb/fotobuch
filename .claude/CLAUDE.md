# Claude

- moderne Codingstandards sind Pflicht
- jeder Teilschritt ist ein conventional commit
- unittests für jedes neue feature sind pflicht
- regelmäßig cargo build ausführen und alle warnings beheben
- **keine Co-Authored-By trailers in Commit-Nachrichten** — verwende nur normales Conventional Commit Format
- benutze clippy --fix vor jedem commit
- benutze eddyxorb@gmail.com als Author-Email für alle Commits und EddyXorb als author

## Planen
- keine Wall of Texts wenn Pläne erstellt werden
- fokussiert auf die Kernfragen bleiben
- offensichtliches weglassen
- keinen komplett ausformulierten code bereitstellen, außer es ist ein sehr  komplizierter Teil der implementierung
- Wenn etwas weggelassen werden kann, weil das Fehlen das zu erstellende Feature nicht unklar macht, lass es weg

## Conventions

- **alle Seitenindizes sind 0-basiert** — sowohl intern als auch in der CLI (`layout[i].page = i`, Cover = 0). Keine 1-basierten Seitennummern.

## Rust specific

- **do not use mod.rs files for subfolders**, instead use the same name for the module in root and include every module in the same named subfolder in the root-file
