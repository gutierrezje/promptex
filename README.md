# PromptEx

Extract and share your AI coding session prompts.

## What is PromptEx?

PromptEx (`pmtx`) intelligently extracts prompts from AI coding sessions and generates PR-ready markdown you can paste into GitHub pull requests. This gives OSS maintainers insight into your reasoning process and builds trust in AI-assisted contributions.

## Quick Start

```bash
# Build the CLI
cargo build --release
cargo install --path .

# Work with your AI agent (automatically journals prompts)
cd ~/myproject
git checkout -b feature/auth-fix
# ... AI agent makes changes ...

# Extract prompts (outputs PR-ready markdown)
pmtx extract

# Copy output and paste into PR description
gh pr create --title "Fix auth validation" \
  --body "$(pmtx extract)"
```

## Commands

- **`pmtx extract`** - Extract prompts (smart defaults, outputs to stdout)
- **`pmtx extract --write [FILE]`** - Write to file instead of stdout
- **`pmtx extract --commits <N>`** - Extract for last N commits (fork workflow)
- **`pmtx extract --uncommitted`** - Only uncommitted changes
- **`pmtx record`** - Journal a prompt (agent-invoked automatically)
- **`pmtx status`** - Show project journal statistics
- **`pmtx projects`** - Manage tracked projects

## How It Works

1. **Automatic journaling**: Agent calls `pmtx record` after significant actions
2. **Home directory storage**: Journals stored in `~/.promptex/` (no project pollution)
3. **Privacy-first**: Sensitive values redacted immediately during journaling
4. **Smart scoping**: Analyzes git state (branch, commits, files) to determine relevance
5. **Correlation**: Matches journal entries to files in extraction scope
6. **Curation**: Filters, categorizes, deduplicates prompts
7. **PR format**: Outputs collapsible markdown for GitHub PR descriptions

## Why Use PromptEx?

- **Build trust**: Show maintainers your reasoning, not just code
- **Zero pollution**: No files in project directory, no .gitignore needed
- **Privacy-first**: Auto-redacts sensitive data during journaling
- **Git-aware**: Works with feature branches, fork workflows, commit-based scoping
- **PR-ready**: Default output is copy/paste into GitHub PR description
- **Fast**: Local processing, no API calls

## Documentation

- `PLAN.md`: Technical architecture and implementation plan
- `CLAUDE.md`: Development workflow notes

## Development

```bash
cargo build
cargo test
./target/debug/pmtx --help
```

## Status

🚧 **In active development** - Prompt extraction with agent integration
- Phase 1: CLI scaffold & project ID ✅
- Phase 2: Git analysis & smart scoping (next)
- Phase 3: Journaling & redaction
- Phase 4: Correlation & filtering
- Phase 5: Curation & categorization
- Phase 6: Output generation (PR format + detailed)
- Phase 7: Agent skill integration
- Phase 8: Polish & additional commands
