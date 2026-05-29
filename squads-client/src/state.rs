//! On-chain account state for the Squads V4 multisig program.
//!
//! Each account type's leading 8 bytes is an Anchor account discriminator. The
//! [`try_deserialize`](Multisig::try_deserialize) constructors validate that
//! discriminator before decoding the body.
//!
//! Borsh ser/de is implemented manually (without `solana-program`'s borsh feature)
//! to keep the dependency footprint minimal and portable across consumers.

use std::io::{Read, Write};

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

use crate::discriminator::account_discriminator;
use crate::error::{Result, SquadsError};

/// Reads a `Pubkey` (32 little-endian bytes) from `reader`.
fn read_pubkey<R: Read>(reader: &mut R) -> std::io::Result<Pubkey> {
    let mut bytes = [0u8; 32];
    reader.read_exact(&mut bytes)?;
    Ok(Pubkey::from(bytes))
}

/// Writes a `Pubkey` (32 little-endian bytes) to `writer`.
fn write_pubkey<W: Write>(writer: &mut W, pk: &Pubkey) -> std::io::Result<()> {
    writer.write_all(&pk.to_bytes())
}

/// Validates the 8-byte Anchor account discriminator at the head of `data` and returns
/// the body slice that follows.
fn split_discriminator<'a>(
    data: &'a [u8],
    type_name: &'static str,
    expected: [u8; 8],
) -> Result<&'a [u8]> {
    if data.len() < 8 {
        return Err(SquadsError::AccountDataTooShort {
            expected: 8,
            actual: data.len(),
        });
    }
    let mut actual = [0u8; 8];
    actual.copy_from_slice(&data[..8]);
    if actual != expected {
        return Err(SquadsError::DiscriminatorMismatch {
            type_name,
            expected,
            actual,
        });
    }
    Ok(&data[8..])
}

// ============================================================================
// Permissions
// ============================================================================

/// A single permission flag stored inside a [`Permissions`] bitmask.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Permission {
    /// May create vault transactions and proposals.
    Initiate = 1 << 0,
    /// May approve / reject / cancel proposals.
    Vote = 1 << 1,
    /// May execute an approved proposal.
    Execute = 1 << 2,
}

/// Bitmask of [`Permission`]s held by a multisig member.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Permissions {
    /// The raw permission bits: bit 0 = Initiate, bit 1 = Vote, bit 2 = Execute.
    pub mask: u8,
}

impl Permissions {
    /// Returns `true` if `permission`'s bit is set in this mask.
    pub fn has(&self, permission: Permission) -> bool {
        (self.mask & (permission as u8)) != 0
    }

    /// Constructs a [`Permissions`] from a slice of [`Permission`] flags.
    pub fn from_vec(permissions: &[Permission]) -> Self {
        let mut mask = 0u8;
        for p in permissions {
            mask |= *p as u8;
        }
        Self { mask }
    }
}

impl BorshSerialize for Permissions {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        writer.write_all(&[self.mask])
    }
}

impl BorshDeserialize for Permissions {
    fn deserialize_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let mut buf = [0u8; 1];
        reader.read_exact(&mut buf)?;
        Ok(Self { mask: buf[0] })
    }
}

// ============================================================================
// Member
// ============================================================================

/// A multisig member.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Member {
    /// The member's public key.
    pub key: Pubkey,
    /// The member's permission bitmask.
    pub permissions: Permissions,
}

impl Member {
    /// Serialized size of a `Member` in bytes (32 for the pubkey, 1 for the mask).
    pub const SERIALIZED_SIZE: usize = 33;
}

impl BorshSerialize for Member {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        write_pubkey(writer, &self.key)?;
        self.permissions.serialize(writer)?;
        Ok(())
    }
}

impl BorshDeserialize for Member {
    fn deserialize_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let key = read_pubkey(reader)?;
        let permissions = Permissions::deserialize_reader(reader)?;
        Ok(Self { key, permissions })
    }
}

// ============================================================================
// Multisig
// ============================================================================

/// The on-chain `Multisig` account.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Multisig {
    /// Key used to seed the Multisig PDA derivation.
    pub create_key: Pubkey,
    /// Authority that may change members/threshold via direct instruction.
    /// `Pubkey::default()` denotes an "autonomous" multisig (config changes must go through voting).
    pub config_authority: Pubkey,
    /// Approval threshold.
    pub threshold: u16,
    /// Mandatory delay (seconds) between approval and execution.
    pub time_lock: u32,
    /// Last used transaction index. `0` means no transactions have been created yet.
    pub transaction_index: u64,
    /// Transactions at or before this index are stale and cannot be voted on.
    pub stale_transaction_index: u64,
    /// Optional rent collector address (where reclaimed rent from terminal transactions is sent).
    pub rent_collector: Option<Pubkey>,
    /// PDA bump seed.
    pub bump: u8,
    /// Multisig members.
    pub members: Vec<Member>,
}

impl Multisig {
    /// Anchor account discriminator for `Multisig` (`sha256("account:Multisig")[..8]`).
    pub fn discriminator() -> [u8; 8] {
        account_discriminator("Multisig")
    }

    /// Deserialize a `Multisig` from raw on-chain account data, validating the leading
    /// 8-byte Anchor discriminator.
    pub fn try_deserialize(data: &[u8]) -> Result<Self> {
        let body = split_discriminator(data, "Multisig", Self::discriminator())?;
        let mut cursor = body;
        let create_key = read_pubkey(&mut cursor).map_err(SquadsError::BorshDecode)?;
        let config_authority = read_pubkey(&mut cursor).map_err(SquadsError::BorshDecode)?;
        let threshold = u16::deserialize_reader(&mut cursor).map_err(SquadsError::BorshDecode)?;
        let time_lock = u32::deserialize_reader(&mut cursor).map_err(SquadsError::BorshDecode)?;
        let transaction_index =
            u64::deserialize_reader(&mut cursor).map_err(SquadsError::BorshDecode)?;
        let stale_transaction_index =
            u64::deserialize_reader(&mut cursor).map_err(SquadsError::BorshDecode)?;
        let rent_collector =
            <Option<Pubkey32>>::deserialize_reader(&mut cursor)
                .map_err(SquadsError::BorshDecode)?
                .map(|w| w.0);
        let mut bump_buf = [0u8; 1];
        cursor
            .read_exact(&mut bump_buf)
            .map_err(SquadsError::BorshDecode)?;
        let bump = bump_buf[0];
        let members =
            <Vec<Member>>::deserialize_reader(&mut cursor).map_err(SquadsError::BorshDecode)?;
        Ok(Self {
            create_key,
            config_authority,
            threshold,
            time_lock,
            transaction_index,
            stale_transaction_index,
            rent_collector,
            bump,
            members,
        })
    }

    /// Returns `Some(index)` if `member` is a member of this multisig. The members vec is
    /// sorted by key on-chain; this performs a linear search to avoid relying on that
    /// invariant being preserved through every code path.
    pub fn is_member(&self, member: &Pubkey) -> Option<usize> {
        self.members.iter().position(|m| &m.key == member)
    }

    /// Returns `true` if `member` is a member of this multisig and holds `permission`.
    pub fn member_has_permission(&self, member: &Pubkey, permission: Permission) -> bool {
        match self.is_member(member) {
            Some(idx) => self.members[idx].permissions.has(permission),
            None => false,
        }
    }

    /// Returns the number of members holding the [`Permission::Initiate`] flag.
    pub fn num_proposers(&self) -> usize {
        self.members
            .iter()
            .filter(|m| m.permissions.has(Permission::Initiate))
            .count()
    }

    /// Returns the number of members holding the [`Permission::Vote`] flag.
    pub fn num_voters(&self) -> usize {
        self.members
            .iter()
            .filter(|m| m.permissions.has(Permission::Vote))
            .count()
    }

    /// Returns the number of members holding the [`Permission::Execute`] flag.
    pub fn num_executors(&self) -> usize {
        self.members
            .iter()
            .filter(|m| m.permissions.has(Permission::Execute))
            .count()
    }
}

/// Borsh-friendly newtype wrapper around `Pubkey` so we can implement
/// `BorshSerialize`/`BorshDeserialize` without depending on `solana-program`'s
/// optional `borsh` features. Used internally only.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Pubkey32(Pubkey);

impl BorshSerialize for Pubkey32 {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        write_pubkey(writer, &self.0)
    }
}

impl BorshDeserialize for Pubkey32 {
    fn deserialize_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        Ok(Self(read_pubkey(reader)?))
    }
}

// Implement BorshDeserialize/BorshSerialize for raw Pubkey so derives on outer
// structs (where we don't go through Pubkey32) keep working. The trait
// implementations live behind their own module path here; we use them directly.
// Note: implementing borsh traits for an external type would be a coherence
// violation, hence Pubkey32 above. For containers we use Pubkey32 explicitly.

// ============================================================================
// ProposalStatus
// ============================================================================

/// Lifecycle status of a [`Proposal`]. Each non-deprecated variant wraps the unix
/// timestamp at which the status was set. Borsh tag values match the on-chain
/// enum order (0..=6).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProposalStatus {
    /// Draft (created but not yet active for voting). Tag = 0.
    Draft { timestamp: i64 },
    /// Active and accepting votes. Tag = 1.
    Active { timestamp: i64 },
    /// Rejected by reaching the rejection cutoff. Tag = 2.
    Rejected { timestamp: i64 },
    /// Approved (threshold reached); awaiting execution. Tag = 3.
    Approved { timestamp: i64 },
    /// Deprecated transient state — present for wire-format compatibility only. Tag = 4.
    Executing,
    /// Successfully executed. Tag = 5.
    Executed { timestamp: i64 },
    /// Cancelled by member vote. Tag = 6.
    Cancelled { timestamp: i64 },
}

impl BorshSerialize for ProposalStatus {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        match self {
            Self::Draft { timestamp } => {
                writer.write_all(&[0])?;
                timestamp.serialize(writer)
            }
            Self::Active { timestamp } => {
                writer.write_all(&[1])?;
                timestamp.serialize(writer)
            }
            Self::Rejected { timestamp } => {
                writer.write_all(&[2])?;
                timestamp.serialize(writer)
            }
            Self::Approved { timestamp } => {
                writer.write_all(&[3])?;
                timestamp.serialize(writer)
            }
            Self::Executing => writer.write_all(&[4]),
            Self::Executed { timestamp } => {
                writer.write_all(&[5])?;
                timestamp.serialize(writer)
            }
            Self::Cancelled { timestamp } => {
                writer.write_all(&[6])?;
                timestamp.serialize(writer)
            }
        }
    }
}

impl BorshDeserialize for ProposalStatus {
    fn deserialize_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let mut tag = [0u8; 1];
        reader.read_exact(&mut tag)?;
        let read_ts = |r: &mut R| -> std::io::Result<i64> { i64::deserialize_reader(r) };
        Ok(match tag[0] {
            0 => Self::Draft {
                timestamp: read_ts(reader)?,
            },
            1 => Self::Active {
                timestamp: read_ts(reader)?,
            },
            2 => Self::Rejected {
                timestamp: read_ts(reader)?,
            },
            3 => Self::Approved {
                timestamp: read_ts(reader)?,
            },
            4 => Self::Executing,
            5 => Self::Executed {
                timestamp: read_ts(reader)?,
            },
            6 => Self::Cancelled {
                timestamp: read_ts(reader)?,
            },
            other => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("unknown ProposalStatus variant: {}", other),
                ))
            }
        })
    }
}

// ============================================================================
// Proposal
// ============================================================================

/// On-chain `Proposal` account. Tracks the voting state for a specific transaction index.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Proposal {
    /// The multisig this proposal belongs to.
    pub multisig: Pubkey,
    /// Index of the multisig transaction this proposal is associated with.
    pub transaction_index: u64,
    /// Current proposal status.
    pub status: ProposalStatus,
    /// PDA bump seed.
    pub bump: u8,
    /// Members who have voted to approve.
    pub approved: Vec<Pubkey>,
    /// Members who have voted to reject.
    pub rejected: Vec<Pubkey>,
    /// Members who have voted to cancel (only meaningful after `Approved`).
    pub cancelled: Vec<Pubkey>,
}

impl Proposal {
    /// Anchor account discriminator for `Proposal`.
    pub fn discriminator() -> [u8; 8] {
        account_discriminator("Proposal")
    }

    /// Deserialize a `Proposal` from raw account data, validating the discriminator.
    pub fn try_deserialize(data: &[u8]) -> Result<Self> {
        let body = split_discriminator(data, "Proposal", Self::discriminator())?;
        let mut cursor = body;
        let multisig = read_pubkey(&mut cursor).map_err(SquadsError::BorshDecode)?;
        let transaction_index =
            u64::deserialize_reader(&mut cursor).map_err(SquadsError::BorshDecode)?;
        let status =
            ProposalStatus::deserialize_reader(&mut cursor).map_err(SquadsError::BorshDecode)?;
        let mut bump_buf = [0u8; 1];
        cursor
            .read_exact(&mut bump_buf)
            .map_err(SquadsError::BorshDecode)?;
        let bump = bump_buf[0];
        let approved = read_pubkey_vec(&mut cursor).map_err(SquadsError::BorshDecode)?;
        let rejected = read_pubkey_vec(&mut cursor).map_err(SquadsError::BorshDecode)?;
        let cancelled = read_pubkey_vec(&mut cursor).map_err(SquadsError::BorshDecode)?;
        Ok(Self {
            multisig,
            transaction_index,
            status,
            bump,
            approved,
            rejected,
            cancelled,
        })
    }
}

/// Reads a `Vec<Pubkey>` with a `u32` little-endian length prefix.
fn read_pubkey_vec<R: Read>(reader: &mut R) -> std::io::Result<Vec<Pubkey>> {
    let len = u32::deserialize_reader(reader)? as usize;
    let mut out = Vec::with_capacity(len.min(4096));
    for _ in 0..len {
        out.push(read_pubkey(reader)?);
    }
    Ok(out)
}

// ============================================================================
// VaultTransaction
// ============================================================================

/// On-chain `VaultTransaction` account. Stores the compiled instruction set that the
/// vault will execute once the proposal is approved.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VaultTransaction {
    /// The multisig this transaction belongs to.
    pub multisig: Pubkey,
    /// The multisig member that created the transaction.
    pub creator: Pubkey,
    /// Transaction index within the multisig.
    pub index: u64,
    /// PDA bump seed.
    pub bump: u8,
    /// Index of the vault that will execute this transaction.
    pub vault_index: u8,
    /// PDA bump for the vault PDA.
    pub vault_bump: u8,
    /// PDA bumps for any ephemeral signer PDAs.
    pub ephemeral_signer_bumps: Vec<u8>,
    /// The compiled transaction message (stored form, NOT the wire `TransactionMessage`).
    /// Stored as raw bytes here so callers can re-serialize or inspect without committing
    /// to a specific decoded shape; use `VaultTransactionStoredMessage::try_from_slice`
    /// on this field to decode.
    pub message_bytes: Vec<u8>,
}

impl VaultTransaction {
    /// Anchor account discriminator for `VaultTransaction`.
    pub fn discriminator() -> [u8; 8] {
        account_discriminator("VaultTransaction")
    }

    /// Deserialize a `VaultTransaction` from raw account data, validating the discriminator.
    ///
    /// `message_bytes` captures the remainder of the account body (the stored
    /// `VaultTransactionMessage`); we don't decode it eagerly because consumers usually
    /// only need the metadata.
    pub fn try_deserialize(data: &[u8]) -> Result<Self> {
        let body = split_discriminator(data, "VaultTransaction", Self::discriminator())?;
        let mut cursor = body;
        let multisig = read_pubkey(&mut cursor).map_err(SquadsError::BorshDecode)?;
        let creator = read_pubkey(&mut cursor).map_err(SquadsError::BorshDecode)?;
        let index = u64::deserialize_reader(&mut cursor).map_err(SquadsError::BorshDecode)?;
        let mut single = [0u8; 1];
        cursor
            .read_exact(&mut single)
            .map_err(SquadsError::BorshDecode)?;
        let bump = single[0];
        cursor
            .read_exact(&mut single)
            .map_err(SquadsError::BorshDecode)?;
        let vault_index = single[0];
        cursor
            .read_exact(&mut single)
            .map_err(SquadsError::BorshDecode)?;
        let vault_bump = single[0];
        let ephemeral_signer_bumps =
            <Vec<u8>>::deserialize_reader(&mut cursor).map_err(SquadsError::BorshDecode)?;
        // The rest is the stored message bytes.
        let message_bytes = cursor.to_vec();
        Ok(Self {
            multisig,
            creator,
            index,
            bump,
            vault_index,
            vault_bump,
            ephemeral_signer_bumps,
            message_bytes,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permissions_has_all_combinations() {
        for mask in 0u8..=7 {
            let p = Permissions { mask };
            assert_eq!(p.has(Permission::Initiate), (mask & 0b001) != 0);
            assert_eq!(p.has(Permission::Vote), (mask & 0b010) != 0);
            assert_eq!(p.has(Permission::Execute), (mask & 0b100) != 0);
        }
    }

    #[test]
    fn permissions_from_vec_combines_flags() {
        let p = Permissions::from_vec(&[Permission::Initiate, Permission::Execute]);
        assert!(p.has(Permission::Initiate));
        assert!(!p.has(Permission::Vote));
        assert!(p.has(Permission::Execute));
        assert_eq!(p.mask, 0b101);
    }

    #[test]
    fn member_roundtrip() {
        let m = Member {
            key: Pubkey::new_unique(),
            permissions: Permissions { mask: 0b011 },
        };
        let mut buf = vec![];
        m.serialize(&mut buf).unwrap();
        assert_eq!(buf.len(), Member::SERIALIZED_SIZE);
        let decoded = Member::deserialize_reader(&mut buf.as_slice()).unwrap();
        assert_eq!(decoded, m);
    }

    #[test]
    fn multisig_deserialize_rejects_wrong_discriminator() {
        let data = vec![0u8; 200];
        let err = Multisig::try_deserialize(&data).unwrap_err();
        match err {
            SquadsError::DiscriminatorMismatch { type_name, .. } => {
                assert_eq!(type_name, "Multisig");
            }
            other => panic!("expected DiscriminatorMismatch, got {:?}", other),
        }
    }

    #[test]
    fn multisig_deserialize_rejects_too_short() {
        let err = Multisig::try_deserialize(&[]).unwrap_err();
        match err {
            SquadsError::AccountDataTooShort { expected, actual } => {
                assert_eq!(expected, 8);
                assert_eq!(actual, 0);
            }
            other => panic!("expected AccountDataTooShort, got {:?}", other),
        }
    }

    #[test]
    fn multisig_full_roundtrip() {
        let m = Multisig {
            create_key: Pubkey::new_unique(),
            config_authority: Pubkey::default(),
            threshold: 2,
            time_lock: 0,
            transaction_index: 42,
            stale_transaction_index: 0,
            rent_collector: Some(Pubkey::new_unique()),
            bump: 254,
            members: vec![
                Member {
                    key: Pubkey::new_unique(),
                    permissions: Permissions {
                        mask: 0b001 | 0b010 | 0b100,
                    },
                },
                Member {
                    key: Pubkey::new_unique(),
                    permissions: Permissions { mask: 0b010 },
                },
            ],
        };
        // Manually construct the on-chain byte representation by re-implementing the
        // serializer (the production code only reads, never writes, Multisig accounts).
        let mut bytes = vec![];
        bytes.extend_from_slice(&Multisig::discriminator());
        bytes.extend_from_slice(&m.create_key.to_bytes());
        bytes.extend_from_slice(&m.config_authority.to_bytes());
        bytes.extend_from_slice(&m.threshold.to_le_bytes());
        bytes.extend_from_slice(&m.time_lock.to_le_bytes());
        bytes.extend_from_slice(&m.transaction_index.to_le_bytes());
        bytes.extend_from_slice(&m.stale_transaction_index.to_le_bytes());
        // Option<Pubkey> as borsh: tag byte + payload (Some)
        if let Some(rc) = &m.rent_collector {
            bytes.push(1);
            bytes.extend_from_slice(&rc.to_bytes());
        } else {
            bytes.push(0);
        }
        bytes.push(m.bump);
        bytes.extend_from_slice(&(m.members.len() as u32).to_le_bytes());
        for mem in &m.members {
            bytes.extend_from_slice(&mem.key.to_bytes());
            bytes.push(mem.permissions.mask);
        }
        let decoded = Multisig::try_deserialize(&bytes).unwrap();
        assert_eq!(decoded, m);
    }

    #[test]
    fn multisig_member_has_permission_lookup() {
        let voter = Pubkey::new_unique();
        let initiator = Pubkey::new_unique();
        let outsider = Pubkey::new_unique();
        let m = Multisig {
            create_key: Pubkey::default(),
            config_authority: Pubkey::default(),
            threshold: 1,
            time_lock: 0,
            transaction_index: 0,
            stale_transaction_index: 0,
            rent_collector: None,
            bump: 255,
            members: vec![
                Member {
                    key: voter,
                    permissions: Permissions { mask: 0b010 },
                },
                Member {
                    key: initiator,
                    permissions: Permissions { mask: 0b001 },
                },
            ],
        };

        assert!(m.member_has_permission(&voter, Permission::Vote));
        assert!(!m.member_has_permission(&voter, Permission::Initiate));
        assert!(m.member_has_permission(&initiator, Permission::Initiate));
        assert!(!m.member_has_permission(&initiator, Permission::Vote));
        assert!(!m.member_has_permission(&outsider, Permission::Vote));
        assert_eq!(m.num_proposers(), 1);
        assert_eq!(m.num_voters(), 1);
        assert_eq!(m.num_executors(), 0);
    }

    #[test]
    fn proposal_status_borsh_tags() {
        for (status, expected_tag) in [
            (ProposalStatus::Draft { timestamp: 1 }, 0),
            (ProposalStatus::Active { timestamp: 2 }, 1),
            (ProposalStatus::Rejected { timestamp: 3 }, 2),
            (ProposalStatus::Approved { timestamp: 4 }, 3),
            (ProposalStatus::Executing, 4),
            (ProposalStatus::Executed { timestamp: 5 }, 5),
            (ProposalStatus::Cancelled { timestamp: 6 }, 6),
        ] {
            let mut buf = vec![];
            status.serialize(&mut buf).unwrap();
            assert_eq!(
                buf[0], expected_tag,
                "wrong borsh tag for {:?}",
                status
            );
        }
    }

    #[test]
    fn proposal_status_roundtrips() {
        let statuses = [
            ProposalStatus::Draft { timestamp: 100 },
            ProposalStatus::Active { timestamp: 200 },
            ProposalStatus::Rejected { timestamp: 300 },
            ProposalStatus::Approved { timestamp: 400 },
            ProposalStatus::Executing,
            ProposalStatus::Executed { timestamp: 500 },
            ProposalStatus::Cancelled { timestamp: 600 },
        ];
        for s in statuses {
            let mut buf = vec![];
            s.serialize(&mut buf).unwrap();
            let decoded = ProposalStatus::deserialize_reader(&mut buf.as_slice()).unwrap();
            assert_eq!(decoded, s);
        }
    }
}
