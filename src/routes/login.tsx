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

export const Route = createFileRoute('/login')({
  component: LoginPage,
})

function LoginPage() {
  return (
    <div className="flex min-h-screen items-center justify-center bg-background">
      <Card className="w-full max-w-sm">
        <CardHeader className="text-center">
          <CardTitle className="text-2xl">Sign in to Issuance</CardTitle>
          <CardDescription>
            Connect your GitHub account to start discovering impactful issues.
          </CardDescription>
        </CardHeader>
        <CardContent>
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
