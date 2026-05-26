# Vaults and Multiple Projects

A **vault** is a folder that is also a Git repository. It holds any number of fotobuch projects — each on its own branch named `fotobuch/<project-name>`.

## Branch layout

All projects share the same repository root. At any time the working tree shows the files of the currently checked-out project:

```
vault/
├── .git/
├── Italy-2024.yaml        ← current project (checked out)
├── Italy-2024.typ
└── .fotobuch/
    └── cache/
        ├── Italy-2024/
        └── Hiking-2025/   ← cache for all projects, always present
```

## Creating a second project

Run `fotobuch project new` from **inside** an existing vault. fotobuch detects the existing repository and creates the new project in the repo root (no subdirectory):

```bash
cd Italy-2024/
fotobuch project new Hiking-2025 --width 210 --height 297
```

## Switching between projects

```bash
fotobuch project list           # show all projects
fotobuch project switch Hiking-2025
```

`project switch` is a `git checkout` under the hood. It is blocked if there are uncommitted changes — run `fotobuch build` (or `fotobuch undo`) to clean up first.

## First project vs. vault

When you run `fotobuch project new` from a directory that is **not** yet a git repository, fotobuch creates a new subdirectory with its own repository — the standard single-project setup. Come back to that directory later and run `project new` again to add a second project to the same vault.
