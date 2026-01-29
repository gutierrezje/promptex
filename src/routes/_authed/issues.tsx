import { createFileRoute } from '@tanstack/react-router'
import { TopBar } from '@/components/layout/TopBar'

export const Route = createFileRoute('/_authed/issues')({
  component: IssuesPage,
})

function IssuesPage() {
  return (
    <>
      <TopBar title="Issues" />
      <main className="flex-1 p-6">
        <p className="text-muted-foreground">
          Issues across your subscribed repos will appear here.
        </p>
      </main>
    </>
  )
}
