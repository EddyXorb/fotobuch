# Installation

## Pre-built binaries (recommended)

Download the latest binary for your platform from the
[Releases page](https://github.com/EddyXorb/fotobuch/releases/latest):

| Platform                | File                            |
| ----------------------- | ------------------------------- |
| Linux x86_64            | `fotobuch-linux-x86_64.tar.gz`  |
| Windows x86_64          | `fotobuch-windows-x86_64.zip`   |
| macOS arm64 (M1 and up) | `fotobuch-macos-arm64.tar.gz`   |

Each archive contains both binaries: `fotobuch` (CLI) and `fotobuch-gui` (GUI).
Extract the archive and place them somewhere on your `PATH`.

**Verify the install:**

```
fotobuch --version
```

## macOS: "Apple could not verify that fotobuch is free of malware"

fotobuch is not signed with an Apple Developer ID and therefore not notarized by
Apple. Anything downloaded through a browser gets the `com.apple.quarantine`
attribute, and Gatekeeper blocks quarantined binaries that carry no notarization
ticket — the dialog only offers *Move to Trash* and *Done*. The binary is fine;
macOS simply cannot ask Apple whether it has been checked.

Pick whichever way you prefer:

**1. Install from the terminal (recommended).** `curl` does not set the
quarantine attribute, so nothing is ever blocked:

```bash
curl -fsSL https://github.com/EddyXorb/fotobuch/releases/latest/download/fotobuch-macos-arm64.tar.gz | tar -xz
mkdir -p ~/.local/bin && mv fotobuch fotobuch-gui ~/.local/bin/
~/.local/bin/fotobuch --version
```

Add `~/.local/bin` to your `PATH` if it is not there yet:

```bash
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc
```

**2. Remove the quarantine flag from an existing download:**

```bash
xattr -dr com.apple.quarantine ~/Downloads/fotobuch-macos-arm64
```

**3. Without the terminal:** double-click the binary, dismiss the warning, then
open **System Settings → Privacy & Security**, scroll to the security section
and click **Open Anyway** next to the message about `fotobuch`. Confirm once per
binary — `fotobuch` and `fotobuch-gui` are separate executables.

> Apple Silicon only: the release contains an `arm64` build. Intel Macs need a
> [build from source](#build-from-source).

## Build from source

Requirements: [Rust (stable)](https://rustup.rs).

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
