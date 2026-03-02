# PromptEx - Development Guidelines

## Project Overview
PromptEx (`pmtx`) is a **Rust CLI tool** that extracts and curates AI prompts for OSS contributions.

**Core Philosophy:** Help OSS contributors share their AI-assisted reasoning with maintainers by generating PR-ready prompt history.

## Architecture
This is a Rust CLI that:
1. **Extracts** prompts directly from AI tool session logs (Claude Code, Codex) — no manual recording
2. **Analyzes** git state to determine extraction scope (branch, commits, files)
3. **Correlates** log entries to files/commits in scope
4. **Curates** prompts (artifact filter, Jaccard deduplication)
5. **Outputs** structured JSON to stdout — the agent categorizes and renders the final markdown

## Project Documentation
- **PLAN.md** - Full technical architecture and implementation phases
- **README.md** - User-facing documentation

## Skill Files
The `prompt-history` skill ships in two locations:
- **`skills/prompt-history/`** — canonical version, committed to the repo, distributed via `npx skills add`
- **`.claude/skills/prompt-history/`** — local copy for Claude Code dogfooding

Always edit `skills/prompt-history/` first, then sync to `.claude/skills/prompt-history/`. Never edit the `.claude` copy directly — it will be overwritten on the next sync.

## Runtime & Build System
Always use **cargo** for Rust development.

## Common Commands
- `cargo build` — build debug binary
- `cargo build --release` — build optimized binary
- `cargo run -- <args>` — run the CLI with arguments
- `cargo fmt && cargo clippy -- -D warnings` — format and lint
- `cargo test` — run tests
- `./target/debug/pmtx` — run the built CLI directly

## Development Workflow
1. Make changes to Rust source files in `src/`
2. Build with `cargo build`
3. Test with `./target/debug/pmtx <command>`
4. Run `cargo fmt && cargo clippy -- -D warnings` before committing
5. Run tests with `cargo test`

## Current Status
All phases complete. CI enforces fmt, clippy (`-D warnings`), and tests on every push and PR.

## Key Design Decisions
1. **JSON output only** - `pmtx extract` always emits structured JSON; the agent renders markdown
2. **Home directory storage** - `~/.promptex/projects/<id>/` to avoid project pollution
3. **Git-aware scoping** - Feature branches, fork workflows, commit-based and time-based extraction
4. **Native extractors only** - Reads Claude Code and Codex logs directly; no manual recording
5. **Privacy-first** - Redacts secrets, tokens, and emails from all output
6. **Commit-based correlation** - Prompts correlate to file changes, not just branches
7. **Local-only** - No API calls, no cloud sync, Rust performance
