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
├── RULES.md      # Project-wide contribution rules (synthesized via local agentic CLI)
└── issues/
    └── <issue-number>/
        ├── ISSUE.md      # Ground truth for this issue
        ├── CODEMAP.md    # Issue-scoped file mapping
        ├── SIGNALS.md    # Issue-scoped local ambient signals
        ├── HANDOFF.md    # AI tool entry point for this issue
        ├── NEXT.md       # Immediate next 3 actions for this issue
        ├── PROMPTS.md    # Trust log for this issue session
        └── metadata.json # Session metadata for this issue
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

### 6. NEXT.md - Immediate Action Plan

`NEXT.md` is generated to reduce first-step paralysis and keep startup cost low.

```markdown
# Next Actions

1. Run focused tests: `pytest tests/test_dependencies.py -q`
2. Inspect suspect files: `rg -n "solve_dependencies|dependency_overrides" fastapi/dependencies`
3. Start implementation in `fastapi/dependencies/utils.py`
```

### 7. PROMPTS.md - Trust Log (Core)

`PROMPTS.md` is a curated, reviewed log of AI prompts that materially changed investigation,
implementation, or testing decisions. This is a trust artifact for maintainers, not a raw chat dump.

---

## CLI Commands

### `issuance grab <url> [directory]`
Clones the repository like `git clone` (default folder: repo name, optional custom directory),
then fetches the issue and generates the context pack.

```bash
$ issuance grab https://github.com/fastapi/fastapi/issues/1284 fastapi-work

✓ Cloning fastapi/fastapi (shallow)
✓ Fetching issue #1284 (12 comments)
✓ Running language tools...
  ✓ ruff check --statistics
  ✓ pytest --collect-only
✓ Extracting signals (recent commits, CI config, TODO/FIXME)
✓ Synthesizing RULES.md via OpenCode
✓ Generating context pack

📁 fastapi-work/.issuance/ ready

Files created:
  .issuance/RULES.md                                  (project-wide rules)
  .issuance/issues/1284/ISSUE.md                      (ground truth)
  .issuance/issues/1284/CODEMAP.md                    (suspected files + tool output)
  .issuance/issues/1284/SIGNALS.md                    (recent commits, CI config, TODO/FIXME)
  .issuance/issues/1284/HANDOFF.md                    (AI tool entry point)
  .issuance/issues/1284/NEXT.md                       (immediate next 3 actions)
  .issuance/issues/1284/metadata.json                 (issue session metadata)

Next: Open your AI tool and say "Fix the issue in .issuance/issues/1284/HANDOFF.md"
```

**No paid API calls.** By default, `grab` analyzes `CONTRIBUTING.md`, CI config, and repo
conventions, then invokes a local agentic CLI (OpenCode) to synthesize `RULES.md`.

**Utility defaults:**
- Fast path first: prioritize project `RULES.md` + issue `ISSUE.md`, `HANDOFF.md`, `NEXT.md`
- Add issue `CODEMAP.md` and `SIGNALS.md` from local tools without heavy analysis
- Reserve deeper enrichment for explicit flags (`--deep`) if needed

### `issuance clean`
Wipes the context folder.

```bash
$ issuance clean
✓ Removed .issuance/
```

### `issuance prompts extract`
Builds a curated trust log from AI tool prompts (time-filtered + reviewed).

```bash
$ issuance prompts extract

🔍 Extracting prompts since 2024-02-03 16:30

Found conversations:
  ✓ OpenCode: 24 prompts (2 hours ago)

📝 Kept after curation: 8 prompts

Investigation:
  [16:31] (opencode) "help me understand this race condition"
  [16:45] (opencode) "show me the traceback analysis"

Solution:
  [17:02] (opencode) "write a fix for cleanup on reused fibers"

Testing:
  [17:30] (opencode) "generate regression tests"

Include these in PROMPTS.md with evidence links? [y/N/edit] y
✓ Saved to .issuance/issues/1284/PROMPTS.md

💡 Tip: Review PROMPTS.md and redact any sensitive information before committing
```

**How it works:**
1. Reads `started_at` timestamp from `.issuance/issues/<issue-number>/metadata.json`
2. Parses tool logs (Claude first, adapters for others later)
3. Filters prompts after the start timestamp
4. Keeps only prompts tied to a concrete artifact (file, command, or test)
5. Categorizes prompts (Investigation, Solution, Testing)
6. Redacts sensitive values and prompts for review/approval
7. Generates `.issuance/issues/<issue-number>/PROMPTS.md` with evidence + confidence tags

## Why This Architecture Wins

1. **Zero Marginal Cost** - GitHub API (issues only) + local tools + local agentic CLI
2. **Deterministic Core** - Base inputs are reproducible; synthesized output may vary
3. **Debuggable** - You can read and edit every file
4. **Review Trust** - `PROMPTS.md` captures how conclusions were reached, not just final output
5. **Model Agnostic** - Works with OpenCode, Cursor, Claude Code, Copilot, whatever wins next week
6. **Uses the Ecosystem** - Runs `ruff`, `tsc`, `pytest` instead of reinventing them
7. **Composable** - Each file is standalone, use what you need

---

## What NOT to Build

- Custom AST walkers (use existing language tools)
- Deep call graph logic (AI tools handle this)
- Cross-language parsing (out of scope)
- MCP server (not a conversational assistant)
- Per-call API payments (use subscription-based local tools)
- Global repo index (issue-scoped only)
- Anything an agent can get faster with `rg`, `git log`, or filesystem inspection
- Raw prompt dumps without curation, evidence, or redaction

---

## Local Agentic Synthesis (Default)

Issuance is designed for local agentic CLIs by default. After generating deterministic facts,
`grab` invokes OpenCode (via `opencode` CLI) to synthesize `RULES.md`.

**How it works:**
1. `issuance` generates base files deterministically (GitHub API + language tools)
2. Invokes `opencode` CLI with a specific prompt to synthesize `RULES.md`
3. If `opencode` is not available, falls back to deterministic output

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
│   │   ├── github.rs       # GitHub API (issues + comments only)
│   │   ├── agentic.rs      # Local agentic CLI integration (OpenCode)
│   │   ├── tools.rs        # Local tooling + local signal collectors
│   │   ├── extractor.rs    # Keyword/file extraction (no model use)
│   │   └── generator.rs    # Render templates → context files
│   └── templates/
│       ├── mod.rs          # Template embedding
│       ├── issue.md.tera
│       ├── codemap.md.tera
│       ├── signals.md.tera
│       ├── rules.md.tera
│       ├── handoff.md.tera
│       └── next.md.tera
├── README.md
└── tests/
    ├── extractor_test.rs
    ├── generator_test.rs
    └── ai_logs_test.rs
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
│   ├── github.rs       # Issue + comments + repository clone
│   ├── agentic.rs      # Local agentic CLI integration (OpenCode)
│   ├── tools.rs        # Run language tools + collect local signals
│   ├── extractor.rs    # Keywords, file mentions
│   └── generator.rs    # Generate all context files
└── templates/
    ├── issue.md.tera
    ├── codemap.md.tera
    ├── signals.md.tera
    ├── rules.md.tera
    ├── handoff.md.tera
    └── next.md.tera
```

**`services/github.rs`:**
```rust
pub async fn fetch_issue(owner: &str, repo: &str, issue_num: u64) -> Result<Issue>
pub async fn fetch_comments(owner: &str, repo: &str, issue_num: u64) -> Result<Vec<Comment>>
pub fn clone_repo(owner: &str, repo: &str, destination: Option<&Path>, shallow: bool) -> Result<PathBuf>
```

**`services/tools.rs`:**
```rust
pub fn detect_project_type(repo_path: &Path) -> ProjectType  // Python, TypeScript, Go, Rust
pub fn run_linter(repo_path: &Path, project_type: ProjectType) -> Result<Option<LintOutput>>
pub fn discover_tests(repo_path: &Path, project_type: ProjectType) -> Result<Vec<String>>
pub fn collect_ci_config(repo_path: &Path) -> Result<Vec<String>>
pub fn collect_recent_commits(repo_path: &Path, paths: &[String], limit: usize) -> Result<Vec<String>>
pub fn collect_todo_fixme(repo_path: &Path, paths: &[String]) -> Result<Vec<String>>
```

**`services/extractor.rs`:**
```rust
pub fn extract_keywords(text: &str) -> Vec<String>  // filenames, symbols
pub fn extract_mentioned_files(text: &str, repo_files: &[String]) -> Vec<String>
pub fn extract_stack_traces(text: &str) -> Vec<StackTrace>
```

**Deliverable:** Project-level `.issuance/RULES.md` + issue-scoped context folder with 6 files + `metadata.json`.

**Additional:** Create issue-scoped `metadata.json` with session start timestamp for prompt extraction.

---

### Phase 3: Prompt Transparency (NEXT PRIORITY)

**Goal:** Generate a maintainer-trustworthy reasoning log, not a raw transcript

**Why this matters:**
- Increases trust by showing how conclusions were reached
- Reduces "AI slop" concerns with inspectable reasoning artifacts
- Gives maintainers concrete evidence paths (files, tests, commands)

**Scope (OpenCode first):**
1. **OpenCode** - `~/.config/opencode/sessions.db`
2. Adapter hooks for Codex/Claude later (same normalized schema)

**Files:**
```
src/
├── commands/
│   └── prompts.rs          # extract + review + export-pr-summary
└── services/
    └── ai_logs.rs          # extraction, curation, redaction, normalization
```

**Commands:**
```bash
# Build curated trust log for current issue session
issuance prompts extract

# Review/redact entries before writing PROMPTS.md
issuance prompts review

# Generate PR-ready transparency section from PROMPTS.md
issuance prompts export-pr-summary
```

**Normalized entry schema (`ai_logs.rs`):**
```rust
#[derive(Debug)]
pub struct SessionMetadata {
    pub tool: String,              // "opencode"
    pub model: Option<String>,     // e.g. "gpt-5-codex"
    pub cli_version: Option<String>,
    pub agentic_environment: Option<String>, // codex-cli, opencode desktop, etc.
    pub os: Option<String>,        // macOS/Linux
    pub shell: Option<String>,     // zsh/bash
}

#[derive(Debug)]
pub struct PromptEntry {
    pub tool: String,             // "opencode"
    pub timestamp: String,        // ISO-8601
    pub intent: Intent,           // Investigation | Solution | Testing
    pub prompt: String,           // Redacted text
    pub artifact: String,         // File path, command, or test name
    pub result: String,           // What changed or what was ruled out
    pub confidence: Confidence,   // low | medium | high
}
```

**Curation rules (non-negotiable):**
1. Keep only prompts tied to a concrete artifact (`file`, `command`, or `test`)
2. Keep failed hypotheses if they changed direction
3. Drop repetitive prompts that produced no action
4. Always run sensitive-value detection + user review before write

**Timestamp window strategy:**
1. `grab` writes `.issuance/issues/<issue-number>/metadata.json` with `started_at`
2. `prompts extract` only considers entries with `timestamp >= started_at`

**Generated `PROMPTS.md` (curated):**
```markdown
# Prompting Session for Issue #1284

> Generated from local AI logs (curated)
> Extracted: 2024-02-03 18:45
> Tool: OpenCode
> Model: gpt-5-codex
> Agentic environment: Codex CLI
> CLI version: 0.24.0
> OS/Shell: macOS / zsh

## Investigation
- [16:31] Prompt: "help me understand this race condition in utils.py"
  Artifact: `fastapi/dependencies/utils.py`
  Result: Identified cleanup path with missing guard
  Confidence: medium

## Solution
- [17:25] Prompt: "write a fix that prevents cleanup on reused fibers"
  Artifact: `fastapi/dependencies/dependants.py`
  Result: Added guard condition and updated call path
  Confidence: high

## Testing
- [17:40] Prompt: "write a test that reproduces the memory leak"
  Artifact: `tests/test_dependencies.py::test_cleanup_on_reused_fibers`
  Result: Added regression test, passing locally
  Confidence: high
```

**PR transparency export (`issuance prompts export-pr-summary`):**
```markdown
## AI Assistance Transparency

- Tool: OpenCode
- Model: gpt-5-codex
- Agentic environment: Codex CLI
- Session window: 2024-02-03T16:30:00Z to 2024-02-03T18:45:00Z
- Kept prompts: 8 (curated from 24)

### Reasoning Evidence
- Investigation: linked to `fastapi/dependencies/utils.py`
- Solution: linked to `fastapi/dependencies/dependants.py`
- Testing: linked to `tests/test_dependencies.py::test_cleanup_on_reused_fibers`
```

**Privacy controls:**
- Metadata fields are collected as optional and can be redacted before export
- `prompts review` supports removing model/tool/environment details per entry or globally

**Deliverables:**
- `issuance prompts extract` with curation + redaction
- `PROMPTS.md` with artifact/result/confidence + optional session metadata
- `issuance prompts export-pr-summary` with environment/model transparency fields

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
tools = ["opencode"]  # Adapter hooks for others later
include_session_metadata = true   # model, cli version, environment, os/shell
redact_session_metadata = false   # force-hide metadata in exported PR summary
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
| 2-4 | Grab | Repo clone + project/issue context generation |
| 5-7 | Prompt Transparency | Curated PROMPTS.md + PR summary export |
| 8 | Polish | Rich output, tests, installable binary |

**Total: ~8 days**

---

## Verification

```bash
# 1. Build
cd issuance
cargo build --release

# 2. Install globally (optional)
cargo install --path .

# 3. Test grab (clone + context generation)
issuance grab https://github.com/fastapi/fastapi/issues/1284 fastapi-work
ls fastapi-work/.issuance/
# Should see: RULES.md, issues/
ls fastapi-work/.issuance/issues/1284/
# Should see: ISSUE.md, CODEMAP.md, SIGNALS.md, HANDOFF.md, NEXT.md, metadata.json

# 4. Work with AI tools
opencode "help me understand this race condition"
opencode "write a fix for the cleanup issue"

# 5. Extract prompts (time-filtered)
issuance prompts extract
ls fastapi-work/.issuance/issues/1284/
# Now also see: PROMPTS.md

cat fastapi-work/.issuance/issues/1284/PROMPTS.md
# Should show categorized prompts from your session

# 5b. Generate PR transparency summary
issuance prompts export-pr-summary

# 6. Full workflow example
cd ~/some-project
issuance grab https://github.com/owner/repo/issues/123 repo-workdir

# Work with your preferred AI tool
opencode "Fix the issue described in .issuance/issues/123/HANDOFF.md"

# After solving, extract and include your prompts
issuance prompts extract

# Review the full context pack
ls -la repo-workdir/.issuance/
# RULES.md, issues/
ls -la repo-workdir/.issuance/issues/123/
# ISSUE.md, CODEMAP.md, SIGNALS.md, HANDOFF.md, NEXT.md, PROMPTS.md, metadata.json

# Clean up when done
issuance clean
```
