import { createFileRoute } from '@tanstack/react-router'

export const Route = createFileRoute('/')({
  component: LandingPage,
})

function LandingPage() {
  return (
    <div className="flex min-h-screen flex-col items-center justify-center bg-slate-950">
      <h1 className="mb-4 text-5xl font-bold tracking-tight text-white">
        Issuance
      </h1>
      <p className="mb-8 max-w-lg text-center text-lg text-slate-400">
        A Bloomberg-style terminal for your GitHub repositories. Monitor
        issues, commits, PRs, and releases with AI-powered insights.
      </p>
      <a
        href="/login"
        className="rounded-lg bg-emerald-600 px-6 py-3 font-semibold text-white transition-colors hover:bg-emerald-500"
      >
        Sign in with GitHub
      </a>
    </div>
  )
}
