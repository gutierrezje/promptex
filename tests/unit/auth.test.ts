import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { PGlite } from '@electric-sql/pglite'
import { createTestDb, cleanupTestDb } from '../helpers/db'
import {
  exchangeCodeForToken,
  fetchGitHubUser,
  upsertUserAndCreateSession,
  deleteUserSessions,
  type GitHubUser,
} from '@/server/services/auth.service'
import { users, sessions } from '@/server/db/schema'

describe('Auth Service', () => {
  describe('exchangeCodeForToken', () => {
    it('returns access token on successful exchange', async () => {
      const mockFetch = vi.fn().mockResolvedValue({
        json: async () => ({ access_token: 'gh_token_123' }),
      })

      const result = await exchangeCodeForToken(
        { fetch: mockFetch },
        {
          clientId: 'test-client',
          clientSecret: 'test-secret',
          tokenUrl: 'https://github.com/login/oauth/access_token',
        },
        'auth-code-123',
      )

      expect(result).toEqual({ accessToken: 'gh_token_123' })
      expect(mockFetch).toHaveBeenCalledWith(
        'https://github.com/login/oauth/access_token',
        expect.objectContaining({
          method: 'POST',
          body: JSON.stringify({
            client_id: 'test-client',
            client_secret: 'test-secret',
            code: 'auth-code-123',
          }),
        }),
      )
    })

    it('returns error when no access token in response', async () => {
      const mockFetch = vi.fn().mockResolvedValue({
        json: async () => ({ error: 'bad_code' }),
      })

      const result = await exchangeCodeForToken(
        { fetch: mockFetch },
        {
          clientId: 'test-client',
          clientSecret: 'test-secret',
          tokenUrl: 'https://github.com/login/oauth/access_token',
        },
        'bad-code',
      )

      expect(result).toEqual({ error: 'token_exchange' })
    })

    it('returns error on network failure', async () => {
      const mockFetch = vi.fn().mockRejectedValue(new Error('Network error'))

      const result = await exchangeCodeForToken(
        { fetch: mockFetch },
        {
          clientId: 'test-client',
          clientSecret: 'test-secret',
          tokenUrl: 'https://github.com/login/oauth/access_token',
        },
        'auth-code',
      )

      expect(result).toEqual({ error: 'token_exchange' })
    })
  })

  describe('fetchGitHubUser', () => {
    it('returns user profile on success', async () => {
      const mockFetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({
          id: 12345,
          login: 'testuser',
          name: 'Test User',
          avatar_url: 'https://github.com/avatar.png',
        }),
      })

      const result = await fetchGitHubUser(
        { fetch: mockFetch },
        'gh_token_123',
        'https://api.github.com',
      )

      expect(result).toEqual({
        user: {
          id: 12345,
          login: 'testuser',
          name: 'Test User',
          avatar_url: 'https://github.com/avatar.png',
        },
      })
      expect(mockFetch).toHaveBeenCalledWith('https://api.github.com/user', {
        headers: { Authorization: 'Bearer gh_token_123' },
      })
    })

    it('returns error when response is not ok', async () => {
      const mockFetch = vi.fn().mockResolvedValue({
        ok: false,
      })

      const result = await fetchGitHubUser(
        { fetch: mockFetch },
        'bad_token',
        'https://api.github.com',
      )

      expect(result).toEqual({ error: 'profile_fetch' })
    })

    it('returns error on network failure', async () => {
      const mockFetch = vi.fn().mockRejectedValue(new Error('Network error'))

      const result = await fetchGitHubUser(
        { fetch: mockFetch },
        'gh_token',
        'https://api.github.com',
      )

      expect(result).toEqual({ error: 'profile_fetch' })
    })
  })

  describe('upsertUserAndCreateSession (with PGlite)', () => {
    let db: Awaited<ReturnType<typeof createTestDb>>['db']
    let client: PGlite

    beforeEach(async () => {
      const testDb = await createTestDb()
      db = testDb.db
      client = testDb.client
    })

    afterEach(async () => {
      await cleanupTestDb(client)
    })

    it('creates new user and session', async () => {
      const mockEncrypt = vi.fn((token) => `encrypted:${token}`)
      const ghUser: GitHubUser = {
        id: 12345,
        login: 'newuser',
        name: 'New User',
        avatar_url: 'https://github.com/avatar.png',
      }

      const result = await upsertUserAndCreateSession(
        { db, encrypt: mockEncrypt },
        ghUser,
        'gh_token_123',
      )

      // Verify return value
      expect(result.userId).toBeDefined()
      expect(mockEncrypt).toHaveBeenCalledWith('gh_token_123')

      // Verify user was inserted
      const insertedUsers = await db.select().from(users)
      expect(insertedUsers).toHaveLength(1)
      expect(insertedUsers[0]).toMatchObject({
        githubId: 12345,
        username: 'newuser',
        displayName: 'New User',
        avatarUrl: 'https://github.com/avatar.png',
        accessToken: 'encrypted:gh_token_123',
      })

      // Verify session was created
      const insertedSessions = await db.select().from(sessions)
      expect(insertedSessions).toHaveLength(1)
      expect(insertedSessions[0].userId).toBe(result.userId)
      expect(insertedSessions[0].expiresAt.getTime()).toBeGreaterThan(Date.now())
    })

    it('updates existing user on conflict and creates new session', async () => {
      const mockEncrypt = vi.fn((token) => `encrypted:${token}`)

      // Insert initial user
      const [initialUser] = await db
        .insert(users)
        .values({
          githubId: 12345,
          username: 'oldusername',
          displayName: 'Old Name',
          avatarUrl: 'https://old-avatar.png',
          accessToken: 'encrypted:old_token',
        })
        .returning()

      // Upsert with updated data
      const ghUser: GitHubUser = {
        id: 12345, // Same GitHub ID
        login: 'newusername',
        name: 'New Name',
        avatar_url: 'https://new-avatar.png',
      }

      const result = await upsertUserAndCreateSession(
        { db, encrypt: mockEncrypt },
        ghUser,
        'gh_new_token',
      )

      // Verify user was updated (same ID)
      expect(result.userId).toBe(initialUser.id)

      const updatedUsers = await db.select().from(users)
      expect(updatedUsers).toHaveLength(1)
      expect(updatedUsers[0]).toMatchObject({
        id: initialUser.id,
        githubId: 12345,
        username: 'newusername',
        displayName: 'New Name',
        avatarUrl: 'https://new-avatar.png',
        accessToken: 'encrypted:gh_new_token',
      })

      // Verify new session was created
      const insertedSessions = await db.select().from(sessions)
      expect(insertedSessions).toHaveLength(1)
      expect(insertedSessions[0].userId).toBe(result.userId)
    })
  })

  describe('deleteUserSessions (with PGlite)', () => {
    let db: Awaited<ReturnType<typeof createTestDb>>['db']
    let client: PGlite

    beforeEach(async () => {
      const testDb = await createTestDb()
      db = testDb.db
      client = testDb.client
    })

    afterEach(async () => {
      await cleanupTestDb(client)
    })

    it('deletes all sessions for a user', async () => {
      // Create a user
      const [user] = await db
        .insert(users)
        .values({
          githubId: 12345,
          username: 'testuser',
          accessToken: 'encrypted:token',
        })
        .returning()

      // Create multiple sessions
      await db.insert(sessions).values([
        { userId: user.id, expiresAt: new Date(Date.now() + 1000000) },
        { userId: user.id, expiresAt: new Date(Date.now() + 2000000) },
      ])

      // Verify sessions exist
      const sessionsBefore = await db.select().from(sessions)
      expect(sessionsBefore).toHaveLength(2)

      // Delete sessions
      await deleteUserSessions({ db }, user.id)

      // Verify sessions were deleted
      const sessionsAfter = await db.select().from(sessions)
      expect(sessionsAfter).toHaveLength(0)
    })

    it('does nothing when user has no sessions', async () => {
      // Create a user without sessions
      const [user] = await db
        .insert(users)
        .values({
          githubId: 12345,
          username: 'testuser',
          accessToken: 'encrypted:token',
        })
        .returning()

      // Attempt to delete (should not throw)
      await expect(deleteUserSessions({ db }, user.id)).resolves.not.toThrow()

      // Verify no sessions exist
      const sessionsAfter = await db.select().from(sessions)
      expect(sessionsAfter).toHaveLength(0)
    })
  })
})
