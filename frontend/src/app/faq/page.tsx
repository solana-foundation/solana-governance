import type { Metadata } from "next";

export const metadata: Metadata = {
  title: "Governance FAQ | Solana Validator Governance",
  description:
    "How Solana Governance Proposals (SGPs) work: phases, thresholds, voting, and where to participate.",
};

const SGP_REPO_URL =
  "https://github.com/solana-foundation/solana-governance-proposals";
const CONSTITUTION_PR_URL =
  "https://github.com/solana-foundation/solana-governance-proposals/pull/3";
const SGP_DISCUSSIONS_URL =
  "https://github.com/solana-foundation/solana-governance-proposals/discussions";
const DOCS_URL = "https://docs.governance.solana.com";

function ExternalLink({
  href,
  children,
}: {
  href: string;
  children: React.ReactNode;
}) {
  return (
    <a
      href={href}
      target="_blank"
      rel="noopener noreferrer"
      className="text-primary underline underline-offset-4 hover:text-white transition-colors"
    >
      {children}
    </a>
  );
}

interface FaqEntry {
  question: string;
  answer: React.ReactNode;
}

const faqEntries: FaqEntry[] = [
  {
    question: "What is an SGP?",
    answer: (
      <>
        A <strong>Solana Governance Proposal (SGP)</strong> is a stake-weighted,
        on-chain <em>signaling vote</em>. It captures a directional decision
        rather than a detailed technical specification. A &quot;yes&quot; 
        outcome is a mandate to proceed; the
        implementation that follows is normally specified in one or more SIMDs.
        Proposals live in the{" "}
        <ExternalLink href={SGP_REPO_URL}>SGP repository on GitHub</ExternalLink>
        .
      </>
    ),
  },
  {
    question: "How is an SGP different from a SIMD?",
    answer: (
      <>
        They answer different questions. An SGP answers{" "}
        <em>&quot;should we do this?&quot;</em> and gauges community support.
        SGPs are decided by a stake-weighted vote of validators and stakers.
        A{" "}
        <ExternalLink href="https://github.com/solana-foundation/solana-improvement-documents">
          SIMD
        </ExternalLink>{" "}
        answers <em>&quot;how exactly do we do this?&quot;</em>. A SIMD is a
        detailed protocol specification reviewed by core developers. SIMDs pass
        optimistically and are not voted on, unless enough stake demands a vote
        (see below).
      </>
    ),
  },
  {
    question: "When does a vote happen?",
    answer: (
      <>
        A vote occurs when the validator set asks for one. An SGP vote is
        triggered when <strong>15% of active stake</strong> supports
        holding it. If less than 15% of stake signals support within the
        support window, no vote occurs. This keeps voting reserved for
        systemic decisions with genuine community interest.
      </>
    ),
  },
  {
    question: "What are the phases, and how long do they take?",
    answer: (
      <>
        <p className="mb-3">
          Once a proposal is created on-chain, it moves through fixed,
          program-enforced phases (one epoch is roughly two days):
        </p>
        <ul className="list-disc space-y-1.5 pl-5">
          <li>
            <strong>Support</strong> — up to 7 epochs for validators
            representing 15% of stake to sponsor the proposal. If the threshold
            is not met in time, the proposal expires. During the support phase,
            if at least 15% of stake supports a proposal, then the discussion
            phase begins in the following epoch.
          </li>
          <li>
            <strong>Review (discussion)</strong> — 7 epochs. The proposal text
            is frozen at a specific GitHub commit; the community studies it.
          </li>
          <li>
            <strong>Snapshot</strong> — 1 epoch. The Node Consensus Network
            (NCN) fixes the stake distribution used to weight votes.
          </li>
          <li>
            <strong>Voting</strong> — 3 epochs. Validators and stakers cast
            stake-weighted votes: For, Against, or Abstain.
          </li>
        </ul>
        <p className="mt-3">
          A proposal that reaches the support threshold early advances early —
          the review clock starts when support succeeds, not when the support
          window would have ended.
        </p>
      </>
    ),
  },
  {
    question: "Who can create a proposal?",
    answer: (
      <>
        Anyone can author a draft SGP as a pull request in the{" "}
        <ExternalLink href={SGP_REPO_URL}>SGP repository</ExternalLink>.
        Creating the on-chain proposal must be done by a validator
        with at least <strong>100,000 SOL of active stake</strong>. The minimum
        stake required for proposals prevents spam.
      </>
    ),
  },
  {
    question: "How is the vote weighted?",
    answer: (
      <>
        By active stake, at the snapshot taken before voting opens. Each vote is
        verified on-chain against a Merkle proof of that snapshot. Validators
        can cast their full stake on one option or split it across options in
        basis points.
      </>
    ),
  },
  {
    question: "Do stakers have the ability to vote?",
    answer: (
      <>
        Yes. By default your stake votes with your validator, but you have{" "}
        <strong>vote sovereignty</strong>: you can cast an override vote with
        your own stake account before, after, or in the absence of your
        validator&apos;s vote. A staker can override the validator&apos;s vote
        for the portion of stake they delegate to the validator.
      </>
    ),
  },
  {
    question: "What does it take for a vote to pass?",
    answer: (
      <>
        Per the proposed{" "}
        <ExternalLink href={CONSTITUTION_PR_URL}>
          Solana Constitution (SGP-0001)
        </ExternalLink>
        : a quorum of <strong>one-third of network stake</strong> must
        participate (For + Against + Abstain), and{" "}
        <strong>two-thirds of participating stake</strong> must vote For. If
        quorum is not met, the outcome is <em>inconclusive</em>. An SGP that
        does not reach quorum does not pass and does not block any associated
        SIMD.
      </>
    ),
  },
  {
    question: "Where do I discuss proposals?",
    answer: (
      <>
        The canonical venue is GitHub. Discussions should occur on each
        proposal&apos;s pull request and the repository&apos;s{" "}
        <ExternalLink href={SGP_DISCUSSIONS_URL}>Discussions</ExternalLink> in
        the <ExternalLink href={SGP_REPO_URL}>SGP repository</ExternalLink>.
        Conversation on other platforms is welcome as advisory input, but
        GitHub is where deliberation formally happens.
      </>
    ),
  },
  {
    question: "Where can I learn how the on-chain system works?",
    answer: (
      <>
        The <ExternalLink href={DOCS_URL}>technical documentation</ExternalLink>{" "}
        covers the on-chain programs, the NCN snapshot process, CLI usage for
        validators and stakers, and program reference material.
      </>
    ),
  },
];

export default function FaqPage() {
  return (
    <main className="mx-auto max-w-3xl px-4 py-12 space-y-10">
      <header className="space-y-4">
        <h1 className="h2 font-semibold text-white">Governance FAQ</h1>
        <p className="text-sm leading-relaxed text-white/70">
          A high-level guide to how Solana Governance Proposals (SGPs) work.
          For the full process and policy, see the{" "}
          <ExternalLink href={SGP_REPO_URL}>
            SGP repository README
          </ExternalLink>{" "}
          and the proposed{" "}
          <ExternalLink href={CONSTITUTION_PR_URL}>
            Solana Constitution (SGP-0001)
          </ExternalLink>
          .
        </p>
      </header>

      <div className="space-y-4">
        {faqEntries.map(({ question, answer }) => (
          <details
            key={question}
            className="glass-card group rounded-2xl border border-white/10 p-5 open:pb-6"
          >
            <summary className="cursor-pointer list-none text-base font-medium text-white marker:content-none [&::-webkit-details-marker]:hidden">
              <span className="mr-2 inline-block text-primary transition-transform group-open:rotate-90">
                ›
              </span>
              {question}
            </summary>
            <div className="mt-3 text-sm leading-relaxed text-white/70">
              {answer}
            </div>
          </details>
        ))}
      </div>

      <footer className="text-sm text-white/50">
        Something missing? Open a{" "}
        <ExternalLink href={`${SGP_DISCUSSIONS_URL}/new/choose`}>discussion</ExternalLink> in
        the <ExternalLink href={SGP_REPO_URL}>SGP repository</ExternalLink>.
      </footer>
    </main>
  );
}
