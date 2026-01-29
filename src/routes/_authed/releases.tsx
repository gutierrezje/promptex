import { createFileRoute } from '@tanstack/react-router'
import { TopBar } from '@/components/layout/TopBar'

export const Route = createFileRoute('/_authed/releases')({
  component: ReleasesPage,
})

function ReleasesPage() {
  return (
    <>
      <TopBar title="Releases" />
      <main className="flex-1 p-6">
        <p className="text-muted-foreground">
          Release activity will appear here.
        </p>
      </main>
    </>
  )
}
