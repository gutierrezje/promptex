# PromptEx - Prompt Extraction for OSS Contributions

## The Problem

OSS maintainers reviewing AI-assisted PRs face a trust gap:
- "What prompts were used?"
- "Was this thoughtful or just slop?"
- "How did they arrive at this solution?"

## The Philosophy

**PromptEx (`pmtx`) extracts and curates AI prompts so maintainers can see your reasoning.**

It's not a raw chat dump. It's not prompt analytics.
It's a **Prompt History Tool**.

**Smart extraction workflow:**
```
WORK → EXTRACT → SHARE
```

1. **Work** - Use any AI coding tool (Claude Code, Cursor, etc.)
2. **Extract** - `pmtx extract` intelligently finds relevant prompts
3. **Share** - Paste PR-formatted output into GitHub PR description

Everything else is noise.

---

## Storage & Output

**Storage:** `~/.promptex/projects/<project-id>/`
- Continuous journal (append-only log)
- No pollution of project working directory
- No .gitignore needed

**Output:** PR-formatted markdown to stdout (default)
- Copy/paste into GitHub PR description
- Optional: `--write` flag saves to file
- Collapsible `<details>` section for maintainer convenience

**Key principle:** Show reasoning, not raw transcripts.

---

## Smart Extraction Logic

### How `pmtx extract` Works (No Args)

**Step 1: Determine scope (smart defaults)**
```
What branch am I on?
├─ Feature branch (not main/master/develop)
│   └─ Extract since branch creation
│       Files: All changes on this branch
│       Time: Since branch diverged from parent
│
└─ Main branch (or fork workflow)
    ├─ Uncommitted changes exist?
    │   └─ Extract for uncommitted + recent commits
    │       Interactive: "Include last N commits?"
    │
    └─ No uncommitted changes
        └─ Extract last commit (or prompt user)
            Interactive: "Extract last N commits?"
```

**User can override with explicit flags:**
- `--uncommitted` - Only uncommitted work
- `--commits N` - Last N commits
- `--since-commit HASH` - Since specific commit
- `--branch-lifetime` - Full branch history
- `--since "2 days ago"` - Time-based

**Step 2: Load journal from home directory**
```
~/.promptex/projects/<project-id>/journal.jsonl
```
Project ID determined by:
1. Git remote origin URL (preferred - stable across clones)
2. Git repo root path (fallback)

**Step 3: Filter journal by scope**
1. Match file paths (prompts that touched files in scope)
2. Match time window (prompts within relevant timeframe)
3. Match branch context (for metadata/context)

**Step 4: Curate prompts**
1. Keep only prompts with concrete artifacts (file edits, commands, tests)
2. Keep failed attempts if they changed direction
3. Drop repetitive prompts with no action
4. Already redacted during journaling (privacy-first)

**Step 5: Categorize & format**
- Investigation (understanding code)
- Solution (implementation)
- Testing (validation)

**Step 6: Output**
- Default: PR-formatted markdown to stdout (collapsible)
- With `--write`: Save to PROMPTS.md (or custom filename)

### Output Format

**Default (stdout, PR description format):**
````markdown
## 🤖 Prompt History

<details>
<summary>6 prompts over 1h 46m - Click to expand</summary>

**Session Details**
- Tools: Claude Code (claude-sonnet-4.5) - 5 prompts, Cursor (gpt-4) - 1 prompt
- Commits: `abc123`, `abc124` (2 commits)
- Branch: `feature/auth-fix`
- Time range: 2024-02-26 14:32 - 16:18
- Modified files: `src/auth.rs`, `tests/auth_test.rs`

---

### 🔍 Investigation

**[14:35] (Claude Code) Understanding JWT validation**
```
help me understand how the JWT validation works in auth.rs
```
→ Identified missing expiry check
→ Files: `src/auth.rs:45-67`
→ Confidence: high

**[14:42] (Claude Code) Exploring test coverage**
```
show me existing auth tests
```
→ Found `tests/auth_test.rs` but no expiry tests
→ Files: `tests/auth_test.rs`

### 🔧 Solution

**[15:10] (Claude Code) Implementing expiry check**
```
add JWT expiry validation to the verify_token function
```
→ Added expiry check with proper error handling
→ Files: `src/auth.rs:52-58`
→ Commit: `abc123`

**[15:25] (Cursor) Refining error messages**
```
make the error messages more specific for expired tokens
```
→ Updated error enum and messages
→ Files: `src/auth.rs:12-18`, `src/auth.rs:55`

### ✅ Testing

**[15:45] (Claude Code) Writing expiry tests**
```
write a test that verifies expired tokens are rejected
```
→ Added `test_expired_token_rejected`
→ Files: `tests/auth_test.rs:89-105`
→ Commit: `abc124`

---

**Summary:** 6 prompts (3 investigation, 2 solution, 1 testing) across 2 tools

</details>
````

**With `--write` flag (file output, more detailed):**
```markdown
# Prompt History

> **Generated:** 2024-02-26
> **Project:** myproject
> **Branch:** feature/auth-fix
> **Commits:** abc123, abc124
> **Time Range:** 14:32 - 16:18 (1h 46m)

## Extraction Details
- Tools: Claude Code (5 prompts), Cursor (1 prompt)
- Models: claude-sonnet-4.5, gpt-4
- Modified files: `src/auth.rs`, `tests/auth_test.rs`

## Session Overview
- Investigation: 2 prompts
- Solution: 2 prompts
- Testing: 2 prompts
- Total: 6 curated prompts

[... detailed format similar to PR format but expanded ...]
```

---

## CLI Commands

### `pmtx extract` (Smart Default - PR Format to Stdout)

Intelligently extracts prompts and outputs PR-ready markdown.

```bash
$ pmtx extract

🔍 Analyzing workspace...
  ✓ Branch: feature/auth (created Feb 20)
  ✓ Found 3 modified files in 2 commits
  ✓ Time range: 2024-02-26 14:32 - 16:18 (1h 46m)

🔎 Loading journal from ~/.promptex/...
  ✓ Found 18 prompts in time range
  ✓ Filtered to 6 relevant prompts

📝 Curating prompt log...
  Investigation: 2 prompts
  Solution: 2 prompts
  Testing: 2 prompts

## 🤖 AI Assistance Transparency

<details>
<summary>View Prompt History (6 prompts over 1h 46m)</summary>

[... PR-formatted markdown output ...]

</details>

💡 Copy the output above and paste into your PR description
```

**Smart defaults:**
- Feature branch → Extract since branch creation
- Main branch → Prompt for commit range or extract uncommitted
- Output to stdout (pipe to `gh pr create` or copy/paste)
- Already redacted (privacy-first journaling)

### `pmtx extract --write [FILE]`

Write to file instead of stdout.

```bash
# Write to PROMPTS.md
$ pmtx extract --write
✓ Written to PROMPTS.md

# Custom filename
$ pmtx extract -w analysis.md
✓ Written to analysis.md

# Project-specific location (if repo adopts this standard)
$ pmtx extract -w .github/prompts/pr-auth-fix.md
```

### `pmtx extract --commits <N>`

Extract for specific number of recent commits (useful for fork/main workflow).

```bash
# Last commit only (single PR from main)
$ pmtx extract --commits 1

# Last 3 commits
$ pmtx extract --commits 3

# Since specific commit
$ pmtx extract --since-commit abc123
```

### `pmtx extract --uncommitted`

Only extract prompts for uncommitted changes.

```bash
$ pmtx extract --uncommitted

🔍 Analyzing uncommitted changes...
  ✓ Found 2 modified files (1 staged, 1 unstaged)

🔎 Extracting related prompts...
  ✓ Found 8 prompts that touched these files

[... outputs PR-formatted markdown ...]
```

### `pmtx extract --interactive`

Interactive commit selection (for complex scenarios).

```bash
$ pmtx extract --interactive

Recent commits:
  [x] abc125 (2h ago) feat: add JWT expiry validation
  [x] abc124 (3h ago) test: add expiry tests
  [ ] abc123 (2 days ago) refactor: clean up auth module
  [ ] abc122 (3 days ago) fix: unrelated bug

Select commits to include: [space to toggle, enter to confirm]

✓ Extracting prompts for 2 selected commits
```

### `pmtx record` (Agent-invoked, automatic)

Journal a prompt entry (called automatically by agent skill).

```bash
# Agent calls this after tool use
$ pmtx record --prompt "add auth validation" \
              --files "src/auth.rs" \
              --tool-calls "Edit,Bash" \
              --outcome "Added expiry check"

✓ Journaled to ~/.promptex/projects/<project-id>/journal.jsonl
```

### `pmtx status`

Show current project journal status.

```bash
$ pmtx status

Project: myproject (github.com/user/myproject)
Branch: feature/auth
Journal: ~/.promptex/projects/github-com-user-myproject-abc123/

Prompts logged: 18
  - feature/auth: 12 prompts
  - main: 6 prompts

Last journal entry: 5 minutes ago
Last extraction: 2 hours ago (6 prompts extracted)
```

### `pmtx projects`

Manage tracked projects.

```bash
# List all projects
$ pmtx projects list

myproject (github.com/user/myproject)
  Path: /Users/dev/myproject
  Prompts: 18
  Last accessed: 5 minutes ago

another-project (path-based)
  Path: /Users/dev/sandbox/test
  Prompts: 5
  Last accessed: 3 days ago

# Clean old projects
$ pmtx projects clean --older-than 30d

# Remove specific project
$ pmtx projects remove <project-id>
```

## Why This Architecture Wins

1. **Zero Config** - Smart defaults mean `pmpx extract` just works
2. **Git-Aware** - Correlates prompts with actual code changes
3. **Tool Agnostic** - Works with Claude Code, Cursor, Copilot, whatever you use
4. **Privacy-First** - Auto-detects sensitive values, prompts for review
5. **Maintainer-Friendly** - PROMPTS.md shows reasoning, not raw chat logs
6. **Single File Output** - Just commit PROMPTS.md with your PR
7. **Fast** - Local extraction, no API calls, Rust performance

---

## What NOT to Build

- Prompt analytics or dashboards (just extraction)
- Real-time session monitoring (post-hoc analysis only)
- Prompt optimization or suggestions (maintainer trust focus)
- Multi-user collaboration features (single-user workflow)
- Cloud sync or storage (local-only)
- Automatic PR creation (just generate PROMPTS.md)
- Raw chat dumps without curation
- Deep semantic analysis of prompts (keep it simple)

---

## Future Extractor Support

Active extractors: **Claude Code** ✅ and **Codex CLI/Desktop** ✅.

The following are planned but not yet implemented:

| Tool | Blocker / Notes |
|------|-----------------|
| **OpenCode** (sst/opencode) | Migrated to SQLite (v1.2+). Needs query against `~/.local/share/opencode/opencode.db`. MessageV2 schema: `MessageTable` + `PartTable` (parts have `type: "tool"` not `"tool-invocation"`). Legacy JSON file extractor exists in `src/extractors/opencode.rs` but is disabled. |
| **Cursor** | Log format and storage path TBD — needs investigation. |
| **GitHub Copilot** | Log format and storage path TBD — needs investigation. |

---

## Tech Stack

### CLI Tool (Rust)
| Component | Choice | Reason |
|-----------|--------|--------|
| Language | Rust | Single binary, fast startup, impressive for portfolio |
| CLI Framework | clap | Industry standard, derive macros |
| HTTP Client | reqwest | Async, robust |
| GitHub API | octocrab | Type-safe GitHub API client |
| Async Runtime | tokio | De facto standard |
| Serialization | serde + serde_json | JSON/TOML parsing |
| Templating | tera | Jinja2-like syntax |
| Terminal UI | indicatif + console | Progress bars, colors, styling |
| Config | TOML (~/.issuance/config.toml) | Standard, editable |
| Output | Markdown files | Human-readable, AI-consumable |

### Why Rust Over Python
- **Single binary distribution** - No runtime deps, just download and run
- **~5ms startup** vs ~200ms+ for Python - matters for CLI tools
- **Interview story** - "I built a Rust CLI" >> "I built a Python CLI"
- **Type safety** - Catches bugs at compile time
- **Learning opportunity** - Great project scope for Rust

---

## Supported OS

- **macOS**: First-class support (explicit paths and tooling)
- **Linux**: Best-effort support
- **Windows**: Not supported initially (path + tooling differences)

---

## Project Structure

### Source Code
```
promptex/
├── Cargo.toml              # Dependencies + metadata
├── src/
│   ├── main.rs             # Entry point, clap CLI setup
│   ├── config.rs           # Config loading (~/.promptex/config.toml)
│   ├── project_id.rs       # Project identification (git remote, path hash)
│   ├── commands/
│   │   ├── mod.rs
│   │   ├── extract.rs      # pmtx extract (smart scoping + output)
│   │   ├── record.rs       # pmtx record (agent-invoked journaling)
│   │   ├── status.rs       # pmtx status (project journal info)
│   │   └── projects.rs     # pmtx projects (manage tracked projects)
│   ├── journal/
│   │   ├── mod.rs
│   │   ├── entry.rs        # JournalEntry struct
│   │   ├── writer.rs       # Append to journal.jsonl
│   │   └── reader.rs       # Load and filter journal
│   ├── extractors/
│   │   ├── mod.rs          # detect() — picks extractor based on tool in use
│   │   ├── traits.rs       # PromptExtractor trait
│   │   ├── claude_code.rs  # ~/.claude/projects/{slug}/*.jsonl
│   │   ├── opencode.rs     # ~/.local/share/opencode/storage/
│   │   ├── codex.rs        # ~/.codex/sessions/YYYY/MM/DD/*.jsonl
│   │   └── manual.rs       # fallback: journal.jsonl written by pmtx record
│   ├── analysis/
│   │   ├── mod.rs
│   │   ├── git.rs          # Git operations (status, diff, log, branch info)
│   │   ├── scope.rs        # Determine extraction scope (commits, files, time)
│   │   └── correlation.rs  # Match prompts to files/commits
│   ├── curation/
│   │   ├── mod.rs
│   │   ├── filter.rs       # Remove low-value prompts
│   │   ├── categorize.rs   # Investigation/Solution/Testing
│   │   └── redact.rs       # Sensitive value detection (for journaling)
│   └── output/
│       ├── mod.rs
│       ├── pr_format.rs    # Collapsible PR description format (default)
│       └── detailed.rs     # Detailed file format (--write)
├── README.md
└── tests/
    ├── git_test.rs
    ├── scope_test.rs
    ├── journal_test.rs
    └── curation_test.rs
```

### User Data (Home Directory)
```
~/.promptex/
├── config.toml                              # Global config
├── projects/
│   ├── github-com-user-myproject-abc123/    # Project-specific state
│   │   ├── journal.jsonl                    # Continuous append-only log
│   │   └── metadata.json                    # Project info, branch tracking
│   └── github-com-org-another-def456/
│       ├── journal.jsonl
│       └── metadata.json
└── cache/                                   # (Future: parsed log cache)
```

**No database. No server. No paid API calls. No project directory pollution.**
Output is PR-formatted markdown (stdout) or optional file via `--write`.

---

## Implementation Phases

### Phase 1: CLI Scaffold ✅

**Goal:** Basic CLI structure with command parsing

**Deliverable:**
```bash
$ pmtx --help
Commands:
  extract    Extract prompts (smart defaults, PR format)
  record     Journal a prompt entry (agent-invoked)
  status     Show project journal status
  projects   Manage tracked projects
```

**Status:** Can reuse existing clap setup, update command names

---

### Phase 2: Project ID & Home Directory Storage ✅

**Goal:** Identify projects and set up home directory storage structure

**Files:**
```
src/
├── project_id.rs       # Project identification logic
└── journal/
    ├── writer.rs       # Append to journal.jsonl
    └── reader.rs       # Load journal entries
```

**Key functions:**
```rust
// project_id.rs
pub fn get_project_id(cwd: &Path) -> Result<String> {
    // 1. Try git remote origin → hash URL
    // 2. Fallback: git root path → hash path
    // Returns: "github-com-user-repo-abc123"
}

pub fn get_project_dir(project_id: &str) -> PathBuf {
    home_dir().join(".promptex/projects").join(project_id)
}

pub fn ensure_project_dir(project_id: &str) -> Result<PathBuf>

// journal/writer.rs
pub fn append_entry(project_id: &str, entry: &JournalEntry) -> Result<()>

// journal/reader.rs
pub fn load_journal(project_id: &str) -> Result<Vec<JournalEntry>>
```

**Deliverable:**
- `~/.promptex/projects/<id>/` directory creation
- `journal.jsonl` append working
- `metadata.json` tracking

---

### Phase 3: Git Analysis & Smart Scoping ✅

**Goal:** Determine what to extract based on git state

**Files:**
```
src/analysis/
├── git.rs          # Git operations (status, diff, log, branch info)
└── scope.rs        # Determine extraction scope
```

**Key functions:**
```rust
// git.rs
pub fn current_branch() -> Result<String>
pub fn is_mainline_branch(branch: &str) -> bool  // main/master/develop
pub fn branch_creation_time(branch: &str) -> Result<DateTime>
pub fn parent_branch(branch: &str) -> Result<String>  // Where branch diverged
pub fn modified_files() -> Result<Vec<PathBuf>>  // git status + diff
pub fn commits_in_range(range: &str) -> Result<Vec<Commit>>
pub fn files_in_commits(commits: &[Commit]) -> Vec<PathBuf>

// scope.rs
pub enum ExtractionScope {
    BranchLifetime(String),      // Feature branch: since creation
    Commits(usize),              // Last N commits
    CommitRange(String, String), // Specific range
    Uncommitted,                 // Only uncommitted changes
    Interactive,                 // User selects commits
}

pub fn determine_smart_scope() -> Result<ExtractionScope> {
    // 1. Feature branch? → BranchLifetime
    // 2. Main with uncommitted? → Prompt user
    // 3. Main without uncommitted? → Commits(1) or Interactive
}
```

**Deliverable:** Smart scope detection working for all workflows

---

### Phase 4: Journaling (pmtx record + redaction) ✅

**Goal:** Record prompts to journal with redaction

**Files:**
```
src/
├── commands/record.rs     # pmtx record command
└── curation/redact.rs     # Immediate redaction during journaling
```

**Journal entry structure:**
```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct JournalEntry {
    pub timestamp: DateTime<Utc>,
    pub branch: String,
    pub commit: String,
    pub prompt: String,           // Already redacted
    pub files_touched: Vec<String>,
    pub tool_calls: Vec<String>,  // "Edit", "Bash", "Read"
    pub outcome: String,
    pub tool: String,             // "claude-code", "cursor"
    pub model: Option<String>,
}
```

**Redaction (privacy-first):**
```rust
// redact.rs
pub fn redact_sensitive_values(text: &str) -> (String, Vec<Redaction>)
// Detects and redacts:
// - API keys (patterns: sk-*, ghp_*, etc.)
// - Tokens (JWT patterns)
// - Credentials (password=*, auth=*)
// - Email addresses (optional)
// - IP addresses (optional)

pub fn record_prompt(prompt: &str, context: &Context) -> Result<()> {
    let (redacted, redactions) = redact_sensitive_values(prompt);

    if !redactions.is_empty() {
        warn!("🔒 Redacted {} sensitive values", redactions.len());
    }

    let entry = JournalEntry {
        prompt: redacted,  // Safe by default
        // ... other fields
    };

    append_to_journal(project_id, entry)
}
```

**Deliverable:** `pmtx record` working, privacy-safe journaling

---

### Phase 5: Log Extraction (Primary Data Source) ✅

**Goal:** Read prompts directly from AI tool session logs — zero token overhead,
no agent cooperation required. `pmtx record` becomes a fallback only.
Built before correlation because correlation needs extracted entries to filter.

**Log locations (all JSONL):**
| Tool | Path |
|------|------|
| Claude Code | `~/.claude/projects/{slug}/*.jsonl` |
| OpenCode | `~/.local/share/opencode/storage/message/` |
| Codex CLI | `~/.codex/sessions/YYYY/MM/DD/rollout-*.jsonl` |
| Manual fallback | `~/.promptex/projects/{id}/journal.jsonl` |

**Files:**
```
src/extractors/
├── mod.rs          # detect() — auto-selects extractor from available logs
├── traits.rs       # PromptExtractor trait
├── claude_code.rs  # Parse Claude Code JSONL sessions
├── opencode.rs     # Parse OpenCode message storage
├── codex.rs        # Parse Codex CLI session rollouts
└── manual.rs       # Reads pmtx record journal.jsonl (fallback)
```

**Detection logic (in order):**
1. Claude Code logs present for this project → `ClaudeCodeExtractor`
2. OpenCode logs present → `OpenCodeExtractor`
3. Codex logs present → `CodexExtractor`
4. None found → `ManualExtractor` (reads `pmtx record` journal)

**`pmtx record` in a log-available context:**
When logs are detected, `pmtx record` writes a lightweight timestamp-only
anchor entry. The extractor uses it to narrow the log window rather than
re-reading the entire session history.

**PromptExtractor trait:**
```rust
pub trait PromptExtractor {
    /// True if this tool's logs exist for the current project.
    fn is_available(project_root: &Path) -> bool;
    /// Extract entries within the given time window.
    fn extract(&self, since: DateTime<Utc>, until: DateTime<Utc>) -> Result<Vec<JournalEntry>>;
}
```

**Deliverable:** `pmtx extract` pulls real prompt data from whichever tool the
user is running, with no setup or token cost.

---

### Phase 6: Correlation & Filtering

**Goal:** Match extracted entries to the git scope (files, commits, time window)

**Files:**
```
src/analysis/correlation.rs
```

**Key functions:**
```rust
pub fn filter_by_scope(
    entries: &[JournalEntry],
    scope: &ExtractionScope,
    git_ctx: &GitContext,
) -> Vec<JournalEntry> {
    // 1. Resolve files in scope (from commits or git status)
    // 2. Resolve time range for scope
    // 3. Keep entries that touched files in scope OR fall within time range
}

pub fn has_artifact(entry: &JournalEntry) -> bool {
    // Must have file edits, commands, or meaningful investigation
    !entry.tool_calls.is_empty() || !entry.files_touched.is_empty()
}
```

**Deliverable:** Filtered, scoped prompt list ready for curation

---

### Phase 7: Curation & Categorization

**Goal:** Categorize and deduplicate the correlated prompts

**Files:**
```
src/curation/
├── filter.rs       # Remove repetitive/low-value prompts
└── categorize.rs   # Investigation / Solution / Testing
```

**Categorization:**
```rust
pub enum Intent {
    Investigation,  // Reading code, understanding
    Solution,       // Writing code, implementing
    Testing,        // Running tests, validation
}

pub fn categorize(entry: &JournalEntry) -> Intent {
    // Heuristics:
    // - "understand", "explain", "show me" → Investigation
    // - "implement", "add", "fix", "write" → Solution
    // - "test", "verify", "check" → Testing
    // - Tool calls: mostly Read → Investigation
    // - Tool calls: Edit/Write → Solution
    // - Tool calls: Bash (test commands) → Testing
}

pub fn remove_duplicates(entries: Vec<JournalEntry>) -> Vec<JournalEntry> {
    // Remove near-identical prompts (typo corrections, minor rephrases)
}
```

**Deliverable:** Categorized, deduplicated prompts

---

### Phase 8: Output Generation

**Goal:** Generate PR format (stdout) and detailed format (--write)

**Files:**
```
src/output/
├── pr_format.rs    # Collapsible PR description (default)
└── detailed.rs     # Full PROMPTS.md format (--write)
```

**PR format (stdout):**
```rust
pub fn generate_pr_format(prompts: &[CuratedPrompt], ctx: &ExtractionContext) -> String {
    // ## 🤖 AI Assistance Transparency
    // <details>
    // <summary>View Prompt History (N prompts)</summary>
    // ... categorized prompts ...
    // </details>
}
```

**Detailed format (--write):**
```rust
pub fn generate_detailed_format(prompts: &[CuratedPrompt], ctx: &ExtractionContext) -> String {
    // More verbose, includes all metadata
}
```

**Deliverable:** `pmtx extract` outputs PR-ready markdown

---

### Phase 9: Polish & Commands

**Additional commands:**
```
pmtx status      # Show journal stats for current project
pmtx projects    # List/manage tracked projects
```

**Additional `pmtx extract` flags:**
- `--since <DURATION>` — time-based scope (e.g. `2h`, `1d`, `3d`, `1w`). More ergonomic than `--commits N` for straight-to-main workflows where users think in time rather than commit count. Parses to a `DateTime` cutoff and filters commits by author timestamp.
- `--interactive` — commit picker UI for selecting exactly which commits to include

**Rich CLI output (indicatif + console):**
- Progress spinners for extraction
- Colored category headers (Investigation/Solution/Testing)
- File/commit correlation display
- Clear redaction warnings during journaling

**Config (`~/.promptex/config.toml`):**
```toml
[project]
identify_by = "git_remote"    # or "git_root" for separate journals per clone

[journaling]
auto_redact = true            # Redact sensitive values on journal write
warn_on_redaction = true      # Show warning when redacting

[extraction]
default_format = "pr"         # or "detailed"
interactive_on_main = true    # Prompt for commit range when on main branch

[output]
include_tool_metadata = true  # Show tool/model in output
include_timestamps = true     # Show prompt timestamps
```

**Tests:**
- Unit tests for git analysis (scope detection, branch operations)
- Unit tests for journaling (append, redaction)
- Unit tests for correlation (file matching, time filtering)
- Unit tests for curation (categorization, deduplication)
- Integration test with mock journal
- Redaction detection tests

**Build & Distribute:**
```bash
cargo build --release
# Binary at target/release/pmtx (~3MB)
cargo install --path .
```

---

## Interactive Output (UX Enhancement)

When `pmtx extract` outputs to a terminal (not piped), show interactive prompt:

### UX Flow

```bash
$ pmtx extract

🔍 Analyzing workspace...
  ✓ Branch: feature/auth
  ✓ Found 6 prompts in 2 commits

## 🤖 Prompt History

<details>
<summary>6 prompts over 1h 46m - Click to expand</summary>

[... full PR-formatted markdown ...]

</details>

┌─────────────────────────────────────────┐
│  'c' - Copy to clipboard                │
│  'w' - Write to PROMPTS.md              │
│  Enter/other key - Exit                 │
└─────────────────────────────────────────┘
```

**When piping (non-TTY):**
```bash
$ pmtx extract | gh pr create --body-file -
# No interactive prompt - just outputs markdown directly
```

### Implementation

**Dependencies:**
```toml
arboard = "3.3"        # Cross-platform clipboard
crossterm = "0.27"     # Terminal interaction
```

**Key functions:**
```rust
// src/output/interactive.rs
pub fn output_with_interactive(markdown: &str) -> Result<()> {
    // Print markdown
    println!("{}", markdown);

    // Only interactive if stdout is TTY
    if !io::stdout().is_terminal() {
        return Ok(());
    }

    // Show prompt and handle keypresses
    show_interactive_prompt(markdown)
}

fn show_interactive_prompt(markdown: &str) -> Result<()> {
    println!("\n┌─────────────────────────────────────────┐");
    println!("│  'c' - Copy to clipboard                │");
    println!("│  'w' - Write to PROMPTS.md              │");
    println!("│  Enter/other key - Exit                 │");
    println!("└─────────────────────────────────────────┘");

    terminal::enable_raw_mode()?;

    let result = match event::read()? {
        Event::Key(KeyEvent { code: KeyCode::Char('c'), .. }) => {
            copy_to_clipboard(markdown)?;
            println!("\r✓ Copied to clipboard!");
            Ok(())
        }
        Event::Key(KeyEvent { code: KeyCode::Char('w'), .. }) => {
            write_to_file(markdown, "PROMPTS.md")?;
            println!("\r✓ Written to PROMPTS.md");
            Ok(())
        }
        _ => Ok(()),
    };

    terminal::disable_raw_mode()?;
    result
}

fn copy_to_clipboard(text: &str) -> Result<()> {
    let mut clipboard = Clipboard::new()?;
    clipboard.set_text(text)?;
    Ok(())
}
```

**Graceful degradation (Linux without clipboard):**
```rust
fn clipboard_available() -> bool {
    Clipboard::new().is_ok()
}

// If clipboard not available, only show 'w' option
```

---

## Timeline

| Day | Phase | Deliverable |
|-----|-------|-------------|
| 1 | Scaffold & Project ID | ✅ CLI structure, home dir storage |
| 1-2 | Git Analysis & Scoping | Smart scope detection (branch/commits) |
| 2-3 | Journaling & Redaction | `pmtx record`, privacy-safe journaling |
| 3-4 | Correlation & Filtering | Match journal to scope, filter by files |
| 4-5 | Curation & Categorization | Dedupe, categorize, enrich prompts |
| 5-6 | Output Generation | PR format (stdout) + detailed (--write) |
| 6-7 | Agent Integration | Skill definition, auto-journaling docs |
| 7-8 | Polish & Commands | `status`, `projects`, interactive output (clipboard), config, tests |

**Total: ~8 days** (foundational + agent integration + polish)

**Note:** Claude Code log parsing skipped initially - agent integration via `pmtx record` is primary path. Can add log parsing later for retroactive extraction.

---

## Verification

### Build & Install
```bash
cd promptex
cargo build --release
cargo install --path .

# Verify installation
pmtx --help
```

### Full Workflow: Feature Branch

```bash
# Create feature branch
cd ~/myproject
git checkout -b feature/auth-fix

# Work with agent (agent automatically journals via pmtx record)
# Agent: [Invokes pmtx record after each significant action]

# Check journal status
pmtx status
# Project: myproject (github.com/user/myproject)
# Branch: feature/auth-fix
# Prompts logged: 8
# Last journal entry: 2 minutes ago

# Extract prompts (smart default: branch lifetime)
pmtx extract
# ✓ Branch: feature/auth-fix (created Feb 20)
# ✓ Found 8 prompts in 2 commits
# ✓ Curated to 6 prompts
#
# ## 🤖 Prompt History
# <details>
# <summary>6 prompts over 1h 46m - Click to expand</summary>
# [... PR-formatted markdown ...]
# </details>
#
# ┌─────────────────────────────────────────┐
# │  'c' - Copy to clipboard                │
# │  'w' - Write to PROMPTS.md              │
# │  Enter/other - Exit                     │
# └─────────────────────────────────────────┘

[User presses 'c']
✓ Copied to clipboard!

# Create PR (paste from clipboard)
gh pr create --title "Fix auth validation"
# (Paste into PR description field)

# Or write to file if repo wants it
pmtx extract -w .github/prompts/pr-auth-fix.md
git add .github/prompts/pr-auth-fix.md
git commit -m "feat: auth fix with prompt log"
```

### Full Workflow: Fork/Main Branch

```bash
# Fork workflow (working on main)
cd ~/fork-of-upstream
git checkout main

# Work with agent
# Agent: [Journals prompts automatically]

# Make commits
git commit -m "fix: auth bug"
git push origin main

# Extract for single commit (interactive on main)
pmtx extract
# ⚠ Working on main branch
# How many commits to extract?
#   [1] Last commit only (recommended)
#   [2] Last 3 commits
#   [3] Uncommitted only
# Choice: 1
#
# ✓ Extracting prompts for commit abc123
# [... PR-formatted output ...]

# Or explicit
pmtx extract --commits 1

# Create PR to upstream
gh pr create --repo upstream/repo \
  --base main \
  --title "Fix auth bug" \
  --body "$(pmtx extract --commits 1)"
```

### Workflow: Multiple PRs from Same Project

```bash
# PR #1
git checkout -b feature/auth
# ... work ...
pmtx extract > /tmp/pr1.txt
gh pr create --body-file /tmp/pr1.txt

# PR #2 (concurrent)
git checkout main
git checkout -b feature/logging
# ... work ...
pmtx extract > /tmp/pr2.txt
gh pr create --body-file /tmp/pr2.txt

# Each extraction automatically isolated by branch!
```

### Check Project Status

```bash
# See all tracked projects
pmtx projects list
# myproject (github.com/user/myproject)
#   Path: /Users/dev/myproject
#   Prompts: 18 across 2 branches
#   Last accessed: 5 minutes ago

# Clean old projects
pmtx projects clean --older-than 30d
```

### Manual Journaling (Without Agent Integration)

```bash
# If agent doesn't auto-journal, you can manually record
pmtx record \
  --prompt "implement JWT expiry validation" \
  --files "src/auth.rs" \
  --tool-calls "Edit,Bash" \
  --outcome "Added expiry check and tests"

# Then extract as normal
pmtx extract
```
