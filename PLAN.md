# Issuance - Context Orchestrator

## The Problem

The biggest friction isn't finding an issue; it's the **2 hours you spend orienting yourself**:
- "How do I build this?"
- "Where is the relevant code?"
- "What is the maintainer's vibe?"

## The Philosophy

**Issuance curates, validates, and stages context so AI tools can do their best work.**

It's not a static analysis engine. It's not an AI agent replacement.
It's a **Context Orchestrator**.

**3-Stage Pipeline:**
```
DISCOVER → STAGE → HANDOFF
```

1. **Discover** - Find what's worth working on
2. **Stage** - Prepare high-signal context
3. **Handoff** - Deliver to AI tools, editors, humans

Everything else is noise.

---

## The Context Pack (v2)

```
.issuance/
├── ISSUE.md      # Ground truth (GitHub API, no interpretation)
├── CODEMAP.md    # Lightweight, tool-assisted file mapping
├── SIGNALS.md    # Focused ambient signals (recent activity, CI config, TODO/FIXME)
├── RULES.md      # Contribution rules (synthesized via local agentic CLI)
├── HANDOFF.md    # AI tool entry point (short, actionable)
├── PROMPTS.md    # Your AI-assisted investigation journey (optional)
└── metadata.json # Session metadata (timestamp, issue info)
```

**Key principle:** Assemble evidence, don't derive understanding.

---

## The Context Pack Files

### 1. ISSUE.md - Ground Truth

**Source:** GitHub API only. No interpretation.

```markdown
# Issue #1284: Async Session Race Condition

**Repository:** fastapi/fastapi
**Author:** @user123
**Created:** 2024-01-15
**Labels:** bug, async, needs-triage
**Milestone:** v0.110.0
**Linked PRs:** None

## Description
[Raw text of the issue, unedited...]

## Comments (12 total)

### @tiangolo (Maintainer) - 2024-01-16
I think this is related to the `dependency_overrides` logic in `dependants.py`.

### @user123 - 2024-01-16
Here is a traceback:
```
Traceback (most recent call last):
  File "fastapi/dependencies/utils.py", line 234
  ...
```

### @contributor - 2024-01-17
I can confirm this on Python 3.11 with uvicorn 0.25.0
```

**Reinforces the "don't editorialize" principle.**

### 2. CODEMAP.md - Tool-Assisted File Mapping

**How it's generated:**
1. Extract keywords from issue text (filenames, symbols)
2. Use local code search to expand likely files and symbols:
   - `rg -n "<keyword>"` in the repo
   - `rg --files` + simple substring match
3. Run existing project tools if present:
   - `tsc --noEmit --pretty false`
   - `ruff check --statistics`
   - `go list ./...`
   - `cargo metadata`
4. Capture file paths, module boundaries, public signatures (only if cheap)

```markdown
# Code Map for Issue #1284

## Suspected Files
- src/dependencies/utils.py (mentioned in traceback)
- src/dependencies/dependants.py (mentioned by maintainer)
- src/routing.py (imports dependants)

## Related Modules
- fastapi.dependencies
- fastapi.routing

## Existing Tool Signals
- TypeScript errors: none
- Lint warnings in utils.py: 2 (unused import, line too long)
- Test coverage: tests/test_dependencies.py exists
```

**Key insight:** Surfacing where to look, not what to think.

### 3. SIGNALS.md - Ambient Context (Focused)

Issuance collects a small, high-signal set of ambient context that should change your next
action within 5 minutes of reading the issue. If it doesn't, it doesn't belong here.

```markdown
# Signals for Issue #1284

## Recent Activity
- 2024-02-01: "fix: handle None in dependency resolution" (a1b2c3d)

## CI Config
- Workflows: `ci.yml`, `lint.yml`

## Code Health (Optional)
- TODO/FIXME count in utils.py: 2
```

**Constraint:** Keep `SIGNALS.md` to 4–6 bullets total. If a signal doesn't change the immediate
next action, drop it.

**Human-grade context that AI tools reason over well.**

### 4. RULES.md - Contribution Rules

Generated from deterministic sources (CONTRIBUTING.md, CI configs, repo conventions) and
synthesized by a local agentic CLI. No new facts should be introduced.

```markdown
# Contribution Rules for fastapi/fastapi

## Commit Convention
**Conventional Commits required.** Use `feat:`, `fix:`, `docs:` prefixes.

## Testing
**Required.** See CONTRIBUTING.md.
Run: `pytest tests/test_dependencies.py -v`

## Style
- Formatter: `black`
- Linter: `ruff`
- Run: `black . && ruff check .`

## Review Process
See CONTRIBUTING.md and CODEOWNERS for review expectations.

## Don'ts
- Don't modify `pyproject.toml` without asking
- Don't import from `starlette` directly (use `fastapi.` wrappers)
- Don't squash commits yourself (maintainers do this)
```

### 5. HANDOFF.md - The AI Tool Entry Point (SECRET WEAPON)

**Short. Very short.** This is the alignment layer.

```markdown
You are working in the repository `fastapi/fastapi`.

Your goal: Fix the issue described in ISSUE.md.

Constraints:
- Follow RULES.md
- Do not invent file paths
- Ask for clarification only after reviewing CODEMAP.md

Suggested approach:
1. Inspect utils.py and dependants.py (see CODEMAP.md)
2. Review SIGNALS.md for recent changes
3. Run existing tests before making changes
4. Add tests if behavior is unclear
```

**This is the alignment layer. The tool already knows where to look.**

---

## CLI Commands

### `issuance grab <url>`
Fetches the issue and generates the full context pack.

```bash
$ issuance grab https://github.com/fastapi/fastapi/issues/1284

✓ Cloning fastapi/fastapi (shallow)
✓ Fetching issue #1284 (12 comments)
✓ Running language tools...
  ✓ ruff check --statistics
  ✓ pytest --collect-only
✓ Extracting signals (recent commits, CI config, TODO/FIXME)
✓ Synthesizing RULES.md via Claude Code
✓ Generating context pack

📁 .issuance/ ready

Files created:
  ISSUE.md      (ground truth)
  CODEMAP.md    (suspected files + tool output)
  SIGNALS.md    (recent commits, CI config, TODO/FIXME)
  RULES.md      (contribution rules)
  HANDOFF.md    (AI tool entry point)

Next: Open your AI tool and say "Fix the issue in .issuance/HANDOFF.md"
```

**No paid API calls.** By default, `grab` analyzes `CONTRIBUTING.md`, CI config, and repo
conventions, then invokes a local agentic CLI (Claude Code) to synthesize `RULES.md`.

### `issuance clean`
Wipes the context folder.

```bash
$ issuance clean
✓ Removed .issuance/
```

### `issuance prompts extract`
Extracts prompts from AI tool conversation logs (time-filtered).

```bash
$ issuance prompts extract

🔍 Extracting prompts since 2024-02-03 16:30

Found conversations:
  ✓ Claude Code: 8 prompts (2 hours ago)

📝 Total: 8 prompts

Investigation:
  [16:31] (claude-code) "help me understand this race condition"
  [16:45] (claude-code) "show me the traceback analysis"

Solution:
  [17:02] (claude-code) "write a fix for cleanup on reused fibers"

Testing:
  [17:30] (claude-code) "generate regression tests"

Include these in PROMPTS.md? [y/N/edit] y
✓ Saved to .issuance/PROMPTS.md

💡 Tip: Review PROMPTS.md and redact any sensitive information before committing
```

**How it works:**
1. Reads `started_at` timestamp from `.issuance/metadata.json`
2. Detects which AI tools were used (Claude Code, Codex CLI, OpenCode)
3. Parses conversation logs from each tool
4. Filters to prompts after the start timestamp
5. Categorizes prompts (Investigation, Solution, Testing)
6. Presents for user review/approval
7. Generates `.issuance/PROMPTS.md`

## Why This Architecture Wins

1. **Zero Marginal Cost** - GitHub API (issues only) + local tools + local agentic CLI
2. **Deterministic Core** - Base inputs are reproducible; synthesized output may vary
3. **Debuggable** - You can read and edit every file
4. **Model Agnostic** - Works with Cursor, Claude Code, Copilot, whatever wins next week
5. **Uses the Ecosystem** - Runs `ruff`, `tsc`, `pytest` instead of reinventing them
6. **Composable** - Each file is standalone, use what you need

---

## What NOT to Build

- Custom AST walkers (use existing language tools)
- Deep call graph logic (AI tools handle this)
- Cross-language parsing (out of scope)
- MCP server (not a conversational assistant)
- Per-call API payments (use subscription-based local tools)
- Global repo index (issue-scoped only)
- Anything an agent can get faster with `rg`, `git log`, or filesystem inspection

---

## Local Agentic Synthesis (Default)

Issuance is designed for local agentic CLIs by default. After generating deterministic facts,
`grab` invokes Claude Code (via `claude` CLI) to synthesize `RULES.md`.

**How it works:**
1. `issuance` generates base files deterministically (GitHub API + language tools)
2. Invokes `claude` CLI with a specific prompt to synthesize `RULES.md`
3. If `claude` is not available, falls back to deterministic output

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

### Recommended Tools
- `rust-analyzer` (editor integration)

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

```
issuance/
├── Cargo.toml              # Dependencies + metadata
├── src/
│   ├── main.rs             # Entry point, clap CLI setup
│   ├── config.rs           # Config loading (~/.issuance/config.toml)
│   ├── commands/
│   │   ├── mod.rs
│   │   ├── grab.rs         # issuance grab <url>
│   │   └── clean.rs        # issuance clean
│   ├── services/
│   │   ├── mod.rs
│   │   ├── github.rs       # GitHub API (issues, commits, CI)
│   │   ├── agentic.rs      # Local agentic CLI integration (Claude Code)
│   │   ├── tools.rs        # Run language-native tools (ruff, tsc)
│   │   ├── extractor.rs    # Keyword/file extraction (no model use)
│   │   └── generator.rs    # Render templates → context files
│   └── templates/
│       ├── mod.rs          # Template embedding
│       ├── issue.md.tera
│       ├── codemap.md.tera
│       ├── signals.md.tera
│       ├── rules.md.tera
│       └── handoff.md.tera
├── README.md
└── tests/
    ├── extractor_test.rs
    └── generator_test.rs
```

**No database. No server. No paid API calls.**
The `.issuance/` folder IS the output.

---

## Implementation Phases

### Phase 1: CLI Scaffold (✅ COMPLETE)

**Goal:** Basic CLI structure, `issuance --help` works

**Deliverable:**
```bash
$ issuance --help
Commands:
  grab     Fetch an issue and generate context pack
  clean    Remove .issuance/ folder
```

**Status:** Complete. All commands parse correctly, clean command fully functional.

**Implementation notes (current):**
- Clap-based CLI is wired up with `grab` and `clean`
- Config loading from `~/.issuance/config.toml` is implemented
- `clean` command is implemented end-to-end

---

### Phase 2: `issuance grab` - Core Pipeline (IN PROGRESS)

**Goal:** Full context pack generation, including RULES.md synthesis via local agentic CLI

**Files:**
```
src/
├── commands/grab.rs
├── services/
│   ├── github.rs       # Issue + comments + signals
│   ├── agentic.rs      # Local agentic CLI integration (Claude Code)
│   ├── tools.rs        # Run language-native tools
│   ├── extractor.rs    # Keywords, file mentions
│   └── generator.rs    # Generate all context files
└── templates/
    ├── issue.md.tera
    ├── codemap.md.tera
    ├── signals.md.tera
    ├── rules.md.tera
    └── handoff.md.tera
```

**`services/github.rs`:**
```rust
pub async fn fetch_issue(owner: &str, repo: &str, issue_num: u64) -> Result<Issue>
pub async fn fetch_comments(owner: &str, repo: &str, issue_num: u64) -> Result<Vec<Comment>>
pub fn clone_repo(owner: &str, repo: &str, shallow: bool) -> Result<PathBuf>
```

**`services/tools.rs`:**
```rust
pub fn detect_project_type(repo_path: &Path) -> ProjectType  // Python, TypeScript, Go, Rust
pub fn run_linter(repo_path: &Path, project_type: ProjectType) -> Result<LintOutput>
pub fn run_test_discovery(repo_path: &Path, project_type: ProjectType) -> Result<Vec<String>>
pub fn collect_ci_config(repo_path: &Path) -> Result<Vec<String>>
pub fn collect_recent_commits(repo_path: &Path, paths: &[String]) -> Result<Vec<Commit>>
pub fn collect_todo_fixme(repo_path: &Path, paths: &[String]) -> Result<Vec<String>>
```

**`services/extractor.rs`:**
```rust
pub fn extract_keywords(text: &str) -> Vec<String>  // filenames, symbols
pub fn extract_mentioned_files(text: &str, repo_files: &[String]) -> Vec<String>
pub fn extract_stack_traces(text: &str) -> Vec<StackTrace>
```

**Deliverable:** Full context pack (5 core files + metadata).

**Additional:** Create `metadata.json` with session start timestamp for prompt extraction.

---

### Phase 3: Prompt Extraction (TODO)

**Goal:** Extract and document prompts from AI coding tool sessions

**Why this matters:**
- Shows maintainers your reasoning process (epistemic humility)
- Helps other contributors learn effective prompting strategies
- Provides transparency in AI-assisted contributions
- Demonstrates thorough investigation vs. copy-paste

**Supported Tools:**
1. **Claude Code** - `~/.claude/projects/<hash>/<session>.jsonl`
2. **OpenAI Codex CLI** - `~/.codex/history.jsonl`
3. **OpenCode** - `~/.config/opencode/sessions.db` (SQLite)

**Files:**
```
src/
├── commands/
│   └── prompts.rs          # extract, review subcommands
└── services/
    └── ai_logs.rs          # Log parsing and extraction
```

**Commands:**
```bash
# Extract prompts from AI tool logs (time-filtered)
issuance prompts extract

# Review and edit extracted prompts before including
issuance prompts review

# Clear prompt history for current issue
issuance prompts clear
```

**`services/ai_logs.rs`:**
```rust
#[derive(Debug)]
pub struct ExtractedPrompt {
    pub tool: String,        // "claude-code", "codex", "opencode"
    pub timestamp: String,
    pub prompt: String,
    pub category: Category,  // Investigation, Solution, Testing
}

#[derive(Debug)]
pub enum Category {
    Investigation,  // "explain", "understand", "show me"
    Solution,       // "fix", "implement", "write"
    Testing,        // "test", "reproduce", "verify"
    Other,
}

/// Detect which AI tools have been used
pub fn detect_ai_tools() -> Result<Vec<AITool>>

/// Extract prompts from Claude Code JSONL logs
pub fn extract_claude_code_prompts(
    project_dir: &Path,
    after: DateTime<Utc>,
) -> Result<Vec<ExtractedPrompt>>

/// Extract prompts from Codex CLI JSONL logs
pub fn extract_codex_prompts(
    codex_home: &Path,
    after: DateTime<Utc>,
) -> Result<Vec<ExtractedPrompt>>

/// Extract prompts from OpenCode SQLite database
pub fn extract_opencode_prompts(
    db_path: &Path,
    after: DateTime<Utc>,
) -> Result<Vec<ExtractedPrompt>>

/// Categorize prompt based on keywords
fn categorize_prompt(text: &str) -> Category

/// Filter prompts relevant to current issue
pub fn filter_by_issue_context(
    prompts: Vec<ExtractedPrompt>,
    issue_keywords: &[String],
) -> Vec<ExtractedPrompt>

/// Detect sensitive information (API keys, paths, URLs)
fn detect_sensitive_info(prompt: &str) -> Vec<SensitivityWarning>
```

**Timestamp Window Strategy:**

1. **When starting work** (`issuance grab`):
   ```json
   // .issuance/metadata.json
   {
     "issue_url": "https://github.com/fastapi/fastapi/issues/1284",
     "started_at": "2024-02-03T16:30:00Z",
     "issue_number": 1284,
     "repo_owner": "fastapi",
     "repo_name": "fastapi"
   }
   ```

2. **When extracting prompts** (`issuance prompts extract`):
   - Read `started_at` from metadata.json
   - Only parse log entries with `timestamp >= started_at`
   - Filters out unrelated conversations from before/after

**Generated PROMPTS.md:**
```markdown
# Prompting Session for Issue #1284

> **Generated from AI tool logs**
> Extracted: 2024-02-03 18:45
> Tools used: Claude Code

## Investigation Phase

**Understanding the problem:**
- [16:31] "help me understand this race condition in utils.py"
- [16:35] "show me where dependency resolution happens"

**Exploring the codebase:**
- [16:42] "what files are involved in async dependency injection?"

## Solution Development

**Exploring approaches:**
- [17:05] "why would cleanup fail in concurrent mode?"
- [17:12] "what's the proper way to handle fiber reuse?"

**Implementation:**
- [17:25] "write a fix that prevents cleanup on reused fibers"

## Testing

**Creating tests:**
- [17:40] "write a test that reproduces the memory leak"
- [17:52] "verify this works in React 18.2+"

---

**Metadata:**
- Session duration: 1h 22m
- Total prompts: 8
- Prompts extracted from:
  - Claude Code: 8 prompts
```

**Privacy & Control:**
- User reviews all prompts before including
- Auto-detect and warn about sensitive info (API keys, absolute paths)
- Can redact individual prompts
- Opt-out available (skip extraction entirely)

**Dependencies:**
```toml
[dependencies]
chrono = { version = "0.4", features = ["serde"] }
rusqlite = "0.32"  # For OpenCode SQLite
```

**Deliverable:**
- `issuance prompts extract` working for all 3 tools
- Time-filtered extraction using metadata.json
- Interactive review before saving to PROMPTS.md
- Sensitive info detection and warnings

---

### Phase 4: Polish (TODO)

**Rich CLI output (indicatif + console):**
- Progress spinners
- Colored file summaries
- Tables for signals
- Better error messages

**Config (`~/.issuance/config.toml`):**
```toml
[github]
token = "ghp_xxx"

[defaults]
shallow_clone = true
pr_limit = 50

[prompts]
auto_extract = false  # Prompt to extract after solving
tools = ["claude-code", "codex", "opencode"]
```

**Tests:**
- Unit tests for extractors
- Unit tests for AI log parsers
- Integration test with real issue
- Mock AI tool logs for testing

**Build & Distribute:**
```bash
cargo build --release
# Binary at target/release/issuance (~5MB)
cargo install --path .
```

---

## Timeline

| Day | Phase | Deliverable |
|-----|-------|-------------|
| 1 | Scaffold | ✅ CLI structure, `issuance --help` works |
| 2-4 | Grab | Full context pack generation (5 files + metadata) |
| 5-6 | Prompts | AI tool log extraction and PROMPTS.md generation |
| 7-8 | Polish | Rich output, tests, installable binary |

**Total: ~8 days**

---

## Verification

```bash
# 1. Build
cd issuance
cargo build --release

# 2. Install globally (optional)
cargo install --path .

# 3. Test grab (creates context pack + metadata)
issuance grab https://github.com/fastapi/fastapi/issues/1284
ls .issuance/
# Should see: ISSUE.md, CODEMAP.md, SIGNALS.md, RULES.md, HANDOFF.md, metadata.json

# 4. Work with AI tools
claude "help me understand this race condition"
claude "write a fix for the cleanup issue"

# 5. Extract prompts (time-filtered)
issuance prompts extract
ls .issuance/
# Now also see: PROMPTS.md

cat .issuance/PROMPTS.md
# Should show categorized prompts from your session

# 6. Full workflow example
cd ~/some-project
issuance grab https://github.com/owner/repo/issues/123

# Work with your preferred AI tool
claude "Fix the issue described in .issuance/HANDOFF.md"

# After solving, extract and include your prompts
issuance prompts extract

# Review the full context pack
ls -la .issuance/
# ISSUE.md, CODEMAP.md, SIGNALS.md, RULES.md, HANDOFF.md, PROMPTS.md, metadata.json

# Clean up when done
issuance clean
```
