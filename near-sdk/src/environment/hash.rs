use crate::sys;

const ATOMIC_OP_REGISTER: u64 = 0;

fn read_register_fixed(register_id: u64, buf: &mut [u8]) {
    unsafe { sys::read_register(register_id, buf.as_ptr() as _) }
}

/// Cryptographic hashes that can be used within the SDK as a hashing function.
pub trait CryptoHash {
    /// Output type of the hashing function.
    type Digest;

    /// Hashes raw bytes and returns the `Digest` output.
    fn hash(ingest: &[u8]) -> Self::Digest;
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Sha256;

impl CryptoHash for Sha256 {
    type Digest = [u8; 32];

    fn hash(ingest: &[u8]) -> Self::Digest {
        unsafe { sys::sha256(ingest.len() as _, ingest.as_ptr() as _, ATOMIC_OP_REGISTER) };

        let mut hash = [0u8; 32];
        read_register_fixed(ATOMIC_OP_REGISTER, &mut hash);
        hash
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Keccak256;

impl CryptoHash for Keccak256 {
    type Digest = [u8; 32];

    fn hash(ingest: &[u8]) -> Self::Digest {
        unsafe { sys::keccak256(ingest.len() as _, ingest.as_ptr() as _, ATOMIC_OP_REGISTER) };

        let mut hash = [0u8; 32];
        read_register_fixed(ATOMIC_OP_REGISTER, &mut hash);
        hash
    }
}
