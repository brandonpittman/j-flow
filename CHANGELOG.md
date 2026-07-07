# Changelog

All notable changes to jflow will be documented in this file.

## [0.5.2] - 2026-07-08

### 🐛 Fixes

- Config merge: an explicit local `push_style = "squash"` now overrides a global `"append"` (previously "squash" was indistinguishable from "not set" and lost)
- `jf push --squash` reliably resets an append-pushed branch back to the change's single commit, even when the remote moved behind jj's back (e.g. append pushes from another machine) — previously this could silently no-op

### ✨ New Features

- `jf push --squash` prints a hint when config still resolves to append for a bookmark (next plain push would append again)
- `jf status` tags append-style bookmarks with `(append)`

## [0.2.0] - 2024-12-20

### ✨ New Features

**`jf init` - Repository Initialization**
- Interactive setup wizard
- Auto-detects main branch and remote
- Checks for gh CLI availability
- Creates `.jflow.toml` with smart defaults
- `--defaults` flag for non-interactive setup

## [0.1.0] - 2024-12-20

### 🎉 Initial Release

Complete implementation of all core commands for jflow - a beautiful workflow tool for Jujutsu.

### ✅ Implemented Features

#### Commands

**`jf status`**
- Beautiful stack visualization with Unicode/ASCII icons
- Color themes: Catppuccin, Nord, Dracula, Default
- Shows change IDs, descriptions, bookmarks
- Detects working copy
- Provides helpful suggestions
- Queries jj using revsets: `::@ ~ ::main@origin`

**`jf pr <change-id> <bookmark-name>`**
- Creates bookmark with configurable prefix
- Pushes bookmark to remote
- Integrates with `gh` CLI for automatic PR creation
- Adds stack context to PR descriptions
- Falls back to manual PR creation if `gh` not available

**`jf sync`**
- Updates all bookmarks to current commit positions
- Pushes all changes to remote
- Handles jj's stable change IDs correctly
- Dry-run mode for preview
- Clear progress output

**`jf pull`**
- Fetches from configurable remote
- Rebases stack onto main
- Shows updated stack after pull
- Equivalent to `jj git fetch && jj rebase -d main@origin`

#### Core Architecture

**Revset-Powered**
- Zero state files - queries jj directly
- Uses jj's powerful revset language
- Stack is always `::@ ~ ::main@origin`

**Beautiful Output**
- 4 color themes with TrueColor support
- Unicode and ASCII icon sets
- Clean box-drawing characters
- Colored change IDs and status indicators

**Configuration System**
- `.jflow.toml` configuration file
- Customizable revsets, themes, icons
- Bookmark prefix configuration
- GitHub integration settings

**Type-Safe JJ Integration**
- Parses jj JSON output
- Structured Change and Bookmark types
- Error handling with context

### 🎨 Themes & Icons

**Themes:**
- Catppuccin Mocha (default) - Warm pastels
- Nord - Cool arctic palette
- Dracula - High contrast
- Default - Terminal colors

**Icon Sets:**
- Unicode: ●○◆→💡✓✗
- ASCII: *o#->!OKXX

### 🛠️ Technical Details

**Language:** Rust
**Dependencies:**
- clap - CLI parsing
- serde - JSON serialization
- colored - Terminal colors
- console - Terminal utilities
- anyhow - Error handling

**Requirements:**
- Jujutsu (jj) installed
- Optional: gh CLI for PR creation
- Rust toolchain for building

### 📦 File Structure

```
jflow/
├── src/
│   ├── main.rs          # CLI entry
│   ├── config.rs        # Configuration
│   ├── commands/        # All 4 commands
│   ├── jj/              # JJ integration
│   └── ui/              # Rendering & themes
├── Cargo.toml
├── README.md
└── .jflow.toml.example
```

### 🚀 Usage

```bash
# View stack
jf status

# Create PR
jf pr abc1234 my-feature

# Sync bookmarks
jf sync

# Pull & rebase
jf pull
```

### 🎯 Design Principles

1. **Query, don't track** - No metadata files
2. **Revset-first** - Leverage jj's query language
3. **Beautiful by default** - Great UX out of the box
4. **Four commands only** - Radical simplicity
5. **Config optional** - Works with defaults

### 📚 Documentation

- README.md - Complete guide
- QUICKSTART.md - Build instructions
- example-workflow.sh - Demo script
- .jflow.toml.example - Config template

### 🙏 Credits

Inspired by:
- Jujutsu by Martin von Zweigbergk
- Drew Deponte's patch stack methodology
- Steve Klabnik's Jujutsu tutorial
- Catppuccin, Nord, and Dracula color schemes

---

## Future Enhancements (Ideas)

- [ ] GitHub API integration (without gh CLI)
- [ ] PR status querying (approvals, CI)
- [ ] Interactive TUI mode
- [ ] Multi-stack support
- [ ] Auto-cleanup merged bookmarks
- [ ] Conflict visualization
- [ ] Integration tests
- [ ] Homebrew/package distribution
