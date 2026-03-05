# Claude

- moderne Codingstandards sind Pflicht
- jeder Teilschritt ist ein conventional commit
- unittests für jedes neue feature sind pflicht
- regelmäßig cargo build ausführen und alle warnings beheben

## Rust specific

- do not use mod.rs files for subfolders, instead use the same name for the module in root and include every module in the same named subfolder in the root-file
