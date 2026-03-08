# TODOs

In dieser Reihe abzuarbeiten.

1. [x] [state_manager](docs/design/state_and_data_persistence/statemanager.md)
2. [ ] Der Reihe nach wie nummeriert alle cli-commands in docs/design/cli:
   - [x] [project new](docs/design/cli/1_new.md)
   - [x] [add](docs/design/cli/2_add.md)
   - [ ] [build](docs/design/cli/3_build.md)
     - [x] cache/common.rs (Pfad-Helpers, mtime-Check, resize mit Lanczos3/Triangle)
     - [x] cache/preview.rs (parallele Preview-Generierung mit rayon)
     - [x] cache/final_cache.rs (300 DPI mit DPI-Validierung)
     - [x] output/typst.rs (Typst-Kompilierung via typst-crate)
     - [x] commands/build.rs (first_build, incremental_build, release_build)
     - [x] **BUGFIX**: SimpleWorld in output/typst.rs muss VirtualPath::within_root() verwenden
       - Problem: FileId wird mit VirtualPath::new() erstellt, sollte aber within_root() nutzen
       - Lösung: root vor FileId berechnen, dann `VirtualPath::within_root(path, &root)` verwenden
       - Symptom: Integration-Tests schlagen mit "file not found (searched at ...)" fehl
     - [x] Integration-Tests (tests/build_test.rs) zum Laufen bringen
       - 7 Tests erstellt, alle schlagen wegen SimpleWorld-Bug fehl
       - Tests prüfen: first_build, incremental_build, release, --pages filter, clean check
     - [ ] CLI-Integration in main.rs und cli.rs
   - [ ] [rebuild](docs/design/cli/4_rebuild.md)
   - [ ] [place](docs/design/cli/5_place.md)
   - [ ] [remove](docs/design/cli/6_remove.md)
   - [ ] [status](docs/design/cli/7_status.md)
   - [ ] [config](docs/design/cli/8_config.md)
   - [ ] [history](docs/design/cli/9_history.md)
   - [ ]  [project list](docs/design/cli/10_project_list.md)
   - [ ]  [project switch](docs/design/cli/11_project_switch.md)
