import type { ErrorEvent as SentryErrorEvent, Exception } from "@sentry/nextjs";

/**
 * Filenames the browser stack parser emits when a frame has no real source location. They
 * carry no attribution either way, so they are skipped rather than counted as third party.
 */
const UNATTRIBUTABLE_FILENAMES = new Set([
  "<anonymous>",
  "[native code]",
  "native",
]);

/**
 * Filename patterns that can only come from code we did not ship.
 *
 * Deliberately a denylist rather than an allowlist of our own paths: an unrecognized filename
 * leaves the event alone, so a new bundler layout can never silently swallow our own errors.
 */
const THIRD_PARTY_FILENAME_PATTERNS = [
  // `@sentry/nextjs` rewrites the origin of every frame to `app://` before the event is sent
  // (nextjsClientStackFrameNormalizationIntegration), so a wallet provider injected at
  // `chrome-extension://<id>/inpage.js` reaches us as `app:///inpage.js`. Our own scripts are
  // always under `app:///_next/...` and `public/` holds no root-level files, so a bare
  // filename directly under `app:///` is never ours. Serving a root-level `.js` from
  // `public/` would mean revisiting this.
  /^app:\/\/\/[^/]+\.js$/i,
  // Urban VPN injects numeric scripts under `executors/`. Sentry strips the extension origin,
  // leaving paths such as `app:///executors/200.js`. This app has no such public directory;
  // keep the numeric filename constraint so an unrelated future route is not hidden.
  /^app:\/\/\/executors\/\d+\.js$/i,
  // Extension schemes survive the rewrite above in browsers that report an opaque origin for
  // them, since there is then no origin substring to replace.
  /^(chrome|moz|safari-web|safari|ms-browser|opera)-extension:\/\//i,
  /^webkit-masked-url:\/\//i,
  /^(chrome|resource|extensions):\/\//i,
  // Injected script whose sourceURL was a bare filename, with no origin to strip.
  /^[^/:?#]+\.js$/i,
];

const isThirdPartyFilename = (filename: string): boolean => {
  // A script URL can carry a cache-busting query or a fragment, which the origin rewrite
  // leaves in place. Only the path identifies the script, and the patterns anchored on `.js$`
  // would otherwise fall through and treat the frame as ours.
  const path = filename.replace(/[?#].*$/, "");

  return THIRD_PARTY_FILENAME_PATTERNS.some((pattern) => pattern.test(path));
};

/**
 * The exception that was actually thrown. Chained errors prepend their causes, so the root is
 * the last value without a `parent_id` — the same lookup the SDK's own event filters use.
 */
const getRootException = (event: SentryErrorEvent): Exception | undefined => {
  const values = event.exception?.values;
  if (!values?.length) return undefined;

  for (let index = values.length - 1; index >= 0; index--) {
    const value = values[index];
    if (value && value.mechanism?.parent_id === undefined) return value;
  }

  return values[values.length - 1];
};

/** Every frame filename across the whole chain that identifies some real script. */
const attributableFilenames = (event: SentryErrorEvent): string[] =>
  (event.exception?.values ?? [])
    .flatMap((value) => value.stacktrace?.frames ?? [])
    .map((frame) => frame.filename)
    .filter(
      (filename): filename is string =>
        !!filename && !UNATTRIBUTABLE_FILENAMES.has(filename)
    );

/**
 * Whether an event is a crash inside a third-party script injected into the page rather than a
 * fault in our own code.
 *
 * Wallet extensions inject a provider script (conventionally `inpage.js`) into every page, and
 * its internal failures reach the SDK's global handlers as though they were ours.
 * `AppWalletProvider` discovers wallets purely through Wallet Standard, so this arrives from
 * every wallet a visitor happens to have installed, and none of it is ours to fix.
 *
 * Two conditions must both hold, so the check errs toward reporting:
 *  1. The error was unhandled. Anything passed to `captureException` is kept, which is why a
 *     wallet signing failure caught by a modal still reports even though the throw itself
 *     happened inside the wallet's frames.
 *  2. No frame anywhere in the chain belongs to us. A stack running from our code into the
 *     wallet's is our bug to investigate.
 */
export const isThirdPartyScriptError = (event: SentryErrorEvent): boolean => {
  const root = getRootException(event);
  // Message events, and exceptions with no mechanism, carry no evidence either way.
  if (!root || root.mechanism?.handled !== false) return false;

  const filenames = attributableFilenames(event);
  if (filenames.length === 0) return false;

  return filenames.every(isThirdPartyFilename);
};
