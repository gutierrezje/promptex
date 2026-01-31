import { createFileRoute, redirect } from '@tanstack/react-router'
import { getGitHubAuthUrl } from '@/server/functions/auth.fns'

export const Route = createFileRoute('/auth/github')({
  beforeLoad: async () => {
    const url = await getGitHubAuthUrl()
    throw redirect({ href: url })
  },
  component: () => null,
})
