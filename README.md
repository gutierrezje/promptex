# Issuance

Issuance is a Rust CLI that orchestrates high-signal context for AI-assisted open source contributions.

## Status
- Phase 1: CLI scaffold complete
- Phase 2: Context pack generation in progress

## Quick Start
```bash
cargo build
./target/debug/issuance --help
```

## Commands
- `issuance grab <issue-url>`: Generate a context pack for a GitHub issue
- `issuance clean`: Remove `.issuance/`

## Output
`issuance grab` generates a context pack in `.issuance/`:
- `ISSUE.md`
- `CODEMAP.md`
- `SIGNALS.md`
- `RULES.md`
- `HANDOFF.md`
- `metadata.json`

## Local Agentic CLI
`grab` is designed to invoke a local agentic CLI (Claude Code) by default to synthesize `RULES.md`
from deterministic inputs. No paid API calls.

## Documentation
- `PLAN.md`: Internal engineering plan and architecture
- `CLAUDE.md`: Agent/dev workflow notes

## Development
```bash
cargo build
cargo test
```

## Recommended Tools
- `rust-analyzer` (editor integration)
