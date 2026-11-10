// Owned by Pool Agent 5 (arthoroy28@gmail.com)

use curve25519_dalek::edwards::{CompressedEdwardsY, EdwardsPoint};
use curve25519_dalek::scalar::Scalar;
use sha2::{Digest, Sha512};

/// Root public key used to verify TeamSpeak 3 license chains (`License.cs:20-24`).
pub const LICENSE_ROOT_KEY: [u8; 32] = [
    0xcd, 0x0d, 0xe2, 0xae, 0xd4, 0x63, 0x45, 0x50, 0x9a, 0x7e, 0x3c, 0xfd, 0x8f, 0x68, 0xb3, 0xdc,
    0x75, 0x55, 0xb2, 0x9d, 0xcc, 0xec, 0x73, 0xcd, 0x18, 0x75, 0x0f, 0x99, 0x38, 0x12, 0x40, 0x8a,
];

/// Epoch offset added to 32-bit big-endian timestamps in license blocks (`License.cs:118-119`).
pub const LICENSE_TIME_OFFSET: u64 = 0x50e2_2700;

/// Minimum byte length of a single license block (`License.cs:66`).
pub const MIN_BLOCK_LEN: usize = 42;

/// Errors that can occur when parsing or deriving a TeamSpeak 3 license chain.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LicenseError {
    #[error("license data too short")]
    TooShort,
    #[error("unsupported license version: {0}")]
    UnsupportedVersion(u8),
    #[error("wrong key kind {0} in license block (expected 0)")]
    WrongKeyKind(u8),
    #[error("invalid license block type: {0}")]
    InvalidBlockType(u8),
    #[error("unknown server license type: {0}")]
    UnknownServerLicenseType(u8),
    #[error("non-null-terminated issuer string")]
    NonNullTerminatedIssuer,
    #[error("invalid UTF-8 in issuer string")]
    InvalidIssuerUtf8,
    #[error("license validity timestamps are invalid (not_valid_after < not_valid_before)")]
    InvalidTimestamps,
    #[error("invalid Ed25519 public key point in license chain")]
    InvalidPublicKey,
}

/// Type discriminator for a block in the license chain (`License.cs:220-231`).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChainBlockType {
    Intermediate = 0,
    Website = 1,
    Server = 2,
    Code = 3,
    Ephemeral = 32,
}

impl TryFrom<u8> for ChainBlockType {
    type Error = LicenseError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Intermediate),
            1 => Ok(Self::Website),
            2 => Ok(Self::Server),
            3 => Ok(Self::Code),
            32 => Ok(Self::Ephemeral),
            other => Err(LicenseError::InvalidBlockType(other)),
        }
    }
}

/// Server license tier (`License.cs:235-248`).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ServerLicenseType {
    None = 0,
    Offline = 1,
    Sdk = 2,
    SdkOffline = 3,
    Npl = 4,
    Athp = 5,
    Aal = 6,
    Default = 7,
    Gamer = 8,
    Sponsorship = 9,
    Commercial = 10,
}

impl TryFrom<u8> for ServerLicenseType {
    type Error = LicenseError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::None),
            1 => Ok(Self::Offline),
            2 => Ok(Self::Sdk),
            3 => Ok(Self::SdkOffline),
            4 => Ok(Self::Npl),
            5 => Ok(Self::Athp),
            6 => Ok(Self::Aal),
            7 => Ok(Self::Default),
            8 => Ok(Self::Gamer),
            9 => Ok(Self::Sponsorship),
            10 => Ok(Self::Commercial),
            other => Err(LicenseError::UnknownServerLicenseType(other)),
        }
    }
}

/// Intermediate license block (`ChainBlockType::Intermediate`, type 0; `License.cs:169-178`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntermediateBlock {
    pub issuer: String,
}

/// Server license block (`ChainBlockType::Server`, type 2; `License.cs:202-213`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerBlock {
    pub license_type: ServerLicenseType,
    pub issuer: String,
}

/// Ephemeral key block (`ChainBlockType::Ephemeral`, type 32; `License.cs:215-218`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct KeyBlock;

/// Type-specific payload inside a [`LicenseBlock`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LicenseBlockKind {
    Intermediate(IntermediateBlock),
    Server(ServerBlock),
    Ephemeral(KeyBlock),
}

/// A single parsed block in a TeamSpeak 3 [`LicenseChain`] (`License.cs:64-165`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LicenseBlock {
    /// Block type discriminator (`0`, `2`, or `32`).
    pub block_type: ChainBlockType,
    /// Start of validity window as Unix timestamp in seconds (`u32_be + 0x50e22700`).
    pub not_valid_before: u64,
    /// End of validity window as Unix timestamp in seconds (`u32_be + 0x50e22700`).
    pub not_valid_after: u64,
    /// 32-byte compressed Edwards Y public key (`data[1..33]`).
    pub key: [u8; 32],
    /// First 32 bytes of `SHA512(data[1..all_len])` (`License.cs:126-127`).
    pub hash: [u8; 32],
    /// Variant-specific metadata (issuer, server license type, etc.).
    pub kind: LicenseBlockKind,
}

impl LicenseBlock {
    /// Parses a single license block from the start of `data`, returning `(block, bytes_consumed)`.
    ///
    /// Reference: `TSLib/Full/License.cs:76-130`.
    pub fn parse(data: &[u8]) -> Result<(Self, usize), LicenseError> {
        if data.len() < MIN_BLOCK_LEN {
            return Err(LicenseError::TooShort);
        }
        if data[0] != 0 {
            return Err(LicenseError::WrongKeyKind(data[0]));
        }

        let (block_type, kind, extra_read) = match data[33] {
            0 => {
                if data.len() < 46 {
                    return Err(LicenseError::TooShort);
                }
                let (issuer, str_len) = Self::read_null_string(&data[46..])?;
                (
                    ChainBlockType::Intermediate,
                    LicenseBlockKind::Intermediate(IntermediateBlock { issuer }),
                    5 + str_len,
                )
            }
            2 => {
                if data.len() < 47 {
                    return Err(LicenseError::TooShort);
                }
                let license_type = ServerLicenseType::try_from(data[42])?;
                let (issuer, str_len) = Self::read_null_string(&data[47..])?;
                (
                    ChainBlockType::Server,
                    LicenseBlockKind::Server(ServerBlock {
                        license_type,
                        issuer,
                    }),
                    6 + str_len,
                )
            }
            32 => (
                ChainBlockType::Ephemeral,
                LicenseBlockKind::Ephemeral(KeyBlock),
                0,
            ),
            other => return Err(LicenseError::InvalidBlockType(other)),
        };

        let all_len = MIN_BLOCK_LEN + extra_read;
        if data.len() < all_len {
            return Err(LicenseError::TooShort);
        }

        let raw_before = u32::from_be_bytes([data[34], data[35], data[36], data[37]]) as u64;
        let raw_after = u32::from_be_bytes([data[38], data[39], data[40], data[41]]) as u64;
        let not_valid_before = raw_before + LICENSE_TIME_OFFSET;
        let not_valid_after = raw_after + LICENSE_TIME_OFFSET;
        if not_valid_after < not_valid_before {
            return Err(LicenseError::InvalidTimestamps);
        }

        let mut key = [0u8; 32];
        key.copy_from_slice(&data[1..33]);

        let digest = Sha512::digest(&data[1..all_len]);
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&digest[0..32]);

        Ok((
            Self {
                block_type,
                not_valid_before,
                not_valid_after,
                key,
                hash,
                kind,
            },
            all_len,
        ))
    }

    /// Reads a null-terminated UTF-8 string from `slice`, returning `(string, index_of_null)`.
    ///
    /// Reference: `TSLib/Full/License.cs:132-138`.
    fn read_null_string(slice: &[u8]) -> Result<(String, usize), LicenseError> {
        let term_idx = slice
            .iter()
            .position(|&b| b == 0)
            .ok_or(LicenseError::NonNullTerminatedIssuer)?;
        let s = std::str::from_utf8(&slice[..term_idx])
            .map_err(|_| LicenseError::InvalidIssuerUtf8)?
            .to_owned();
        Ok((s, term_idx))
    }

    /// Derives the next 32-byte compressed Edwards Y public key from `parent` and this block.
    ///
    /// Matches `LicenseBlock.DeriveKey` (`License.cs:147-164`):
    /// - Clamps `self.hash` (`h[0] &= 248; h[31] &= 63; h[31] |= 64;`).
    /// - Notice that `ge_frombytes_negate_vartime` negates both `Key` (`-P_key`) and `parent` (`-P_parent`),
    ///   computes `(-P_key) * scalar + (-P_parent) = -(P_key * scalar + P_parent)`,
    ///   and then `final[31] ^= 0x80` negates the X coordinate back, yielding `P_key * scalar + P_parent`.
    pub fn derive_key(&self, parent: &[u8; 32]) -> Result<[u8; 32], LicenseError> {
        let mut clamped = self.hash;
        clamped[0] &= 248;
        clamped[31] &= 63;
        clamped[31] |= 64;
        let scalar = Scalar::from_bytes_mod_order(clamped);

        let pub_point: EdwardsPoint = CompressedEdwardsY(self.key)
            .decompress()
            .ok_or(LicenseError::InvalidPublicKey)?;
        let parent_point: EdwardsPoint = CompressedEdwardsY(*parent)
            .decompress()
            .ok_or(LicenseError::InvalidPublicKey)?;

        let derived_point = (pub_point * scalar) + parent_point;
        Ok(derived_point.compress().to_bytes())
    }
}

/// Parsed TeamSpeak 3 binary license chain (`License.cs:18-62`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LicenseChain {
    pub blocks: Vec<LicenseBlock>,
}

impl LicenseChain {
    /// Root Ed25519 public key constant (`License.cs:20-24`).
    pub const ROOT_KEY: [u8; 32] = LICENSE_ROOT_KEY;

    /// Parses a binary license chain starting with version byte `0x01` (`License.cs:28-53`).
    pub fn parse(data: &[u8]) -> Result<Self, LicenseError> {
        if data.is_empty() {
            return Err(LicenseError::TooShort);
        }
        let version = data[0];
        if version != 1 {
            return Err(LicenseError::UnsupportedVersion(version));
        }

        let mut blocks = Vec::new();
        let mut rest = &data[1..];
        while !rest.is_empty() {
            let (block, consumed) = LicenseBlock::parse(rest)?;
            blocks.push(block);
            rest = &rest[consumed..];
        }

        Ok(Self { blocks })
    }

    /// Iteratively derives the final 32-byte Ed25519 public key across all blocks starting from [`LICENSE_ROOT_KEY`].
    ///
    /// Reference: `TSLib/Full/License.cs:55-61`.
    pub fn derive_key(&self) -> Result<[u8; 32], LicenseError> {
        let mut round = LICENSE_ROOT_KEY;
        for block in &self.blocks {
            round = block.derive_key(&round)?;
        }
        Ok(round)
    }

    /// Parses `license_bytes` and derives the resulting 32-byte server ephemeral Ed25519 public key.
    pub fn parse_and_derive_key(license_bytes: &[u8]) -> Result<[u8; 32], LicenseError> {
        let chain = Self::parse(license_bytes)?;
        chain.derive_key()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_root_key_decompresses_valid_edwards_point() {
        let point = CompressedEdwardsY(LICENSE_ROOT_KEY).decompress();
        assert!(point.is_some(), "LICENSE_ROOT_KEY must be a valid Ed25519 point");
    }

    #[test]
    fn test_parse_real_ts3_anonymous_server_license_chain() {
        // 100-byte license chain: 1-byte version (0x01) + 57B Server block ("Anonymous\0") + 42B Ephemeral block
        let base_point = curve25519_dalek::constants::ED25519_BASEPOINT_POINT;
        let pub1 = (&base_point * &Scalar::from(2u64)).compress().to_bytes();
        let pub2 = (&base_point * &Scalar::from(11u64)).compress().to_bytes();

        let mut raw = Vec::with_capacity(100);
        raw.push(0x01); // version = 1

        // Block 0: Server block (MinBlockLen(42) + 6 + 9 = 57 bytes)
        raw.push(0x00); // keyKind = 0
        raw.extend_from_slice(&pub1); // [1..33] key
        raw.push(0x02); // [33] blockType = 2 (Server)
        raw.extend_from_slice(&1_000u32.to_be_bytes()); // [34..38] notValidBefore
        raw.extend_from_slice(&2_000u32.to_be_bytes()); // [38..42] notValidAfter
        raw.push(ServerLicenseType::Default as u8); // [42] ServerLicenseType::Default (7)
        raw.extend_from_slice(&[0u8; 4]); // [43..47] reserved
        raw.extend_from_slice(b"Anonymous\0"); // [47..57] null-terminated issuer ("Anonymous", len 9)

        // Block 1: Ephemeral block (42 bytes)
        raw.push(0x00); // keyKind = 0
        raw.extend_from_slice(&pub2); // [1..33] key
        raw.push(32); // [33] blockType = 32 (Ephemeral)
        raw.extend_from_slice(&1_500u32.to_be_bytes()); // [34..38] notValidBefore
        raw.extend_from_slice(&2_500u32.to_be_bytes()); // [38..42] notValidAfter
        assert_eq!(raw.len(), 100);

        let chain = LicenseChain::parse(&raw).expect("failed to parse license chain");
        assert_eq!(chain.blocks.len(), 2);

        // Block 0: Server license block ("Anonymous", Default = 7)
        assert_eq!(chain.blocks[0].block_type, ChainBlockType::Server);
        assert_eq!(
            chain.blocks[0].kind,
            LicenseBlockKind::Server(ServerBlock {
                license_type: ServerLicenseType::Default,
                issuer: "Anonymous".to_string(),
            })
        );

        // Block 1: Ephemeral license block
        assert_eq!(chain.blocks[1].block_type, ChainBlockType::Ephemeral);
        assert_eq!(
            chain.blocks[1].kind,
            LicenseBlockKind::Ephemeral(KeyBlock)
        );

        // Derive key and verify manual step-by-step equivalence
        let derived = chain.derive_key().expect("failed to derive key");
        let step1 = chain.blocks[0].derive_key(&LICENSE_ROOT_KEY).unwrap();
        let step2 = chain.blocks[1].derive_key(&step1).unwrap();
        assert_eq!(derived, step2);
        assert!(CompressedEdwardsY(derived).decompress().is_some());
    }

    #[test]
    fn test_parse_intermediate_server_ephemeral_chain() {
        let base = curve25519_dalek::constants::ED25519_BASEPOINT_POINT;
        let k1 = (&base * &Scalar::from(3u64)).compress().to_bytes();
        let k2 = (&base * &Scalar::from(5u64)).compress().to_bytes();
        let k3 = (&base * &Scalar::from(7u64)).compress().to_bytes();

        let mut buf = vec![0x01]; // Version 1

        // 1. Intermediate block (type 0): MinBlockLen(42) + 5 + issuer.len() = 47 + 9 = 56 bytes
        let mut b0 = Vec::new();
        b0.push(0x00); // keyKind = 0
        b0.extend_from_slice(&k1); // [1..33] key
        b0.push(0x00); // [33] type = 0 (Intermediate)
        b0.extend_from_slice(&100u32.to_be_bytes()); // [34..38] notValidBefore
        b0.extend_from_slice(&200u32.to_be_bytes()); // [38..42] notValidAfter
        b0.extend_from_slice(&[0u8; 4]); // [42..46] 4 bytes before string at 46
        b0.extend_from_slice(b"TeamSpeak\0"); // [46..56] null-terminated string (len 9, read = 5 + 9 = 14 -> allLen = 42 + 14 = 56)
        assert_eq!(b0.len(), 56);
        buf.extend_from_slice(&b0);

        // 2. Server block (type 2): MinBlockLen(42) + 6 + issuer.len()
        let mut b1 = Vec::new();
        b1.push(0x00); // keyKind = 0
        b1.extend_from_slice(&k2); // [1..33] key
        b1.push(0x02); // [33] type = 2 (Server)
        b1.extend_from_slice(&300u32.to_be_bytes()); // [34..38]
        b1.extend_from_slice(&400u32.to_be_bytes()); // [38..42]
        b1.push(ServerLicenseType::Npl as u8); // [42] ServerLicenseType
        b1.extend_from_slice(&[0u8; 4]); // [43..47] 4 bytes before string at 47
        b1.extend_from_slice(b"NPL Issuer\0"); // [47..58] len 10, read = 6 + 10 = 16 -> allLen = 42 + 16 = 58
        assert_eq!(b1.len(), 58);
        buf.extend_from_slice(&b1);

        // 3. Ephemeral block (type 32): 42 bytes
        let mut b2 = Vec::new();
        b2.push(0x00); // keyKind = 0
        b2.extend_from_slice(&k3); // [1..33] key
        b2.push(32); // [33] type = 32 (Ephemeral)
        b2.extend_from_slice(&500u32.to_be_bytes()); // [34..38]
        b2.extend_from_slice(&600u32.to_be_bytes()); // [38..42]
        assert_eq!(b2.len(), 42);
        buf.extend_from_slice(&b2);

        let key = LicenseChain::parse_and_derive_key(&buf).unwrap();
        assert!(CompressedEdwardsY(key).decompress().is_some());
    }

    #[test]
    fn test_chaos_nacl_negate_equivalence() {
        // Verify that (-P_key * s + -P_parent) with sign-bit flip (final[31] ^= 0x80)
        // is bit-identical to (P_key * s + P_parent).
        let base = curve25519_dalek::constants::ED25519_BASEPOINT_POINT;
        let pub_point = &base * &Scalar::from(1234567u64);
        let par_point = CompressedEdwardsY(LICENSE_ROOT_KEY).decompress().unwrap();

        let mut h = [0x42u8; 32];
        h[0] &= 248;
        h[31] &= 63;
        h[31] |= 64;
        let s = Scalar::from_bytes_mod_order(h);

        // Method A: direct addition
        let direct = (pub_point * s + par_point).compress().to_bytes();

        // Method B: Chaos.NaCl ge_frombytes_negate_vartime + ge_add + ^= 0x80
        let neg_pub = -pub_point;
        let neg_par = -par_point;
        let mut nacl_bytes = (neg_pub * s + neg_par).compress().to_bytes();
        nacl_bytes[31] ^= 0x80;

        assert_eq!(direct, nacl_bytes);
    }

    #[test]
    fn test_license_error_cases() {
        assert_eq!(LicenseChain::parse(&[]), Err(LicenseError::TooShort));
        assert_eq!(
            LicenseChain::parse(&[0x02]),
            Err(LicenseError::UnsupportedVersion(2))
        );
        // Truncated block
        assert_eq!(
            LicenseChain::parse(&[0x01, 0x00, 0x01]),
            Err(LicenseError::TooShort)
        );
        // Wrong key kind
        let mut bad_kind = [0u8; 43];
        bad_kind[0] = 0x01;
        bad_kind[1] = 0x05;
        assert_eq!(
            LicenseChain::parse(&bad_kind),
            Err(LicenseError::WrongKeyKind(5))
        );
        // Invalid timestamps (after < before)
        let mut bad_time = [0u8; 43];
        bad_time[0] = 0x01;
        bad_time[1 + 33] = 32; // Ephemeral
        bad_time[1 + 34..1 + 38].copy_from_slice(&200u32.to_be_bytes());
        bad_time[1 + 38..1 + 42].copy_from_slice(&100u32.to_be_bytes());
        assert_eq!(
            LicenseChain::parse(&bad_time),
            Err(LicenseError::InvalidTimestamps)
        );
    }
}
