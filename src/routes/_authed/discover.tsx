import { createFileRoute } from '@tanstack/react-router'
import { TopBar } from '@/components/layout/TopBar'

export const Route = createFileRoute('/_authed/discover')({
  component: DiscoverPage,
})

function DiscoverPage() {
  return (
    <>
      <TopBar title="Discover" />
      <main className="flex-1 p-6">
        <p className="text-muted-foreground">
          High-impact issues to contribute to will appear here.
        </p>
      </main>
    </>
  )
}
