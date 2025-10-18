# Feature Plan: Init Copy Claude Config

**Branch**: `feat/init-copy-claude-config`
**Status**: Planning
**Date**: 2025-10-17

## Overview

Add functionality to `codanna init` command to intelligently copy `.claude` folder configuration files (agents, commands, prompts) to the project being initialized.

**Key Design Decision**: Template files will be **embedded directly into the binary at compile time** using `rust-embed`. This approach:
- Works seamlessly with `cargo install` (no binary releases needed)
- Requires no network access or GitHub API calls
- Always provides the latest templates matching the installed version
- Works offline
- Zero runtime dependencies for template access

## Quick Summary

**What's being added:**
- `codanna init --copy-claude` - Copies embedded .claude templates to project
- `codanna init --copy-claude-from <path>` - Copies custom templates from filesystem
- Templates embedded at build time from `.claude/` directory
- Smart conflict resolution (skip existing files, unless `--force`)

**Backwards compatible:**
- `codanna init` without flags works exactly as before (only creates settings.toml)

**Architecture:**
```
┌─────────────────────────────────────────┐
│   At Compile Time (cargo build)         │
│                                          │
│  .claude/                                │
│  ├── agents/                             │
│  ├── commands/                           │
│  ├── prompts/                            │
│  └── hooks/                              │
│           ↓                              │
│    rust-embed macro                      │
│           ↓                              │
│  Embedded in codanna binary              │
└─────────────────────────────────────────┘
              ↓
┌─────────────────────────────────────────┐
│   At Runtime (codanna init)              │
│                                          │
│  User runs: codanna init --copy-claude   │
│           ↓                              │
│  Extract embedded files                  │
│           ↓                              │
│  Check for conflicts                     │
│           ↓                              │
│  Write to .codanna/ (skip if exists)     │
│           ↓                              │
│  Report: X copied, Y skipped             │
└─────────────────────────────────────────┘
```

## Feature Request

> "Feature request to add a project init feature that will create / copy across agents, command and prompts during the init process to save manually creating and copying across"

The goal is to add an argument to `codanna init` that will copy the content of a `.claude` folder intelligently to the project folder being initialized, with "intelligently" meaning we need to avoid overwriting files if they exist.

## Current Implementation Analysis

### Current `codanna init` behavior

Located in [src/main.rs:612-635](src/main.rs#L612-L635):

```rust
Commands::Init { force } => {
    let config_path = PathBuf::from(".codanna/settings.toml");

    if config_path.exists() && !force {
        eprintln!("Configuration file already exists at: {}", config_path.display());
        eprintln!("Use --force to overwrite");
        std::process::exit(1);
    }

    match Settings::init_config_file(*force) {
        Ok(path) => {
            println!("Created configuration file at: {}", path.display());
            println!("Edit this file to customize your settings.");
        }
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }
    return;
}
```

Currently:
- Creates `.codanna/settings.toml` only
- Has `--force` flag to overwrite existing config
- No support for copying additional Claude configuration files

### Existing .claude Structure (from codanna project itself)

```
.claude/
├── README.md
├── agents/
│   └── codanna-navigator.md
├── commands/
│   ├── deps.md
│   └── find.md
├── hooks/
│   └── hooks-config.yml
├── prompts/
│   └── mcp-workflow.md
└── settings.local.json
```

## Proposed Design

### 1. CLI Interface

Add a new optional flag to the `Init` command:

```rust
Init {
    /// Force overwrite existing configuration
    #[arg(short, long)]
    force: bool,

    /// Copy embedded .claude configuration to project
    #[arg(long)]
    copy_claude: bool,

    /// Copy .claude configuration from custom source directory
    #[arg(long, value_name = "PATH", conflicts_with = "copy_claude")]
    copy_claude_from: Option<PathBuf>,
}
```

**Usage examples:**
```bash
# Standard init - creates only settings.toml (current behavior)
codanna init

# Init with embedded templates - copies built-in .claude config
codanna init --copy-claude

# Init with copying from specific source (custom templates)
codanna init --copy-claude-from /path/to/template/project

# Init with force overwrite
codanna init --force --copy-claude
```

**Note**: We may want to simplify this to just make `--copy-claude` the default behavior in the future, but for now we keep backwards compatibility.

### 2. Source Location Strategy

**Key Consideration**: Since codanna has no binary releases and users install via `cargo install` or build from source, we need to embed the template files directly in the binary at compile time.

Multiple options for where to find the template `.claude` folder:

1. **Embedded templates** (primary): Files embedded in binary at compile time from `.claude/` directory
   - Uses `rust-embed` or `include_str!` macro
   - Always available regardless of how codanna was installed
   - Contains the latest `.claude` configuration from the repo at compile time

2. **Explicit path** (via `--copy-claude-from` flag): User-specified source directory
   - Allows users to use custom templates
   - Overrides embedded templates

3. **Global template location** (future): `~/.codanna/templates/claude/`
   - For users who want to customize their default templates
   - Lower priority than embedded but higher than "no copy"

**Priority order:**
1. If `--copy-claude-from` is provided, use that path (explicit custom override)
2. Else if `--copy-claude` flag is set, use embedded templates (built into binary)
3. Else skip copying (maintain backwards compatibility - only create settings.toml)

**Why Embedded Templates?**
- No dependency on GitHub API or network access
- Works offline
- Guaranteed to match the version of codanna installed
- No need to distribute separate template files
- Users get the "official" templates automatically

**Comparison of Approaches:**

| Approach | Pros | Cons | Decision |
|----------|------|------|----------|
| **Embedded files (rust-embed)** | ✅ Works with cargo install<br>✅ No network needed<br>✅ Version-matched templates<br>✅ Zero runtime deps | ❌ Slightly larger binary (~10KB)<br>❌ Can't update templates without recompiling | ✅ **CHOSEN** |
| GitHub API fetch at runtime | ✅ Always latest templates<br>✅ No binary size increase | ❌ Requires network<br>❌ API rate limits<br>❌ Extra complexity<br>❌ Fails offline | ❌ Not suitable |
| Distribute template files separately | ✅ Updatable without recompile | ❌ Doesn't work with cargo install<br>❌ Where to store them?<br>❌ Version mismatch issues | ❌ Not suitable |
| No default templates (custom only) | ✅ Simplest code | ❌ Poor UX - defeats feature purpose<br>❌ Users must find templates | ❌ Not suitable |

### 3. Directory Structure to Copy

Copy these subdirectories from `.claude/`:
- `agents/` - Custom agent definitions
- `commands/` - Slash command definitions
- `prompts/` - Reusable prompt templates
- `hooks/` - Hook configurations

**Do NOT copy:**
- `settings.local.json` - Project-specific settings
- `README.md` - Project-specific documentation
- Any `.gitignore` or version control files

### 4. Conflict Resolution Strategy

**Key Principle**: Never overwrite existing files unless `--force` is used

**Behavior:**

| Scenario | Without `--force` | With `--force` |
|----------|-------------------|----------------|
| File doesn't exist | Copy file | Copy file |
| File exists | Skip, print info message | Overwrite, print warning |
| Directory doesn't exist | Create directory | Create directory |
| Directory exists | Continue into directory | Continue into directory |

**Example output (embedded templates):**
```
Copying embedded .claude configuration...
✓ Created .codanna/agents/
✓ Copied .codanna/agents/codanna-navigator.md
✓ Created .codanna/commands/
✓ Copied .codanna/commands/find.md
✓ Copied .codanna/commands/deps.md
✓ Created .codanna/prompts/
✓ Copied .codanna/prompts/mcp-workflow.md
✓ Created .codanna/hooks/
✓ Copied .codanna/hooks/hooks-config.yml

Summary:
- 5 files copied from embedded templates
- Templates version: codanna v0.x.x
```

**Example output (with existing files):**
```
Copying embedded .claude configuration...
✓ Created .codanna/agents/
✓ Copied .codanna/agents/codanna-navigator.md
✓ Created .codanna/commands/
ℹ Skipped .codanna/commands/find.md (already exists)
✓ Copied .codanna/commands/deps.md
✓ Created .codanna/prompts/
✓ Copied .codanna/prompts/mcp-workflow.md

Summary:
- 4 files copied
- 1 file skipped (already exists)
- Use --force to overwrite existing files
```

**Example output (custom path):**
```
Copying .claude configuration from /path/to/custom/template...
✓ Created .codanna/agents/
✓ Copied .codanna/agents/my-custom-agent.md
✓ Created .codanna/commands/
✓ Copied .codanna/commands/custom-command.md

Summary:
- 2 files copied from custom template
- Source: /path/to/custom/template
```

### 5. Implementation Plan

#### Phase 1: Setup Embedded Templates
1. Add `rust-embed` dependency to `Cargo.toml`
2. Create embedded templates struct in `src/init/claude_config.rs`:
   ```rust
   #[derive(RustEmbed)]
   #[folder = ".claude/"]
   #[exclude = "settings.local.json"]
   #[exclude = "README.md"]
   struct ClaudeTemplates;
   ```
3. Add helper functions to extract embedded files

#### Phase 2: Core Infrastructure
4. Update `Commands::Init` enum to add `copy_claude` and `copy_claude_from` fields
5. Create new module `src/init/claude_config.rs` with:
   - `ClaudeTemplates` struct with `#[derive(RustEmbed)]`
   - `copy_embedded_templates()` - Copy from embedded files
   - `copy_from_path()` - Copy from filesystem path
   - `copy_claude_config()` - Main entry point, routes to embedded or filesystem
   - `should_copy_file()` - Filter logic for what to copy
   - `write_template_file()` - Write file to destination with conflict handling

#### Phase 3: File Operations
6. Implement copying logic that works with both:
   - Embedded files (from rust-embed)
   - Filesystem files (from `--copy-claude-from`)
7. Add progress/status reporting
8. Handle errors gracefully (permissions, disk space, etc.)

#### Phase 4: Testing & Documentation
9. Add unit tests for:
   - Embedded template access
   - File filtering logic
   - Conflict resolution
   - Path resolution
10. Add integration test for full init workflow
11. Update README and help text
12. Document that templates are embedded from repo at compile time

### 6. Code Structure

```
src/
├── init.rs                      # Existing global init code
├── init/
│   ├── mod.rs                   # Module declarations
│   └── claude_config.rs         # New: Claude config copying logic with embedded templates
├── main.rs                      # Update Commands::Init handling
└── config.rs                    # Existing settings code

.claude/                         # Embedded at compile time
├── agents/
│   └── codanna-navigator.md     # Embedded in binary
├── commands/
│   ├── deps.md                  # Embedded in binary
│   └── find.md                  # Embedded in binary
├── hooks/
│   └── hooks-config.yml         # Embedded in binary
└── prompts/
    └── mcp-workflow.md          # Embedded in binary

# These files are NOT embedded:
# - .claude/README.md (project-specific)
# - .claude/settings.local.json (project-specific)
```

**Dependency additions:**
```toml
[dependencies]
rust-embed = "8.5"  # For embedding .claude files at compile time
```

**How embedded templates work:**

```rust
// In src/init/claude_config.rs
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = ".claude/"]
#[exclude = "settings.local.json"]
#[exclude = "README.md"]
#[exclude = ".gitignore"]
struct ClaudeTemplates;

// Access embedded files:
for file in ClaudeTemplates::iter() {
    let content = ClaudeTemplates::get(&file).unwrap();
    // Write to .codanna/ directory
}
```

At compile time, `rust-embed` reads all files from `.claude/` directory and embeds them directly into the binary. This means:
- No runtime filesystem access to codanna's source
- Works with `cargo install` from crates.io
- Always in sync with the version being built
- Zero network dependencies

### 7. Edge Cases & Considerations

**Security:**
- Validate that source path is readable and not malicious
- Don't follow symlinks outside of source directory
- Limit file size for safety (e.g., max 1MB per file)

**Cross-platform:**
- Handle Windows vs Unix path separators
- Respect filesystem permissions
- Handle long paths on Windows

**User Experience:**
- Clear error messages for common failures
- Dry-run mode? (future enhancement: `--dry-run` to preview what would be copied)
- Interactive mode? (future enhancement: prompt user to select which files to copy)

**Backwards Compatibility:**
- Ensure `codanna init` without flags behaves exactly as before
- No breaking changes to existing workflows

## Testing Strategy

### Unit Tests
```rust
#[test]
fn test_should_copy_file_includes_agents() { }

#[test]
fn test_should_copy_file_excludes_readme() { }

#[test]
fn test_find_claude_template_source_explicit_path() { }

#[test]
fn test_find_claude_template_source_global_fallback() { }

#[test]
fn test_copy_respects_existing_files() { }

#[test]
fn test_force_flag_overwrites_existing() { }
```

### Integration Tests
```rust
#[test]
fn test_init_with_claude_copy_full_workflow() {
    // Setup temp dirs
    // Create source .claude structure
    // Run init with --copy-claude-from
    // Verify files copied correctly
    // Verify settings.toml also created
}
```

### Manual Testing
1. Test with empty target directory
2. Test with existing `.codanna/` directory
3. Test with partial existing files (some exist, some don't)
4. Test with `--force` flag
5. Test with invalid source path
6. Test with missing source `.claude/` directory
7. Test without `--copy-claude-from` (should work like today)

## Documentation Updates

### README.md
- Add section on init command with claude config copying
- Explain template structure
- Show usage examples

### Help Text
```
Set up .codanna directory with default configuration

Usage: codanna init [OPTIONS]

Options:
  -c, --config <CONFIG>
          Path to custom settings.toml file

  -f, --force
          Force overwrite existing configuration

      --copy-claude
          Copy embedded .claude configuration (agents, commands, prompts, hooks)
          from codanna's built-in templates to your project's .codanna folder.
          These templates are embedded in the binary at compile time.
          Existing files are preserved unless --force is used.

      --copy-claude-from <PATH>
          Copy .claude configuration from a custom source directory.
          Use this to apply your own template instead of the built-in one.
          Cannot be used together with --copy-claude.

      --info
          Show detailed loading information

  -h, --help
          Print help
```

## Future Enhancements

1. **Interactive mode**: Prompt user which files to copy
2. **Dry-run mode**: Preview what would be copied with `--dry-run`
3. **Template management commands**:
   - `codanna template save` - Save current project's `.claude` as template
   - `codanna template list` - List available templates
   - `codanna template delete` - Remove a template
4. **Named templates**: Support multiple templates with names
   ```bash
   codanna init --template rust-mcp-server
   codanna init --template typescript-cli
   ```
5. **Remote templates**: Download templates from GitHub/URLs
6. **Merge strategies**: Smart merging of YAML/TOML files instead of skip/overwrite

## Success Criteria

- [ ] Can run `codanna init --copy-claude-from <path>` successfully
- [ ] Files are copied without overwriting existing files
- [ ] `--force` flag allows overwriting
- [ ] Clear status messages during copying
- [ ] All tests pass
- [ ] Documentation updated
- [ ] Backwards compatible (plain `codanna init` still works)
- [ ] Works cross-platform (Windows, macOS, Linux)

## Non-Goals

- Not implementing template management commands in this PR (future work)
- Not implementing remote template fetching (future work)
- Not implementing interactive selection UI (future work)
- Not implementing merge strategies for config files (future work)
