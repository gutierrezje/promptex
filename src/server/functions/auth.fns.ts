import { createServerFn } from '@tanstack/react-start'
import { setCookie, getCookie, deleteCookie, updateSession, getSession, clearSession } from '@tanstack/react-start/server'
import { randomBytes } from 'crypto'
import {
  GITHUB_OAUTH_URL,
  GITHUB_TOKEN_URL,
  GITHUB_API_URL,
  GITHUB_SCOPES,
} from '@/lib/constants'
import { env } from '@/server/lib/env'
import { db } from '@/server/db'
import { encrypt } from '@/server/lib/crypto'
import { sessionConfig } from '@/server/lib/session'
import {
  exchangeCodeForToken,
  fetchGitHubUser,
  upsertUserAndCreateSession,
  deleteUserSessions,
  type AuthConfig,
} from '@/server/services/auth.service'

export type { SafeUser } from './session.fn'

// Dependency injection for auth service
const authDeps = {
  db,
  fetch: globalThis.fetch,
  encrypt,
}

// OAuth configuration
const authConfig: AuthConfig = {
  clientId: env.GITHUB_CLIENT_ID,
  clientSecret: env.GITHUB_CLIENT_SECRET,
  tokenUrl: GITHUB_TOKEN_URL,
  apiUrl: GITHUB_API_URL,
}

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
    // Validate OAuth state
    const storedState = getCookie('oauth_state')
    deleteCookie('oauth_state', { path: '/' })

    if (!state || state !== storedState) {
      return { error: 'invalid_state' }
    }

    // Exchange code for access token
    const tokenResult = await exchangeCodeForToken(authDeps, authConfig, code)
    if (tokenResult.error || !tokenResult.accessToken) {
      return { error: tokenResult.error }
    }

    // Fetch GitHub user profile
    const userResult = await fetchGitHubUser(
      authDeps,
      tokenResult.accessToken,
      authConfig.apiUrl,
    )
    if (userResult.error || !userResult.user) {
      return { error: userResult.error }
    }

    // Upsert user and create session
    const { userId } = await upsertUserAndCreateSession(
      authDeps,
      userResult.user,
      tokenResult.accessToken,
    )

    // Set encrypted session cookie
    await updateSession(sessionConfig, { userId })

    return {}
  })

// ─── Logout ──────────────────────────────────────────────────────────────────

export const logout = createServerFn({ method: 'POST' }).handler(async () => {
  const session = await getSession(sessionConfig)
  const userId = session.data.userId as string | undefined

  if (userId) {
    await deleteUserSessions(authDeps, userId)
  }

  await clearSession(sessionConfig)
})
