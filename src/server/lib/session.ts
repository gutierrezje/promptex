import type { SessionConfig } from '@tanstack/react-start/server'
import { env } from './env'

export const sessionConfig: SessionConfig = {
  password: env.SESSION_SECRET,
  name: 'issuance_session',
  maxAge: 60 * 60 * 24 * 7, // 7 days
  cookie: {
    httpOnly: true,
    secure: env.NODE_ENV === 'production',
    sameSite: 'lax',
    path: '/',
  },
}
