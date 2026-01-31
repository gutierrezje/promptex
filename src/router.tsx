import { createRouter } from '@tanstack/react-router'
import { routeTree } from './routeTree.gen'
import type { SafeUser } from '@/server/functions/session.fn'

export interface RouterContext {
  user: SafeUser | null
}

export const getRouter = () => {
  const router = createRouter({
    routeTree,
    context: { user: null } satisfies RouterContext,
    scrollRestoration: true,
    defaultPreloadStaleTime: 0,
  })
  return router
}

declare module '@tanstack/react-router' {
  interface Register {
    router: ReturnType<typeof getRouter>
  }
}
