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
- **Exit 1**: your tool isn't supported yet. pmtx currently supports Claude Code and Codex CLI/Desktop.

If `pmtx` is not found, see [Troubleshooting](#troubleshooting) below.

---

## Extracting

When the user wants PR-ready output, or at the end of a session, run:

```bash
pmtx extract [scope-flags]
```

This outputs structured JSON containing the curated entries. You then:
1. **Categorize** entries into Investigation, Solution, or Testing
2. **Render** PR markdown using `references/rendering-rules.md`

### Empty-result handling

`pmtx extract` always returns JSON. If `entries` is an empty array, treat that as a successful extraction with no in-scope prompts.

For empty results:
- Do **not** treat this as unsupported-tool or parse failure.
- Do **not** generate or post empty PR markdown by default.
- Return a short summary with scope-widening suggestions (`--since 2h`, `--since 1d`, `--branch-lifetime`, `--since-commit <hash>`).
- Only post a placeholder comment if the user explicitly asks.

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

- **Investigation** — exploring or understanding (reading code, design questions, error analysis)
- **Solution** — implementing or changing behavior (edits, fixes, refactors, config)
- **Testing** — validating behavior (tests, checks, verification)

**`assistant_context`**: always use it when present, especially for short approvals ("yes", "go ahead") and mixed messages. It often disambiguates intent.

**Noise entries to drop:**
- Meta prompts about running pmtx itself (extract/summarize/invoke skill)
- Entries with no tool calls and no files touched, unless they contain meaningful design reasoning
- Near-duplicate prompts; keep the most recent version
- Short replies with no meaningful tool calls and no clear proposal in `assistant_context`
- Git/workflow housekeeping with no files touched (switch/push/merge/PR admin)

When in doubt, keep the entry. The user can always trim.

### Rendering

Follow `references/rendering-rules.md` for the full format spec, example output, and per-field rules.

### Security gate before writing or posting

Apply defense-in-depth redaction at the skill layer before writing or posting.

- Mask credential-like strings (tokens, keys, passwords, private keys, auth headers, session values).
- Mask secret-like env assignments (`*_TOKEN`, `*_KEY`, `*_SECRET`, `PASSWORD`, `AUTH`, `CREDENTIAL`).
- If suspicious values remain, do **not** auto-post; save locally and ask for confirmation.
- Follow `references/rendering-rules.md` posting safety rules.

Generate the markdown, then write it to `~/.promptex/projects/{id}/PROMPTS-YYYYMMDD-HHMM.md` using your file-writing tool. Do **not** render the full markdown in chat.

**After writing the file**, post it as a comment to the open PR:

```bash
gh pr view --json number -q '.number' 2>/dev/null && \
  gh pr comment --body-file ~/.promptex/projects/{id}/PROMPTS-YYYYMMDD-HHMM.md
```

Then confirm with a brief one-line summary in chat — not the full markdown:

```
✓ N prompts (X investigation, Y solution, Z testing) · Xh Ym · posted to PR #N
  Saved to ~/.promptex/projects/{id}/PROMPTS-YYYYMMDD-HHMM.md
```

If no open PR is detected, skip the comment step and tell the user:

```
✓ N prompts (X investigation, Y solution, Z testing) · Xh Ym
  Saved to ~/.promptex/projects/{id}/PROMPTS-YYYYMMDD-HHMM.md
  No open PR found — run `gh pr comment --body-file <path>` when ready.
```

**If the user wants to update the PR description instead of posting a comment** (confirm first — this overwrites the existing PR body):

```bash
gh pr edit --body-file ~/.promptex/projects/{id}/PROMPTS-YYYYMMDD-HHMM.md
```

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

**`entries` is empty (`"entries": []`)**
The scope's time window may not align with your session. Try:
- `--since 2h` or `--since 1d` to widen the search
- `--branch-lifetime` to capture the full branch history
- `--since-commit <hash>` to anchor the window manually
