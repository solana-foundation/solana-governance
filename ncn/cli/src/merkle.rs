use crate::merkle_tree::{get_proof, MerkleTree};
use crate::utils::{decompress_gzip_with_limit, max_snapshot_bytes, read_all_with_limit};
use borsh::{BorshDeserialize, BorshSerialize};
use flate2::{write::GzEncoder, Compression};
use ncn_snapshot::{MetaMerkleLeaf, StakeMerkleLeaf};
use solana_sdk::hash::{hash, Hash};
use std::fs::File;
use std::io::{self, Write};
use std::path::PathBuf;

#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub struct MetaMerkleSnapshot {
    /// Hash of MetaMerkleTree
    pub root: [u8; 32],
    /// Each bundle contains the meta-level leaf, its stake-level leaves, and proof.
    pub leaf_bundles: Vec<MetaMerkleLeafBundle>,
    /// Slot where the tree was generated.
    pub slot: u64,
}

impl MetaMerkleSnapshot {
    pub fn to_compressed_bytes(&self) -> io::Result<Vec<u8>> {
        let data = self.try_to_vec()?;
        let mut enc = GzEncoder::new(Vec::new(), Compression::default());
        enc.write_all(&data)?;
        enc.finish()
    }

    pub fn save_compressed(&self, path: PathBuf) -> io::Result<()> {
        let data = self.to_compressed_bytes()?;
        let file = File::create(path)?;
        let mut writer = io::BufWriter::new(file);
        writer.write_all(&data)?;
        writer.flush()?;

        Ok(())
    }

    pub fn read_from_bytes_with_hash(
        buf: Vec<u8>,
        is_compressed: bool,
    ) -> io::Result<(Self, Hash)> {
        let max_size = max_snapshot_bytes();
        let decompressed_buf = if is_compressed {
            decompress_gzip_with_limit(&buf[..], max_size)?
        } else {
            if buf.len() > max_size {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "payload too large",
                ));
            }
            buf
        };

        let snapshot = Self::try_from_slice(&decompressed_buf)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let hash = hash(&decompressed_buf);
        Ok((snapshot, hash))
    }

    pub fn read(path: PathBuf, is_compressed: bool) -> io::Result<Self> {
        let max_size = max_snapshot_bytes();
        let file = File::open(path)?;
        let buf = if is_compressed {
            decompress_gzip_with_limit(file, max_size)?
        } else {
            read_all_with_limit(file, max_size)?
        };

        Self::try_from_slice(&buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    pub fn snapshot_hash(path: PathBuf, is_compressed: bool) -> io::Result<Hash> {
        let file = File::open(path)?;
        let buf = if is_compressed {
            decompress_gzip_with_limit(file, max_snapshot_bytes())?
        } else {
            read_all_with_limit(file, max_snapshot_bytes())?
        };

        Ok(hash(&buf))
    }

    /// Recompute every derived field from the stake leaves: each bundle's
    /// stake merkle root and active-stake sum, then the meta merkle root and
    /// per-bundle proofs — the same derivation used when a snapshot is first
    /// generated. Call after mutating stake leaves or reducing the bundle set
    /// so the snapshot is internally coherent again (the verifier service
    /// rejects uploads whose nested roots or stake sums do not line up).
    pub fn remerklize(&mut self) -> io::Result<()> {
        for bundle in self.leaf_bundles.iter_mut() {
            // Match generate_meta_merkle_snapshot's canonical leaf order. The
            // Merkle tree canonicalizes child pairs, but not the input leaves.
            bundle
                .stake_merkle_leaves
                .sort_by_key(|leaf| leaf.stake_account);

            let stake_nodes: Vec<[u8; 32]> = bundle
                .stake_merkle_leaves
                .iter()
                .map(|leaf| leaf.hash().to_bytes())
                .collect();
            bundle.meta_merkle_leaf.stake_merkle_root = MerkleTree::new(&stake_nodes[..], true)
                .get_root()
                .ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidData, "bundle has no stake leaves")
                })?
                .to_bytes();
            bundle.meta_merkle_leaf.active_stake = bundle
                .stake_merkle_leaves
                .iter()
                .map(|leaf| leaf.active_stake)
                .sum();
        }

        // Keep bundles paired with their now-derived stake roots while ordering
        // the meta-level leaves exactly as snapshot generation does.
        self.leaf_bundles
            .sort_by_key(|bundle| bundle.meta_merkle_leaf.vote_account);

        let meta_nodes: Vec<[u8; 32]> = self
            .leaf_bundles
            .iter()
            .map(|bundle| bundle.meta_merkle_leaf.hash().to_bytes())
            .collect();
        let meta_tree = MerkleTree::new(&meta_nodes[..], true);
        self.root = meta_tree
            .get_root()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "snapshot has no bundles"))?
            .to_bytes();
        for (index, bundle) in self.leaf_bundles.iter_mut().enumerate() {
            bundle.proof = Some(get_proof(&meta_tree, index));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub struct MetaMerkleLeafBundle {
    /// MetaMerkleLeaf constructed from the StakeMerkleTree.
    pub meta_merkle_leaf: MetaMerkleLeaf,
    /// Leaf nodes of the StakeMerkleTree.
    pub stake_merkle_leaves: Vec<StakeMerkleLeaf>,
    /// Proof to verify MetaMerkleLeaf existence in MetaMerkleTree.
    pub proof: Option<Vec<[u8; 32]>>,
}

impl MetaMerkleLeafBundle {
    pub fn get_stake_merkle_proof(self, index: usize) -> Vec<[u8; 32]> {
        let hashed_nodes: Vec<[u8; 32]> = self
            .stake_merkle_leaves
            .iter()
            .map(|n| n.hash().to_bytes())
            .collect();
        let stake_merkle = MerkleTree::new(&hashed_nodes[..], true);
        get_proof(&stake_merkle, index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anchor_lang::prelude::Pubkey;

    fn pubkey(seed: u8) -> Pubkey {
        Pubkey::new_from_array([seed; 32])
    }

    fn stake_leaf(stake_account: u8, active_stake: u64) -> StakeMerkleLeaf {
        StakeMerkleLeaf {
            voting_wallet: pubkey(1),
            stake_account: pubkey(stake_account),
            active_stake,
        }
    }

    fn bundle(vote_account: u8, stake_leaves: Vec<StakeMerkleLeaf>) -> MetaMerkleLeafBundle {
        MetaMerkleLeafBundle {
            meta_merkle_leaf: MetaMerkleLeaf {
                voting_wallet: pubkey(2),
                vote_account: pubkey(vote_account),
                stake_merkle_root: [0; 32],
                active_stake: 0,
            },
            stake_merkle_leaves: stake_leaves,
            proof: None,
        }
    }

    #[test]
    fn remerklize_canonicalizes_stake_and_vote_leaf_order() {
        let first_bundle = bundle(10, vec![stake_leaf(4, 40), stake_leaf(3, 30)]);
        let second_bundle = bundle(20, vec![stake_leaf(2, 20), stake_leaf(1, 10)]);

        let mut snapshot = MetaMerkleSnapshot {
            root: [0; 32],
            leaf_bundles: vec![second_bundle.clone(), first_bundle.clone()],
            slot: 42,
        };
        let mut reordered_snapshot = MetaMerkleSnapshot {
            root: [0; 32],
            leaf_bundles: vec![
                bundle(
                    10,
                    vec![
                        first_bundle.stake_merkle_leaves[1].clone(),
                        first_bundle.stake_merkle_leaves[0].clone(),
                    ],
                ),
                bundle(
                    20,
                    vec![
                        second_bundle.stake_merkle_leaves[1].clone(),
                        second_bundle.stake_merkle_leaves[0].clone(),
                    ],
                ),
            ],
            slot: 42,
        };

        snapshot.remerklize().unwrap();
        reordered_snapshot.remerklize().unwrap();

        assert_eq!(snapshot.root, reordered_snapshot.root);
        for (bundle, reordered_bundle) in snapshot
            .leaf_bundles
            .iter()
            .zip(&reordered_snapshot.leaf_bundles)
        {
            assert_eq!(bundle.meta_merkle_leaf, reordered_bundle.meta_merkle_leaf);
            assert_eq!(bundle.proof, reordered_bundle.proof);
        }
        assert!(snapshot.leaf_bundles.windows(2).all(|bundles| {
            bundles[0].meta_merkle_leaf.vote_account < bundles[1].meta_merkle_leaf.vote_account
        }));
        assert!(snapshot.leaf_bundles.iter().all(|bundle| {
            bundle
                .stake_merkle_leaves
                .windows(2)
                .all(|leaves| leaves[0].stake_account < leaves[1].stake_account)
        }));
    }
}
