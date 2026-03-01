---
name: pmtx
description: Journal and extract AI prompt history for pull requests using PromptEx (pmtx). Use this skill at the START of a coding session to detect whether prompts are captured automatically or must be journaled manually. Use it AGAIN whenever the user asks to generate prompts for a PR, create a prompt history, add AI context to a pull request, share AI reasoning with maintainers, or run pmtx extract. Trigger phrases include: "add my prompts to the PR", "what prompts should I include?", "generate prompt summary", "extract my session", "show my AI reasoning".
---

# PromptEx — Journal and Extract AI Prompt History

PromptEx (`pmtx`) reads AI tool session logs (or a manual journal), correlates prompts to git changes in scope, and outputs PR-ready markdown.

---

## On session start — detect recording mode

Run this once at the beginning of a coding session:

```bash
pmtx check
```

- **Exit 0 — native support**: your AI tool's logs are captured automatically. No journaling needed. Skip to [Extracting](#extracting) when ready.
- **Exit 1 — no native support**: prompts must be recorded manually after each significant action. Follow [Recording](#recording) below throughout the session.

If `pmtx` is not found, tell the user to install it:
- In the promptex repo: `cargo install --path .`
- Otherwise: see the project README for install instructions

---

## Recording

*Only needed when `pmtx check` exits 1 (non-native tool).*

After each significant action — completing a task, writing code, fixing a bug, running tests — call:

```bash
pmtx record \
  --prompt "<the user's prompt or your summary of the task>" \
  --files "<comma-separated list of files touched, e.g. src/main.rs,src/lib.rs>" \
  --tool-calls "<comma-separated tools used, e.g. Edit,Bash,Read>" \
  --outcome "<one sentence: what was accomplished>" \
  --tool "<your tool slug, e.g. opencode, cursor, copilot>" \
  --model "<model identifier, e.g. gpt-4o, gemini-2.0-flash>"
```

`--tool` defaults to `claude-code` if omitted. `--model` is optional but recommended for attribution. Omit both if the defaults are correct for your tool.

**What counts as a "significant action":**
- Writing or editing source code
- Running tests or build commands
- Debugging and fixing an error
- Researching or explaining a design decision

**What to capture:**
- `--prompt`: the user's original request, or a concise description of the task
- `--files`: all files that were created, edited, or read as part of this action
- `--tool-calls`: the tools or capabilities used (Edit, Bash, Read, Search, etc.)
- `--outcome`: a single sentence describing what was done or decided

Skip trivial actions (status checks, reading unrelated files, clarifying questions with no code output).

---

## Extracting

When the user wants PR-ready output, or at the end of a session, use the `--json` flag so you can categorize entries semantically before rendering:

```bash
pmtx extract [scope-flags] --json
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

Run `pmtx extract [scope-flags] --write` to save the markdown to `~/.promptex/projects/{id}/PROMPTS.md`. pmtx will print the path and automatically open the file in the system default editor. Tell the user to select all, copy, and paste into their GitHub PR description.

If `--write` is not used (agent rendered in chat), write the file yourself and open it:
```bash
# write
# then open (cross-platform):
open ~/.promptex/projects/{id}/PROMPTS.md        # macOS
xdg-open ~/.promptex/projects/{id}/PROMPTS.md   # Linux
start ~/.promptex/projects/{id}/PROMPTS.md       # Windows
```

Other options if the user asks:
- **Specific path**: `pmtx extract [flags] --write path/to/file.md`
- **Add to open PR directly**: `gh pr edit --body "$(pmtx extract [flags])"` *(confirm first — overwrites PR body)*

### Flag reference

| Flag | Effect |
|------|--------|
| `--json` | Output structured JSON for agent categorization (use this when running via skill) |
| `--since 2h` | Commits from the last duration (30m, 2h, 1d, 3w) |
| `--commits N` | Last N commits |
| `--since-commit HASH` | Since a specific commit (exclusive) |
| `--branch-lifetime` | Full feature branch since diverge point |
| `--uncommitted` | Uncommitted changes only |
| `--write [FILE]` | Write to `~/.promptex/projects/{id}/PROMPTS.md` or a named file (incompatible with `--json`) |
