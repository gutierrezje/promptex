import { createFileRoute } from '@tanstack/react-router'
import { TopBar } from '@/components/layout/TopBar'

export const Route = createFileRoute('/_authed/pulls')({
  component: PullsPage,
})

function PullsPage() {
  return (
    <>
      <TopBar title="Pull Requests" />
      <main className="flex-1 p-6">
        <p className="text-muted-foreground">
          Pull request activity will appear here.
        </p>
      </main>
    </>
  )
}
