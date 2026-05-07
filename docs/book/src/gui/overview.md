# GUI Overview

The fotobuch GUI is an immediate-mode desktop application built with egui/eframe.

## Layout

```
┌──────────────────────────────────────────────────────────┐
│  Toolbar                                                  │
├───────────────┬──────────────────────────┬───────────────┤
│               │                          │               │
│  Pool panel   │    Central panel         │  Nav strip    │
│  (left)       │    (canvas)              │  (right)      │
│               │                          │               │
└───────────────┴──────────────────────────┴───────────────┘
│  Status bar                                               │
└──────────────────────────────────────────────────────────┘
```

## Workflow

1. **Add photos** — `Ctrl+O` or the **Add** button opens a folder picker. Selected photos land in the Pool panel.
2. **Place photos** — drag from the Pool onto a page, or select pool photos and press **P** to auto-distribute.
3. **Rebuild layout** — select pages and press **R**, or use the **Rebuild** button. The planner re-fits photos to slots.
4. **Adjust manually** — switch a page to Manual mode (click the mode pill or press **A**) to drag and resize slots with the right mouse button.
5. **Export** — press **Ctrl+Shift+B** or click **Release** to generate the final PDF.

## Projects and Vaults

A *vault* is a git-managed folder that holds one or more *projects* (branches named `fotobuch/<name>`). Use the project dropdown in the toolbar to switch or create projects, or switch to a different vault entirely.
