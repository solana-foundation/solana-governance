import type {
  ErrorEvent as SentryErrorEvent,
  Exception,
  StackFrame,
} from "@sentry/nextjs";
import { isThirdPartyScriptError } from "../sentryFilters";

type Mechanism = NonNullable<Exception["mechanism"]>;

/** What the SDK attaches to an error caught by `window.onerror`. */
const UNHANDLED: Mechanism = {
  type: "auto.browser.global_handlers.onerror",
  handled: false,
};

/** What the SDK attaches to anything passed to `captureException`. */
const HANDLED: Mechanism = { type: "generic", handled: true };

const frames = (...filenames: string[]): StackFrame[] =>
  filenames.map((filename) => ({ filename, in_app: true }));

const exception = (
  value: string,
  mechanism: Mechanism | undefined,
  stackFrames: StackFrame[] | undefined
): Exception => ({
  type: "Error",
  value,
  mechanism,
  ...(stackFrames ? { stacktrace: { frames: stackFrames } } : {}),
});

// `type` is required on ErrorEvent and is always undefined — that is what distinguishes an
// error event from a transaction.
const eventOf = (...values: Exception[]): SentryErrorEvent => ({
  type: undefined,
  exception: { values },
});

// The reported issue, as it arrives post-normalization. Sentry orders frames oldest first,
// i.e. the reverse of the printed stack trace.
const SSE_ERROR_FRAMES = frames(
  "app:///inpage.js",
  "app:///inpage.js",
  "app:///inpage.js",
  "app:///inpage.js",
  "app:///inpage.js",
  "app:///inpage.js"
);

describe("isThirdPartyScriptError", () => {
  it("drops the unhandled wallet provider crash we see in production", () => {
    const event = eventOf(
      exception("func sseError not found", UNHANDLED, SSE_ERROR_FRAMES)
    );

    expect(isThirdPartyScriptError(event)).toBe(true);
  });

  it("keeps the same error when our own code captured it", () => {
    // A modal catching a wallet signing failure: the throw is inside the wallet, but we asked
    // for the report, so it is signal.
    const event = eventOf(
      exception("func sseError not found", HANDLED, SSE_ERROR_FRAMES)
    );

    expect(isThirdPartyScriptError(event)).toBe(false);
  });

  it("drops unhandled errors from wrapped browser callbacks", () => {
    // Injected scripts commonly throw out of setTimeout/addEventListener, which the SDK
    // reports under its own mechanism type. The gate is `handled`, never the type string.
    const event = eventOf(
      exception(
        "func sseError not found",
        {
          type: "auto.browser.browserapierrors.setTimeout",
          handled: false,
        },
        SSE_ERROR_FRAMES
      )
    );

    expect(isThirdPartyScriptError(event)).toBe(true);
  });

  it.each([
    "chrome-extension://bfnaelmomeimhlpmgjnjophhpkkoljpa/inpage.js",
    "moz-extension://d9b3a4f1-0c62-4f7e-8f2b-1a2b3c4d5e6f/inpage.js",
    "safari-web-extension://B2D5A1C0-1234-4321-ABCD-0123456789AB/inpage.js",
    "webkit-masked-url://hidden/",
    "chrome://internals/script.js",
    "inpage.js",
    // A cache-busting query or a fragment survives the origin rewrite. Only the path
    // identifies the script, so the patterns anchored on `.js$` must not be thrown off by it.
    "app:///inpage.js?v=2",
    "app:///inpage.js?",
    "app:///inpage.js#bridge",
    "app:///inpage.js?v=2#bridge",
    "app:///inpage.js?redirect=/_next/static/chunks/page.js",
    "inpage.js?v=2",
    "chrome-extension://bfnaelmomeimhlpmgjnjophhpkkoljpa/inpage.js?v=2",
  ])("drops unhandled errors from %s", (filename) => {
    const event = eventOf(
      exception("provider bridge failed", UNHANDLED, frames(filename))
    );

    expect(isThirdPartyScriptError(event)).toBe(true);
  });

  it("keeps an unhandled error whose stack runs from our code into the wallet's", () => {
    const event = eventOf(
      exception(
        "func sseError not found",
        UNHANDLED,
        frames(
          "app:///_next/static/chunks/app/page-abc123.js",
          "app:///inpage.js"
        )
      )
    );

    expect(isThirdPartyScriptError(event)).toBe(false);
  });

  it("keeps an unhandled error entirely within our own bundles", () => {
    const event = eventOf(
      exception(
        "Cannot read properties of undefined",
        UNHANDLED,
        frames(
          "app:///_next/static/chunks/main-app-0a1b2c3d.js",
          "app:///_next/static/chunks/app/proposal/[proposalPk]/page-abc123.js"
        )
      )
    );

    expect(isThirdPartyScriptError(event)).toBe(false);
  });

  it.each([
    "app:///_next/static/chunks/app/page-abc123.js?v=2",
    "app:///_next/static/chunks/app/page-abc123.js#L1",
  ])("keeps our own bundles when %s carries a query or fragment", (filename) => {
    // Stripping the query leaves the path's slashes intact, so `[^/]+` still cannot span
    // them and our own chunks stay first party.
    const event = eventOf(exception("boom", UNHANDLED, frames(filename)));

    expect(isThirdPartyScriptError(event)).toBe(false);
  });

  it("keeps errors from inline scripts in our own documents", () => {
    // The rewrite strips the origin from the document URL too, and a route is not a script.
    const event = eventOf(
      exception("boom", UNHANDLED, frames("app:///proposal/abc"))
    );

    expect(isThirdPartyScriptError(event)).toBe(false);
  });

  it.each([
    [
      "a message event",
      { type: undefined, message: "something happened" } as SentryErrorEvent,
    ],
    ["an event with no exception values", eventOf()],
    [
      "an exception with no stack trace",
      eventOf(exception("Script error.", UNHANDLED, undefined)),
    ],
    [
      "an exception with no attributable frames",
      eventOf(
        exception("boom", UNHANDLED, frames("<anonymous>", "[native code]"))
      ),
    ],
    [
      "an exception with frames but no filenames",
      eventOf({
        type: "Error",
        value: "boom",
        mechanism: UNHANDLED,
        stacktrace: { frames: [{ function: "b", in_app: true }] },
      }),
    ],
    [
      "an exception with no mechanism",
      eventOf(exception("boom", undefined, SSE_ERROR_FRAMES)),
    ],
  ])("keeps %s", (_label, event) => {
    expect(isThirdPartyScriptError(event)).toBe(false);
  });

  it("keeps a chained error when one of its causes is ours", () => {
    // Causes are prepended and carry a parent_id; the root is the error actually thrown.
    const event = eventOf(
      exception(
        "failed to build transaction",
        { type: "chained", handled: true, parent_id: 0 },
        frames("app:///_next/static/chunks/app/page-abc123.js")
      ),
      exception("func sseError not found", UNHANDLED, SSE_ERROR_FRAMES)
    );

    expect(isThirdPartyScriptError(event)).toBe(false);
  });

  it("drops a chained error that is third party the whole way down", () => {
    const event = eventOf(
      exception(
        "provider bridge failed",
        { type: "chained", handled: true, parent_id: 0 },
        frames("app:///inpage.js")
      ),
      exception("func sseError not found", UNHANDLED, SSE_ERROR_FRAMES)
    );

    expect(isThirdPartyScriptError(event)).toBe(true);
  });

  it("treats any root-level script under app:/// as third party", () => {
    // Documents the assumption behind the `app:///` pattern: `public/` serves no root-level
    // `.js`, so adding one means revisiting THIRD_PARTY_FILENAME_PATTERNS.
    const event = eventOf(
      exception("boom", UNHANDLED, frames("app:///some-root-script.js"))
    );

    expect(isThirdPartyScriptError(event)).toBe(true);
  });
});
