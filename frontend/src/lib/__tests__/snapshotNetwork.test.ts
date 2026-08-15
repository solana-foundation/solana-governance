const mockGetGenesisHash = jest.fn();
jest.mock("@solana/web3.js", () => ({
  ...jest.requireActual("@solana/web3.js"),
  Connection: jest.fn().mockImplementation(() => ({
    getGenesisHash: () => mockGetGenesisHash(),
  })),
}));

import { Connection } from "@solana/web3.js";

import {
  CLUSTER_GENESIS_HASHES,
  SNAPSHOT_UNAVAILABLE_MESSAGE,
  isKnownSnapshotNetwork,
  networkFromGenesisHash,
  requireKnownSnapshotNetwork,
  resolveSnapshotNetwork,
} from "../snapshotNetwork";

describe("networkFromGenesisHash", () => {
  it("maps each known cluster genesis hash", () => {
    expect(networkFromGenesisHash(CLUSTER_GENESIS_HASHES.mainnet)).toBe(
      "mainnet",
    );
    expect(networkFromGenesisHash(CLUSTER_GENESIS_HASHES.devnet)).toBe(
      "devnet",
    );
    expect(networkFromGenesisHash(CLUSTER_GENESIS_HASHES.testnet)).toBe(
      "testnet",
    );
  });

  it("returns undefined for an unrecognized hash", () => {
    expect(networkFromGenesisHash("unknown-genesis")).toBeUndefined();
  });
});

describe("isKnownSnapshotNetwork", () => {
  it("is true only for mainnet, testnet, and devnet", () => {
    expect(isKnownSnapshotNetwork("mainnet")).toBe(true);
    expect(isKnownSnapshotNetwork("testnet")).toBe(true);
    expect(isKnownSnapshotNetwork("devnet")).toBe(true);
    expect(isKnownSnapshotNetwork("custom")).toBe(false);
    expect(isKnownSnapshotNetwork(undefined)).toBe(false);
  });
});

describe("requireKnownSnapshotNetwork", () => {
  it("returns a known cluster", () => {
    expect(requireKnownSnapshotNetwork("testnet")).toBe("testnet");
  });

  it("throws when the cluster is not a known snapshot network", () => {
    expect(() => requireKnownSnapshotNetwork(undefined)).toThrow(
      SNAPSHOT_UNAVAILABLE_MESSAGE,
    );
    expect(() => requireKnownSnapshotNetwork("custom")).toThrow(
      SNAPSHOT_UNAVAILABLE_MESSAGE,
    );
  });
});

describe("resolveSnapshotNetwork", () => {
  afterEach(() => {
    mockGetGenesisHash.mockReset();
    jest.mocked(Connection).mockClear();
  });

  it("returns a preset endpoint without calling the RPC", async () => {
    await expect(
      resolveSnapshotNetwork("testnet", "https://example.invalid"),
    ).resolves.toBe("testnet");
    expect(Connection).not.toHaveBeenCalled();
  });

  it("resolves a custom RPC from its genesis hash", async () => {
    mockGetGenesisHash.mockResolvedValue(CLUSTER_GENESIS_HASHES.devnet);

    await expect(
      resolveSnapshotNetwork("custom", "https://my-rpc.example"),
    ).resolves.toBe("devnet");
    expect(Connection).toHaveBeenCalledWith(
      "https://my-rpc.example",
      "confirmed",
    );
  });

  it("returns undefined when a custom RPC is not a known cluster", async () => {
    mockGetGenesisHash.mockResolvedValue("localnet-genesis");

    await expect(
      resolveSnapshotNetwork("custom", "http://localhost:8899"),
    ).resolves.toBeUndefined();
  });
});
