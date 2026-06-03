# PRD: windevkit — Windows Development Environment Toolkit

> **Status**: Draft | **Author**: Grill Session Synthesis | **Date**: 2026-06-03

---

## Problem Statement

Reinstalling Windows is a painful process. After a fresh install, developers must:
1. Manually download and install Node.js, Java, Maven, and other runtimes — then configure PATH and environment variables for each.
2. Reinstall dozens of familiar applications (Chrome, 7zip, PowerToys, Notepad++, etc.) one by one.
3. Deal with version management — different projects require different Node.js or Java versions, but manually switching between them is error-prone.

Existing tools solve pieces of this puzzle (nvm-windows for Node.js, jEnv for Java, winget for apps), but none provide a **unified, offline-capable, single-tool solution** that covers:
- Runtime installation **and** version switching
- Application detection, export, and batch restore
- Offline-first design for machines without internet access

**windevkit** aims to be the one tool you copy to a USB drive, plug into a fresh Windows machine, and run — turning hours of setup into minutes.

---

## Solution

windevkit is a **Rust CLI tool for Windows** that provides three integrated capabilities:

1. **Runtime Management** — One-command install, switch, and uninstall of Node.js, Java (JDK), and Maven. Multiple versions coexist; active version is managed via symlinks.
2. **Application Export/Import** — Scan installed Windows apps from winget and registry, let users select what to keep, export an offline toolbox (with installers and portable apps), and restore everything on a new machine.
3. **Self-Contained Offline Operation** — The toolbox is a directory on a USB drive. No internet required for restore after initial export.

---

## User Stories

1. As a developer who just reinstalled Windows, I want to run `windevkit import --from D:\my-toolbox` so that all my runtimes (Node, Java, Maven) are installed and configured in one command.
2. As a developer who just reinstalled Windows, I want to run `windevkit import` for my app toolbox so that Chrome, 7zip, PowerToys, and other utilities are silently installed without manual clicking.
3. As a developer, I want to run `windevkit install node 22.11.0` so that Node.js is downloaded, extracted, and configured with PATH set up.
4. As a developer, I want to run `windevkit install node 22.11.0 --from D:\backup\node-v22.11.0-win-x64.zip` so that I can install from a local file without internet.
5. As a developer, I want to run `windevkit use node 18.20.0` so that the active Node.js version switches and all new terminals see the correct version.
6. As a developer, I want to run `windevkit use java 21.0.3` so that the active JDK switches and `java -version` reflects the change.
7. As a developer, I want to run `windevkit list node` so that I can see which Node versions are installed locally.
8. As a developer with multiple projects, I want to run `windevkit exec node 18 -- npm run build` so that a single command runs in a specific version's environment without changing my global default.
9. As a developer migrating to a new PC, I want to run `windevkit app scan` so that all my installed applications are detected from winget and registry.
10. As a developer migrating to a new PC, I want to interactively select/deselect scanned apps before export so that I only carry over what I actually need.
11. As a developer migrating to a new PC, I want to run `windevkit app export` so that a complete offline toolbox is produced in `~/.windevkit/export/` with manifest, installers, and portable apps.
12. As a developer with portable tools (e.g., Everything, Notepad++ portable), I want to run `windevkit app add-path D:\tools\everything` so that portable apps are included in my export toolbox.
13. As a developer who previously exported a toolbox, I want to run `windevkit import --yes` on my new machine so that all apps are installed non-interactively.
14. As a developer, I want to run `windevkit uninstall node 18.20.0` so that a specific runtime version is removed cleanly (files + symlink).
15. As a developer, I want to run `windevkit doctor` to diagnose issues with PATH integrity, broken symlinks, and missing runtime files.
16. As a developer, I want to run `windevkit doctor --fix` to automatically repair common issues (missing PATH entries, dangling symlinks).
17. As a developer, I want to run `windevkit restore` so that my PATH is rolled back to the state before windevkit modified it (emergency recovery).
18. As a developer, I want to run `windevkit self-update` so that the tool updates itself to the latest version from GitHub Releases.
19. As a developer, I want to run `windevkit status` so that I can see an overview of all installed runtimes and their active versions.
20. As a developer on a fresh Windows install, I want to run `windevkit init` so that `~/.windevkit/` is created with the proper directory structure and config.
21. As a developer in China, I want to use `windevkit install node 22.11.0 --mirror aliyun` so that downloads use a local mirror for faster speeds.
22. As a developer, I want to exclude certain detected apps from scan results (e.g., Windows system components, KB updates) so that my app list is clean and manageable.
23. As a developer, I want to run `windevkit install node --latest` so that the most recent stable version is automatically resolved and installed.
24. As a developer, I want to see download progress bars when installing runtimes so that I know the operation is progressing.
25. As a developer with an existing Node.js installation, I want windevkit to detect the conflict and prompt me (continue/overwrite/cancel) before proceeding.
26. As a developer using nvm-windows, I want windevkit to detect nvm's presence and warn me about incompatibility before modifying PATH.
27. As a developer, I want to use `windevkit status --verbose` to see detailed environment information for debugging.
28. As a developer, I want windevkit to detect whether Windows Developer Mode is enabled and guide me through enabling it if not.
29. As a developer who only needs certain runtimes, I want to install only Node.js without Java or Maven.
30. As a developer, I want the exported toolbox to be copyable to a USB drive so that I can physically transport it to my new machine.

---

## Implementation Decisions

### Module Architecture

The codebase is organized into 7 modules, each with a simple public interface and testable internals:

| Module | Responsibility | Public Interface |
|--------|---------------|-----------------|
| `cli` | Command tree, argument parsing, `--help` generation, `--yes` flag | clap derive macros |
| `config` | `~/.windevkit/config.toml` read/write/validate, mirror config | `Config::load()`, `Config::save()` |
| `runtime` | Node/Java/Maven install, switch, list, uninstall | `RuntimeManager` trait |
| `app` | App scan/export/import, portable dir add | `AppManager` trait |
| `self_update` | GitHub Releases check, download, self-replace | `SelfUpdate::run()` |
| `doctor` | PATH/symlink/config integrity check + auto-fix | `Doctor::run()`, `Doctor::fix()` |
| `backup` | PATH snapshot before mutation, restore | `Backup::snapshot()`, `Backup::restore()` |

### Directory Structure

```
%USERPROFILE%\.windevkit\          ← Home directory
├── config.toml                    ← Global config
├── versions\
│   ├── node\v22.11.0\            ← Extracted Node.js
│   ├── java\jdk21.0.3\           ← Extracted JDK
│   └── maven\3.9.6\              ← Extracted Maven
├── active\                        ← Symlink farm
│   ├── node → ..\versions\node\v22.11.0\
│   ├── java → ..\versions\java\jdk21.0.3\
│   └── maven → ..\versions\maven\3.9.6\
├── export\                        ← Exported toolbox output
│   ├── manifest.toml
│   ├── installers\               ← App installers (.exe/.msi)
│   ├── portables\                ← Portable app directories
│   └── runtimes\                 ← Runtime ZIPs (for offline install)
├── cache\                         ← Download cache (disabled by default)
├── backups\                       ← PATH snapshots
└── logs\windevkit.log             ← Log file (tracing, rotated)
```

### Config Schema (`config.toml`)

```toml
[core]
dev_mode = false              # Windows Developer Mode status
mirror = "aliyun"             # mirror: aliyun | huawei | npmmirror | direct
export_dir = "~/.windevkit/export"

[env]
path_scope = "user"           # PATH scope: user | system

[runtimes.node]
default = "22.11.0"

[runtimes.java]
default = "21.0.3"

[runtimes.maven]
default = "3.9.6"

[app_scan]
exclude_patterns = ["KB*", "Microsoft Visual C++*", ".NET*"]
include_scoop = false
include_choco = false

[app_export]
auto_download_installers = true
```

### Symlink-Based Version Switching

- All runtime versions are extracted to `~/.windevkit/versions/<tool>/<version>/`
- A fixed activation directory `~/.windevkit/active/` contains symlinks pointing to the active version
- `%USERPROFILE%\.windevkit\active\bin` is added to user PATH once at install time
- `windevkit use <tool> <version>` replaces the symlink target atomically
- Requires Windows Developer Mode (symlink creation without admin) or elevated UAC prompt as fallback
- On `init`, detect Developer Mode and guide user; store status in `config.toml`

### Runtime Download and Installation

- Users specify version explicitly (`node 22.11.0`) or `--latest`
- Download URL is constructed from built-in mirror templates:
  - Node: `{mirror}/node/v{version}/node-v{version}-win-x64.zip`
  - Java: Adoptium API `https://api.adoptium.net/v3/binary/latest/{version}/ga/windows/x64/jdk/hotspot/normal/eclipse`
  - Maven: `https://dlcdn.apache.org/maven/maven-3/{version}/binaries/apache-maven-{version}-bin.zip`
- Support `--from <path>` for local file install (no download needed)
- ZIP/TGZ extracted with Rust native crates (`zip`, `flate2` + `tar`)
- Installation is transactional: download → verify → extract → (only then) update symlink + PATH

### PATH Management

- Appends `%USERPROFILE%\.windevkit\active\bin` to user-level PATH on first install
- Before any PATH mutation, current PATH is snapshotted to `~/.windevkit/backups/path-<timestamp>.txt`
- `windevkit restore` reads the most recent snapshot and restores it
- `windevkit doctor --fix` validates PATH entries and removes dangling active/bin references

### App Scan

- Sources: winget (`winget list`), registry (`HKLM\...\Uninstall`), optional Scoop/Choco
- Results merged and deduplicated by app name + version
- Priority for install source: winget > scoop > choco > file
- Built-in exclude list filters system components (KB updates, VC++ redist, .NET runtimes)
- User-defined exclude patterns in `config.toml`
- Interactive selection via `inquire` crate multi-select
- Portable app directories added manually via `windevkit app add-path`

### App Export/Import

**Export** (`windevkit app export`):
- Generates `manifest.toml` describing all selected apps and runtimes
- Downloads winget installers to `installers/` (for offline use)
- Copies portable app directories to `portables/`
- Copies runtime ZIPs to `runtimes/` (for offline reinstall)

**Import** (`windevkit app import`):
- Reads manifest.toml
- For runtimes: extracts ZIPs from `runtimes/` → `versions/`, creates symlinks
- For winget apps: runs installer with known silent args (configurable per-app in manifest)
- For portable apps: copies to target directory, optionally updates PATH
- Interactive by default (`--yes` for non-interactive batch mode)
- Unknown installers are presented to user with a choice: run now / skip / add silent args

### Self-Update

- `windevkit self-update` checks GitHub Releases API for latest version
- Downloads new EXE to temp directory, verifies checksum
- Renames current EXE to `windevkit.old`, moves new EXE into place
- On next launch, deletes `windevkit.old` silently

### CLI Command Tree

```
windevkit
├── init                          # Initialize home directory + config
├── install <tool> <version>      # Install runtime (--from, --mirror, --latest)
├── use <tool> <version>          # Switch active version
├── exec <tool> <version> -- <cmd> # Run command in version-specific env
├── list <tool>                   # List installed versions
├── uninstall <tool> [version]    # Remove version (--all for all)
├── app
│   ├── scan                      # Scan installed applications
│   ├── add-path <dir> [--name]   # Add portable app directory
│   ├── export                    # Export offline toolbox
│   └── import                    # Import and restore on new machine
├── self-update                   # Update windevkit itself
├── doctor [--fix]                # Diagnose and repair
├── restore                       # Rollback PATH to snapshot
├── status [--verbose]            # Show environment overview
└── cache clean                   # Clean download cache
```

### Logging

- Uses `tracing` + `tracing-subscriber`
- Default: `info` level to console (success/error messages only)
- `-v`: `debug` level (download progress, extraction details)
- `-vv`: `trace` level (HTTP request details, PATH changes)
- File: `~/.windevkit/logs/windevkit.log`, auto-rotated (5 files × 1MB)

### Conflict Detection

On `install` or `use`, check:
- Is the tool already in PATH from another source (non-windevkit)?
- Is nvm-windows `.nvm` directory present?
- Is the requested version already installed?
- On conflict: interactive prompt with three options → **Continue** / **Overwrite** / **Cancel**

---

## Testing Decisions

### Testing Philosophy

- Test **external behavior**, not implementation details
- A good test verifies that a module's public interface produces the correct output for a given input, or that a realistic end-to-end scenario succeeds
- Mock external systems (HTTP, filesystem, registry) for deterministic tests
- Use real OS operations only in isolated temp directories

### Test Strategy by Module

| Module | Test Approach | Sandboxing |
|--------|--------------|------------|
| `cli` | Integration tests invoking parsed CLI args, verify dispatch to correct handler | Pure Rust |
| `config` | Unit tests: parse sample TOML, verify field access, test validation rules | Pure Rust |
| `runtime.downloader` | Mock HTTP server (wiremock) for URL template tests; real download test skipped in CI | Mock |
| `runtime.extractor` | Unit tests with known ZIP/TGZ fixtures | Temp dir |
| `runtime.symlink` | Integration test: create symlink, read target, delete symlink | Temp dir (requires Dev Mode or admin in CI) |
| `runtime.path` | Unit: PATH parsing/formatting; Integration: modify PATH in temp env | Temp env var |
| `app.scanner` | Mock winget output via `Command` mock; mock registry via `windows` crate test helpers | Mock |
| `app.exporter` | Integration: scan mock data, produce manifest + file artifacts | Temp dir |
| `app.importer` | Integration: read pre-built manifest, simulate restore | Temp dir |
| `doctor` | Unit: inject broken symlinks, verify detection; Integration: `doctor --fix` repairs them | Temp dir |
| `backup` | Unit: snapshot/restore roundtrip | Temp env var |

### CI Pipeline (GitHub Actions)

- Windows runner only
- `cargo test` — unit + mock integration tests (every PR)
- `cargo test -- --ignored` — real integration tests (nightly, manual trigger)
- `cargo clippy`, `cargo fmt --check`

---

## Out of Scope

- **Runtime version auto-discovery** — No remote version list fetching. Users specify version explicitly. `--latest` fetches the one latest version by hitting the API.
- **Python, Go, Rust support** in v1.0 — Only Node.js, Java (JDK), and Maven. Rust is managed by rustup, which is out of scope.
- **Cross-platform support** — Windows-only. No macOS/Linux support planned.
- **Package manager metadata** — No integration with Chocolatey/Scoop as installation sources (winget + registry only).
- **GUI/TUI** — Pure CLI. The `inquire` interactive prompts are the only interactive element.
- **Automatic tool update** — No background update checker. Only `windevkit self-update` on demand.
- **MSI installer** — Single EXE distribution only. No MSI, no winget publish (v1.0).
- **Code signing** — EXE will not be Authenticode signed (SmartScreen warning expected).
- **E2E tests in CI** — Too costly for CI. Manual testing on real Windows VM only.

---

## Further Notes

- **Rust toolchain**: Requires Rust 1.75+ with `x86_64-pc-windows-msvc` target
- **Key crates**: `clap` v4, `toml`, `reqwest` (native-tls), `windows` (0.58+), `zip`, `flate2`, `tar`, `indicatif`, `tracing`, `inquire`, `anyhow`, `thiserror`, `rstest`
- **GitHub repo**: `windevkit/windevkit` (or user's org)
- **License**: MIT
- **Minimum Windows version**: Windows 10 1809+ (for native symlink support in Developer Mode)
- **First-time flow**: `windevkit init` → detect Developer Mode → create `~/.windevkit/` → done
- **Export portability**: The `export/` directory is designed to be copied to any FAT32/NTFS USB drive. No absolute paths in manifest — all paths are relative.

### Risk Mitigation

| Risk | Mitigation |
|------|-----------|
| ZIP extraction corrupts in edge cases | Verify extracted directory structure matches expected layout before symlink update |
| PATH grows unbounded across multiple install/uninstall cycles | `doctor --fix` deduplicates and validates PATH entries |
| Self-update fails mid-replacement | Keep backup `.old` binary; on next launch detect and retry |
| winget not available on target Windows version | Detect absence in `doctor`; export falls back to file-based install |
| User deletes `active/` directory accidentally | `doctor` detects missing symlinks and re-creates from latest installed version |
