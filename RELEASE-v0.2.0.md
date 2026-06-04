# windevkit v0.2.0

## Highlights

- New full-screen app selection TUI powered by `ratatui + crossterm`
- `windevkit app export` now opens TUI by default
- New `windevkit app tui` entry point
- Built-in installer rules plus user-overridable `%USERPROFILE%\.windevkit\rules.toml`
- Concurrent installer/runtime downloads with configurable concurrency
- More resilient import flow with `Retry / Skip / Abort`
- Local artifact import support for `exe`, `msi`, `zip`, and `portable`
- Export now produces `manifest.toml`, `apps.md`, and a final `.zip`

## User-visible changes

### TUI

```powershell
windevkit app tui
windevkit app export --output D:\my-toolbox
```

Keybindings:

- `/` search
- `space` toggle
- `a` select/clear visible
- `c` cycle category
- `s` selected-only
- `enter` confirm
- `q` / `esc` quit

### Rules

User rules file:

```text
%USERPROFILE%\.windevkit\rules.toml
```

Supported fields:

- `match`
- `download_url`
- `silent_args`
- `installer_type`
- `category`
- `portable`

### Export concurrency

Default:

```toml
[app_export]
download_concurrency = 3
```

Override per command:

```powershell
windevkit app export --concurrency 5
```

### Import behavior

Interactive import:

```powershell
windevkit app import D:\my-toolbox
```

Non-interactive import:

```powershell
windevkit app import D:\my-toolbox --yes
```

Behavior:

- continue on failure by default
- summarize successes / skips / failures at end
- interactive failure handling supports `Retry / Skip / Abort`

## Compatibility notes

- Existing runtime commands remain compatible
- Existing app scan/export/import workflows remain available
- Older `config.toml` files remain compatible; missing `download_concurrency` falls back to default `3`
- Current non-interactive export path is `--yes`

## Suggested release checklist

- run `cargo test`
- verify `windevkit app export --help`
- verify `windevkit app tui --help`
- verify export/import on a sample toolbox
- tag and publish `v0.2.0`
