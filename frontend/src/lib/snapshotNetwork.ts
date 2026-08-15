import { Connection } from "@solana/web3.js";
import type { RPCEndpoint } from "@/types";

export type KnownSnapshotNetwork = Exclude<RPCEndpoint, "custom">;

/**
 * Shown when an action needs the snapshot service but it cannot be used: the NCN API never
 * returned a slot, or a custom RPC's genesis hash is not a known cluster. Surfaced to the
 * user by the modals' error handlers rather than letting a mutation proceed with no network.
 */
export const SNAPSHOT_UNAVAILABLE_MESSAGE =
  "Snapshot service unavailable in this network";

/**
 * Known cluster genesis hashes for each network.
 */
export const CLUSTER_GENESIS_HASHES: Record<KnownSnapshotNetwork, string> = {
  mainnet: "5eykt4UsFv8P8NJdTREpY1vzqKqZKvdpKuc147dw2N9d",
  devnet: "EtWTRABZaYq6iMfeYKouRu166VU2xqa1wcaWoxPkrZBG",
  testnet: "4uhcVJyU9pJkvQyS88uRDiswHXSCkY3zQawwpjk2NsNY",
};

export function isKnownSnapshotNetwork(
  network: string | undefined,
): network is KnownSnapshotNetwork {
  return network !== undefined && network in CLUSTER_GENESIS_HASHES;
}

export function networkFromGenesisHash(
  genesisHash: string,
): KnownSnapshotNetwork | undefined {
  return (Object.keys(CLUSTER_GENESIS_HASHES) as KnownSnapshotNetwork[]).find(
    (network) => CLUSTER_GENESIS_HASHES[network] === genesisHash,
  );
}

async function fetchGenesisHash(endpointUrl: string): Promise<string> {
  const connection = new Connection(endpointUrl, "confirmed");
  return connection.getGenesisHash();
}

/**
 * NCN proof fetches require a known network (mainnet/testnet/devnet). Preset endpoints
 * already have one. A custom RPC is identified via `getGenesisHash`; unknown clusters
 * cannot be routed to the snapshot service.
 */
export async function resolveSnapshotNetwork(
  endpointType: RPCEndpoint,
  endpointUrl: string,
): Promise<KnownSnapshotNetwork | undefined> {
  if (endpointType !== "custom") {
    return endpointType;
  }

  return networkFromGenesisHash(await fetchGenesisHash(endpointUrl));
}

export function requireKnownSnapshotNetwork(
  network: string | undefined,
): KnownSnapshotNetwork {
  if (!isKnownSnapshotNetwork(network)) {
    throw new Error(SNAPSHOT_UNAVAILABLE_MESSAGE);
  }
  return network;
}
