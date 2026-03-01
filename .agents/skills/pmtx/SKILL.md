---
name: pmtx
description: Extract AI prompt history for pull requests using PromptEx (pmtx). Use this skill at the START of a coding session to verify your tool is supported. Use it AGAIN whenever the user asks to generate prompts for a PR, create a prompt history, add AI context to a pull request, share AI reasoning with maintainers, or run pmtx extract. Trigger phrases include: "add my prompts to the PR", "what prompts should I include?", "generate prompt summary", "extract my session", "show my AI reasoning".
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

If `pmtx` is not found, tell the user to install it:
- In the promptex repo: `cargo install --path .`
- Otherwise: see the project README for install instructions

---

## Extracting

When the user wants PR-ready output, or at the end of a session, run:

```bash
pmtx extract [scope-flags]
```

This outputs structured JSON containing the curated entries and a `format_spec`. You then:
1. **Categorize** each entry into one of three sections (see below)
2. **Render** the final PR markdown following the format spec

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

**Short replies with `assistant_context`**: when an entry's prompt is short (a bare "yes", "go ahead", "looks good"), the `assistant_context` field contains the tail of the preceding assistant turn — the proposal or question that was being approved. Use it to categorize these entries correctly. For example, "yes" with `assistant_context: "Should I refactor the auth module to use JWT?"` is a Solution approval, not noise.

**Noise entries to drop** (in addition to what pmtx already filtered):
- Short replies with no `assistant_context` and no meaningful tool calls
- Prompts that are purely clarifying questions with no implementation output

When in doubt, keep the entry. The user can always trim.

### Rendering format

Follow the `format_spec` from the JSON response. In detail:

```markdown
## 🤖 Prompt History

<details>
<summary>N prompts over Xh Ym - Click to expand</summary>

**Session Details**
- Tools: Claude Code (claude-sonnet-4-6) - 5 prompts, Cursor - 1 prompt
- Branch: `feature/auth-fix`
- Time range: 2024-01-15 14:00 - 2024-01-15 16:00
- Commits: `abc1234`, `def5678` (2 commits)
- Modified files: `src/auth.rs`, `src/lib.rs`

---

### 🔍 Investigation

**[14:05] (Claude Code · claude-sonnet-4-6)**
> explain how the JWT validation middleware works

→ Understood token structure and expiry logic

---

### 🔧 Solution

**[14:23] (Claude Code · claude-sonnet-4-6)**
> implement JWT expiry checking in src/auth.rs

→ Added validate_expiry() with 5-minute clock skew tolerance
→ Files: `src/auth.rs`, `src/middleware.rs`
→ Commit: `abc1234`

---

**Summary:** 6 prompts (2 investigation, 3 solution, 1 testing) · 1 tool

</details>

---

*Generated with [PromptEx](https://github.com/gutierrezje/promptex)*
```

Rules:
- Use `→` (not `->`) for outcome, files, and commit lines
- If `assistant_context` is present and its last sentence contains a `?`, add a `→ Re:` line after the blockquote (last sentence, capped at 120 chars): `→ Re: *"Want me to fix pr_format.rs with that approach?"*`. Omit the line if the context tail is a declarative statement — it's a non-sequitur and adds noise.
- Omit outcome line if `outcome` field is empty
- Omit files line if `files_touched` is empty
- Omit commit line if `commit` field is empty or not a hex hash ≥ 7 chars
- Show model in tool line only if `model` field is present: `(Tool · model)`
- Show at most 8 files in the Session Details modified files line; append `+N more` if there are more
- Omit empty category sections entirely
- Duration format: `< 1m`, `32m`, `1h 46m`, `2h`

### After rendering — write to file

After rendering in chat, write the markdown to `~/.promptex/projects/{id}/PROMPTS-YYYYMMDD-HHMM.md` and open it. Tell the user to select all, copy, and paste into their GitHub PR description.

Use `pmtx status` to find the project ID, then write the file and open it:
```bash
# then open (cross-platform):
open ~/.promptex/projects/{id}/PROMPTS-YYYYMMDD-HHMM.md        # macOS
xdg-open ~/.promptex/projects/{id}/PROMPTS-YYYYMMDD-HHMM.md   # Linux
start ~/.promptex/projects/{id}/PROMPTS-YYYYMMDD-HHMM.md       # Windows
```

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
