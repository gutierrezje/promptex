---
name: prompt-history
description: Extracts and curates AI prompt history into PR-ready markdown using PromptEx (pmtx). Reads AI tool session logs, correlates prompts to git scope, and renders categorized output for pull request descriptions. Use when the user asks about prompt history, wants to document their AI-assisted reasoning for a PR, or asks to extract or summarize their session.
compatibility: Requires pmtx binary installed. Run `cargo install --path .` from https://github.com/gutierrezje/promptex or see the README for other install options.
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

This outputs structured JSON containing the curated entries. You then:
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

**`assistant_context`**: the tail of the preceding assistant turn is always captured when one exists. Use it to improve categorization for any entry — not just short ones. It's especially valuable for bare confirmations ("yes", "go ahead", "looks good") and for hybrid messages that begin with approval before adding new context ("yes fix that. also check..."). For example, "yes" with `assistant_context` ending in "Should I refactor the auth module to use JWT?" is a Solution approval, not noise.

**Noise entries to drop:**
- Prompts invoking pmtx itself or asking to extract/summarize prompt history — these are meta, not development work (e.g. "extract my prompts", "add my prompts to the PR", the skill invocation turn)
- Entries with no tool calls and no files touched, unless the prompt shows significant deliberation — architectural questions, design tradeoffs, or reasoning that visibly shaped what came next are worth keeping even without artifacts
- Near-duplicate prompts (semantically the same ask, rephrased slightly) — keep the most recent version, which is usually the more refined one
- Short replies with no meaningful tool calls and no clear proposal in `assistant_context`

When in doubt, keep the entry. The user can always trim.

### Rendering

Follow `references/rendering-rules.md` for the full format spec, example output, and per-field rules.

After rendering in chat, generate a timestamp and write the markdown to `~/.promptex/projects/{id}/PROMPTS-YYYYMMDD-HHMM.md` using your file-writing tool rather than shell heredocs. Then open it:

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
