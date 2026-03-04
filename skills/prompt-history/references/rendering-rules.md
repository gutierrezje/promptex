# Rendering Rules

## Output format

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

---

### 🔧 Solution

**[14:23] (Claude Code · claude-sonnet-4-6)**
> implement JWT expiry checking in src/auth.rs

→ Files: `src/auth.rs`, `src/middleware.rs`
→ Commit: `abc1234`

---

**Summary:** 6 prompts (2 investigation, 3 solution, 1 testing) · 1 tool

</details>

---

*Generated with [PromptEx](https://github.com/gutierrezje/promptex)*
```

## Per-field rules

- Use `→` (not `->`) for files, commit, and Re: lines
- If `assistant_context` is present and its last sentence contains a `?`, add a `→ Re:` line after the blockquote (last sentence, capped at 120 chars): `→ Re: *"Want me to fix pr_format.rs with that approach?"*`. Omit if the context tail is a declarative statement — it adds noise. Before extracting the last sentence, strip any lines that consist entirely of backtick/dash separator characters (e.g. `` `─────────────────────────────────────────────────` ``).
- Omit files line if `files_touched` is empty
- Omit commit line if `commit` field is empty or not a hex hash ≥ 7 chars
- Show model in tool line only if `model` field is present: `(Tool · model)`
- Show at most 8 files in the Session Details modified files line; append `+N more` if there are more
- Omit empty category sections entirely
- Duration format: `< 1m`, `32m`, `1h 46m`, `2h`
