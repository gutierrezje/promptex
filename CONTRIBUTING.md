# Contributing to PromptEx

Thanks for your interest in contributing. This is a focused Rust CLI — contributions that stay true to its core design (local-only, privacy-first, zero setup) are especially welcome.

## Getting started

```bash
git clone https://github.com/gutierrezje/promptex
cd promptex
cargo build
cargo test
./target/debug/pmtx --help
```

## Finding something to work on

Check the [open issues](https://github.com/gutierrezje/promptex/issues) — issues labelled `good first issue` are a good starting point.

## Making changes

1. Fork the repo and create a feature branch
2. Make your changes in `src/`
3. Run `cargo fmt && cargo clippy -- -D warnings` — CI will enforce both
4. Run `cargo test` to make sure nothing is broken
5. Open a PR against `main`

## Commit style

- Single-line commit messages — detail belongs in the PR body
- Use the `fix:` / `feat:` / `chore:` / `refactor:` prefix convention

## PR workflow

This repo uses **rebase merge** to keep a linear history on `main`. Keep your branch rebased on `main` before opening a PR.

## Architecture notes

Before diving in, skim `ARCHITECTURE.md` for the full technical architecture and `CLAUDE.md` for development guidelines. Key constraints:
- `pmtx extract` always outputs JSON to stdout — never render markdown in the binary
- No network requests — everything is local
- All persistent state in `~/.promptex/` — nothing written to the project directory
- Secrets, tokens, and emails must be redacted before any output
