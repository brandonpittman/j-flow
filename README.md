# jflow (jf) - Beautiful Workflow Tool for Jujutsu

A radically simple workflow tool for [Jujutsu](https://github.com/martinvonz/jj) that makes patch-stack development with GitHub beautiful and effortless.

## Philosophy

**Query, don't track.** jflow has zero state files—it queries jj directly using powerful revsets. Your stack is always `::@ ~ ::main@origin`. Simple.

**A handful of commands. That's it.**
- `jf status` - See your beautiful stack (also just `jf`)
- `jf push` - Create bookmarks + PRs, push updates
- `jf pull` - Fetch + rebase
- `jf land` - Clean up after PRs merge
- `jf reorder` - Rearrange changes in your stack
- `jf wip` - Sync work-in-progress between machines

## Installation

### Prerequisites

- [Jujutsu (jj)](https://github.com/martinvonz/jj) installed
- Rust toolchain (for building)
- [GitHub CLI (`gh`)](https://cli.github.com/) — optional, needed for automatic PR creation and `jf land`'s merge detection

### Build from source

```bash
cargo install --path .
```

Or with just the binary name:

```bash
cargo build --release
cp target/release/jf ~/.local/bin/  # or wherever in your PATH
```

## Quick Start

```bash
# In your jj repository
cd my-project

# Initialize jflow (creates .jflow.toml)
jf init

# See your stack
jf status

# Push your stack: creates bookmarks + PRs
jf push

# After a PR merges, clean up and rebase
jf land

# Pull latest and rebase
jf pull
```

## Commands

### `jf status` (or just `jf`)

Beautiful visualization of your stack with PR status. Running `jf` with no command shows status.

```
╭─ Your Stack ────────────────────────────────╮
│                                              │
│  ●  qwer5678  Add login screen              │
│      💡 ready to create PR                   │
│  │                                           │
│  ○  tyui9012  Add backend API               │
│      → add-backend-api                      │
│      ⏳ awaiting review                      │
│  │                                           │
│  ○  asdf1234  Add REST library              │
│      → add-rest-library                     │
│      ✅ approved, ready to merge             │
│  │                                           │
│  ◆  main@origin                             │
│                                              │
╰──────────────────────────────────────────────╯
```

**Icons:**
- `●` Working copy (@)
- `○` Change in stack
- `◆` Main branch
- `→` Has bookmark
- `💡` Ready for action

### `jf init`

Initialize jflow in your repository.

```bash
jf init                # Interactive configuration
jf init --defaults     # Skip prompts, use defaults
jf init --local        # Force a local .jflow.toml even if ~/.jflow.toml exists
jf init --github       # Also create a GitHub repo for this project (uses gh CLI)
```

**What it does:**
1. Checks if you're in a jj repository
2. Detects your primary branch (main, master, or trunk)
3. Detects your remote name
4. Creates `.jflow.toml` with detected settings

If a global `~/.jflow.toml` already exists, `jf init` does nothing (jflow is ready to use); pass `--local` to create a repo-local config that overrides it.

**Interactive mode prompts for:**
- Primary branch name (detected default)
- Remote name (detected default)
- Push style (`squash` or `append`)
- Bookmark prefix (e.g. `jf/`, empty for none)

### `jf push`

Push your stack to GitHub: creates bookmarks and PRs for changes that need them, updates the rest.

```bash
jf push                        # Push the entire stack
jf push -r <revset>            # Push only the given revset
jf push -b my-feature          # Bookmark name for a new change
jf push --squash               # Force squash-style push (override config)
jf push --append               # Force append-style push (override config)
jf push -n                     # Dry run - show what would be pushed
```

**What it does, per change in the stack:**
1. Ensures the primary branch exists on the remote (creates it if missing)
2. Creates a bookmark if the change doesn't have one (uses `-b`, or prompts; configured prefix is applied)
3. Pushes the bookmark to the remote
4. Creates a GitHub PR if none exists (requires `gh` CLI), based against the parent change's bookmark—or the primary branch for the bottom of the stack

Changes without descriptions are rejected—describe them first with `jj describe`.

**Push styles:**
- `squash` (default) - force-push each bookmark so the PR always shows the current, rewritten commit
- `append` - intended to push incremental commits so review context is preserved across updates

> **Note:** `append` is not yet implemented—both styles currently behave identically (force-push). The setting and flags are accepted for forward compatibility.

**With stack context enabled** (default), the PR description includes:
```markdown
Add REST library

---

**Part of stack:**

- **This PR** (Add REST library)
- ⏳ Add backend API (bookmark: `add-backend-api`)
- ⏳ Add login screen (bookmark: `add-login-screen`)
```

**Requirements:**
- `gh` CLI installed for automatic PR creation
- Without `gh`, bookmarks are pushed but PRs must be created manually

### `jf pull`

Fetch from remote and rebase your stack onto the primary branch.

```bash
jf pull                # Fetch from configured remote
jf pull -r upstream    # Fetch from a different remote
```

Equivalent to:
```bash
jj git fetch
jj rebase -d main@origin
```

### `jf land`

Clean up after PRs are merged.

```bash
jf land                # Auto-detect merged PRs (via gh CLI)
jf land my-feature     # Land a specific bookmark (checks it's merged first)
jf land -n             # Dry run - show what would be cleaned up
```

**What it does:**
1. Fetches latest from the remote
2. Finds bookmarks whose PRs are merged (`gh pr view`)
3. Deletes those bookmarks locally and on the remote
4. Rebases the remaining stack onto the primary branch
5. Cleans up leftover empty commits

### `jf reorder`

Rearrange changes in your stack.

```bash
jf reorder abc def ghi        # Reorder changes into the given order
jf reorder -f xyz abc def     # Reorder starting from xyz (inclusive)
jf reorder --invert           # Reverse the entire stack
jf reorder --invert -f abc    # Reverse from abc to @ (inclusive)
```

After reordering, remember to `jf push` so the PRs reflect the new order.

### `jf wip`

Sync work-in-progress between machines using a personal `wip/<username>` bookmark (derived from jj's `user.name`).

```bash
jf wip           # Show wip branch status
jf wip push      # Push your stack to the wip branch
jf wip pull      # Fetch the wip branch and rebase onto main
jf wip clean     # Delete the wip branch (local + remote)
```

**Safety rails:**
- `jf wip push` refuses to overwrite an existing remote wip branch (use `--force`)
- `jf wip pull` refuses to run if you have local changes
- `jf wip clean` refuses to delete changes that aren't in any PR (use `--force`)

Typical flow: `jf wip push` on your desktop at the end of the day, `jf wip pull` on your laptop, then `jf wip clean` once everything is in PRs.

## Configuration

Config is loaded with this hierarchy (later overrides earlier):

1. Built-in defaults
2. Global `~/.jflow.toml`
3. Local `.jflow.toml` in the repo (or a parent directory)

```toml
[remote]
name = "origin"           # Remote name
primary = "main"          # Primary branch (main/master/...)

[github]
push_style = "squash"     # "squash" (force-push) or "append" (incremental)
merge_style = "squash"    # "squash", "merge", or "rebase"
stack_context = true      # Add stack info to PR descriptions

[display]
theme = "catppuccin"      # catppuccin, nord, dracula, default
icons = "unicode"         # unicode, ascii, nerdfont
show_commit_ids = false   # Show git commit hashes alongside change IDs

[bookmarks]
prefix = ""               # Prefix for auto-created bookmarks (e.g., "jf/")
```

See [`.jflow.toml.example`](.jflow.toml.example) for a commented template.

## Themes

**Catppuccin Mocha** (default)
- Warm, pastel colors
- Excellent contrast

**Nord**
- Cool, arctic palette
- Easy on the eyes

**Dracula**
- High contrast
- Popular dark theme

**Default**
- Uses terminal colors
- Maximum compatibility

Icons come in `unicode`, `ascii`, and `nerdfont` flavors.

## How It Works

### Revset-Powered

jflow uses jj's revset language under the hood:

```rust
// Your stack
"::@ ~ ::main@origin"

// Changes with bookmarks
"bookmarks() & (::@ ~ ::main@origin)"

// Changes ready for PR
"(::@ ~ ::main@origin) ~ bookmarks()"
```

No metadata files. No state tracking. Just queries.

### GitHub Integration

GitHub operations (PR creation, merge detection) go through the [`gh` CLI](https://cli.github.com/). Without it, jflow still pushes bookmarks—you just create PRs manually.

## Workflow Example

### Complete Patch Stack Workflow

```bash
# 0. Initialize jflow (first time only)
jf init

# 1. Start work (outside-in development)
jj new -m "Add REST library"
# ... implement library ...

jj new -m "Add backend API"
# ... implement API using library ...

jj new -m "Add login screen"
# ... implement UI using API ...

# 2. View your stack
jf status

# Output:
# ╭─ Your Stack ────────────────────────────────╮
# │  ●  xyz789  Add login screen                │
# │      💡 ready to create PR                   │
# │  ○  def456  Add backend API                 │
# │      💡 ready to create PR                   │
# │  ○  abc123  Add REST library                │
# │      💡 ready to create PR                   │
# │  ◆  main@origin                             │
# ╰──────────────────────────────────────────────╯

# 3. Push the stack - creates bookmarks + PRs bottom-up
jf push
# Prompts for a bookmark name per change, pushes, opens PRs

# 4. Teammate reviews library PR and requests changes
# Edit the library commit directly
jj edit abc123
# ... make changes ...

# 5. Push again - all bookmarks and PRs update
jf push

# 6. Library PR gets merged - clean up and rebase
jf land

# Output:
# ℹ Found 1 merged PR(s)
# ℹ Deleting bookmark 'rest-library'...
# ℹ Rebasing stack onto main@origin...
# ✓ Cleanup complete!
#
# Stack now shows:
# ╭─ Your Stack ────────────────────────────────╮
# │  ●  xyz789  Add login screen                │
# │  ○  def456  Add backend API                 │
# │  ◆  main@origin                             │
# ╰──────────────────────────────────────────────╯

# 7. Push the remaining PRs against their new bases
jf push
```

### Daily Workflow Commands

```bash
# Morning: Pull latest changes
jf pull

# Create new work
jj new -m "Feature X"

# Check status anytime
jf status

# Push stack when ready (creates PRs)
jf push

# After PRs merge
jf land

# Moving between machines
jf wip push   # desktop
jf wip pull   # laptop
```

## Development Status

Currently implemented:
- ✅ `jf status` - Beautiful stack visualization (also plain `jf`)
- ✅ `jf init` - Initialize jflow with smart defaults
- ✅ `jf push` - Create bookmarks + PRs, push updates
- ✅ `jf pull` - Fetch + rebase stack
- ✅ `jf land` - Clean up merged PRs
- ✅ `jf reorder` - Reorder or invert the stack
- ✅ `jf wip` - Sync work-in-progress between machines

Ready to use for daily workflow!

## Contributing

This is an experimental project. Contributions welcome!

```bash
# Run with example
cd /path/to/your/jj/repo
jf status

# Build
cargo build

# Test
cargo test
```

## License

MIT

## Credits

Inspired by:
- [Jujutsu](https://github.com/martinvonz/jj) by Martin von Zweigbergk
- [Drew Deponte's patch stack methodology](https://drewdeponte.com/blog/how-we-should-be-using-git/)
- [Steve Klabnik's Jujutsu tutorial](https://steveklabnik.github.io/jujutsu-tutorial/)

Icons and colors from:
- [Catppuccin](https://github.com/catppuccin/catppuccin)
- [Nord](https://www.nordtheme.com/)
- [Dracula](https://draculatheme.com/)
