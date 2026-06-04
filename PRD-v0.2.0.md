# PRD v0.2.0: windevkit — TUI Export, Stronger Offline Rules, Safer Import

> Status: Implemented (release prep)  
> Scope: v0.2.0  
> Based on: v0.2.0 Grill Session  
> Date: 2026-06-04

---

## Problem Statement

windevkit v0.1.x already supports runtime installation/switching and app scan/export/import, but the current workflow still has three major gaps:

1. **App selection is still too CLI-heavy**  
   Even after grouping and filtering, selecting dozens of apps from large scan results is still cumbersome. Users need a fast, visual, keyboard-driven selection workflow.

2. **Offline export is still too weak for many apps**  
   Export currently relies heavily on registry metadata and only partially supports offline installer capture. Many apps still fall back to “install later via winget” instead of producing a strong offline toolbox.

3. **Import behavior is not resilient enough**  
   Installer failures are common on Windows. The current import flow needs better retry/skip/abort behavior, stronger silent-install coverage, and better failure summaries.

The result: windevkit is already useful, but not yet strong enough to feel like a polished “rebuild-my-machine” tool.

---

## Solution

v0.2.0 upgrades windevkit in three coordinated directions:

1. **Lightweight TUI for app export flows**  
   `windevkit app export` enters a full-screen TUI by default, allowing category browsing, search, bulk selection, and selected-only views.

2. **Stronger offline export via rule-based installer downloads**  
   windevkit gains a built-in installer rule library for mainstream apps plus user-overridable `rules.toml`, enabling direct download of installers beyond raw registry-only export.

3. **Safer and clearer import behavior**  
   Import supports better silent-install rules, interactive retry/skip/abort handling, and final failure summaries for partial success scenarios.

This keeps the current CLI architecture intact while upgrading the most painful app-management workflows.

---

## User Stories

1. As a developer preparing to reinstall Windows, I want `windevkit app export` to open a full-screen selector so that I can quickly choose which apps to carry over.
2. As a developer with 80+ installed apps, I want to search within the TUI so that I can find “Chrome”, “JetBrains”, or “Python” instantly.
3. As a developer with many noisy system components, I want apps grouped by category so that I can focus on Browser / IDE / Runtime / Utility instead of scrolling through a flat list.
4. As a developer, I want to toggle all apps in a category so that I can select all IDEs or all Utilities with one action.
5. As a developer, I want to view only currently selected apps so that I can verify my export set before confirming.
6. As a developer, I want `windevkit app export --no-tui` to keep working so that I can automate export in scripts.
7. As a developer, I want `windevkit app export --category IDE` so that I can export only IDE-related tools.
8. As a developer, I want `windevkit app export --filter chrome` so that I can export only matching apps.
9. As a developer, I want mainstream apps like Chrome, VS Code, Git, 7-Zip, and Firefox to download real installers during export so that my toolbox is truly useful offline.
10. As a developer using niche or internal company software, I want to add my own download/install rules in `rules.toml` so that windevkit can export those apps too.
11. As a developer, I want built-in installer rules to work out of the box so that I do not need to hand-write rules for mainstream apps.
12. As a developer, I want user rules to override built-in rules so that I can correct or replace upstream defaults.
13. As a developer, I want installer downloads to happen concurrently so that large toolbox exports complete much faster.
14. As a developer, I want the export flow to generate both `manifest.toml` and `apps.md` so that I have both machine-readable and human-readable records.
15. As a developer, I want the export flow to also produce a `.zip` bundle so that I can easily move the toolbox to USB or cloud storage.
16. As a developer restoring a new machine, I want import to continue past failures so that one broken installer does not abort the whole restore.
17. As a developer using interactive import, I want to choose retry / skip / abort when an installer fails so that I retain control.
18. As a developer using non-interactive import, I want failures summarized at the end so that I can manually fix only what failed.
19. As a developer, I want silent-install parameters for more apps so that restore needs less manual clicking.
20. As a developer, I want portable apps to remain supported in v0.2.0 so that manually added folders still export/import correctly.
21. As a developer, I want current commands (`init/install/use/list/uninstall/status/doctor`) to keep working unchanged so that v0.2.0 does not break existing workflows.
22. As a power user, I want `windevkit app tui` as an explicit entry point so that I can enter the selector directly without starting export.
23. As a developer, I want my last app selection remembered so that repeated exports do not require re-selecting the same apps every time.
24. As a developer, I want the TUI to support keyboard-first navigation so that export is fast without using a mouse.
25. As a developer, I want app categories to be derived consistently from rules and heuristics so that the grouped view remains predictable.

---

## Implementation Decisions

### 1. Lightweight TUI Scope

The TUI is intentionally limited to the app-selection/export workflow. It does **not** replace the entire CLI.

Covered by TUI:
- App scan results
- Search/filtering
- Category grouping
- Multi-select and bulk actions
- Export confirmation

Not covered by TUI in v0.2.0:
- Runtime install/switch flows
- Doctor/status dashboards
- Full import orchestration UI

### 2. TUI Technology

Use **`ratatui + crossterm`**.

Reasoning:
- `inquire` is no longer sufficient for search + grouped lists + selected-only filtering + bulk actions.
- `ratatui` gives full-screen list panes, status bars, keybinding hints, and category sidebars without requiring a GUI.

### 3. TUI Entry Strategy

The product keeps both automatic and explicit entry modes:
- Default: `windevkit app export` enters TUI
- Explicit: `windevkit app tui`
- Current non-interactive escape hatch: `windevkit app export --yes`

This preserves scriptability while improving the default human workflow.

### 4. TUI Interaction Model

Minimum TUI interaction model includes:
- `/` = search
- `space` = toggle selection
- `a` = select all / clear all
- `c` = category switch / category-scoped operation
- `s` = selected-only view
- `enter` = confirm export
- `esc` / `q` = cancel/back

### 5. Rule System Strategy

Installer rule handling is **hybrid**:
- Built-in default rules compiled into the app
- User `rules.toml` for override/append behavior

User rules override built-in rules by match precedence.

### 6. Minimal Rule Model for v0.2.0

Supported rule fields in v0.2.0:
- `match`
- `download_url`
- `silent_args`
- `installer_type`
- `category`
- `portable`

Not in scope for v0.2.0 rule model:
- Arbitrary scripting hooks
- Conditional expressions
- Templated variable expansion language
- Full semantic version constraints

### 7. Export Download Strategy

Installer downloads use **fixed concurrency** with a default of `3`.

Rationale:
- Faster than serial download
- Simpler and more predictable than adaptive download scheduling
- Easy to expose as config later

### 8. Import Failure Handling

Import uses **two modes**:
- Non-interactive mode: continue through failures, summarize at end
- Interactive mode: prompt on failure with retry / skip / abort

This is the agreed B + C behavior from the Grill Session.

### 9. CLI Compatibility Guarantee

v0.2.0 does not break the existing command set:
- `init`
- `install`
- `use`
- `exec`
- `list`
- `uninstall`
- `status`
- `doctor`
- current `app` subcommands

Behavior may improve, but command intent stays stable.

### 10. Export Artifacts

App export now produces:
- `manifest.toml`
- `apps.md`
- installer files (where available)
- portable directories
- runtime archives
- final `.zip` bundle

### 11. Category Strategy

App categories remain heuristic/rule-driven and include:
- Browser
- IDE
- Runtime
- Dev Tool
- Utility
- Other

Rules may explicitly set categories; heuristics fill gaps.

---

## Testing Decisions

### Testing Philosophy

Only test externally visible behavior:
- TUI state transitions and selection behavior
- Rule resolution behavior
- Export artifact creation
- Import retry/skip/abort control flow
- Failure summary correctness

Avoid testing internal implementation details like exact widget layout internals unless they encode a product decision.

### Modules to Test

1. **TUI state model**
   - search filter behavior
   - category switching
   - bulk select / clear
   - selected-only view
   - persisted selection reuse

2. **Rule resolution**
   - built-in rule hit
   - user rule override
   - missing rule fallback
   - category assignment from rules

3. **Export planner**
   - filter/category CLI constraints
   - generated manifest
   - generated apps.md
   - zip bundle generation
   - concurrent download scheduling behavior

4. **Import executor**
   - retry / skip / abort state machine
   - failure summary generation
   - silent arg resolution

5. **Backward compatibility**
   - existing CLI commands still parse and dispatch

### Good Test Definition

A good test should verify:
- what the user sees
- what files were produced
- what was selected or skipped
- whether recovery behavior matches product decisions

A bad test would assert:
- exact private helper structure
- exact internal field order unrelated to UX
- specific rendering implementation details that do not change behavior

---

## Out of Scope

The following are explicitly out of scope for v0.2.0:

- Full product-wide TUI replacing all CLI flows
- GUI / native windowed interface
- Full dynamic rule DSL
- Auto-discovery of arbitrary installer download links from the internet
- Perfect installer capture for every app ecosystem
- Full enterprise package-management support
- Code signing changes
- Linux/macOS support
- Replacing current runtime architecture

---

## Further Notes

### Proposed v0.2.0 Module Expansion

Likely additions/refactors:
- `app::rules`
- `app::tui`
- `app::planner`
- `app::import_flow`

### Recommended Delivery Order

1. Rule system
2. TUI state model (without rendering complexity first)
3. `app export` TUI integration
4. Concurrent download engine for installers
5. Import retry/skip/abort workflow
6. CLI polish and docs

### Release Goal

v0.2.0 should make windevkit feel like a real “prepare my rebuild kit” tool, not just a CLI prototype.

---

## Implementation Snapshot

Implemented in the current codebase:

- `app::tui` based on `ratatui + crossterm`
- `windevkit app tui`
- `windevkit app export` defaulting to TUI unless `--yes`
- saved last selection reuse
- built-in rules + `%USERPROFILE%\\.windevkit\\rules.toml`
- rule-driven category / silent args / installer type / portable behavior
- concurrent export downloads with default concurrency `3`
- config-driven `app_export.download_concurrency`
- CLI override via `windevkit app export --concurrency <n>`
- import summary + interactive `Retry / Skip / Abort`
- local artifact handling for `exe` / `msi` / `zip` / `portable`
- `manifest.toml` + `apps.md` + final `.zip`

Known gap versus earlier draft wording:

- `--no-tui` is not implemented; `--yes` is the current non-interactive path.
