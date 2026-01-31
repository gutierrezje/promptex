import { createFileRoute } from '@tanstack/react-router'
import { Github } from 'lucide-react'
import { Button } from '@/components/ui/button'
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card'

const errorMessages: Record<string, string> = {
  invalid_state: 'Authentication was interrupted. Please try again.',
  token_exchange: 'Could not complete sign-in with GitHub. Please try again.',
  profile_fetch: 'Could not fetch your GitHub profile. Please try again.',
}

export const Route = createFileRoute('/login')({
  validateSearch: (search: Record<string, unknown>) => ({
    error: (search.error as string) || '',
  }),
  component: LoginPage,
})

function LoginPage() {
  const { error } = Route.useSearch()

  return (
    <div className="flex min-h-screen items-center justify-center bg-background">
      <Card className="w-full max-w-sm">
        <CardHeader className="text-center">
          <CardTitle className="text-2xl">Sign in to Issuance</CardTitle>
          <CardDescription>
            Connect your GitHub account to start discovering impactful issues.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {error && (
            <div className="rounded-md bg-destructive/10 border border-destructive/20 px-3 py-2 text-sm text-destructive">
              {errorMessages[error] ?? 'An unexpected error occurred. Please try again.'}
            </div>
          )}
          <Button asChild className="w-full" size="lg">
            <a href="/auth/github">
              <Github className="mr-2 h-5 w-5" />
              Sign in with GitHub
            </a>
          </Button>
        </CardContent>
      </Card>
    </div>
  )
}
