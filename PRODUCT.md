# Issuance - Product Vision

## Mission

**Help students gain real development experience through guided open source contributions.**

We believe the best way to learn software development is by working on real codebases, with real constraints, solving real problems. But the gap between "I finished a tutorial" and "I can contribute to production code" is massive. Issuance bridges that gap.

---

## The Problem

### For Students
- **Need experience to get jobs**, but can't get jobs without experience
- **Tutorials feel fake** - contrived problems, no real users, no stakes
- **Open source is intimidating** - don't know where to start, fear embarrassment
- **AI tools are overwhelming** - too much context, no guidance on what matters
- **First contributions often fail** - wrong tone, missing tests, don't follow conventions

### For Open Source Projects
- **Need contributors**, but get flooded with low-quality PRs
- **Students mean well**, but lack context on project norms
- **"I can help with this!" comments** add noise without value
- **Maintainer time is precious** - can't onboard every newcomer

### The Gap
There are millions of students who want to contribute and thousands of open source projects that need help, but **there's no structure to connect them effectively.**

---

## The Solution

**Issuance is your mentor for open source contributions.**

Think of it like having a senior developer who:
- Shows you issues that match your interests and skill level
- Sets up your development environment and gathers context
- Prepares everything so your AI tools (Claude Code, Cursor, etc.) can guide you effectively
- Reviews your work before you submit to prevent embarrassing mistakes
- Teaches you professional habits through real-world practice

### It's NOT:
❌ An AI that does the work for you
❌ A job board or freelance platform
❌ Just an issue tracker
❌ A productivity tool for senior developers

### It IS:
✅ An educational platform for learning by doing
✅ A context layer that makes AI tools effective for OSS
✅ A quality gate that builds professional habits
✅ A confidence builder through structured real-world experience

---

## Who It's For

### Primary: Students & Early-Career Developers
- Computer science students (all levels)
- Bootcamp graduates building portfolios
- Self-taught developers seeking real-world experience
- Junior developers wanting to level up
- Career switchers proving their skills

### Secondary: Open Source Projects
- Projects that want high-quality student contributions
- Maintainers tired of low-quality "I can help!" comments
- Communities building educational pathways

---

## How It Works

### The Student Journey

#### 1. **Discovery** (Web App)
Browse a curated directory of contribution opportunities, like an assignment board at your first job:

```
🎯 Your Opportunities

[Frontend] React - Add loading spinner to Suspense fallback
⭐ 35k stars  |  🎓 Good for beginners  |  ⏱️ 2-3 hours
Skills: React, TypeScript, CSS
What you'll learn: React internals, component composition
👥 3 others learning from this project

[Backend] Fastify - Fix CORS preflight handling
⭐ 28k stars  |  🎓 Intermediate  |  ⏱️ 4-6 hours
Skills: Node.js, HTTP, Testing
What you'll learn: HTTP protocols, middleware architecture
👥 5 others learning from this project
```

**Key features:**
- Curated repos (~1000) known to be welcoming to contributors
- Issues tagged by skill level, technology, and learning outcomes
- Time estimates so you can plan around school/work
- Social proof (see others learning from the same project)
- Educational feed showing successful contribution stories

#### 2. **Setup** (CLI)
When you pick an issue, the CLI acts like a senior dev on your first day:

```bash
$ issuance start facebook/react/issues/28361

👋 Welcome! Let's get you set up for this issue.

📋 Issue #28361: Memory leak in useEffect cleanup
   Reporter: @developer123  |  Labels: bug, confirmed
   Maintainer says: "Confirmed in React 18.2+"

🎓 What you'll learn:
   - React internals (Fiber reconciliation)
   - Memory management in JavaScript
   - Writing regression tests

⏱️  Estimated time: 4-6 hours

📦 Setting up the codebase...
   ✓ Cloned facebook/react
   ✓ Installed dependencies
   ✓ Built React packages
   ✓ Ran existing tests (all passing ✓)

🔍 Context summary:
   - 47 comments in discussion
   - Maintainer @gaearon confirmed the bug
   - Reproduction steps provided
   - Related to concurrent mode rendering

📁 Files you'll likely work with:
   1. packages/react-reconciler/src/ReactFiberHooks.js
      (Where useEffect cleanup happens)
   2. packages/react-reconciler/src/__tests__/ReactHooks-test.js
      (Where you'll add a regression test)

📚 React project conventions:
   ✓ All bug fixes require regression tests
   ✓ Use Conventional Commits for PR titles
   ✓ Run full test suite before submitting
   ✓ Reference issue number in commit message

🤖 Context prepared for your AI assistant.

   Ready to start? Try:
   $ claude-code "Help me understand issue #28361"

   Or open your editor - context is in .issuance/context.json

Good luck! When you're ready to submit:
   $ issuance review
```

**What the CLI does:**
- Clones repo and sets up dev environment
- Fetches and analyzes the issue + discussion
- Identifies likely files you'll need to modify
- Extracts project conventions and contribution guidelines
- Generates structured context for AI tools
- Explains what you'll learn (educational framing)

**What it DOESN'T do:**
- Write code for you
- Make decisions for you
- Hide complexity (you need to learn it)

#### 3. **Implementation** (Your AI Tool + You)
Use your preferred AI coding assistant (Claude Code, Cursor, Copilot, etc.) with the rich context Issuance prepared:

- Your AI tool receives structured context about the issue
- You work together to understand the codebase
- You learn by implementing the fix yourself (with AI guidance)
- You write tests and follow project conventions
- You gain real experience with real constraints

**The AI is your pair programming partner. Issuance is the mentor who set up the pairing session.**

#### 4. **Quality Gate** (CLI)
Before you submit, Issuance reviews your work like a senior developer:

```bash
$ issuance review

📝 Reviewing your changes before you submit...

✅ Good work:
   ✓ You added a regression test (React requires this)
   ✓ All tests are passing
   ✓ You followed commit message format
   ✓ You referenced issue #28361

⚠️  Before you submit, consider these improvements:

1. Certainty Check (Epistemic Humility)
   Your PR says: "This fixes the root cause"
   Consider: "This might fix the issue by preventing cleanup on reused fibers"

   Why? You're new to this codebase. Showing humility builds trust
   with maintainers and protects you if the fix is incomplete.

2. Evidence Check (Show Your Work)
   Missing: How did you test this?
   Add: "I reproduced the issue using the steps from #28361,
        applied this fix, and verified the memory leak no longer occurs.
        I also ran the full test suite (all passing)."

   Why? Maintainers trust contributors who show their testing process.

3. Tone Match (Project Culture)
   React's style: Formal and technical
   Your tone: Slightly casual

   Suggestion: Review 2-3 recent merged PRs to match their tone.

📊 Quality Score: 7.5/10 (Strong, with minor improvements)

🎓 Professional Assessment:
   This would likely get accepted. The suggestions above will:
   - Increase merge probability from ~70% to ~90%
   - Build credibility for future contributions
   - Demonstrate professional communication skills

Ready to submit? [y/N]
```

**Quality gate checks:**
- **Epistemic humility**: Are you overconfident? ("This fixes..." → "This might fix...")
- **Evidence**: Did you show your work? (Testing process, reproduction results)
- **Tone matching**: Does your communication fit the project culture?
- **Convention compliance**: Tests included? Proper format? References issue?
- **Value-add**: Does your comment/PR add new information?

**This is where the learning happens:**
- Prevents public embarrassment
- Teaches professional communication
- Builds good habits before bad ones form
- Protects your reputation as a contributor

#### 5. **Learn & Iterate**
After submission, track your progress:

```
🎉 PR submitted to facebook/react!

📈 Your Progress:
   - Contributions: 5 PRs (3 merged, 2 in review)
   - Projects: React, Vitest, Fastify
   - Skills practiced: TypeScript, Testing, HTTP APIs
   - Avg. review time: 2.3 days
   - Quality score trend: 6.2 → 7.1 → 7.8 → 8.2 → 7.5

💡 Learning from this contribution:
   - You improved at writing tests (+0.8)
   - Consider: More humble language in PRs
   - Next challenge: Try an intermediate-level issue

🏆 Achievements Unlocked:
   ✓ First Merged PR
   ✓ Test Writer (3 PRs with comprehensive tests)
   ✓ Multi-Project Contributor (3+ projects)
```

Feed shows case studies of successful contributions so you can learn patterns.

---

## Core Features

### 1. Web App: Your Assignment Board

**Curated Repository Directory**
- ~1000 repos vetted for:
  - Active maintenance (merged PR in last 30 days)
  - Welcoming to contributors (clear CONTRIBUTING.md, responsive maintainers)
  - Educational value (diverse technologies, clear issues)
  - Appropriate complexity (good first issues through advanced challenges)

**Smart Filtering**
- By technology (React, Python, Rust, etc.)
- By skill level (beginner, intermediate, advanced)
- By time commitment (1-2 hours, 2-4 hours, 4+ hours)
- By learning goal (testing, API design, performance, UI/UX)

**Educational Feed: "How Others Succeeded"**
Not just "here's what happened," but "here's how people like you got their PRs merged":

```
📚 Success Story: @newdev's First Contribution to Svelte

Day 1: Found issue #8234 (documentation typo)
       Read 12 related issues to understand context

Day 2: Set up environment (took 3h due to Node version issues)
       💡 Tip: Use Node 18, not Node 20 for Svelte

Day 3: Made fix, wrote tests, opened PR
       Maintainer feedback: "Can you add an example?"

Day 4: Added example, PR approved ✓

Day 5: MERGED! 🎉

What made this successful:
- Small, focused change (good first contribution)
- Responsive to feedback (< 24h turnaround)
- Added more than requested (example was extra value)
- Patient with review process (didn't ping maintainers)
```

**Progress Tracking**
- Portfolio of contributions (merged PRs, in review, closed)
- Skills practiced (technologies, patterns)
- Quality score trends (are you improving?)
- Learning achievements (milestones unlocked)

### 2. CLI: Your Senior Dev Setting You Up

**`issuance start <issue-url>`**
- Clones/updates repository
- Fetches issue data + discussion thread
- Analyzes context (maintainer responses, related issues, linked PRs)
- Sets up dev environment (installs deps, runs initial tests)
- Identifies likely files to modify
- Extracts project conventions (testing requirements, commit format, code style)
- Generates structured context for AI tools (`.issuance/context.json`)
- Explains what you'll learn (educational framing)

**Context Structure** (for AI tools):
```json
{
  "issue": {
    "url": "...",
    "title": "...",
    "description": "...",
    "labels": ["bug", "confirmed"],
    "discussion_summary": "Maintainer confirmed this affects v18.2+. Related to concurrent mode."
  },
  "learning_outcomes": [
    "React Fiber reconciliation",
    "Memory management in JS",
    "Writing regression tests"
  ],
  "affected_files": [
    {
      "path": "packages/react-reconciler/src/ReactFiberHooks.js",
      "reason": "Contains useEffect cleanup logic",
      "lines_of_interest": [847, 892]
    }
  ],
  "project_conventions": {
    "test_requirements": "All bug fixes require regression tests",
    "commit_format": "Conventional Commits",
    "pr_checklist": ["Tests pass", "Issue referenced", "Description clear"]
  },
  "maintainer_patterns": {
    "avg_response_time": "2.3 days",
    "tone": "formal",
    "common_feedback": ["Add tests", "Reference issue number", "Explain testing process"]
  }
}
```

**`issuance review`**
- Analyzes your changes (git diff)
- Checks PR description / commit message
- Runs quality checks:
  - Epistemic humility (overconfident claims?)
  - Evidence (did you show testing?)
  - Tone matching (fits project culture?)
  - Convention compliance (tests? proper format?)
  - Value-add (contributing new info vs. noise?)
- Provides educational feedback with explanations
- Suggests improvements before you submit publicly

**`issuance watch <repo>`**
- Adds repo to your learning feed
- Syncs activity (issues, PRs, comments)
- Shows contribution patterns to learn from

### 3. AI Tool Integration: The Implementation Layer

Issuance prepares context for **any AI coding tool**:

**Claude Code** (recommended)
- Reads `.issuance/context.json` automatically
- Can reference via `.claude-context` or CLAUDE.md
- Full conversation with access to local codebase

**Cursor**
- Consumes context via `.cursorrules` or context files
- AI understands project conventions

**GitHub Copilot / Other Tools**
- Generic JSON format can be consumed by any tool
- User can summarize context and paste into chat

**Standalone Mode** (no AI tool)
```bash
$ issuance start <issue> --analyze
```
Uses built-in AI (via OpenCode) to provide analysis and suggestions if you don't have an AI coding assistant.

---

## What Makes It Different

### vs. "Good First Issue" Labels
- **Good First Issue labels**: Just a tag, no context, no guidance
- **Issuance**: Curated opportunities with setup, context, and quality gates

### vs. GitHub Search
- **GitHub Search**: Overwhelming, no context, no learning support
- **Issuance**: Filtered for educational value, full context provided, tracks your growth

### vs. AI Coding Tools Alone
- **AI tools alone**: No OSS context, don't know project conventions, can produce embarrassing code
- **Issuance + AI tools**: Context-aware, convention-following, quality-checked

### vs. Repomix / Context Tools
- **Repomix**: Packs entire codebase for AI consumption (generic understanding)
- **Issuance**: Issue-centric context for specific contributions (focused learning)

### vs. Educational Platforms (Codecademy, etc.)
- **Educational platforms**: Contrived tutorials, sandboxed environments
- **Issuance**: Real codebases, real constraints, real stakes

### vs. Freelance Platforms
- **Freelance**: Competitive, transactional, experienced devs only
- **Issuance**: Educational, collaborative, designed for learners

---

## Success Metrics

We measure success through **learning outcomes**, not productivity:

### For Students
- **Contributions made**: PRs opened, merged, in review
- **Skills practiced**: Technologies and patterns encountered
- **Quality improvement**: Quality scores trending up over time
- **Confidence gained**: Progressing from beginner to intermediate issues
- **Portfolio built**: Merged PRs demonstrate real-world experience
- **Jobs/internships secured**: Ultimate outcome (tracked via user surveys)

### For Open Source Projects
- **High-quality contribution rate**: % of Issuance-guided PRs that get merged
- **Maintainer time saved**: Fewer low-quality PRs to review
- **New contributor retention**: % of first-time contributors who contribute again
- **Community health**: More contributors, more diversity

### For The Ecosystem
- **Gap bridged**: Students → OSS connection rate
- **Learning efficiency**: Time from "first contribution" to "confident contributor"
- **Professional habits**: Students who use quality gates develop better practices

---

## Philosophy

### Education Over Automation
We don't automate contributions—we teach you how to contribute well.

**Wrong approach (automation):**
- AI writes the code for you
- Copy/paste without understanding
- Optimize for speed, not learning

**Right approach (education):**
- You implement with AI guidance
- Understand what you're doing and why
- Build skills that compound over time

### Confidence Through Structure
Open source is intimidating because it's unstructured. We provide scaffolding:

- **Curated opportunities**: Not overwhelming
- **Setup automation**: Removes friction, not learning
- **Quality gates**: Build confidence through feedback before public submission
- **Success stories**: Social proof that "people like me can do this"

### Professional Habits Early
The habits you build now stick with you. We teach:

- **Epistemic humility**: Admit uncertainty, show reasoning
- **Evidence-based communication**: Show your testing process
- **Tone awareness**: Match the community culture
- **Incremental contributions**: Small, focused changes
- **Responsive collaboration**: Engage with feedback

### Real-World Context
The best learning happens in real-world contexts:

- **Real stakes**: Your work might get merged into production
- **Real constraints**: Follow conventions, pass tests, satisfy maintainers
- **Real feedback**: Maintainers review your work (+ our quality gate prepares you)
- **Real portfolio**: Merged PRs prove your skills

---

## The Vision

**In 5 years, we want every CS student to have contributed to open source before graduating.**

Not because it looks good on a resume (though it does), but because **it's the best way to learn software development**:

- Learn by doing, not by watching
- Work with real constraints, not contrived tutorials
- Build confidence through real achievements
- Develop professional habits in a safe environment
- Contribute to the commons while learning

**Issuance is the bridge between "I finished a tutorial" and "I can work on production code."**

---

## Open Questions

As we build this product, we're exploring:

1. **How do we maintain curation quality as we scale?**
   - Manual curation (~1000 repos) vs. algorithmic scoring
   - Community voting on "good learning repos"?

2. **How do we measure learning outcomes?**
   - Self-reported skill growth?
   - Quality score improvements over time?
   - Job placement rates?

3. **How do we monetize without compromising the mission?**
   - Always free for students
   - Premium features for teams/bootcamps?
   - Sponsorships from companies hiring?

4. **How do we support non-English speakers?**
   - Translate UI?
   - Find repos with maintainers who speak other languages?

5. **How do we handle student contributions that still get rejected?**
   - Learning opportunity: what can we improve in quality gate?
   - Emotional support: rejection is part of learning

---

## Conclusion

**Issuance isn't just a tool—it's a movement.**

A movement to:
- Make open source accessible to everyone
- Bridge the experience gap for students
- Teach professional habits through real-world practice
- Build the next generation of confident, capable developers

**We're not building a better issue tracker. We're building a better way to learn software development.**
