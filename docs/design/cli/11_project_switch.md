# `fotobuch project switch`

## CLI-Interface

```text
$ fotobuch project switch --help
Switch to another photobook project

Usage: fotobuch project switch <NAME>

Arguments:
  <NAME>  Project name to switch to

Options:
  -h, --help  Print help
```

## Verhalten

Wechselt zum Branch `fotobuch/<name>`. Der Working Tree zeigt danach `<name>.yaml` und `<name>.typ`. Uncommitted Changes im Working Tree verhindern den Wechsel.

## Fehlerbehandlung

| Situation                                | Verhalten                                                                                    |
| ---------------------------------------- | -------------------------------------------------------------------------------------------- |
| Branch `fotobuch/<name>` existiert nicht | Fehler: `Project '<name>' not found. Use 'fotobuch project list' to see available projects.` |
| Uncommitted Changes                      | Fehler: `Working tree has uncommitted changes. Commit or stash before switching.`            |
| Bereits auf dem Ziel-Branch              | Hinweis: `Already on project '<name>'` (kein Fehler)                                         |
| Ungültiger Projektname                   | Fehler aus Namensvalidierung                                                                 |
