# PromptEx Architecture

PromptEx (`pmtx`) extracts AI prompt history from tool session logs, correlates it to git history, applies semantic curation via a lightweight decisions sidecar, and renders PR-ready markdown.

---

## The Core Problem

AI-assisted OSS contributions carry invisible reasoning. A maintainer sees the code change but not the exploration that shaped it — the dead ends, the design tradeoffs considered, the prompts that guided the implementation. PromptEx surfaces that reasoning at PR time, without asking the developer to do anything manually.

---

## System Overview

```text
┌──────────────────────────────────────────────────────────────────┐
│                          pmtx extract                            │
│                                                                  │
│  Git State ──► Scope ──► Time Window                             │
│                               │                                  │
│  AI Tool Logs ────────────────┤                                  │
│   ~/.claude.json              │                                  │
│   ~/.codex/...                ▼                                  │
│   ~/.gemini/                  Raw Entries                        │
│                               │                                  │
│                          Correlation  ◄── Scope Files            │
│                               │                                  │
│                    Canonical JSON Output ───────────────────────►│
└───────────────────────────────┬──────────────────────────────────┘
                                │
   ┌────────────────────────────▼─────────────────────────────┐
   │                       pmtx curate                        │
   │                                                          │
   │               Canonical JSON (from stdin)                │
   │                            +                             │
   │               decisions.json (from LLM/TUI)              │
   │                            =                             │
   │  Curated JSON Output (Categorized, noise filtered)       │
   └──────────────────────────────────────────────────────────┘
                                │
   ┌────────────────────────────▼─────────────────────────────┐
   │                       pmtx format                        │
   │                                                          │
   │            Consumes Curated JSON (from stdin)            │
   │                            │                             │
   │                    Markdown Output                       │
   └──────────────────────────────────────────────────────────┘
```

---

## Pipeline

### 1. Scope Resolution (`src/analysis/scope.rs`)

Before reading any logs, `pmtx` determines *what range of work* to cover. Scope is either explicit (via CLI flags) or inferred from git state:

| Situation | Resolved Scope |
|-----------|---------------|
| `--uncommitted` | Staged + unstaged changes only |
| `--since 2h` | Commits authored in the last 2 hours |
| `--commits N` | Last N commits |
| `--since-commit HASH` | Since a specific commit (exclusive) |
| `--branch-lifetime` | Full branch since diverge point |
| Feature branch (smart default) | Same as `--branch-lifetime` |
| Mainline + uncommitted (smart default) | Same as `--uncommitted` |
| Mainline, no changes (smart default) | Last 1 commit |

### 2. Git Context (`src/analysis/correlation.rs`, `src/analysis/git.rs`)

Given the scope, `build_git_context` shells out to git to produce a `GitContext`:

```rust
GitContext {
    since: DateTime<Utc>,   // start of time window
    until: DateTime<Utc>,   // end of time window (usually now)
    commits: Vec<Commit>,   // commits in scope
    scope_files: Vec<String>, // files touched by those commits
}
```

This is the lens everything else is filtered through.

### 3. Extraction (`src/extractors/`)

Each supported AI tool has a dedicated extractor that reads its direct log format:

**Claude Code** (`claude_code.rs`)
- Reads `~/.claude/projects/{slug}/*.jsonl`

**Codex CLI / Desktop** (`codex.rs`)
- Reads `~/.codex/sessions/YYYY/MM/DD/rollout-{timestamp}-{uuid}.jsonl`

All extractors produce a common `PromptEntry`:

```rust
PromptEntry {
    id: String,               // {tool}-{timestamp_ms}
    timestamp: DateTime<Utc>,
    branch: String,
    commit: String,
    prompt: String,           // the user's message text
    files_touched: Vec<String>,
    tool_calls: Vec<String>,  // normalized tool names
    tool: String,             
    model: Option<String>,    
    assistant_context: Option<String>, 
    category: Option<String>, // Injected during curation
}
```

Extraction is bounded by the time window from step 2 — only entries within `[since, until]` are read.

### 4. Correlation & Redaction (`src/analysis/correlation.rs`, `src/curation/redact.rs`)

Raw entries are filtered down. An entry passes if its timestamp falls within the time window, **and** it has file overlap with `scope_files` or its timestamp closely precedes a scoped commit.

Redaction strips secrets, API tokens, and email addresses from prompt text before any output. Runs in-process before JSON serialization — best effort.

### 5. Curation (`src/commands/curate.rs`)

`pmtx extract` emits a massive Canonical JSON payload. To categorize prompts without requiring an LLM to rewrite the entire JSON envelope, we use a patch-based approach:

- The LLM (or a human TUI) reads the payload and emits a tiny `decisions.json` map:
  ```json
  {
    "version": 1,
    "decisions": {
      "codex-1710656289759": { "action": "keep", "category": "Solution" },
      "claude-2834710293481": { "action": "drop" }
    }
  }
  ```
- `pmtx curate --decisions decisions.json` consumes the canonical JSON from stdin, looks up each prompt by its stable ID, applies the decision, and outputs the curated JSON.

### 6. Formatting (`src/commands/format.rs`, `src/output/markdown_format.rs`)

`pmtx format` consumes the curated JSON and deterministically renders the PR-ready markdown. This cleanly decouples formatting layout from the semantic AI skill.

---

## The AI Split

This is the key architectural decision: **pmtx handles deterministic data routing and layout; the AI handles semantic judgment.**

| pmtx (deterministic) | Agent (semantic) |
|----------------------|------------------|
| Time window math | Filtering out noise (tangential meta-prompts) |
| File overlap correlation | Near-duplicate detection |
| Secret redaction | Categorization (Investigation / Solution / Testing) |
| JSON generation | Producing the `decisions.json` map |
| PR Markdown string formatting | |

**Guiding principle:** When faced with a classification or quality problem that requires understanding *meaning*, it belongs to the AI agent. When it involves heavy string manipulation, byte sizes, or rigid syntax, it belongs to Rust.

## Storage

All state lives in `~/.promptex/projects/{id}/` — never in the project directory. The project ID is derived from the git remote URL (`owner-repo` slug), so multiple clones of the same repo share the same project record.

---

## Module Map

```text
src/
├── main.rs                   Entry point, clap command dispatch
├── project_id.rs             Derive project ID from git remote
├── prompt.rs                 PromptEntry — shared data type
├── analysis/
│   ├── scope.rs              ExtractionScope enum + determine_scope()
│   ├── git.rs                Git shell-outs (branch, commits, files)
│   └── correlation.rs        build_git_context(), filter_by_scope()
├── extractors/
│   ├── traits.rs             Extractor trait
│   ├── claude_code.rs        Claude Code JSONL reader
│   ├── codex.rs              Codex CLI/Desktop JSONL reader
│   └── opencode.rs           Disabled (SQLite migration needed)
├── curation/
│   └── redact.rs             Secret/token/email redaction
├── output/
│   ├── json_format.rs        JSON envelope serialization
│   └── markdown_format.rs    PR format renderer
└── commands/
    ├── extract.rs            Full extraction orchestration
    ├── curate.rs             Decision manifest application
    ├── format.rs             Markdown renderer command
    ├── check.rs              Tool support detection
    ├── status.rs             Project/prompt stats
    └── projects.rs           List + remove projects
```

---

## Privacy Design

Redaction runs before anything leaves the process — before JSON serialization, before any file write. The redaction pass strips:
- Common secret patterns (API keys, tokens)
- Email addresses
- Values assigned to environment variables matching known secret names

This is a best-effort heuristic, not a cryptographic guarantee. Users should review output before posting to public PRs.
