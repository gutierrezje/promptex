---
name: prompt-history
description: Extracts and curates AI prompt history into PR-ready markdown using PromptEx (pmtx). Use when the user asks about prompt history, wants to document their AI-assisted reasoning for a PR, or asks to extract or summarize their session.
compatibility: Requires pmtx binary installed. Run `cargo install --path .` from https://github.com/gutierrezje/promptex or see the README for other install options.
metadata:
  author: gutierrezje
  version: 1.0.0
---

# PromptEx — Journal and Extract AI Prompt History

PromptEx (`pmtx`) reads AI tool session logs, correlates prompts to git changes in scope, and outputs structured JSON for agent-side rendering.

---

## On start

Run once to verify your tool is supported:

```bash
pmtx check
```

- **Exit 0**: your tool's logs are captured automatically. Run `pmtx extract` when ready.
- **Exit 1**: your tool isn't supported yet. pmtx currently supports Claude Code and Codex CLI/Desktop.

If `pmtx` is not found, see [Troubleshooting](#troubleshooting) below.

---

## Extracting

When the user wants PR-ready output, run:

```bash
pmtx extract [scope-flags]
```

This outputs structured JSON containing the curated entries. You then:
1. **Categorize and Filter** entries by modifying the JSON payload in memory. Add `"category": "Investigation"`, `"category": "Solution"`, `"category": "Testing"`, or `"category": "Ignore"` to each entry object.
2. **Format** the resulting JSON by piping it to `pmtx format > out.md` which will generate the PR markdown formatting for you.

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
| "Last hour / 2 days / 3 weeks" | `--since 1h` (or `2d`, `3w`) |
| "The whole branch" | `--branch-lifetime` |
| Unsure | Ask the user |

### Categorization

Assign each entry a `"category"` attribute in the JSON payload:

- **`Investigation`** — exploring or understanding (reading code, design questions, error analysis)
- **`Solution`** — implementing or changing behavior (edits, fixes, refactors, config)
- **`Testing`** — validating behavior (tests, checks, verification)
- **`Ignore`** — noise to drop completely from the final output

**`assistant_context`**: use it when need to disambiguate intent, especially for short approvals ("yes", "go ahead") and mixed messages. Edit to earliest complete sentence.

**Noise entries to ignore (mark as `"category": "Ignore"`):**
- Meta prompts about running pmtx itself (extract/summarize/invoke skill)
- Entries with no tool calls and no files touched, unless they contain meaningful design reasoning
- Near-duplicate prompts; keep the most recent version
- Short replies with no meaningful tool calls and no clear proposal in `assistant_context`
- Git/workflow housekeeping with no files touched (switch/push/merge/PR admin)

When in doubt, keep the entry (Investigation/Solution/Testing). The user can always trim.

### Formatting

Do not attempt to write the markdown formatting string manually. Once you've added standard `"category"` strings to the JSON, write that JSON to a temporary file or pipe it to the formatting engine:

```bash
cat categorized.json | pmtx format --out ~/.promptex/projects/{id}
```

The `pmtx format --out` command will automatically generate a file named `PROMPTS-YYYYMMDD-HHMM.md` and print its absolute path to stdout. Capture this output to use in subsequent commands.

### Security gate before writing or posting

Apply defense-in-depth redaction at the skill layer before writing or posting.

- Mask credential-like strings (tokens, keys, passwords, private keys, auth headers, session values).
- Mask secret-like env assignments (`*_TOKEN`, `*_KEY`, `*_SECRET`, `PASSWORD`, `AUTH`, `CREDENTIAL`).
- **NEVER** autonomously post to GitHub without explicit user confirmation. 
- **NEVER** even prompt the user to post to GitHub if you detect sensitive redacted values (`[REDACTED:... ]`) or suspicious raw credentials in the logs. If sensitive content is found, only save locally and explicitly warn the user.

Generate the markdown via `pmtx format --out ~/.promptex/projects/{id}` to automatically save it safely. Do **not** render the full markdown in chat.

**After writing the file**, you MUST prompt the user to confirm before posting to the open PR. Do not auto-post. Only if the user explicitly approves _and_ no sensitive data was flagged, post it as a comment:

```bash
# Assuming the path was saved in $PROMPT_FILE
gh pr view --json number -q '.number' 2>/dev/null && \
  gh pr comment --body-file $PROMPT_FILE
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
