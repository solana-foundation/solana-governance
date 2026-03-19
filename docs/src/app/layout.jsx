import { Footer, Layout, Navbar } from 'nextra-theme-docs';
import { Head } from 'nextra/components';
import { getPageMap } from 'nextra/page-map';
import 'nextra-theme-docs/style.css';
import { readFileSync } from 'fs';
import { join } from 'path';

export const metadata = {
  title: {
    default: 'Solana Governance',
    template: '%s | Solana Governance',
  },
  description:
    'Complete documentation for Solana Governance — decentralized, stake-weighted on-chain governance powered by NCN snapshots and Merkle proof verification.',
};

function getVersion() {
  if (process.env.NEXT_PUBLIC_DOCS_VERSION) {
    return process.env.NEXT_PUBLIC_DOCS_VERSION;
  }
  try {
    const packageJsonPath = join(process.cwd(), 'package.json');
    const packageJson = JSON.parse(readFileSync(packageJsonPath, 'utf8'));
    return `v${packageJson.version || '0.1.0'}`;
  } catch {
    return 'v0.1.0';
  }
}

const version = getVersion();

const Logo = () => (
  <span
    style={{
      display: 'flex',
      alignItems: 'center',
      gap: '0.5rem',
      fontWeight: 700,
      fontSize: '1rem',
      letterSpacing: '-0.02em',
    }}
  >
    <svg width="22" height="22" viewBox="0 0 32 32" fill="none" xmlns="http://www.w3.org/2000/svg">
      <circle cx="16" cy="16" r="15" stroke="currentColor" strokeWidth="2" />
      <path d="M8 20l8-12 8 12H8z" fill="currentColor" opacity="0.85" />
      <circle cx="16" cy="11" r="2.5" fill="currentColor" />
    </svg>
    Solana Governance
  </span>
);

const navbar = (
  <Navbar
    logo={<Logo />}
    rightContent={
      <span
        style={{
          fontSize: '11px',
          opacity: 0.5,
          fontFamily: 'monospace',
          background: 'var(--nextra-bg)',
          border: '1px solid var(--nextra-border-color)',
          borderRadius: '4px',
          padding: '2px 6px',
        }}
      >
        {version}
      </span>
    }
  />
);

const footer = (
  <Footer>
    <span style={{ opacity: 0.6, fontSize: '0.85rem' }}>
      {new Date().getFullYear()} © Solana Governance — NCN · svmgov
    </span>
  </Footer>
);

export default async function RootLayout({ children }) {
  return (
    <html lang="en" dir="ltr" suppressHydrationWarning>
      <Head />
      <body>
        <Layout
          navbar={navbar}
          pageMap={await getPageMap()}
          docsRepositoryBase="https://github.com/3uild-3thos/svmgov_program/tree/main/docs"
          footer={footer}
          sidebar={{ defaultMenuCollapseLevel: 1 }}
          editLink="Edit this page on GitHub"
        >
          {children}
        </Layout>
      </body>
    </html>
  );
}
