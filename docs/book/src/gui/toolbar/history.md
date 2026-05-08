# Undo, Redo and History

fotobuch keeps a full history of every change you make, so you can always go back.

## Undo and Redo

| Button | Shortcut | Action |
|--------|----------|--------|
| ↩ | `Ctrl+Z` | Undo the last change |
| ↪ | `Ctrl+Y` or `Ctrl+Shift+Z` | Redo (bring back an undone change) |

You can undo and redo as many steps as you like.

## History panel

Click **⏱ History** in the toolbar to open the history panel on the right side. It lists every saved change in order, newest first, so you can see exactly what was done and when.

The history is stored as a series of snapshots. Each change you make — placing photos, rebuilding a page, changing settings — is a separate entry in the list.

## Technical: git as the underlying mechanism

fotobuch stores each project as a git repository. Every command that modifies the project creates a git commit. This means:

- The full history is a plain git log — you can inspect it with any git client.
- The CLI and the GUI share the same history. A change made from the command line appears in the GUI history panel, and vice versa.
- Undo and redo are implemented as git revert and re-apply operations, so history is never lost.
