import { createFileRoute } from '@tanstack/react-router'
import { TopBar } from '@/components/layout/TopBar'

export const Route = createFileRoute('/_authed/settings')({
  component: SettingsPage,
})

function SettingsPage() {
  return (
    <>
      <TopBar title="Settings" />
      <main className="flex-1 p-6">
        <p className="text-muted-foreground">
          Your account and preferences will appear here.
        </p>
      </main>
    </>
  )
}
