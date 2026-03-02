# PromptEx

Extract and share your AI coding session prompts.

## What is PromptEx?

PromptEx (`pmtx`) reads prompts directly from AI coding tool logs, correlates them to the git changes in your current scope, and emits structured JSON for your agent to categorize and render as PR-ready markdown. This gives OSS maintainers insight into your reasoning process and builds trust in AI-assisted contributions.

## Quick Start

```bash
# Install (Apple Silicon)
curl -L https://github.com/gutierrezje/promptex/releases/latest/download/pmtx-aarch64-apple-darwin.tar.gz | tar -xz
mv pmtx /usr/local/bin/

# Or build from source
cargo install --path .

# Work with your AI agent normally (pmtx reads logs automatically)
cd ~/myproject
git checkout -b feature/auth-fix
# ... work with Claude Code or Codex ...

# At the end of your session, ask your agent:
# "add my prompts to the PR"
# The agent runs pmtx extract, categorizes the output, and writes the markdown file.
```

The agent skill handles the full workflow. Install it once:

```bash
npx skills add gutierrezje/promptex
```

## How It Works

1. **Log extraction**: Reads prompts directly from AI tool session logs — no agent setup required, zero token overhead
   - Claude Code: `~/.claude/projects/{slug}/*.jsonl`
   - Codex CLI / Desktop app: `~/.codex/sessions/YYYY/MM/DD/*.jsonl`
   - OpenCode, Cursor, GitHub Copilot: planned
2. **Smart scoping**: Analyzes git state to determine relevant range — feature branch lifetime, last N commits, uncommitted changes, or a time window
3. **Correlation**: Matches extracted entries to files and commits in scope (time window + file overlap)
4. **Correlation**: Filters entries to those relevant to the scope (time window + file overlap)
5. **JSON output**: Emits structured data including entries, git context, and a rendering spec — the agent categorizes semantically and writes the final markdown
6. **Privacy-first**: Redacts secrets, tokens, and email addresses from prompt text before any output
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
```

Output is always structured JSON. Feed it to an agent (via the skill) to get PR-ready markdown.

### Other commands

```bash
pmtx check                   # Check if your AI tool is natively supported (exit 0 = yes, exit 1 = unsupported)
pmtx status                  # Show current project info and extraction count
pmtx projects list           # List all tracked projects
pmtx projects remove <N|id>  # Remove a project by number or full ID
```

## Supported Tools

| Tool | Status |
|------|--------|
| Claude Code | ✅ Native |
| Codex CLI / Desktop | ✅ Native |
| OpenCode | ⏳ Planned (SQLite rewrite needed) |
| Cursor | ⏳ Planned |
| GitHub Copilot | ⏳ Planned |

Native support means `pmtx extract` reads the tool's logs directly with no setup required.

## Why Use PromptEx?

- **Zero setup**: Reads existing AI tool logs — no agent configuration needed
- **Build trust**: Show maintainers your reasoning, not just code
- **Agent-rendered**: Semantic categorization by the running agent, not hardcoded rules
- **Zero pollution**: No files in project directory, no `.gitignore` needed
- **Privacy-first**: Auto-redacts secrets, tokens, and emails from all output
- **Git-aware**: Feature branches, fork workflows, commit-based and time-based scoping
- **Fast**: Local processing, no API calls, Rust performance

## Development

```bash
cargo build
cargo fmt && cargo clippy -- -D warnings
cargo test
./target/debug/pmtx --help
```

After cloning, activate the pre-commit hook (keeps `.claude/skills/` in sync with `skills/`):

```bash
git config core.hooksPath .githooks
```

CI runs format, lint (`-D warnings`), and tests on every push and PR.
