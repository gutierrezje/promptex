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
- `issuance grab <issue-url> [directory]`: Clone repository (like `git clone`) and generate issue context
- `issuance clean`: Remove `.issuance/`

## Output
`issuance grab` generates project-level and issue-level context:
- `.issuance/` (project-level context and shared assets)
- `.issuance/issues/<issue-number>/` (issue-scoped context files)

Project-level files:
- `RULES.md`

Issue-scoped files:
- `ISSUE.md`
- `CODEMAP.md`
- `SIGNALS.md`
- `HANDOFF.md`
- `NEXT.md`
- `PROMPTS.md` (after extraction)
- `metadata.json`

## Local Agentic CLI
`grab` is designed to invoke a local agentic CLI (OpenCode) by default to synthesize `RULES.md`
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
