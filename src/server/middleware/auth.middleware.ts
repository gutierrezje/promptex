import { createMiddleware } from '@tanstack/react-start'
import { eq, and, gt } from 'drizzle-orm'
import type { SafeUser } from '@/server/functions/session.fn'

export const authMiddleware = createMiddleware().server(async ({ next }) => {
  const { getSession } = await import('@tanstack/react-start/server')
  const { sessionConfig } = await import('@/server/lib/session')
  const { db } = await import('@/server/db')
  const { sessions, users } = await import('@/server/db/schema')

  const session = await getSession(sessionConfig)
  const userId = session.data.userId as string | undefined

  let user: SafeUser | null = null

  if (userId) {
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

    if (result[0]) {
      const { user: dbUser } = result[0]
      user = {
        id: dbUser.id,
        githubId: dbUser.githubId,
        username: dbUser.username,
        displayName: dbUser.displayName,
        avatarUrl: dbUser.avatarUrl,
        createdAt: dbUser.createdAt,
        updatedAt: dbUser.updatedAt,
      }
    }
  }

  return next({ context: { user } })
})
