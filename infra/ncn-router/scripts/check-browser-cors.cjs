// Run with Playwright available through NODE_PATH (see the deployment README).
const { chromium, webkit } = require("playwright");

async function checkBrowser(name, engine) {
  const browser = await engine.launch();
  try {
    const page = await browser.newPage();
    // Give the probe the production frontend origin without running app queries in parallel.
    await page.route("https://governance.solana.com/", (route) =>
      route.fulfill({ contentType: "text/html", body: "<!doctype html><title>NCN CORS probe</title>" }),
    );
    await page.goto("https://governance.solana.com/");
    const responses = await page.evaluate(async () => {
      const base = "https://ncn-governance.solana.com";
      const results = [];
      async function read(path) {
        const response = await fetch(`${base}${path}`, {
          signal: AbortSignal.timeout(20_000),
        });
        if (!response.ok) throw new Error(`${path}: HTTP ${response.status} at ${response.url}`);
        const data = await response.json();
        results.push({ path, status: response.status, operator: new URL(response.url).host });
        return data;
      }
      const latest = await read("/meta?network=mainnet");
      const slot = String(latest.slot);
      if (!/^\d+$/.test(slot)) throw new Error("Latest metadata has no numeric slot");
      await read(`/meta?network=mainnet&slot=${slot}`);
      await read(`/metas?network=mainnet&slots=${slot}`);
      return results;
    });
    console.log(`${name}: ${JSON.stringify(responses)}`);
  } finally {
    await browser.close();
  }
}

(async () => {
  for (const [name, engine] of Object.entries({ chromium, webkit })) {
    await checkBrowser(name, engine);
  }
})().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
