# fotobuch

**fotobuch** turns a folder of photos into a print-ready PDF photo book —
no cropping, no distortion, no manual drag-and-drop.

Point it at your photos and fotobuch figures out the layout automatically:
aspect ratios stay intact, reading order flows naturally from top-left to
bottom-right, and photos from the same event stay together on the same page.
Start fully automatic, then fine-tune individual pages by hand as needed.

## Documentation

| Where to go                                          | What you'll find                                     |
| ---------------------------------------------------- | ---------------------------------------------------- |
| [GUI Quickstart](gui/quickstart.md)                  | Open the app and create your first book              |
| [CLI Quickstart](cli/quickstart.md)                  | Zero to PDF from the command line                    |
| [Core Concepts](general/concepts.md)                 | Projects, photos, pages, weights                     |
| [Configuration](general/configuration.md)            | Full YAML reference                                  |
| [CLI Commands](cli/commands.md)                      | Every CLI command at a glance                        |
| [Full Flag Reference](cli/reference-generated.md)    | Every flag, auto-generated from source               |
| [Printing](general/printing.md)                      | Exporting for Saal Digital & friends                 |
| [Glossary](glossary.md)                              | Domain terms used throughout fotobuch                |

## How fotobuch works

fotobuch stores photo books as a simple **data model**: a YAML file, a Typst
template, and cached images, all tracked by Git inside a *vault* folder. The
data model can be read and modified through:

- **the GUI** — a visual, interactive interface for browsing and adjusting pages
- **the CLI** — scriptable commands for batch operations and automation

Both tools operate on exactly the same project data. You can mix and match:
import photos with the CLI, review and tweak in the GUI, then kick off the
release build from either side.

## Example project

The repository ships with a complete example in `docs/examples/` — a ready-made
project with sample images and both preview and release PDFs. A good way to
explore the YAML, the Typst template, and the output without creating your own
project first.

## Source code

[github.com/EddyXorb/fotobuch](https://github.com/EddyXorb/fotobuch)
