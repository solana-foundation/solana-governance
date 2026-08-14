import { GovernanceDashboard } from '@/components/governance/GovernanceDashboard'
import Link from 'next/link'

const SGP_REPO_URL =
  'https://github.com/solana-foundation/solana-governance-proposals'

function GovernanceProcessCard() {
  return (
    <section className="glass-card flex flex-col gap-3 rounded-2xl border border-white/10 p-5 sm:flex-row sm:items-center sm:justify-between">
      <div>
        <h2 className="text-sm font-semibold text-white">
          New to Solana governance?
        </h2>
        <p className="mt-1 text-sm text-white/60">
          Learn how proposals move from idea to on-chain vote — phases,
          thresholds, and how your stake counts.
        </p>
      </div>
      <div className="flex shrink-0 gap-4 text-sm font-medium">
        <Link
          href="/faq"
          className="text-primary transition-colors hover:text-white"
        >
          Read the FAQ
        </Link>
        <a
          href={SGP_REPO_URL}
          target="_blank"
          rel="noopener noreferrer"
          className="text-white/60 transition-colors hover:text-white"
        >
          SGP repository
        </a>
      </div>
    </section>
  )
}

export default function DashboardPage() {
  return (
    <main className="py-8 space-y-10">
      <GovernanceProcessCard />
      <GovernanceDashboard />
    </main>
  )
}
