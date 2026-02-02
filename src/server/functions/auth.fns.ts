import { createServerFn } from '@tanstack/react-start'
import { setCookie, getCookie, deleteCookie, updateSession, getSession, clearSession } from '@tanstack/react-start/server'
import { eq } from 'drizzle-orm'
import { randomBytes } from 'crypto'
import {
  GITHUB_OAUTH_URL,
  GITHUB_TOKEN_URL,
  GITHUB_API_URL,
  GITHUB_SCOPES,
} from '@/lib/constants'
import { env } from '@/server/lib/env'
import { db } from '@/server/db'
import { users, sessions } from '@/server/db/schema'
import { encrypt } from '@/server/lib/crypto'
import { sessionConfig } from '@/server/lib/session'

export type { SafeUser } from './session.fn'

// ─── OAuth: generate GitHub redirect URL ─────────────────────────────────────

export const getGitHubAuthUrl = createServerFn({ method: 'GET' }).handler(
  async (): Promise<string> => {
    const state = randomBytes(32).toString('hex')

    setCookie('oauth_state', state, {
      httpOnly: true,
      sameSite: 'lax',
      path: '/',
      maxAge: 600,
      secure: env.NODE_ENV === 'production',
    })

    const params = new URLSearchParams({
      client_id: env.GITHUB_CLIENT_ID,
      redirect_uri: `${env.APP_URL}/auth/github/callback`,
      scope: GITHUB_SCOPES,
      state,
    })

    return `${GITHUB_OAUTH_URL}?${params}`
  },
)

// ─── OAuth: handle callback ──────────────────────────────────────────────────

export const handleGitHubCallback = createServerFn({ method: 'POST' })
  .inputValidator((input: { code: string; state: string }) => input)
  .handler(async ({ data: { code, state } }): Promise<{ error?: string }> => {
    const storedState = getCookie('oauth_state')
    deleteCookie('oauth_state', { path: '/' })

    if (!state || state !== storedState) {
      return { error: 'invalid_state' }
    }

    // Exchange code for access token
    let accessToken: string
    try {
      const tokenRes = await fetch(GITHUB_TOKEN_URL, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Accept: 'application/json',
        },
        body: JSON.stringify({
          client_id: env.GITHUB_CLIENT_ID,
          client_secret: env.GITHUB_CLIENT_SECRET,
          code,
        }),
      })
      const tokenData = (await tokenRes.json()) as {
        access_token?: string
        error?: string
      }
      if (!tokenData.access_token) {
        return { error: 'token_exchange' }
      }
      accessToken = tokenData.access_token
    } catch {
      return { error: 'token_exchange' }
    }

    // Fetch GitHub user profile
    let ghUser: {
      id: number
      login: string
      name: string | null
      avatar_url: string
    }
    try {
      const userRes = await fetch(`${GITHUB_API_URL}/user`, {
        headers: { Authorization: `Bearer ${accessToken}` },
      })
      if (!userRes.ok) return { error: 'profile_fetch' }
      ghUser = (await userRes.json()) as typeof ghUser
    } catch {
      return { error: 'profile_fetch' }
    }

    // Encrypt token and upsert user
    const encryptedToken = encrypt(accessToken)

    const [user] = await db
      .insert(users)
      .values({
        githubId: ghUser.id,
        username: ghUser.login,
        displayName: ghUser.name,
        avatarUrl: ghUser.avatar_url,
        accessToken: encryptedToken,
      })
      .onConflictDoUpdate({
        target: users.githubId,
        set: {
          username: ghUser.login,
          displayName: ghUser.name,
          avatarUrl: ghUser.avatar_url,
          accessToken: encryptedToken,
          updatedAt: new Date(),
        },
      })
      .returning({ id: users.id })

    // Create DB session (7-day expiry)
    const expiresAt = new Date(Date.now() + 7 * 24 * 60 * 60 * 1000)
    await db.insert(sessions).values({ userId: user.id, expiresAt })

    // Set encrypted session cookie
    await updateSession(sessionConfig, { userId: user.id })

    return {}
  })

// ─── Logout ──────────────────────────────────────────────────────────────────

export const logout = createServerFn({ method: 'POST' }).handler(async () => {
  const session = await getSession(sessionConfig)
  const userId = session.data.userId as string | undefined

  if (userId) {
    await db.delete(sessions).where(eq(sessions.userId, userId))
  }

  await clearSession(sessionConfig)
})
