# fotobuch CLI — Leitprinzipien

- **Nutzerperspektive zuerst.** Die CLI-Struktur ergibt sich aus den Aktionen des Benutzers, nicht aus der internen Architektur.
- **Wenige, selbsterklärende Kommandos.** Vorbilder: `cargo`, `uv`, modernes `git`.
- **Schnelles Feedback.** Jede Aktion gibt sofort Rückmeldung. Langsame Operationen (Layout-Berechnung, Bildexport) sind explizite eigene Kommandos.
- **Textbasiert und editierbar.** Der Projektzustand liegt in YAML, das Layout in Typst. Beides menschenlesbar, versionierbar, manuell anpassbar.
- **Ein Projekt pro Branch** (git-Modell). Kein `--project`-Flag nötig — der aktuelle Branch bestimmt den Kontext.

Die maßgeblichen Kommando-Beschreibungen liegen in den Einzeldokumenten (`1_new.md` bis `undo.md`).
