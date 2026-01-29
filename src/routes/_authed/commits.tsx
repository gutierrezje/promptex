import { createFileRoute } from '@tanstack/react-router'
import { TopBar } from '@/components/layout/TopBar'

export const Route = createFileRoute('/_authed/commits')({
  component: CommitsPage,
})

function CommitsPage() {
  return (
    <>
      <TopBar title="Commits" />
      <main className="flex-1 p-6">
        <p className="text-muted-foreground">
          Commit activity will appear here.
        </p>
      </main>
    </>
  )
}
