import {
  address,
  generateKeyPair,
  getAddressFromPublicKey,
  signBytes,
  type Transaction,
  type TransactionMessageBytes,
} from "@solana/kit";
import { webcrypto } from "node:crypto";
import {
  assertSignedByAccount,
  isWalletSigningCancellation,
  WalletSigningError,
} from "../walletSigning";

describe("isWalletSigningCancellation", () => {
  it("recognises a rejection the connector wrapped in its own signing error", () => {
    // Regression: the connector reports a rejected approval as SIGNING_FAILED
    // with the wallet's 4001 error attached, which showed an error toast and
    // went to Sentry instead of reading as a cancellation.
    const walletError = Object.assign(new Error("User rejected the request."), {
      code: 4001,
    });
    const connectorError = Object.assign(
      new Error("Failed to sign transaction 1 of 1"),
      { code: "SIGNING_FAILED", originalError: walletError },
    );

    expect(isWalletSigningCancellation(connectorError)).toBe(true);
  });

  it("recognises a wrapped rejection that carries only a message", () => {
    const connectorError = Object.assign(
      new Error("Failed to sign transaction 1 of 1"),
      {
        code: "SIGNING_FAILED",
        originalError: new Error("User rejected the request."),
      },
    );

    expect(isWalletSigningCancellation(connectorError)).toBe(true);
  });

  it("keeps a genuine signing failure reportable", () => {
    const connectorError = Object.assign(
      new Error("Failed to sign transaction 1 of 1"),
      {
        code: "SIGNING_FAILED",
        originalError: new Error("Ledger device disconnected"),
      },
    );

    expect(isWalletSigningCancellation(connectorError)).toBe(false);
    expect(isWalletSigningCancellation(new Error("User rejected"))).toBe(false);
  });
});

describe("assertSignedByAccount", () => {
  const messageBytes = new Uint8Array([1, 2, 3]) as unknown as TransactionMessageBytes;

  beforeAll(() => {
    Object.defineProperty(globalThis, "crypto", {
      configurable: true,
      value: webcrypto,
    });
    Object.defineProperty(globalThis, "isSecureContext", {
      configurable: true,
      value: true,
    });
  });

  it("accepts a signature made by the expected account", async () => {
    const keyPair = await generateKeyPair();
    const signer = await getAddressFromPublicKey(keyPair.publicKey);
    const transaction = {
      messageBytes,
      signatures: { [signer]: await signBytes(keyPair.privateKey, messageBytes) },
    } as Transaction;

    await expect(assertSignedByAccount(transaction, signer)).resolves.toBeUndefined();
  });

  it("rejects a signature another account made in the expected account's slot", async () => {
    const [expected, other] = await Promise.all([generateKeyPair(), generateKeyPair()]);
    const signer = await getAddressFromPublicKey(expected.publicKey);
    const transaction = {
      messageBytes,
      signatures: { [signer]: await signBytes(other.privateKey, messageBytes) },
    } as Transaction;

    await expect(assertSignedByAccount(transaction, signer)).rejects.toThrow(
      WalletSigningError,
    );
    await expect(assertSignedByAccount(transaction, signer)).rejects.toThrow(
      "different account",
    );
  });

  it("rejects a transaction the expected account did not sign", async () => {
    const signer = address("11111111111111111111111111111111");
    const transaction = { messageBytes, signatures: { [signer]: null } } as Transaction;

    await expect(assertSignedByAccount(transaction, signer)).rejects.toThrow(
      "did not sign",
    );
  });
});
