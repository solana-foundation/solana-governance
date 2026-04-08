# Governance config API cache

The `GET /api/governance/config` endpoint uses Next.js **`use cache: remote`** to cache the on-chain GlobalConfig in Vercel’s Runtime Cache.

## Why remote cache?

- **`use cache`** (in-memory): No extra Vercel charge, but on serverless the cache does not persist across requests (each request can hit a different instance), so we’d refetch from RPC often.
- **`use cache: remote`**: Uses a shared remote cache (Vercel Runtime Cache). Cache is shared across all instances, so we get real cache reuse and fewer RPC calls. Usage is [charged](https://vercel.com/docs/runtime-cache#limits-and-usage) by Vercel (regional / Managed Infrastructure pricing).

We use **remote** so that on serverless the cache actually persists across requests and RPC load stays low.

## Cost estimate (~1000 MAU)

**Assumptions**

- ~1000 MAU.
- ~2–3 config API requests per user per month (e.g. once per session when opening governance) → **~2,500 requests/month**.
- Vercel does not publish separate “Runtime Cache” unit prices; [Data Cache pricing](https://vercel.com/blog/improved-infrastructure-pricing) is used as a proxy: **$0.40/million reads**, **$4.00/million writes**.

**Usage (ballpark)**

| Metric     | Estimate       | Notes                                                                                                                |
| ---------- | -------------- | -------------------------------------------------------------------------------------------------------------------- |
| **Reads**  | ~2,500 / month | One cache read per API request.                                                                                      |
| **Writes** | ~700 / month   | One write per revalidate window (1h) when there’s at least one request; 720 windows/month → ~700 hours with traffic. |

**Cost (Data Cache–style rates)**

- Reads: (2,500 / 1,000,000) × $0.40 ≈ **$0.001**
- Writes: (700 / 1,000,000) × $4.00 ≈ **$0.003**
- **Total ≈ $0.004/month** (well under **$0.05/month**).

So for ~1k MAU, Runtime Cache cost for this single endpoint is negligible. Exact cost depends on Vercel’s actual Runtime Cache pricing and region; see [Vercel regional pricing](https://vercel.com/docs/pricing/regional-pricing) and the project’s **Observability → Runtime Cache** in the dashboard.

## Configuration

- **Cache tag:** `governance-config`. Invalidate on demand with `revalidateTag('governance-config')` (e.g. from a Server Action when config changes on-chain).
- **Revalidate:** 3600 seconds (1 hour).
- **Implementation:** `frontend/src/app/api/governance/config/route.ts`.
