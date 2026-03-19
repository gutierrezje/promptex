# PromptEx guidelines for Gemini

## 1. The Surgical Execution Rule (Anti-Refactor)
- **Scope Restriction:** When asked to fix a bug or add a minor feature, modify ONLY the specific lines required. 
- **No Unprompted Refactoring:** Do not rewrite surrounding architecture, "clean up" existing logic, or alter working code unless explicitly requested.
- **Architectural Pause:** If you think a minor fix requires a major structural change to satisfy the borrow checker, or some other constraint, STOP. Provide a summary of the required architectural change and wait for approval.

## 2. Strict Modularity (Anti-Lump)
- **Modular design/Separation of Concerns:** Don't always just dump all logic into a single file like `main.rs` or `mod.rs`. If it seems like a file is getting too big, or if a new struct/trait has distinct behavior, extract it into a new module. Plan this out before you start coding, think hard about organization and quality.

## 3. Testing Standards
- **Colocated Unit Tests:** Write unit tests at the bottom of the exact file they are testing using `#[cfg(test)]`. If need to implement helper functions for tests, create them within the same test module, unless they can be reused by other test modules.
- **Test Quality:** Do not write shallow, happy-path tests just to achieve coverage. Test edge cases, handle expected failures (using `#[should_panic]` or checking `Result::Err`), and mock standard I/O streams when testing the command-line outputs. Look at the existing tests for inspiration and guidance.

## 4. Reliability and Verification (Anti-Hallucination)
- **No Guessing:** Do not invent behavior, APIs, flags, or file paths. If uncertain, inspect the actual codebase first and cite the file(s) you relied on.
- **Scope Lock:** Before coding, restate the exact requested scope in one short sentence and keep changes strictly within that scope.
- **Ambiguity Rule:** If ambiguity materially changes the implementation, ask one focused question only after completing all non-blocked work.
- **Verify Before Claiming Done:** Run relevant checks before finalizing (`cargo fmt`, `cargo clippy -- -D warnings`, `cargo test`, or the project's equivalent).
- **Transparent Final Report:** Summarize each changed file with why it changed, list verification commands run, and call out anything not verified.
