# Issuance - GitHub Issue Productivity Tool

## Summary
High-productivity tool connecting devs to impactful open-source issues. Discover high-visibility/low-competition repos and issues, monitor activity across subscriptions, and draft quality contributions with AI-assisted context analysis and quality gates.

## Tech Stack
| Layer | Choice |
|-------|--------|
| Framework | TanStack Start (beta, full-stack SSR) |
| Database | PostgreSQL (Neon free tier) + Drizzle ORM |
| Auth | GitHub OAuth 2.0 (custom, no library) |
| Sessions | `iron-session` (encrypted cookie) |
| AI | Gemini 3.0 Flash via `@google/genai` (free tier) |
| Runtime + Package Manager | Bun (full runtime) |
| GitHub API | `octokit` |
| Styling | Tailwind CSS v4, Radix UI primitives |
| Testing | Vitest + Testing Library (unit), Playwright (e2e) |
| Hosting | Vercel (free) |

## Database Schema (9 tables)

**users** - `id`, `github_id` (unique), `username`, `display_name`, `avatar_url`, `access_token` (encrypted), timestamps

**sessions** - `id`, `user_id` FK, `expires_at`, timestamps (optional if cookie-only sessions suffice)

**subscriptions** - `id`, `user_id` FK, `repo_owner`, `repo_name`, `webhook_id`, `webhook_secret`, `last_synced_at`, timestamps. UNIQUE(user_id, repo_owner, repo_name)

**github_events** - `id`, `github_event_id` (unique, dedup), `repo_owner`, `repo_name`, `event_type` (issue_opened, issue_comment, push, pr_opened, release, etc.), `title`, `body`, `actor_username`, `actor_avatar`, `metadata` (JSONB), `github_url`, `occurred_at`, timestamps. Indexed on (repo_owner, repo_name, occurred_at DESC). **Shared across users** - no per-user duplication.

**cached_issues** - `id`, `repo_owner`, `repo_name`, `issue_number`, `title`, `body`, `state`, `author_username`, `labels` (JSONB), `assignees` (JSONB), `comment_count`, `github_url`, GitHub timestamps, `synced_at`. UNIQUE(repo_owner, repo_name, issue_number)

**ai_drafts** - `id`, `user_id` FK, `issue_id` FK, `repo_owner`, `repo_name`, `issue_number`, `draft_type` (response/assignee_suggestion), `content`, `status` (pending/accepted/discarded), `model_used`, timestamps

**repo_ai_insights** - `id`, `repo_owner`, `repo_name`, `insight_type` (triage/label_suggestion/digest/stale_detection/contribution_score), `target_issue_number` (nullable), `content` (JSONB), `model_used`, `generated_at`, timestamps. UNIQUE(repo_owner, repo_name, insight_type, target_issue_number)

**repo_scores** - `id`, `repo_owner`, `repo_name`, `stars`, `open_issues_count`, `contributor_count`, `avg_issue_response_hours` (nullable), `maintainer_activity_score` (0-100), `contribution_opportunity_score` (0-100, computed: high stars + low contributors + slow response = high score), `last_scored_at`, timestamps. UNIQUE(repo_owner, repo_name)

**issue_scores** - `id`, `repo_owner`, `repo_name`, `issue_number`, `labels` (JSONB), `has_good_first_issue` (bool), `has_help_wanted` (bool), `comment_count`, `linked_pr_count`, `days_since_last_activity`, `has_repro` (bool, AI-assessed), `has_clear_description` (bool, AI-assessed), `contribution_fit_score` (0-100), `last_scored_at`, timestamps. UNIQUE(repo_owner, repo_name, issue_number)

## Project Structure
```
issuance/
├── app.config.ts
├── drizzle.config.ts
├── vitest.config.ts
├── playwright.config.ts
├── drizzle/                         # Migration SQL files
├── app/
│   ├── client.tsx, ssr.tsx, router.tsx
│   ├── routes/
│   │   ├── __root.tsx               # HTML shell, nav, auth context
│   │   ├── index.tsx                # Landing / redirect
│   │   ├── login.tsx
│   │   ├── auth/
│   │   │   ├── github.tsx           # Initiate OAuth
│   │   │   └── github.callback.tsx  # Handle callback
│   │   ├── _authed.tsx              # Auth guard layout
│   │   ├── _authed/
│   │   │   ├── dashboard.tsx        # Main newsfeed
│   │   │   ├── discover.tsx         # Issue discovery - high visibility/low competition
│   │   │   ├── issues.tsx           # Issues list
│   │   │   ├── issues.$issueId.tsx  # Issue detail + context panel + AI draft
│   │   │   ├── commits.tsx
│   │   │   ├── pulls.tsx
│   │   │   ├── releases.tsx
│   │   │   ├── subscriptions.tsx    # Manage repo subscriptions
│   │   │   └── settings.tsx
│   │   └── api/
│   │       ├── webhooks.github.ts   # GitHub webhook receiver
│   │       └── cron.insights.ts     # Vercel Cron: repo-level AI inference
│   ├── components/
│   │   ├── ui/                      # Button, Card, Input, Dialog
│   │   ├── layout/                  # AppShell, Sidebar, TopBar
│   │   ├── feed/                    # FeedItem, FeedList, FeedFilters
│   │   ├── issues/                  # IssueCard, IssueDetail, IssueTimeline
│   │   ├── ai/                      # DraftPanel, DraftActions, ContextPanel, QualityGate
│   │   ├── discover/                # ScoreCard, DiscoverFilters, OpportunityList
│   │   └── subscriptions/           # RepoSearch, RepoList
│   ├── server/
│   │   ├── db/
│   │   │   ├── index.ts             # Drizzle client (neon-http driver)
│   │   │   └── schema.ts            # All table definitions
│   │   ├── functions/               # createServerFn definitions
│   │   │   ├── auth.fns.ts
│   │   │   ├── subscriptions.fns.ts
│   │   │   ├── feed.fns.ts
│   │   │   ├── issues.fns.ts
│   │   │   ├── drafts.fns.ts
│   │   │   ├── github.fns.ts
│   │   │   └── sync.fns.ts
│   │   ├── services/
│   │   │   ├── github.service.ts    # Octokit wrapper, rate limits, ETags
│   │   │   ├── gemini.service.ts    # Gemini API, prompt templates, rate tracking
│   │   │   ├── insights.service.ts  # Scheduled repo-level AI inference
│   │   │   ├── scoring.service.ts   # Repo + issue scoring for discovery
│   │   │   ├── sync.service.ts      # Polling/sync orchestration
│   │   │   └── webhook.service.ts   # Webhook payload processing
│   │   ├── middleware/
│   │   │   └── auth.middleware.ts
│   │   └── lib/
│   │       ├── session.ts           # iron-session config
│   │       ├── crypto.ts            # Token encryption, CSRF
│   │       └── rate-limit.ts        # In-memory rate limit tracker
│   ├── lib/
│   │   ├── utils.ts
│   │   └── constants.ts
│   └── styles/globals.css
├── tests/
│   ├── unit/                        # Services, schema, components, utils
│   ├── integration/                 # Auth flow, subscriptions, feed, drafts
│   └── e2e/                         # Login, dashboard, subscriptions, drafts
└── public/
```

## Data Fetching Strategy

**Hybrid: Webhooks + Polling Fallback**
- On subscribe: attempt to register a GitHub webhook (requires repo admin). Falls back to polling if no permission.
- Polling: uses GitHub Events API with ETag-based conditional requests to avoid wasting rate limit.
- Client-side: 60-second polling interval while tab is active.
- Stale check: if `last_synced_at` > 5 minutes old on page load, trigger background sync.
- Multi-user efficiency: events stored per-repo, not per-user. Multiple users subscribing to the same repo share one event set.
- Rate limit awareness: tracks `x-ratelimit-remaining` from GitHub responses; throttles/stops polling as limits approach.

## AI Inference Strategy

### Per-Repo Scheduled Inference (shared across users, cost-efficient)
Runs on a cron schedule (every 4 hours) per subscribed repo. Results shared across all users subscribed to that repo.

**Repo-level AI jobs:**
- **Issue triage**: Classify new/updated open issues (bug/feature/question/docs) + priority estimate
- **Label suggestions**: Batch-suggest labels for unlabeled issues
- **Repo activity digest**: 2-3 sentence summary of recent activity, shown on dashboard
- **Stale issue detection**: Flag issues with no activity in 14+ days

**New table: `repo_ai_insights`**
- `id`, `repo_owner`, `repo_name`, `insight_type` (triage/label_suggestion/digest/stale_detection), `target_issue_number` (nullable), `content` (JSONB), `model_used`, `generated_at`, timestamps
- UNIQUE(repo_owner, repo_name, insight_type, target_issue_number)

**Scheduling**: Vercel Cron (free tier: 2 cron jobs). Cron endpoint iterates subscribed repos, skips recently processed ones, runs inference sequentially to stay within rate limits.

### Per-User On-Demand Inference
Triggered when user clicks "Generate Draft" on a specific issue.

**User-level AI jobs:**
- **Response drafts**: Full draft reply to an issue
- **Assignee suggestions**: Based on user's team context

### Combined Flow
1. Cron runs every 4h -> processes repos -> stores insights in `repo_ai_insights`
2. User opens dashboard -> sees repo digests and triaged issues (from shared insights)
3. User opens issue -> sees pre-computed label suggestions (shared) + can generate personal draft (on-demand)
4. User clicks "Generate Draft" -> Gemini call -> stored in `ai_drafts` (per-user)
5. Rate limit tracking across both paths to stay within free tier

## Issue Discovery & Scoring

### Repo Scoring
Repos are scored for contribution opportunity: `high stars + low contributor count + slow avg response time = high opportunity`.
- **Data sources**: GitHub API `/repos/{owner}/{repo}` (stars, open_issues), `/repos/{owner}/{repo}/contributors` (count), `/repos/{owner}/{repo}/issues?sort=created` (response time sampling)
- **Score formula**: `opportunity = normalize(stars) * 0.3 + normalize(1/contributors) * 0.3 + normalize(avg_response_hours) * 0.2 + normalize(open_issues) * 0.2`
- Scored during cron job. Users can also search/add repos manually; scoring happens on subscribe.

### Issue Scoring
Issues within subscribed repos scored for "contribution fit":
- **Automatic signals** (from GitHub API): `good first issue` / `help wanted` labels, comment count (low = less competition), linked PR count (0 = unclaimed), days since last activity
- **AI-assessed signals** (from Gemini during cron): has reproduction steps, has clear description, estimated complexity
- **Score**: weighted combination. High score = good candidate for contribution.

### Discovery Page (`/discover`)
- Shows top-scored issues across all subscribed repos, sorted by contribution fit
- Filters: language, label, complexity, repo
- Each issue card shows: repo name, issue title, score breakdown, labels, age, competition level (comment/PR count)

## Contribution Quality Gate

### Context-First Flow (before any draft)
When a user opens an issue to potentially contribute, they see an **Issue Context Panel**:
1. **AI Summary**: What the issue is about, what's been discussed, current status
2. **Context Checklist** (AI-assessed, user-verifiable):
   - [ ] Has reproduction steps or clear description
   - [ ] Has environment/version info (if applicable)
   - [ ] Not already being worked on (no linked PRs)
   - [ ] Not a duplicate of another issue
   - [ ] Maintainer has acknowledged / labeled the issue
   - [ ] User has relevant expertise for this issue
3. **Existing Discussion Summary**: Key points from comments so far
4. User reviews checklist. Only after engaging with context can they click "Draft Response".

### Draft Quality Gate (after generating draft)
The AI draft includes a self-assessment:
- **Value Check**: Does this response add new information vs. repeating existing comments?
- **Specificity Check**: Is this actionable, or is it generic "I can help" noise?
- **Qualification Check**: Does the response demonstrate understanding of the codebase/problem?
- Rating: Green (high value) / Yellow (review carefully) / Red (likely not valuable, reconsider)

The user sees the rating before copying/posting. Red-rated drafts show a warning encouraging the user to reconsider.

## Auth Flow
1. User clicks "Sign in with GitHub" -> redirect to GitHub OAuth
2. GitHub redirects back with code -> exchange for access token
3. Upsert user in DB, encrypt token, create iron-session cookie
4. `_authed.tsx` layout guard checks session on all protected routes
5. Scopes: `repo` (read repos, create webhooks) + `read:user`

## Implementation Phases

### Phase 1: Foundation
- Init TanStack Start project with Bun, TS strict mode, Tailwind v4
- Set up Drizzle + Neon, define full schema, run migrations
- Basic layout: `__root.tsx`, `AppShell`, `Sidebar`
- Vitest config + first schema test

### Phase 2: Authentication
- GitHub OAuth App setup
- `iron-session` config, `/login`, `/auth/github`, `/auth/github/callback`
- `getSession` server fn, `auth.middleware.ts`, `_authed.tsx` guard
- Auth integration tests

### Phase 3: Subscriptions + GitHub Sync
- `subscriptions.tsx` page, `RepoSearch`, `RepoList`
- `github.service.ts` (Octokit, rate limits, ETags)
- Subscribe/unsubscribe server fns, `sync.service.ts`
- Webhook registration + `/api/webhooks/github` with HMAC validation
- Unit tests for GitHub service + sync service

### Phase 4: Dashboard / Newsfeed
- `getFeed` server fn with cursor pagination
- `FeedList` (virtualized with `@tanstack/react-virtual`), `FeedItem`, `FeedFilters`
- Auto-refresh polling, dark Bloomberg-style theme
- Component tests

### Phase 5: Issues View
- `issues.tsx` list, `issues.$issueId.tsx` detail
- `IssueCard`, `IssueTimeline`, `IssueDetail`
- On-demand comment fetching with in-memory TTL cache

### Phase 6: Repo + Issue Scoring & Discovery
- `scoring.service.ts` - repo opportunity scoring, issue contribution-fit scoring
- `repo_scores` and `issue_scores` tables + migrations
- `/discover` page with scored issue list, filters, score breakdowns
- Scoring runs during cron + on new subscription
- Unit tests for scoring logic

### Phase 7: AI - Repo-Level Insights (Shared)
- `insights.service.ts` - batch issue triage, label suggestions, activity digests, stale detection
- AI-assessed issue signals (has_repro, has_clear_description) fed into issue_scores
- `repo_ai_insights` table
- `/api/cron.insights.ts` Vercel Cron endpoint
- `gemini.service.ts` with prompt templates and rate tracking
- Unit tests with mocked Gemini

### Phase 8: AI - Context Panel + Quality Gate + Drafts
- **Context Panel**: AI summary of issue, context checklist (repro, env, duplicates, linked PRs, maintainer acknowledgment)
- **Draft generation**: `generateDraft`, `getDrafts`, `updateDraftStatus` server fns
- **Quality Gate**: AI self-assessment of draft (value/specificity/qualification checks, green/yellow/red rating)
- `DraftPanel`, `ContextPanel`, `QualityGate` components
- User must engage with context before drafting
- Unit tests with mocked Gemini

### Phase 9: Secondary Views
- `commits.tsx`, `pulls.tsx`, `releases.tsx` (filter `github_events` by type)

### Phase 10: Polish + E2E
- Loading states, error boundaries, empty states
- `settings.tsx` page
- Playwright E2E tests for all major flows
- Accessibility pass

### Phase 11: Deploy
- Vercel deployment, env vars, Neon connection, webhook URL config

## Verification
- `bun run dev` -> app runs locally, login with GitHub works
- Subscribe to a repo -> events appear in feed within polling interval
- Open an issue -> see repo-level insights (triage, labels) + generate personal draft
- `bun run test` -> all unit + integration tests pass
- `bunx playwright test` -> all e2e tests pass
- `bunx tsc --noEmit` -> no type errors
