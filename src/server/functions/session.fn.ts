import { createServerFn } from '@tanstack/react-start'
import { getSession } from '@tanstack/react-start/server'
import { eq, and, gt } from 'drizzle-orm'
import { sessionConfig } from '@/server/lib/session'
import { db } from '@/server/db'
import { sessions, users } from '@/server/db/schema'

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
