# PromptEx Architecture

PromptEx (`pmtx`) extracts AI prompt history from tool session logs and correlates it to git history, producing structured output that an agent renders into PR-ready markdown.

---

## The Core Problem

AI-assisted OSS contributions carry invisible reasoning. A maintainer sees the code change but not the exploration that shaped it — the dead ends, the design tradeoffs considered, the prompts that guided the implementation. PromptEx surfaces that reasoning at PR time, without asking the developer to do anything manually.

---

## System Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                         pmtx extract                            │
│                                                                 │
│  Git State ──► Scope ──► Time Window                           │
│                               │                                 │
│  AI Tool Logs ────────────────┤                                 │
│   ~/.claude/...               │                                 │
│   ~/.codex/...                ▼                                 │
│                          Raw Entries                            │
│                               │                                 │
│                          Correlation  ◄── Scope Files           │
│                               │                                 │
│                          Curation                               │
│                           ├── Artifact filter                   │
│                           └── Jaccard dedup                     │
│                               │                                 │
│                          JSON Output ──────────────────────────►│
└─────────────────────────────────────────────────────────────────┘
                                                                  │
                                                                  ▼
                                                          Agent (Claude)
                                                               │
                                                     Semantic Categorization
                                                               │
                                                          PR Markdown
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

The scope resolves to a **time window** (`since`, `until`) and a **file set** — the files touched by the commits in scope.

### 2. Git Context (`src/analysis/correlation.rs`, `src/analysis/git.rs`)

Given the scope, `build_git_context` shells out to git to produce a `GitContext`:

```
GitContext {
    since: DateTime<Utc>,   // start of time window
    until: DateTime<Utc>,   // end of time window (usually now)
    commits: Vec<Commit>,   // commits in scope
    scope_files: Vec<String>, // files touched by those commits
}
```

This is the lens everything else is filtered through.

### 3. Extraction (`src/extractors/`)

Each supported AI tool has a dedicated extractor that reads its native log format:

**Claude Code** (`claude_code.rs`)
- Reads `~/.claude/projects/{slug}/*.jsonl`
- JSONL format with `type: "user"` turns containing prompt text, tool calls, and file paths

**Codex CLI / Desktop** (`codex.rs`)
- Reads `~/.codex/sessions/YYYY/MM/DD/rollout-{timestamp}-{uuid}.jsonl`
- JSONL with `session_meta` on line 0, then `event_msg` (user messages) and `response_item` (tool calls) events
- Timestamps extracted per-message from `event_msg` payloads — not from session metadata alone

All extractors produce a common `JournalEntry`:

```
JournalEntry {
    timestamp: DateTime<Utc>,
    branch: String,
    commit: String,
    prompt: String,           // the user's message text
    files_touched: Vec<String>,
    tool_calls: Vec<String>,  // e.g. ["Edit", "Bash", "Read"]
    outcome: String,          // summary of what the agent did (if captured)
    tool: String,             // "claude-code" | "codex"
    model: Option<String>,    // model name if present in logs
    assistant_context: Option<String>, // tail of preceding assistant turn
}
```

Extraction is bounded by the time window from step 2 — only entries within `[since, until]` are read.

### 4. Correlation (`src/analysis/correlation.rs`)

Raw entries are filtered down to those *relevant to the scope*. An entry passes if:
- Its timestamp falls within the time window, **and**
- It has file overlap with `scope_files` **or** its timestamp closely precedes a scoped commit

This prevents unrelated work from the same session leaking into the output.

### 5. Curation (`src/curation/`)

Two passes clean up the correlated entries:

**Artifact filter** (`filter.rs`): Drops entries with no concrete output — no tool calls and no files touched. These are pure "thinking out loud" turns that add noise without adding context for a reviewer.

**Jaccard deduplication** (`filter.rs`): Collapses near-identical prompts into one, keeping the most recent version. Users often rephrase a prompt after an unsatisfying response. Jaccard similarity on word tokens catches these clusters at a threshold of 0.80. Short prompts (< 8 words) are exempt — "yes" twice means two distinct approval moments.

**Redaction** (`redact.rs`): Strips secrets, API tokens, and email addresses from prompt text before any output.

### 6. Output (`src/output/`)

`pmtx extract` always emits **structured JSON** to stdout:

```json
{
  "scope": "branch-lifetime",
  "since": "...",
  "until": "...",
  "commits": [{ "short_hash": "abc1234", "message": "..." }],
  "scope_files": ["src/auth.rs", "src/lib.rs"],
  "entries": [ /* curated JournalEntry objects */ ],
  "format_spec": { /* rendering instructions for the agent */ }
}
```

The `format_spec` field embeds the rendering contract — category names, entry format, header/footer templates — so the agent doesn't need to be told separately how to render.

---

## The Agent Split

This is the key architectural decision: **pmtx handles deterministic work; the agent handles semantic work.**

| pmtx (deterministic) | Agent (semantic) |
|----------------------|------------------|
| Time window math | Categorization (Investigation / Solution / Testing) |
| File overlap correlation | Deciding what's noise |
| Jaccard deduplication | Rendering judgment calls |
| Secret redaction | Writing the markdown file |
| Git shell-outs | |

Rule-based categorization was tried and abandoned. Categories like "Investigation" vs "Solution" depend on intent — "look at auth.rs" could be either, depending on whether a fix followed. A language model reading the prompt text and `assistant_context` makes these calls more reliably than keyword matching.

---

## Storage

All state lives in `~/.promptex/projects/{id}/` — never in the project directory. The project ID is derived from the git remote URL (`owner-repo` slug), so multiple clones of the same repo share the same project record.

---

## Module Map

```
src/
├── main.rs                   Entry point, clap command dispatch
├── project_id.rs             Derive project ID from git remote
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
│   ├── filter.rs             Artifact filter + Jaccard dedup
│   └── redact.rs             Secret/token/email redaction
├── output/
│   └── json_format.rs        JSON envelope serialization
└── commands/
    ├── extract.rs            Full pipeline orchestration
    ├── check.rs              Tool support detection
    ├── status.rs             Project/journal stats
    └── projects.rs           List + remove projects
```

---

## Privacy Design

Redaction runs before anything leaves the process — before JSON serialization, before any file write. The redaction pass strips:
- Common secret patterns (API keys, tokens)
- Email addresses
- Values assigned to environment variables matching known secret names

This is a best-effort heuristic, not a cryptographic guarantee. Users should review output before posting to public PRs.
