# Testing Strategy

This project follows the test pyramid approach with three distinct test levels.

## Test Structure

```
tests/
├── unit/              # Fast, isolated tests with mocked dependencies
│   ├── auth.test.ts   # Currently flat - organize into subdirs when 5+ files per category
│   ├── schema.test.ts
│   └── utils.test.ts
├── integration/       # Tests with real external systems (DB, HTTP)
└── e2e/              # Full end-to-end browser tests with Playwright
```

**Structure Philosophy:** Keep test organization flat until you have enough files (5+) in a category to warrant subdirectories. Let the structure emerge naturally as the codebase grows.

## Test Types

### Unit Tests (`tests/unit/`)

**Purpose:** Test individual functions and modules in isolation

**Characteristics:**
- All external dependencies are mocked
- No database, no network calls, no file system
- Fast execution (milliseconds)
- Run on every commit

**What to mock:**
- Database (Drizzle queries)
- External APIs (fetch, GitHub API)
- File system
- Environment variables
- Crypto operations (when testing logic, not crypto itself)
- TanStack Start framework functions

**Example:**
```typescript
// tests/unit/auth.test.ts
vi.mock('@/server/db', () => ({ db: { insert: vi.fn() } }))
vi.mock('@tanstack/react-start/server', () => ({ getCookie: vi.fn() }))

it('returns error when state mismatches', async () => {
  getCookie.mockReturnValue('stored-state')
  const result = await handleGitHubCallback({ code: 'x', state: 'bad-state' })
  expect(result).toEqual({ error: 'invalid_state' })
})
```

**Current coverage:**
- ✅ Auth functions (OAuth flow, logout)
- ✅ Schema validation
- ✅ Utility functions

### Integration Tests (`tests/integration/`)

**Purpose:** Test multiple components working together with real external systems

**Characteristics:**
- Uses real database (test DB or in-memory SQLite)
- Real HTTP requests (but mocked external APIs like GitHub)
- Real Drizzle ORM queries
- Real crypto operations
- Medium execution time (seconds)
- Run before merging PRs

**What's real vs. mocked:**
- ✅ Real: Database, Drizzle, sessions, crypto
- ❌ Mock: External APIs (GitHub OAuth endpoints)

**Example:**
```typescript
// tests/integration/auth-flow.test.ts
describe('Auth integration', () => {
  beforeAll(() => setupTestDB()) // Real database!

  it('creates user and session in DB', async () => {
    mockGitHubAPI({ access_token: 'token', user: { id: 123 } })

    await handleGitHubCallback({ code: 'x', state: 'valid' })

    // Query real database
    const user = await db.select().from(users).where(eq(users.githubId, 123))
    expect(user[0].username).toBe('testuser')
  })
})
```

**Planned coverage:**
- Auth flow (OAuth + database persistence)
- Subscription management (DB + GitHub API)
- Feed generation (DB queries + aggregation)
- AI draft generation (DB + Gemini API)

### E2E Tests (`tests/e2e/`)

**Purpose:** Test complete user workflows in a real browser

**Characteristics:**
- Uses Playwright
- Real browser automation
- Real backend server
- Test database
- Slowest execution (seconds to minutes)
- Run before production deployments

**Example:**
```typescript
// tests/e2e/login.spec.ts
test('user can log in with GitHub', async ({ page }) => {
  await page.goto('/')
  await page.click('text=Sign in with GitHub')
  // Mock GitHub OAuth redirect
  await page.goto('/auth/github/callback?code=test&state=test')
  await expect(page.locator('text=Dashboard')).toBeVisible()
})
```

**Planned coverage (from PLAN.md Phase 10):**
- Login flow
- Dashboard navigation
- Subscription management
- Issue viewing
- Draft generation

## Running Tests

```bash
# All tests
bun run test

# Unit tests only (fast)
bun test tests/unit/

# Integration tests only
bun test tests/integration/

# E2E tests
bunx playwright test

# With coverage
bun test --coverage

# Watch mode (during development)
bun test --watch
```

## Test Guidelines

### When to Write Unit Tests

- Testing business logic (validation, transformations)
- Testing error handling paths
- Testing edge cases
- Testing utility functions
- Any function that doesn't require external systems

### When to Write Integration Tests

- Testing database operations (queries, transactions)
- Testing API endpoints with real HTTP
- Testing service layer with multiple components
- Testing auth flows with real sessions
- Verifying external API integrations

### When to Write E2E Tests

- Testing critical user journeys (login, core features)
- Testing UI interactions
- Testing full workflows across multiple pages
- Regression testing for production bugs
- Smoke testing after deployments

## Test Coverage Goals

- **Unit tests:** 80%+ coverage for business logic
- **Integration tests:** Cover all critical paths (auth, subscriptions, AI)
- **E2E tests:** Cover happy paths for key features

## CI/CD Integration

```yaml
# .github/workflows/test.yml (example)
test:
  - name: Unit tests
    run: bun test tests/unit/

  - name: Integration tests
    run: bun test tests/integration/
    env:
      TEST_DATABASE_URL: ${{ secrets.TEST_DB }}

  - name: E2E tests
    run: bunx playwright test
    env:
      E2E_BASE_URL: http://localhost:3000
```

## Test Organization Guidelines

### When to Create Subdirectories

**Rule:** Organize tests into subdirectories when you have **5+ files in a category**.

**Example progression:**

**Stage 1: Flat (3-5 files) ✅ Current**
```
tests/unit/
├── auth.test.ts
├── schema.test.ts
└── utils.test.ts
```

**Stage 2: Still flat (6-10 files)**
```
tests/unit/
├── auth.test.ts
├── schema.test.ts
├── utils.test.ts
├── github-service.test.ts
├── gemini-service.test.ts
└── scoring-service.test.ts
```

**Stage 3: Organize by category (10+ files)**
```
tests/unit/
├── auth.test.ts
├── schema.test.ts
├── utils.test.ts
├── services/
│   ├── github.test.ts       ← Group related tests
│   ├── gemini.test.ts
│   ├── scoring.test.ts
│   ├── sync.test.ts
│   └── webhook.test.ts
└── components/
    ├── feed-item.test.tsx
    ├── feed-list.test.tsx
    └── issue-card.test.tsx
```

**Mirror source structure:** When `src/server/services/` has 5+ files, create `tests/unit/services/` to mirror it. Empty directories in tests are a sign of premature organization.

## Future Improvements

- [ ] Set up test database (SQLite in-memory or Docker Postgres)
- [ ] Add integration tests for auth flow
- [ ] Add Playwright E2E tests (Phase 10)
- [ ] Set up CI/CD pipeline with test stages
- [ ] Add visual regression testing (Playwright screenshots)
- [ ] Measure and track test coverage over time
- [ ] Organize tests into subdirectories when categories reach 5+ files
