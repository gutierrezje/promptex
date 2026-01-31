import { createServerFn } from '@tanstack/react-start'
import { eq, and, gt } from 'drizzle-orm'

export type SafeUser = {
  id: string
  githubId: number
  username: string
  displayName: string | null
  avatarUrl: string | null
  createdAt: Date
  updatedAt: Date
}

export const getSessionUser = createServerFn({ method: 'GET' }).handler(
  async (): Promise<SafeUser | null> => {
    const { getSession } = await import('@tanstack/react-start/server')
    const { sessionConfig } = await import('@/server/lib/session')
    const { db } = await import('@/server/db')
    const { sessions, users } = await import('@/server/db/schema')

    const session = await getSession(sessionConfig)
    const userId = session.data.userId as string | undefined

    if (!userId) return null

    const result = await db
      .select({
        session: sessions,
        user: users,
      })
      .from(sessions)
      .innerJoin(users, eq(sessions.userId, users.id))
      .where(
        and(eq(sessions.userId, userId), gt(sessions.expiresAt, new Date())),
      )
      .limit(1)

    if (!result[0]) return null

    const { user } = result[0]
    return {
      id: user.id,
      githubId: user.githubId,
      username: user.username,
      displayName: user.displayName,
      avatarUrl: user.avatarUrl,
      createdAt: user.createdAt,
      updatedAt: user.updatedAt,
    }
  },
)
