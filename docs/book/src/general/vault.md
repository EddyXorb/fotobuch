# Vaults and Multiple Projects

A **fotobuch vault** is a folder that is also a Git repository containing at least one git branch that starts with "fotobuch/". It holds any number of fotobuch projects — each on its own branch named `fotobuch/<project-name>`.
When using the CLI, fotobuch looks for the vault in the current working directory only.
When using the GUI, fotobuch looks for the vault using a priority chain (see below) and opens it automatically on startup.

## Branch layout

All projects share the same repository root. At any time the working tree shows the files of the currently checked-out project:

```
vault/
├── .git/
├── Italy-2024.yaml        ← current project (checked out)
├── Italy-2024.typ
└── .fotobuch/             ← ignored by git, shared cache for all projects 
    └── cache/
        ├── Italy-2024/
        └── Hiking-2025/   ← cache for project Hiking-2025, not visible in the working tree
```

## GUI: Default vault and how fotobuch finds it

When you open fotobuch in the GUI it resolves the vault using this priority chain:

| Priority | Source                         | Description                                                  |
| -------- | ------------------------------ | ------------------------------------------------------------ |
| 1        | `--vault <path>` CLI flag      | Explicit path, overrides everything                          |
| 2        | `FOTOBUCH_VAULT` env variable  | Set in shell or CI environment                               |
| 3        | Current working directory      | Used if the directory already contains `fotobuch/*` branches |
| 4        | Last-used vault                | Remembered from the previous session                         |
| 5        | Default: `~/Pictures/Fotobuch` | Created automatically on first use                           |

The default vault path is `~/Pictures/Fotobuch` (or the platform equivalent of the Pictures directory).
