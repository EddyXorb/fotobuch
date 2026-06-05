# fotobuch

**fotobuch** turns one (or more) folder of photos into a print-ready PDF photo book, in a highly automated way.

Point it at your photos and fotobuch figures out the layout automatically:

- aspect ratios stay intact
- reading order flows naturally from top-left to bottom-right
- photos from the same event stay together on the same page
- more important photos get more space (you decide)
- optionally add titles and an appendix with metadata for each photo, automatically generated from your images' EXIF data

Start fully automatic, then fine-tune individual pages by hand as needed.

## How fotobuch works

fotobuch has a very simple data model. It stores each photo book as a pair of two files: a YAML file (.yaml) and a Typst
template (.typ) tracked by Git inside a *vault* folder.
The photo book can be read and modified through:

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
