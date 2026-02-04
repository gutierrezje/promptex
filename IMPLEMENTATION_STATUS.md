# Issuance - Implementation Status

## ✅ Phase 1: CLI Scaffold (COMPLETE)

**Goal:** Basic CLI structure with working commands

### Completed Features

1. **Cargo Project Setup**
   - Created `Cargo.toml` with all required dependencies
   - Configured binary package with proper metadata

2. **CLI Framework (clap)**
   - Main CLI structure with subcommands
   - Three commands: `grab`, `profile`, `clean`
   - Help text and version info
   - Command-line argument parsing

3. **Config Module**
   - Configuration loading from `~/.issuance/config.toml`
   - Default values for settings
   - GitHub token support
   - Configurable defaults (shallow clone, PR limit)

4. **Command Stubs**
   - `issuance grab <url>` - Placeholder for fetching issues
   - `issuance profile <repo>` - Placeholder for repo analysis
   - `issuance clean` - Fully functional cleanup command

### Test Results

```bash
$ cargo build
✅ Builds successfully in ~33s (first build)

$ ./target/debug/issuance --help
✅ Shows proper help text

$ ./target/debug/issuance grab https://github.com/fastapi/fastapi/issues/1284
✅ Parses URL argument correctly

$ ./target/debug/issuance profile fastapi/fastapi
✅ Parses repo argument correctly

$ ./target/debug/issuance clean
✅ Works correctly
```

### File Structure Created

```
src/
├── main.rs              ✅ Entry point with clap setup
├── config.rs            ✅ Config loading logic
└── commands/
    ├── mod.rs           ✅ Module exports
    ├── grab.rs          ✅ Placeholder implementation
    ├── profile.rs       ✅ Placeholder implementation
    └── clean.rs         ✅ Full implementation
```

---

## 🚧 Phase 2: `issuance grab` - Core Pipeline (IN PROGRESS)

**Goal:** Full context pack generation with 5 files

### Planned Components

1. **GitHub Service** (`src/services/github.rs`)
   - Fetch issues with comments
   - Fetch recent commits
   - Find related issues
   - Clone repositories (shallow)
   - Get CI status

2. **Tools Service** (`src/services/tools.rs`)
   - Detect project type (Python, TypeScript, Go, Rust)
   - Run language-specific linters
   - Discover tests
   - Parse CI config

3. **Extractor Service** (`src/services/extractor.rs`)
   - Extract keywords from issue text
   - Find mentioned files
   - Parse stack traces
   - Identify relevant code locations

4. **Generator Service** (`src/services/generator.rs`)
   - Render templates with Tera
   - Generate all 5 context files

5. **Templates** (`src/templates/*.tera`)
   - `issue.md.tera` - Ground truth (raw GitHub data)
   - `codemap.md.tera` - File mapping with tool output
   - `signals.md.tera` - Ambient context (commits, CI, related issues)
   - `rules.md.tera` - Contribution guidelines
   - `handoff.md.tera` - LLM entry point

### Output Structure

```
.issuance/
├── ISSUE.md      # Raw GitHub issue + comments
├── CODEMAP.md    # Suspected files + tool output
├── SIGNALS.md    # Recent commits, related issues, CI status
├── RULES.md      # Contribution conventions
└── HANDOFF.md    # LLM-facing instructions
```

---

## 📋 Phase 3: `issuance profile` (TODO)

**Goal:** Standalone repo culture analysis

### Planned Features

- Fetch last 50 merged PRs
- Analyze commit conventions
- Extract test requirements
- Review patterns analysis
- Parse CONTRIBUTING.md
- Parse CI config files

---

## 🎨 Phase 4: Polish (TODO)

**Goal:** Production-ready CLI

### Planned Features

- Rich terminal output with progress bars
- Colored file summaries
- Tables for displaying signals
- Better error messages
- Integration tests
- Documentation

---

## Next Steps

To continue implementation:

1. **Create services directory structure**
   ```bash
   mkdir -p src/services
   touch src/services/mod.rs
   touch src/services/github.rs
   touch src/services/tools.rs
   touch src/services/extractor.rs
   touch src/services/generator.rs
   ```

2. **Create templates directory**
   ```bash
   mkdir -p src/templates
   touch src/templates/mod.rs
   touch src/templates/issue.md.tera
   touch src/templates/codemap.md.tera
   touch src/templates/signals.md.tera
   touch src/templates/rules.md.tera
   touch src/templates/handoff.md.tera
   ```

3. **Implement GitHub service first**
   - This is the foundation - all other services depend on it
   - Use `octocrab` crate for GitHub API
   - Start with simple issue fetching

4. **Build incrementally**
   - Get basic issue fetching working
   - Add comments
   - Add signals
   - Add related issues
   - Add CI status

---

## Timeline Estimate

- ✅ Phase 1: Complete (~2 hours)
- 🚧 Phase 2: 2-3 days
- 📋 Phase 3: 1-2 days
- 🎨 Phase 4: 1-2 days

**Total: ~6-9 days remaining**
