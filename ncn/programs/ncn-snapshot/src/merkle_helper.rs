use anchor_lang::{
    err,
    prelude::{msg, Result},
    solana_program::hash::{hashv, Hash},
};

use crate::error::ErrorCode;

macro_rules! hash_leaf {
    {$d:ident} => {
        hashv(&[LEAF_PREFIX, $d])
    }
}

macro_rules! hash_intermediate {
    {$l:ident, $r:ident} => {
        hashv(&[INTERMEDIATE_PREFIX, $l.as_ref(), $r.as_ref()])
    }
}

// We need to discern between leaf and intermediate nodes to prevent trivial second
// pre-image attacks.
// https://flawed.net.nz/2018/02/21/attacking-merkle-trees-with-a-second-preimage-attack
const LEAF_PREFIX: &[u8] = &[0];
const INTERMEDIATE_PREFIX: &[u8] = &[1];

/// Verifies a Merkle proof from a leaf's content and its sibling hashes.
///
/// Hashes the leaf with a prefix, then hashes iteratively with sorted sibling,
/// to reconstruct the parent root.
///
/// Compares the Merkle root with the expected `root` and returns an error if it doesnt match.
pub fn verify_helper(leaf_content: &[u8], proof: &[[u8; 32]], root: Hash) -> Result<()> {
    let mut node = hash_leaf!(leaf_content);

    for &p in proof {
        let sibling_node = Hash::from(p);

        if node <= sibling_node {
            node = hash_intermediate!(node, sibling_node)
        } else {
            node = hash_intermediate!(sibling_node, node)
        }
    }

    if root != node {
        msg!("Root {:?} != Node {:?}", root, node);
        return err!(ErrorCode::InvalidMerkleProof);
    }

    Ok(())
}
