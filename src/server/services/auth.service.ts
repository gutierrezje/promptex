import { eq } from 'drizzle-orm';
import { users, sessions } from '@/server/db/schema';

/**
 * Dependency interfaces for auth service functions.
 * Separated for flexibility - functions only require what they need.
 */
export interface DbDeps {
  db: any; // Runtime-compatible: neon-http in prod, pglite in tests
}

export interface FetchDeps {
  fetch: typeof globalThis.fetch;
}

export interface EncryptDeps {
  encrypt: (token: string) => string;
}

/**
 * Configuration for OAuth token exchange and API calls.
 */
export interface AuthConfig {
  clientId: string;
  clientSecret: string;
  tokenUrl: string;
  apiUrl: string;
}

/**
 * GitHub user profile data.
 */
export interface GitHubUser {
  id: number;
  login: string;
  name: string | null;
  avatar_url: string;
}

/**
 * Result of user upsert and session creation.
 */
export interface UserSession {
  userId: string;
}

/**
 * Exchanges an OAuth authorization code for a GitHub access token.
 */
export async function exchangeCodeForToken(
  deps: FetchDeps,
  config: Pick<AuthConfig, 'clientId' | 'clientSecret' | 'tokenUrl'>,
  code: string,
): Promise<{ accessToken?: string; error?: string }> {
  try {
    const tokenRes = await deps.fetch(config.tokenUrl, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Accept: 'application/json',
      },
      body: JSON.stringify({
        client_id: config.clientId,
        client_secret: config.clientSecret,
        code,
      }),
    });

    const tokenData = (await tokenRes.json()) as {
      access_token?: string;
      error?: string;
    };

    if (!tokenData.access_token) {
      return { error: 'token_exchange' };
    }

    return { accessToken: tokenData.access_token };
  } catch {
    return { error: 'token_exchange' };
  }
}

/**
 * Fetches the authenticated GitHub user profile.
 */
export async function fetchGitHubUser(
  deps: FetchDeps,
  accessToken: string,
  apiUrl: string,
): Promise<{ user?: GitHubUser; error?: string }> {
  try {
    const userRes = await deps.fetch(`${apiUrl}/user`, {
      headers: { Authorization: `Bearer ${accessToken}` },
    });

    if (!userRes.ok) {
      return { error: 'profile_fetch' };
    }

    const user = (await userRes.json()) as GitHubUser;
    return { user };
  } catch {
    return { error: 'profile_fetch' };
  }
}

/**
 * Upserts user record and creates a new session (7-day expiry).
 * Returns the user ID for session cookie creation.
 */
export async function upsertUserAndCreateSession(
  deps: DbDeps & EncryptDeps,
  ghUser: GitHubUser,
  accessToken: string,
): Promise<UserSession> {
  const encryptedToken = deps.encrypt(accessToken);

  const [user] = await deps.db
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
    .returning({ id: users.id });

  // Create session with 7-day expiry
  const expiresAt = new Date(Date.now() + 7 * 24 * 60 * 60 * 1000);
  await deps.db.insert(sessions).values({ userId: user.id, expiresAt });

  return { userId: user.id };
}

/**
 * Deletes all sessions for a given user (logout).
 */
export async function deleteUserSessions(
  deps: DbDeps,
  userId: string,
): Promise<void> {
  await deps.db.delete(sessions).where(eq(sessions.userId, userId));
}
