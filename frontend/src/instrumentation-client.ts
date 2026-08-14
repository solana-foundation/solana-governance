// This file configures the initialization of Sentry on the client.
// The added config here will be used whenever a users loads a page in their browser.
// https://docs.sentry.io/platforms/javascript/guides/nextjs/

import * as Sentry from "@sentry/nextjs";
import { isThirdPartyScriptError } from "@/lib/sentryFilters";
import { env } from "./env";

Sentry.init({
  dsn: env.NEXT_PUBLIC_SENTRY_DSN,

  // Define how likely traces are sampled. Adjust this value in production, or use tracesSampler for greater control.
  tracesSampleRate: 1,

  // Disable sending user PII (Personally Identifiable Information)
  // https://docs.sentry.io/platforms/javascript/guides/nextjs/configuration/options/#sendDefaultPii
  sendDefaultPii: false,

  // Wallet extensions inject their provider script into every page, and their internal
  // failures surface through our global handlers as unactionable noise. Drop those, but only
  // when the error is unhandled and no frame of ours is in the stack — see the reasoning in
  // isThirdPartyScriptError. Dropped counts are still visible in Sentry under Stats > Usage.
  //
  // Note this cannot be done with `denyUrls`, which applies to every event however it was
  // captured, and so would also discard the wallet signing failures the modals report
  // explicitly.
  beforeSend(event) {
    return isThirdPartyScriptError(event) ? null : event;
  },
});

export const onRouterTransitionStart = Sentry.captureRouterTransitionStart;
