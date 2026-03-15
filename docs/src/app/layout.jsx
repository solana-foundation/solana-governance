import { Footer, Layout, Navbar } from 'nextra-theme-docs';
import { Banner, Head } from 'nextra/components';
import { getPageMap } from 'nextra/page-map';
import 'nextra-theme-docs/style.css';
import { readFileSync } from 'fs';
import { join } from 'path';

export const metadata = {};

// Get version from env or package.json
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

const banner = (
  <Banner storageKey="svmgov-docs-banner">svmgov Governance CLI</Banner>
);
const navbar = (
  <Navbar
    logo={<b>svmgov Governance CLI</b>}
    rightContent={
      <span style={{ fontSize: '12px', opacity: 0.7, marginLeft: '1rem' }}>
        {version}
      </span>
    }
  />
);
const footer = (
  <Footer>
    {new Date().getFullYear()} © svmgov. Documentation version {version}
  </Footer>
);

export default async function RootLayout({ children }) {
  return (
    <html lang="en" dir="ltr" suppressHydrationWarning>
      <Head />
      <body>
        <Layout
          banner={banner}
          navbar={navbar}
          pageMap={await getPageMap()}
          docsRepositoryBase="https://github.com/3uild-3thos/govcontract/tree/main/docs"
          footer={footer}
        >
          {children}
        </Layout>
      </body>
    </html>
  );
}
