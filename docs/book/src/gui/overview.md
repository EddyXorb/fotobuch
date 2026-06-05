# GUI Overview

fotobuch is a photo book layout tool. You bring in your photos, the program arranges them across pages automatically, and you export a print-ready PDF — all without any manual design work. The GUI is the main way to work with fotobuch interactively.

## Window layout

The window has five areas:

| Area              | Position | Purpose                                         |
| ----------------- | -------- | ----------------------------------------------- |
| **Toolbar**       | Top      | Quick access to all main actions                |
| **Photo Pool**    | Left     | Your imported photos                            |
| **Central Panel** | Centre   | The page canvas — what your book will look like |
| **Nav Bar**       | Right    | Miniature page overview for jumping around      |
| **Status bar**    | Bottom   | Current state and progress messages             |

## How to create a photo book

fotobuch works in four steps:

**1. Start a project** — When you first open the app, choose *New project* from the welcome screen and give it a name. Projects are saved automatically as you work.

**2. Add your photos** — Click **Add** (or press `Ctrl+O`) and pick a folder. fotobuch imports all photos from that folder into the Photo Pool on the left. You can add more folders at any time.

**3. Place photos on pages** — Drag photos from the Photo Pool onto a page in the canvas, or click **Rebuild** in the toolbar (then confirm) to let fotobuch create all pages and place all photos automatically.

**4. Fine-tune and export** — Browse the pages in the Central Panel and the Nav Bar. If a page does not look right, select it and press **R** to rebuild just that page. When you are happy, click **Release** (or press `Ctrl+Shift+B`) to build the final PDF. The PDF is saved in your project folder.

## Multiple projects

You can keep several photo books — for example one per year or one per event. Use the project dropdown at the top-left of the toolbar to switch between projects or create new ones. All projects in the same folder are kept together, so opening that folder next time shows all of them.

## CLI and GUI interoperability

The GUI and the `fotobuch` command-line tool operate on exactly the same project data. You can open a project in the GUI that was created and populated entirely from the CLI, or use the CLI and GUI on the same project in sequence — for example, run CLI commands for scripting or batch operations, then open the GUI for visual review and fine-tuning.

The shared format is a YAML file and a Typst template tracked by git inside the vault, so every tool that understands git can inspect the full change history.
