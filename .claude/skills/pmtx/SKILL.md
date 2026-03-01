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
  --outcome "<one sentence: what was accomplished>"
```

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

Run when the user wants PR-ready output, or at the end of a session:

```bash
pmtx extract [scope-flags]
```

Output is PR-ready markdown. Show it to the user in full.

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

### After extracting — offer next actions

- **Write to file**: `pmtx extract [flags] --write` → creates `PROMPTS.md`
- **Write to specific file**: `pmtx extract [flags] --write path/to/file.md`
- **Add to open PR**: `gh pr edit --body "$(pmtx extract [flags])"` *(confirm first — overwrites PR body)*
- **Nothing**: markdown was already shown, done

### Flag reference

| Flag | Effect |
|------|--------|
| `--since 2h` | Commits from the last duration (30m, 2h, 1d, 3w) |
| `--commits N` | Last N commits |
| `--since-commit HASH` | Since a specific commit (exclusive) |
| `--branch-lifetime` | Full feature branch since diverge point |
| `--uncommitted` | Uncommitted changes only |
| `--write [FILE]` | Write to `PROMPTS.md` or a named file |
