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
