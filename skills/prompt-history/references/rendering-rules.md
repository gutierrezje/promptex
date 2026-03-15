# Rendering Rules

## Output format

```markdown
## Prompt History

<details>
<summary>N prompts over Xh Ym</summary>

**Session Details**
- Tools: Tool A (model) - N prompts, Tool B - M prompts
- Branch: `feature/example`
- Time range: YYYY-MM-DD HH:MM - YYYY-MM-DD HH:MM
- Commits: `abc1234`, `def5678` (N commits)
- Modified files: `path/a`, `path/b`

---

### Investigation / Solution / Testing

**[HH:MM] (Tool · model)**
> prompt text

→ Re: *"Question from preceding assistant turn?"*
→ Files: `path/a`, `path/b`
→ Commit: `abc1234`

---

**Summary:** N prompts (I investigation, S solution, T testing) · K tools

</details>

---

*Generated with [PromptEx](https://github.com/gutierrezje/promptex)*
```

## Per-field rules

- Use `→` (not `->`) for files, commit, and Re: lines
- Never include plaintext credentials. Redact credential-like values as `[REDACTED]`.
- If uncertain whether a value is sensitive, redact it.
- If `assistant_context` is present and its last sentence contains a `?`, add `→ Re:` after the blockquote (last sentence, max 120 chars). Omit for declarative context. Before extracting the last sentence, strip separator-only lines (e.g. `` `─────────────────────────────────────────────────` ``).
- Omit files line if `files_touched` is empty
- Omit commit line if `commit` field is empty or not a hex hash ≥ 7 chars
- Show model in tool line only if `model` field is present: `(Tool · model)`
- Show at most 8 files in the Session Details modified files line; append `+N more` if there are more
- Omit empty category sections entirely
- Duration format: `< 1m`, `32m`, `1h 46m`, `2h`

## Posting safety

- Perform a final scan of the completed markdown before writing and again before posting to GitHub.
- If unresolved credential-like text remains, do not auto-post. Save locally and ask the user to confirm or sanitize first.
- If input JSON has `"entries": []`, skip markdown rendering and posting by default; report a concise no-in-scope-prompts result instead.
