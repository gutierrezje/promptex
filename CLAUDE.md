# PromptEx - Development Guidelines

## Project Overview
PromptEx (`pmtx`) is a **Rust CLI tool** that extracts and curates AI prompts for OSS contributions.

**Core Philosophy:** Help OSS contributors share their AI-assisted reasoning with maintainers by generating PR-ready prompt history.

## Architecture
This is a Rust CLI that:
1. **Journals** prompts to `~/.promptex/projects/<id>/journal.jsonl` (agent-invoked, auto-redacted)
2. **Analyzes** git state to determine extraction scope (branch, commits, files)
3. **Correlates** journal entries to files/commits in scope
4. **Curates** prompts (filters noise, categorizes, deduplicates)
5. **Outputs** PR-formatted markdown to stdout (or file with `--write`)

## Project Documentation
- **PLAN.md** - Full technical architecture and implementation phases
- **README.md** - User-facing documentation

## Runtime & Build System
Always use **cargo** for Rust development.

## Common Commands
- `cargo build` — build debug binary
- `cargo build --release` — build optimized binary
- `cargo run -- <args>` — run the CLI with arguments
- `cargo test` — run tests
- `./target/debug/pmtx` — run the built CLI directly

## Development Workflow
1. Make changes to Rust source files in `src/`
2. Build with `cargo build`
3. Test with `./target/debug/pmtx <command>`
4. Run tests with `cargo test`

## Current Status
✅ **Phase 1** - CLI scaffold (can reuse existing clap structure)
🎯 **Phase 2 (NEXT)** - Project ID & home directory storage + Git analysis
📋 **Upcoming** - Journaling, correlation, curation, output generation, agent integration

## Key Design Decisions
1. **PR format to stdout is default** - `pmtx extract` outputs PR-ready markdown, not a file
2. **Home directory storage** - `~/.promptex/projects/<id>/` to avoid project pollution
3. **Git-aware scoping** - Feature branches, fork workflows, commit-based extraction
4. **Privacy-first journaling** - Redact sensitive values immediately when journaling
5. **Agent-driven journaling** - `pmtx record` called by agent after tool use
6. **Commit-based correlation** - Prompts correlate to file changes, not just branches
7. **Local-only** - No API calls, no cloud sync, pure local processing
