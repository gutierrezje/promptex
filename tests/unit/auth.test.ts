import { beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('@tanstack/react-start', () => ({
  createServerFn: () => ({
    handler: (fn: unknown) => fn,
    inputValidator: () => ({
      handler: (fn: unknown) => fn,
    }),
  }),
}))

vi.mock('@/server/lib/env', () => ({
  env: {
    DATABASE_URL: 'http://localhost:5432/test',
    GITHUB_CLIENT_ID: 'test-client-id',
    GITHUB_CLIENT_SECRET: 'test-client-secret',
    GEMINI_API_KEY: 'test-gemini-key',
    SESSION_SECRET: 'test-session-secret-32-chars!!',
    APP_URL: 'http://localhost:3000',
    NODE_ENV: 'test',
  },
}))

vi.mock('@tanstack/react-start/server', () => ({
  setCookie: vi.fn(),
  getCookie: vi.fn(),
  deleteCookie: vi.fn(),
  updateSession: vi.fn(),
  getSession: vi.fn(),
  clearSession: vi.fn(),
}))

vi.mock('@/server/lib/crypto', () => ({
  encrypt: (value: string) => `encrypted:${value}`,
}))

const mockSessionConfig = {
  password: 'test-session-secret-32-chars!!',
  name: 'issuance_session',
  maxAge: 60 * 60 * 24 * 7,
  cookie: { httpOnly: true, secure: false, sameSite: 'lax', path: '/' },
}

vi.mock('@/server/lib/session', () => ({
  sessionConfig: mockSessionConfig,
}))

const usersTable = { __table: 'users', githubId: Symbol('githubId') }
const sessionsTable = {
  __table: 'sessions',
  userId: Symbol('userId')  // Mock the userId column for eq() queries
}

vi.mock('@/server/db/schema', () => ({
  users: usersTable,
  sessions: sessionsTable,
}))

const insertMock = vi.fn()
const userReturningMock = vi.fn()
const userOnConflictMock = vi.fn()
const userValuesMock = vi.fn()
const sessionValuesMock = vi.fn()
const deleteMock = vi.fn()
const whereClauseMock = vi.fn()

vi.mock('@/server/db', () => ({
  db: {
    insert: insertMock,
    delete: deleteMock,
  },
}))

vi.mock('drizzle-orm', () => ({
  eq: vi.fn((column, value) => ({ column, value, _op: 'eq' })),
}))

describe('Auth unit tests', () => {
  beforeEach(() => {
    vi.resetAllMocks()
    vi.unstubAllGlobals()

    userReturningMock.mockResolvedValue([{ id: 'user-123' }])
    userOnConflictMock.mockReturnValue({ returning: userReturningMock })
    userValuesMock.mockReturnValue({
      onConflictDoUpdate: userOnConflictMock,
      returning: userReturningMock,
    })
    sessionValuesMock.mockResolvedValue(undefined)
    insertMock.mockImplementation((table) => {
      if (table === usersTable) {
        return { values: userValuesMock }
      }
      if (table === sessionsTable) {
        return { values: sessionValuesMock }
      }
      return { values: vi.fn() }
    })
  })

  it('builds the GitHub auth URL and sets state cookie', async () => {
    const { getGitHubAuthUrl } = await import('@/server/functions/auth.fns')
    const server = await import('@tanstack/react-start/server')

    const url = await getGitHubAuthUrl()

    const setCookie = server.setCookie as unknown as {
      mock: { calls: [string, string, Record<string, unknown>][] }
    }

    expect(setCookie.mock.calls).toHaveLength(1)
    const [cookieName, cookieValue, options] = setCookie.mock.calls[0]
    const parsedUrl = new URL(url)

    expect(cookieName).toBe('oauth_state')
    expect(parsedUrl.searchParams.get('state')).toBe(cookieValue)
    expect(parsedUrl.searchParams.get('client_id')).toBe('test-client-id')
    expect(options).toMatchObject({ httpOnly: true, path: '/' })
  })

  it('rejects callbacks with a mismatched state', async () => {
    const server = await import('@tanstack/react-start/server')
    const getCookie = server.getCookie as unknown as { mockReturnValue: (value: string) => void }
    getCookie.mockReturnValue('stored-state')
    global.fetch = vi.fn()

    const { handleGitHubCallback } = await import('@/server/functions/auth.fns')
    const result = await handleGitHubCallback({
      data: { code: 'code', state: 'bad-state' },
    })

    expect(result).toEqual({ error: 'invalid_state' })
    expect(server.deleteCookie).toHaveBeenCalledWith('oauth_state', { path: '/' })
    expect(global.fetch).not.toHaveBeenCalled()
  })

  it('returns token exchange errors when GitHub fails', async () => {
    const server = await import('@tanstack/react-start/server')
    const getCookie = server.getCookie as unknown as { mockReturnValue: (value: string) => void }
    getCookie.mockReturnValue('stored-state')

    global.fetch = vi.fn().mockResolvedValue({
      json: async () => ({ error: 'bad_code' }),
    })

    const { handleGitHubCallback } = await import('@/server/functions/auth.fns')
    const result = await handleGitHubCallback({
      data: { code: 'code', state: 'stored-state' },
    })

    expect(result).toEqual({ error: 'token_exchange' })
  })

  it('returns profile fetch errors when GitHub user lookup fails', async () => {
    const server = await import('@tanstack/react-start/server')
    const getCookie = server.getCookie as unknown as { mockReturnValue: (value: string) => void }
    getCookie.mockReturnValue('stored-state')

    global.fetch = vi
      .fn()
      .mockResolvedValueOnce({
        json: async () => ({ access_token: 'token' }),
      })
      .mockResolvedValueOnce({
        ok: false,
      })

    const { handleGitHubCallback } = await import('@/server/functions/auth.fns')
    const result = await handleGitHubCallback({
      data: { code: 'code', state: 'stored-state' },
    })

    expect(result).toEqual({ error: 'profile_fetch' })
  })

  it('creates a session on successful callback', async () => {
    const server = await import('@tanstack/react-start/server')
    const getCookie = server.getCookie as unknown as { mockReturnValue: (value: string) => void }
    getCookie.mockReturnValue('matching-state')

    global.fetch = vi
      .fn()
      // POST to github.com/login/oath/access_token
      .mockResolvedValueOnce({
        json: async () => ({ access_token: 'gh_token_123' }),
      })
      // GET to api.github.com/user
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({
          id: 12345,
          login: 'testuser',
          name: 'Test User',
          avatar_url: 'https://github.com/avatar.png',
        }),
      })

    const { handleGitHubCallback } = await import('@/server/functions/auth.fns')
    const result = await handleGitHubCallback({
      data: { code: 'auth-code', state: 'matching-state' },
    })

    // assert not error
    expect(result?.error).toBeUndefined()

    // assert session was set
    expect(server.updateSession).toHaveBeenCalledWith(
      expect.any(Object),
      { userId: 'user-123' },
    )

    // assert user + session inserts happened
    expect(insertMock).toHaveBeenCalledWith(usersTable)
    expect(insertMock).toHaveBeenCalledWith(sessionsTable)

    // assert oauth_state was deleted
    expect(server.deleteCookie).toHaveBeenCalledWith('oauth_state', { path: '/' })
  })

  describe('logout', () => {
    it('deletes DB session and clears cookie when user is logged in', async () => {
      const server = await import('@tanstack/react-start/server')
      const getSession = server.getSession as unknown as { mockResolvedValue: (value: { data: { userId?: string } }) => void }

      // Mock active session with userId
      getSession.mockResolvedValue({ data: { userId: 'user-456' } })

      // Mock the delete chain: db.delete(sessions).where(eq(...))
      whereClauseMock.mockResolvedValue(undefined)
      deleteMock.mockReturnValue({ where: whereClauseMock })

      const { logout } = await import('@/server/functions/auth.fns')
      await logout()

      // Verify db.delete was called with sessionsTable
      expect(deleteMock).toHaveBeenCalledWith(sessionsTable)

      // Verify the where clause was invoked with the eq() matcher
      const { eq } = await import('drizzle-orm')
      expect(whereClauseMock).toHaveBeenCalled()
      expect(eq).toHaveBeenCalledWith(sessionsTable.userId, 'user-456')

      // Verify clearSession was called with sessionConfig
      expect(server.clearSession).toHaveBeenCalledWith(mockSessionConfig)
    })

    it('clears session even when no userId is present', async () => {
      const server = await import('@tanstack/react-start/server')
      const getSession = server.getSession as unknown as { mockResolvedValue: (value: { data: { userId?: string } }) => void }

      // Mock session with no userId
      getSession.mockResolvedValue({ data: {} })

      const { logout } = await import('@/server/functions/auth.fns')
      await logout()

      // Verify db.delete was NOT called (no userId)
      expect(deleteMock).not.toHaveBeenCalled()
      expect(whereClauseMock).not.toHaveBeenCalled()

      // Verify clearSession was still called
      expect(server.clearSession).toHaveBeenCalledWith(mockSessionConfig)
    })
  })
})
