import { createFileRoute, redirect } from '@tanstack/react-router'
import { handleGitHubCallback } from '@/server/functions/auth.fns'

export const Route = createFileRoute('/auth/github/callback')({
  validateSearch: (search: Record<string, unknown>) => ({
    code: (search.code as string) ?? '',
    state: (search.state as string) ?? '',
  }),
  loaderDeps: ({ search }) => ({ code: search.code, state: search.state }),
  loader: async ({ deps: { code, state } }) => {
    const result = await handleGitHubCallback({
      data: { code, state },
    })

    if (result.error) {
      throw redirect({
        to: '/login',
        search: { error: result.error },
      })
    }

    throw redirect({ to: '/dashboard' })
  },
  component: () => null,
})
