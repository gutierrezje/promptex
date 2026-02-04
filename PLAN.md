# Issuance - Context Orchestrator

## The Problem

The biggest friction isn't finding an issue; it's the **2 hours you spend orienting yourself**:
- "How do I build this?"
- "Where is the relevant code?"
- "What is the maintainer's vibe?"

## The Philosophy

**Issuance curates, validates, and stages context so language tooling and LLMs can do their best work.**

It's not a static analysis engine. It's not an AI agent replacement.
It's a **Context Orchestrator**.

**3-Stage Pipeline:**
```
DISCOVER → STAGE → HANDOFF
```

1. **Discover** - Find what's worth working on
2. **Stage** - Prepare high-signal context
3. **Handoff** - Deliver to LLMs, editors, humans

Everything else is noise.

---

## The Context Pack (v2)

```
.issuance/
├── ISSUE.md      # Ground truth (GitHub API, no interpretation)
├── CODEMAP.md    # Lightweight, tool-assisted file mapping
├── SIGNALS.md    # Ambient signals (commits, CI, TODOs)
├── RULES.md      # Contribution rules
└── HANDOFF.md    # LLM-facing entry point (short, actionable)
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

**Mirrors better-context's "don't editorialize" philosophy.**

### 2. CODEMAP.md - Tool-Assisted File Mapping

**How it's generated:**
1. Extract keywords from issue text (filenames, symbols)
2. Run existing project tools if present:
   - `tsc --noEmit --pretty false`
   - `ruff check --statistics`
   - `go list ./...`
   - `cargo metadata`
3. Capture file paths, module boundaries, public signatures (only if cheap)

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

### 3. SIGNALS.md - Ambient Context (NEW)

This is something better-context doesn't do. Issuance collects ambient signals:

```markdown
# Signals for Issue #1284

## Recent Activity
- utils.py modified 6 times in last 30 days
- Last commit: "fix: handle None in dependency resolution" (3 days ago)

## Related Issues
- #1280: "Race condition in async middleware" (open)
- #1256: "Dependency injection fails with nested deps" (closed)

## CI Status
- Last run on main: PASSED (2 hours ago)
- Relevant test file: tests/test_dependencies.py (all passing)

## Code Health
- TODO/FIXME in utils.py: 2
- No tests exist for `solve_dependencies()` specifically
```

**Human-grade context that LLMs reason over excellently.**

### 4. RULES.md - Contribution Rules

```markdown
# Contribution Rules for fastapi/fastapi

## Commit Convention
**Conventional Commits required.** Use `feat:`, `fix:`, `docs:` prefixes.

## Testing
**Required.** 98% of merged PRs include tests.
Run: `pytest tests/test_dependencies.py -v`

## Style
- Formatter: `black`
- Linter: `ruff`
- Run: `black . && ruff check .`

## Review Process
- Average review time: 48 hours
- Required approvals: 1
- Primary reviewer for async: @tiangolo

## Don'ts
- Don't modify `pyproject.toml` without asking
- Don't import from `starlette` directly (use `fastapi.` wrappers)
- Don't squash commits yourself (maintainers do this)
```

### 5. HANDOFF.md - The LLM Entry Point (SECRET WEAPON)

**Short. Very short.** This is where Issuance beats better-context.

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

**This is the alignment layer. The AI already knows where to look.**

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
✓ Extracting signals (commits, related issues)
✓ Generating context pack

📁 .issuance/ ready

Files created:
  ISSUE.md      (ground truth)
  CODEMAP.md    (suspected files + tool output)
  SIGNALS.md    (commits, CI, related issues)
  RULES.md      (contribution rules)
  HANDOFF.md    (LLM entry point)

Next: Open your AI tool and say "Fix the issue in .issuance/HANDOFF.md"
```

**No API calls.** But can optionally invoke local AI harnesses (Claude Code, Cursor) for enhancement.

### `issuance profile <repo>`
Analyzes repo culture standalone (useful before grabbing issues).

```bash
$ issuance profile fastapi/fastapi

✓ Fetching last 50 merged PRs
✓ Analyzing commit conventions
✓ Extracting review patterns
✓ Checking CI config

📄 .issuance/RULES.md created

Summary:
  - Commits: Conventional Commits required
  - Tests: Required (98% of PRs)
  - Review: ~48 hours, 1 approval
  - Style: black + ruff
```

### `issuance clean`
Wipes the context folder.

```bash
$ issuance clean
✓ Removed .issuance/
```

---

## Differentiation from better-context

| | better-context | Issuance |
|---|----------------|----------|
| **Mode** | Query-based | Task-based |
| **Goal** | "Ask questions" | "Prepare to act" |
| **Scope** | Global context | Issue-scoped |
| **Use case** | Tooling for learning | Tooling for shipping |
| **Interaction** | Runtime conversation | Pre-work orchestration |

**That's a clean, defensible distinction.**

---

## Why This Architecture Wins

1. **Zero Marginal Cost** - GitHub API + existing language tools only
2. **Deterministic** - Same input → same output, every time
3. **Debuggable** - You can read and edit every file
4. **Model Agnostic** - Works with Cursor, Claude Code, Copilot, whatever wins next week
5. **Uses the Ecosystem** - Runs `ruff`, `tsc`, `pytest` instead of reinventing them
6. **Composable** - Each file is standalone, use what you need

---

## What NOT to Build

- ❌ Custom AST walkers (use existing language tools)
- ❌ Deep call graph logic (LLMs handle this)
- ❌ Cross-language parsing (out of scope)
- ❌ MCP server (not a conversational assistant)
- ❌ Per-call API payments (use subscription-based local tools)
- ❌ Global repo index (issue-scoped only)

---

## Optional: Local AI Enhancement

The CLI doesn't have to be "dumb." If you have Claude Code or Cursor installed, issuance can invoke them to enhance output:

### Enhancement Modes

**`issuance grab --enhance`**

After generating the base context pack, pipes it through Claude Code:

```bash
$ issuance grab https://github.com/fastapi/fastapi/issues/1284 --enhance

✓ Base context generated (5 files)
✓ Invoking Claude Code for enhancement...
  ✓ CODEMAP.md: Added likely root cause analysis
  ✓ HANDOFF.md: Refined suggested approach

📁 .issuance/ ready (enhanced)
```

**How it works:**
1. `issuance` generates base files deterministically (GitHub API + language tools)
2. If `--enhance` flag, invokes `claude` CLI with a specific prompt
3. Claude Code uses your existing subscription (no API cost)

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
│   │   ├── profile.rs      # issuance profile <repo>
│   │   └── clean.rs        # issuance clean
│   ├── services/
│   │   ├── mod.rs
│   │   ├── github.rs       # GitHub API (issues, PRs, commits)
│   │   ├── tools.rs        # Run language-native tools (ruff, tsc)
│   │   ├── extractor.rs    # Keyword/file extraction (no LLM)
│   │   ├── profiler.rs     # PR pattern analysis
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
    ├── profiler_test.rs
    └── generator_test.rs
```

**No database. No server. No LLM calls.**
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
  profile  Analyze repo contribution culture
  clean    Remove .issuance/ folder
```

**Status:** Complete. All commands parse correctly, clean command fully functional.

---

### Phase 2: `issuance grab` - Core Pipeline (IN PROGRESS)

**Goal:** Full context pack generation

**Files:**
```
src/
├── commands/grab.rs
├── services/
│   ├── github.rs       # Issue + comments + signals
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
pub async fn fetch_recent_commits(owner: &str, repo: &str, path: &str) -> Result<Vec<Commit>>
pub async fn fetch_related_issues(owner: &str, repo: &str, keywords: &[String]) -> Result<Vec<Issue>>
pub fn clone_repo(owner: &str, repo: &str, shallow: bool) -> Result<PathBuf>
```

**`services/tools.rs`:**
```rust
pub fn detect_project_type(repo_path: &Path) -> ProjectType  // Python, TypeScript, Go, Rust
pub fn run_linter(repo_path: &Path, project_type: ProjectType) -> Result<LintOutput>
pub fn run_test_discovery(repo_path: &Path, project_type: ProjectType) -> Result<Vec<String>>
pub async fn get_ci_status(owner: &str, repo: &str) -> Result<CIStatus>
```

**`services/extractor.rs`:**
```rust
pub fn extract_keywords(text: &str) -> Vec<String>  // filenames, symbols
pub fn extract_mentioned_files(text: &str, repo_files: &[String]) -> Vec<String>
pub fn extract_stack_traces(text: &str) -> Vec<StackTrace>
```

**Deliverable:** Full context pack with all 5 files.

---

### Phase 3: `issuance profile` (TODO)

**Goal:** Standalone repo culture analysis

**Files:**
```
src/
├── commands/profile.rs
└── services/profiler.rs
```

**`services/profiler.rs`:**
```rust
pub async fn fetch_merged_prs(owner: &str, repo: &str, limit: usize) -> Result<Vec<PullRequest>>
pub fn analyze_commit_convention(prs: &[PullRequest]) -> CommitConvention
pub fn analyze_test_requirements(prs: &[PullRequest]) -> TestRequirements
pub fn analyze_review_patterns(prs: &[PullRequest]) -> ReviewPatterns
pub fn analyze_merge_strategy(prs: &[PullRequest]) -> MergeStrategy
pub fn parse_contributing_md(content: &str) -> ContributionGuide
pub fn parse_ci_config(repo_path: &Path) -> Result<CIConfig>
```

**Deliverable:** RULES.md generated from PR analysis + config parsing.

---

### Phase 4: Polish (TODO)

**Rich CLI output (indicatif + console):**
- Progress spinners
- Colored file summaries
- Tables for signals

**Config (`~/.issuance/config.toml`):**
```toml
[github]
token = "ghp_xxx"

[defaults]
shallow_clone = true
pr_limit = 50
```

**Tests:**
- Unit tests for extractors
- Integration test with real issue

**Build & Distribute:**
```bash
cargo build --release
# Binary at target/release/issuance (~5MB)
```

---

## Timeline

| Day | Phase | Deliverable |
|-----|-------|-------------|
| 1 | Scaffold | ✅ CLI structure, `issuance --help` works |
| 2-4 | Grab | Full context pack generation (5 files) |
| 5-6 | Profile | Standalone repo culture analysis |
| 7-9 | Polish | Rich output, tests, installable binary |

**Total: ~9 days**

---

## Verification

```bash
# 1. Build
cd issuance
cargo build --release

# 2. Install globally (optional)
cargo install --path .

# 3. Test grab
issuance grab https://github.com/fastapi/fastapi/issues/1284
ls .issuance/
# Should see: ISSUE.md, CODEMAP.md, SIGNALS.md, RULES.md, HANDOFF.md

# 4. Test profile (standalone)
issuance profile fastapi/fastapi
cat .issuance/RULES.md

# 5. Full workflow
cd ~/some-project
issuance grab https://github.com/owner/repo/issues/123
# Open Cursor / Claude Code
# Say: "Fix the issue described in .issuance/HANDOFF.md"
```
