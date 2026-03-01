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
# ... work with Claude Code or Codex CLI ...

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
   - OpenCode, Cursor, GitHub Copilot: planned
2. **Smart scoping**: Analyzes git state to determine relevant range — feature branch lifetime, last N commits, uncommitted changes, or a time window
3. **Correlation**: Matches extracted entries to files and commits in scope (time window + file overlap)
4. **Curation**: Filters noise and deduplicates near-identical prompts. Categorization is done by the running agent (`--json`) for semantic accuracy, or by built-in rules when used directly from the CLI
5. **PR format**: Outputs collapsible markdown sections for GitHub PR descriptions
6. **Privacy-first**: `pmtx record` (the manual fallback) redacts sensitive values immediately on write
7. **Home directory storage**: All state in `~/.promptex/` — no project directory pollution

## Commands

### Extract

```bash
pmtx extract                        # Smart default: branch lifetime or last commit
pmtx extract --since 2h             # Commits from the last 2 hours (also: 30m, 1d, 3w)
pmtx extract --commits <N>          # Last N commits
pmtx extract --since-commit <HASH>  # Since a specific commit
pmtx extract --uncommitted          # Uncommitted changes only
pmtx extract --branch-lifetime      # Full branch history since diverge point
pmtx extract --write                # Write to ~/.promptex/projects/{id}/PROMPTS.md (prints path)
pmtx extract --write <FILE>         # Write to a specific file
pmtx extract --json                 # Output structured JSON for agent-side categorization
```

When run in a terminal, `pmtx extract` offers an interactive prompt after printing:
copy to clipboard (`c`) or write to `PROMPTS.md` (`w`).

### Record (manual fallback)

For AI tools without native log support, journal prompts manually after each significant action:

```bash
pmtx record \
  --prompt "implement JWT validation" \
  --files "src/auth.rs,src/middleware.rs" \
  --tool-calls "Edit,Bash,Read" \
  --outcome "Added JWT validation middleware with expiry checking" \
  --tool opencode \          # optional, defaults to claude-code
  --model gemini-2.0-flash   # optional
```

### Other commands

```bash
pmtx status                  # Show current project journal stats
pmtx projects list           # List all tracked projects
pmtx projects remove <id>    # Remove a project's journal
pmtx check                   # Check if your AI tool is natively supported (exit 0 = yes)
```

## Why Use PromptEx?

- **Zero setup**: Reads existing AI tool logs — no agent configuration needed
- **Build trust**: Show maintainers your reasoning, not just code
- **Zero pollution**: No files in project directory, no .gitignore needed
- **Privacy-first**: Auto-redacts sensitive data when writing manual journal entries
- **Git-aware**: Feature branches, fork workflows, commit-based and time-based scoping
- **PR-ready**: Default output is copy/paste into GitHub PR description
- **Fast**: Local processing, no API calls, Rust performance

## Development

```bash
cargo build
cargo test
./target/debug/pmtx --help
```

After cloning, activate the pre-commit hook (keeps `.claude/skills/` in sync with `skills/`):

```bash
git config core.hooksPath .githooks
```

## Status

- [x] Phase 1 — CLI scaffold
- [x] Phase 2 — Project ID & home directory storage
- [x] Phase 3 — Git analysis & smart scope detection
- [x] Phase 4 — Journaling (`pmtx record`) & redaction
- [x] Phase 5 — Log extraction (Claude Code, Codex, manual)
- [x] Phase 6 — Correlation & filtering
- [x] Phase 7 — Curation & categorization (Investigation / Solution / Testing)
- [x] Phase 8 — Output generation — PR format & `--write`
- [x] Phase 9 — Polish — `status`, `projects`, `--since`, interactive output
