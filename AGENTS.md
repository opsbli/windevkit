# windevkit — Agent Guidelines

## Project Overview

windevkit is a Rust CLI tool for Windows that provides:
1. **Runtime Management** — One-command install, switch, and uninstall of Node.js, Java (JDK), and Maven via symlinks
2. **Application Export/Import** — Scan installed apps from winget/registry, export offline toolbox, restore on new machine
3. **Offline-First Design** — Toolbox directory on USB drive, no internet required for restore

## Architecture

```
src/
├── main.rs            # Entry point
├── lib.rs             # Module declarations
├── cli/               # clap command tree + subcommand handlers
│   ├── mod.rs         # CLI definition + dispatch
│   └── commands/      # One file per command group
├── config/            # ~/.windevkit/config.toml read/write
├── runtime/           # Node/Java/Maven download, extract, symlink, PATH
├── app/               # App scan (winget/registry), export, import
├── self_update/       # GitHub Releases check + self-replace
├── doctor/            # Diagnostics + auto-fix
└── backup/            # PATH snapshot/restore
```

## Key Design Decisions

- **Symlink-based version switching**: `~/.windevkit/active/node -> ../versions/node/v22.11.0/`
- **PATH**: Append `active/bin` once to user PATH at install time
- **Offline**: `install` supports `--from <local-zip>`; export downloads installers for offline use
- **No remote version list**: Users specify version explicitly or `--latest`
- **Admin**: Recommend Windows Developer Mode; fallback to UAC prompt

## PRD

See [PRD.md](./PRD.md) and [Issue #1](https://github.com/opsbli/windevkit/issues/1).

## License

MIT
