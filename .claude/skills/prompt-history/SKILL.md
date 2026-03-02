---
name: prompt-history
description: Extract AI prompt history for pull requests using PromptEx (pmtx). Use when the user says "extract prompts", "extract my prompts", "extract prompts from last commit", "extract prompts from branch", "generate prompt history", "add my prompts to the PR", "what prompts should I include?", "generate prompt summary", "extract my session", or "show my AI reasoning". Also use at session start to verify tool support.
compatibility: Requires pmtx binary installed. Run `cargo install --path .` from the promptex repo, or see README for other install options.
metadata:
  author: gutierrezje
  version: 1.0.0
---

# PromptEx — Journal and Extract AI Prompt History

PromptEx (`pmtx`) reads AI tool session logs, correlates prompts to git changes in scope, and outputs structured JSON for agent-side rendering.

---

## On session start

Run once to verify your tool is supported:

```bash
pmtx check
```

- **Exit 0**: your tool's logs are captured automatically. Run `pmtx extract` when ready.
- **Exit 1**: your tool isn't natively supported yet. pmtx can only extract from supported tools (Claude Code, Codex).

If `pmtx` is not found, see [Troubleshooting](#troubleshooting) below.

---

## Extracting

When the user wants PR-ready output, or at the end of a session, run:

```bash
pmtx extract [scope-flags]
```

This outputs structured JSON containing the curated entries and a `format_spec`. You then:
1. **Categorize** each entry into one of three sections (see below)
2. **Render** the final PR markdown — see `references/rendering-rules.md` for the full format spec

### Choosing a scope

If the user provided flags, use them. Otherwise infer from context:

| Situation | Flag |
|-----------|------|
| Feature branch | *(no flag — smart default)* |
| Mainline + uncommitted changes | `--uncommitted` |
| Mainline, last commit only | `--commits 1` |
| "Last hour / 2 hours / today" | `--since 2h` (or `1h`, `1d`, `3w`) |
| "The whole branch" | `--branch-lifetime` |
| Unsure | Ask the user |

### Categorization

Assign each entry to the most fitting section:

- **🔍 Investigation** — exploring, understanding, researching: reading code, explaining a design, asking what something does, looking into an error, comparing approaches
- **🔧 Solution** — implementing, fixing, changing: writing code, editing files, refactoring, debugging a fix, configuring something
- **✅ Testing** — verifying, validating: running tests, checking output, confirming a fix works, writing test cases

**Short replies with `assistant_context`**: when an entry's prompt is short (a bare "yes", "go ahead", "looks good"), the `assistant_context` field contains the tail of the preceding assistant turn — the proposal or question that was being approved. Use it to categorize correctly. For example, "yes" with `assistant_context: "Should I refactor the auth module to use JWT?"` is a Solution approval, not noise.

**Noise entries to drop** (in addition to what pmtx already filtered):
- Short replies with no `assistant_context` and no meaningful tool calls
- Prompts that are purely clarifying questions with no implementation output

When in doubt, keep the entry. The user can always trim.

### Rendering

Follow `references/rendering-rules.md` for the full format spec, example output, and per-field rules.

After rendering in chat, write the markdown to `~/.promptex/projects/{id}/PROMPTS-YYYYMMDD-HHMM.md` and open it:

```bash
pmtx status   # find the project ID
open ~/.promptex/projects/{id}/PROMPTS-YYYYMMDD-HHMM.md        # macOS
xdg-open ~/.promptex/projects/{id}/PROMPTS-YYYYMMDD-HHMM.md   # Linux
start ~/.promptex/projects/{id}/PROMPTS-YYYYMMDD-HHMM.md       # Windows
```

Tell the user to select all, copy, and paste into their GitHub PR description.

Other options if the user asks:
- **Add to open PR directly**: `gh pr edit --body-file ~/.promptex/projects/{id}/PROMPTS-YYYYMMDD-HHMM.md` *(confirm first — overwrites PR body)*

### Flag reference

| Flag | Effect |
|------|--------|
| `--since 2h` | Commits from the last duration (30m, 2h, 1d, 3w) |
| `--commits N` | Last N commits |
| `--since-commit HASH` | Since a specific commit (exclusive) |
| `--branch-lifetime` | Full feature branch since diverge point |
| `--uncommitted` | Uncommitted changes only |

---

## Troubleshooting

**`pmtx: command not found`**
Install the binary first:
- In the promptex repo: `cargo install --path .`
- Otherwise: see the project README for install instructions

**`0 entries` or no output after filtering**
The scope's time window may not align with your session. Try:
- `--since 2h` or `--since 1d` to widen the search
- `--branch-lifetime` to capture the full branch history
- `--since-commit <hash>` to manually anchor the window
