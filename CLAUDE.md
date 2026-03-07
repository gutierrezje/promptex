# PromptEx - Development Guidelines

## Project Overview
PromptEx (`pmtx`) is a **Rust CLI tool** that extracts and curates AI prompts for OSS contributions.

**Core Philosophy:** Help OSS contributors share their AI-assisted reasoning with maintainers by generating PR-ready prompt history.

## Architecture
This is a Rust CLI that:
1. **Extracts** prompts directly from AI tool session logs (Claude Code, plus WIP Codex support) — no manual recording
2. **Analyzes** git state to determine extraction scope (branch, commits, files)
3. **Correlates** log entries to files/commits in scope
4. **Curates** prompts (artifact filter, Jaccard deduplication)
5. **Outputs** structured JSON to stdout — the agent categorizes and renders the final markdown

## Project Documentation
- **PLAN.md** - Full technical architecture and implementation phases
- **README.md** - User-facing documentation

## Skill Files
The `prompt-history` skill lives in `skills/prompt-history/` — committed to the repo and distributed via `npx skills add`.

**Dogfooding workflow:**
- **First time only:** install the skill globally (agent-agnostic, lands in `~/.agents/skills/`):
  ```bash
  npx skills add prompt-history -g
  ```
- **After any edits:** sync changes from source directly into the global install:
  ```bash
  ./scripts/sync-skill.sh
  # or manually: cp -r skills/prompt-history/ ~/.agents/skills/prompt-history/
  ```

Global skill installs live in `~/.agents/skills/` — outside the project, never committed.

## Runtime & Build System
Always use **cargo** for Rust development.

> **Important:** Never invoke `pmtx` (the PATH binary) directly — it is `~/.cargo/bin/pmtx`
> and is only updated via `cargo install --path .`. It can silently lag behind the source.
> Always use `./target/debug/pmtx` or `cargo run -- <args>` to ensure you're running the
> latest code.

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
6. After editing `skills/`, run `./scripts/sync-skill.sh` to sync to the global install

## Git Conventions
- **Commit messages are single-line** — detail belongs in the PR body or comments, not the commit message
- Use the `fix:` / `chore:` / `feat:` / `refactor:` prefix convention
- **Merge strategy: rebase merge** — preserves individual commits linearly on main, no merge nodes; use squash only for genuinely messy WIP branches

## Current Status
All phases complete. CI enforces fmt, clippy (`-D warnings`), and tests on every push and PR.

## Key Design Decisions
1. **JSON output only** - `pmtx extract` always emits structured JSON; the agent renders markdown
2. **Home directory storage** - `~/.promptex/projects/<id>/` to avoid project pollution
3. **Git-aware scoping** - Feature branches, fork workflows, commit-based and time-based extraction
4. **Direct log extractors** - Reads Claude Code logs directly and includes WIP Codex extraction; no manual recording
5. **Privacy-first** - Redacts secrets, tokens, and emails from all output
6. **Commit-based correlation** - Prompts correlate to file changes, not just branches
7. **Local-only** - No API calls, no cloud sync, Rust performance
