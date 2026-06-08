//! `SmallVec<L, T>` — a vector whose Borsh length prefix is `L` (e.g. `u8` or `u16`)
//! instead of the default `u32`.
//!
//! Required by the on-chain `TransactionMessage` wire format used by Squads V4. The
//! upstream program serializes account keys, instructions, and address-table lookups
//! with `u8` length prefixes; instruction `data` uses a `u16` length prefix.
//!
//! This implementation works on borsh 1.x and does not depend on `anchor-lang`.

use std::io::{Read, Write};
use std::marker::PhantomData;

use borsh::{BorshDeserialize, BorshSerialize};

/// A vector whose serialized length prefix is `L` rather than borsh's default `u32`.
///
/// `L` must be a small unsigned integer type (`u8` or `u16`) that implements
/// `BorshSerialize`, `BorshDeserialize`, and `Into<u32>`. The two combinations actually
/// used by Squads V4 are `SmallVec<u8, T>` and `SmallVec<u16, u8>`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SmallVec<L, T>(Vec<T>, PhantomData<L>);

impl<L, T> SmallVec<L, T> {
    /// Returns the number of elements in the vec.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if the vec contains no elements.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns an immutable reference to the inner `Vec<T>`.
    pub fn as_slice(&self) -> &[T] {
        self.0.as_slice()
    }
}

impl<L, T> From<Vec<T>> for SmallVec<L, T> {
    fn from(v: Vec<T>) -> Self {
        Self(v, PhantomData)
    }
}

impl<L, T> From<SmallVec<L, T>> for Vec<T> {
    fn from(v: SmallVec<L, T>) -> Self {
        v.0
    }
}

impl<L, T> AsRef<[T]> for SmallVec<L, T> {
    fn as_ref(&self) -> &[T] {
        self.0.as_slice()
    }
}

impl<T: BorshSerialize> BorshSerialize for SmallVec<u8, T> {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        let len = u8::try_from(self.0.len())
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "SmallVec<u8, T> overflow: length exceeds u8::MAX"))?;
        writer.write_all(&len.to_le_bytes())?;
        for item in &self.0 {
            item.serialize(writer)?;
        }
        Ok(())
    }
}

impl<T: BorshSerialize> BorshSerialize for SmallVec<u16, T> {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        let len = u16::try_from(self.0.len())
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "SmallVec<u16, T> overflow: length exceeds u16::MAX"))?;
        writer.write_all(&len.to_le_bytes())?;
        for item in &self.0 {
            item.serialize(writer)?;
        }
        Ok(())
    }
}

impl<T: BorshDeserialize> BorshDeserialize for SmallVec<u8, T> {
    fn deserialize_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let mut len_bytes = [0u8; 1];
        reader.read_exact(&mut len_bytes)?;
        let len = u8::from_le_bytes(len_bytes) as usize;

        let mut out = Vec::with_capacity(cautious_capacity::<T>(len));
        for _ in 0..len {
            out.push(T::deserialize_reader(reader)?);
        }
        Ok(Self(out, PhantomData))
    }
}

impl<T: BorshDeserialize> BorshDeserialize for SmallVec<u16, T> {
    fn deserialize_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let mut len_bytes = [0u8; 2];
        reader.read_exact(&mut len_bytes)?;
        let len = u16::from_le_bytes(len_bytes) as usize;

        let mut out = Vec::with_capacity(cautious_capacity::<T>(len));
        for _ in 0..len {
            out.push(T::deserialize_reader(reader)?);
        }
        Ok(Self(out, PhantomData))
    }
}

/// Bounded pre-allocation hint to avoid OOM on a malicious length prefix.
/// Matches the pattern from `borsh::de::hint::cautious`.
fn cautious_capacity<T>(hint: usize) -> usize {
    let el_size = std::mem::size_of::<T>().max(1);
    std::cmp::max(std::cmp::min(hint, 4096 / el_size), 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_program::pubkey::Pubkey;

    #[test]
    fn u8_len_with_u8_elements() {
        let bytes = [
            0x02, // len (2)
            0x05, // vec[0]
            0x09, // vec[1]
        ];
        let v = SmallVec::<u8, u8>::try_from_slice(&bytes).unwrap();
        assert_eq!(v.as_slice(), &[5, 9]);

        let mut buf = vec![];
        v.serialize(&mut buf).unwrap();
        assert_eq!(buf, bytes);
    }

    #[test]
    fn u8_len_with_u32_elements_little_endian() {
        let bytes = [
            0x02, // len (2)
            0x05, 0x00, 0x00, 0x00, // vec[0]
            0x09, 0x00, 0x00, 0x00, // vec[1]
        ];
        let v = SmallVec::<u8, u32>::try_from_slice(&bytes).unwrap();
        assert_eq!(v.as_slice(), &[5, 9]);
    }

    #[test]
    fn u16_len_with_u8_elements() {
        let bytes = [
            0x02, 0x00, // len (2)
            0x05, // vec[0]
            0x09, // vec[1]
        ];
        let v = SmallVec::<u16, u8>::try_from_slice(&bytes).unwrap();
        assert_eq!(v.as_slice(), &[5, 9]);
    }

    #[test]
    fn u8_len_with_pubkey_elements() {
        let pk1 = Pubkey::new_unique();
        let pk2 = Pubkey::new_unique();
        let v: SmallVec<u8, Pubkey> = vec![pk1, pk2].into();

        let mut buf = vec![];
        v.serialize(&mut buf).unwrap();

        let mut expected = vec![0x02u8]; // len
        expected.extend_from_slice(&pk1.to_bytes());
        expected.extend_from_slice(&pk2.to_bytes());
        assert_eq!(buf, expected);

        let decoded = SmallVec::<u8, Pubkey>::try_from_slice(&buf).unwrap();
        assert_eq!(decoded.as_slice(), &[pk1, pk2]);
    }

    #[test]
    fn u8_len_overflow_errors_on_serialize() {
        let v: SmallVec<u8, u8> = vec![0u8; 256].into();
        let mut buf = vec![];
        let result = v.serialize(&mut buf);
        assert!(result.is_err(), "256 elements should overflow u8 length prefix");
    }
}
