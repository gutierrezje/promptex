import { createFileRoute } from '@tanstack/react-router'
import { TopBar } from '@/components/layout/TopBar'

export const Route = createFileRoute('/_authed/subscriptions')({
  component: SubscriptionsPage,
})

function SubscriptionsPage() {
  return (
    <>
      <TopBar title="Subscriptions" />
      <main className="flex-1 p-6">
        <p className="text-muted-foreground">
          Manage your repo subscriptions here.
        </p>
      </main>
    </>
  )
}
