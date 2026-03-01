# PromptEx

Extract and share your AI coding session prompts.

## What is PromptEx?

PromptEx (`pmtx`) reads prompts directly from AI coding tool logs, correlates them to the git changes in your current scope, and generates PR-ready markdown you can paste into GitHub pull requests. This gives OSS maintainers insight into your reasoning process and builds trust in AI-assisted contributions.

## Quick Start

```bash
# Build the CLI
cargo build --release
cargo install --path .

# Work with your AI agent normally (pmtx reads logs automatically)
cd ~/myproject
git checkout -b feature/auth-fix
# ... work with Claude Code, OpenCode, or Codex CLI ...

# Extract prompts correlated to this branch (outputs PR-ready markdown)
pmtx extract

# Pipe directly into a PR
gh pr create --title "Fix auth validation" \
  --body "$(pmtx extract)"
```

## How It Works

1. **Log extraction**: Reads prompts directly from AI tool session logs — no agent setup required, zero token overhead
   - Claude Code: `~/.claude/projects/{slug}/*.jsonl`
   - Codex CLI / Desktop app: `~/.codex/sessions/YYYY/MM/DD/*.jsonl`
   - Manual fallback: `~/.promptex/projects/{id}/journal.jsonl` (via `pmtx record`)
   - OpenCode, Cursor, GitHub Copilot: planned for future phases
2. **Smart scoping**: Analyzes git state to determine relevant range — feature branch lifetime, last N commits, or uncommitted changes
3. **Correlation**: Matches extracted entries to files and commits in scope (time window + file overlap)
4. **Curation**: Filters, categorizes, and deduplicates prompts *(coming in Phase 7)*
5. **PR format**: Outputs collapsible markdown for GitHub PR descriptions *(coming in Phase 8)*
6. **Privacy-first**: `pmtx record` (the manual fallback) redacts sensitive values immediately on write
7. **Home directory storage**: All state in `~/.promptex/` — no project directory pollution

## Commands

- **`pmtx extract`** — Extract prompts (smart git-aware defaults, outputs to stdout)
- **`pmtx extract --write [FILE]`** — Write to file instead of stdout
- **`pmtx extract --commits <N>`** — Extract for last N commits (fork/main workflow)
- **`pmtx extract --since-commit <HASH>`** — Extract since a specific commit
- **`pmtx extract --uncommitted`** — Only uncommitted changes
- **`pmtx extract --branch-lifetime`** — Full branch history since diverge point
- **`pmtx record`** — Journal a prompt manually (fallback when no tool logs exist)
- **`pmtx status`** — Show project journal statistics
- **`pmtx projects`** — Manage tracked projects

## Why Use PromptEx?

- **Zero setup**: Reads existing AI tool logs — no agent configuration needed
- **Build trust**: Show maintainers your reasoning, not just code
- **Zero pollution**: No files in project directory, no .gitignore needed
- **Privacy-first**: Auto-redacts sensitive data when writing manual journal entries
- **Git-aware**: Feature branches, fork workflows, commit-based scoping
- **PR-ready**: Default output is copy/paste into GitHub PR description
- **Fast**: Local processing, no API calls, Rust performance

## Documentation

- `PLAN.md` — Full technical architecture and implementation phases
- `CLAUDE.md` — Development workflow notes

## Development

```bash
cargo build
cargo test
./target/debug/pmtx --help
```

## Status

🚧 **In active development**

| Phase | Description | Status |
|-------|-------------|--------|
| 1 | CLI scaffold | ✅ |
| 2 | Project ID & home directory storage | ✅ |
| 3 | Git analysis & smart scope detection | ✅ |
| 4 | Journaling (`pmtx record`) & redaction | ✅ |
| 5 | Log extraction (Claude Code, OpenCode, Codex, manual) | ✅ |
| 6 | Correlation & filtering — match prompts to git scope | ✅ |
| 7 | Curation & categorization (Investigation / Solution / Testing) | 🎯 next |
| 8 | Output generation — PR format (stdout) & detailed (--write) | ⬜ |
| 9 | Polish — `status`, `projects`, interactive clipboard output, config | ⬜ |
