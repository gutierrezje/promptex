# Issuance - Development Guidelines

## Project Overview
Issuance is currently a **Rust CLI tool** that orchestrates context for AI-assisted open source contributions.

## Architecture
This project contains the **Rust** code for the CLI implementation.

## Project Documentation
- **PLAN.md** - Full technical architecture and plan

## Learning Mode (Default)
- Implement basic plumbing directly.
- Leave critical, educational opportunity, interesting parts unimplemented with guided TODOs.
- Each TODO should include: purpose, inputs, outputs, edge cases, and suggested tests.

## Runtime & Build System
Always use **cargo** for Rust development.

## Common Commands
- `cargo build` — build debug binary
- `cargo build --release` — build optimized binary
- `cargo run -- <args>` — run the CLI with arguments
- `cargo test` — run tests
- `./target/debug/issuance` — run the built CLI directly

## Recommended Tools
- `rust-analyzer` (editor integration)

## Development Workflow
1. Make changes to Rust source files in `src/`
2. Build with `cargo build`
3. Test with `./target/debug/issuance <command>`
4. Run tests with `cargo test`

## Current Status
✅ **Phase 1 Complete** - CLI scaffold with working command structure
🚧 **Phase 2 In Progress** - Core context generation pipeline
