# Installation

## Pre-built binaries (recommended)

Download the latest binary for your platform from the
[Releases page](https://github.com/EddyXorb/fotobuch/releases/latest):

| Platform       | File                           |
| -------------- | ------------------------------ |
| Linux x86_64   | `fotobuch-linux-x86_64.tar.gz` |
| Windows x86_64 | `fotobuch-windows-x86_64.zip`  |

Extract the archive and place the `fotobuch` binary somewhere on your `PATH`.

**Verify the install:**

```
fotobuch --version
```

## Build from source

Requirements: [Rust (stable)](https://rustup.rs) and `cmake` (needed to build
the [HiGHS](https://highs.dev/) optimizer library).

```bash
git clone https://github.com/EddyXorb/fotobuch.git
cd fotobuch
cargo build --release
# binary: ./target/release/fotobuch
```

## Recommended editor setup

fotobuch writes a [Typst](https://typst.app/) source file alongside the PDF.
For a live preview while you work, install
[VS Code](https://code.visualstudio.com/) with the
[Typst Preview](https://marketplace.visualstudio.com/items?itemName=mgt19937.typst-preview)
extension. Open the `.typ` file and the preview updates every time you run
`fotobuch build`.

Alternatively, just keep any PDF viewer open and reload after each build.

## Shell completions

```bash
fotobuch completions --shell bash   >> ~/.bash_completion
fotobuch completions --shell zsh    >> ~/.zshrc
fotobuch completions --shell fish   > ~/.config/fish/completions/fotobuch.fish
fotobuch completions --shell powershell >> $PROFILE
```
