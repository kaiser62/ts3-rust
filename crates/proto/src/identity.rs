use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use ed25519_dalek::{Signature as Ed25519Signature, Verifier as _, VerifyingKey as Ed25519VerifyingKey};
use p256::ecdsa::signature::Signer as _;
use p256::ecdsa::{Signature as P256Signature, SigningKey, VerifyingKey};
use p256::elliptic_curve::sec1::{FromEncodedPoint as _, ToEncodedPoint as _};
use p256::{EncodedPoint, PublicKey, SecretKey};
use rand::rngs::OsRng;
use sha1::{Digest as _, Sha1};
use thiserror::Error;

/// Client-side identity obfuscation key (`TsCrypt.cs:210`).
/// Only the first `min(100, ident.len())` bytes are XORed during obfuscation/deobfuscation (`TsCrypt.cs:232`).
pub const TS_IDENTITY_OBFUSCATION_KEY: &[u8; 128] =
    b"b9dfaa7bee6ac57ac7b65f1094a1c155e747327bc2fe5d51c512023fe54a280201004e90ad1daaae1075d53b7d571c30e063b5a62a4a017bb394833aa0983e6e";

/// TeamSpeak 3 Ed25519 public key used to verify `(platform, version, sign)` tuples (`TsCrypt.cs:771`, `ts3server:0x7c7591`).
/// Base64: `"UrN1jX0dBE1vulTNLCoYwrVpfITyo+NBuq/twbf9hLw="`.
pub const TS_VERSION_SIGN_PUBLIC_KEY: [u8; 32] = [
    0x52, 0xb3, 0x75, 0x8d, 0x7d, 0x1d, 0x04, 0x4d, 0x6f, 0xba, 0x54, 0xcd, 0x2c, 0x2a, 0x18, 0xc2,
    0xb5, 0x69, 0x7c, 0x84, 0xf2, 0xa3, 0xe3, 0x41, 0xba, 0xaf, 0xed, 0xc1, 0xb7, 0xfd, 0x84, 0xbc,
];

/// Errors that can occur during TeamSpeak 3 identity parsing, import, or cryptographic operations.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum IdentityError {
    #[error("base64 decode error: {0}")]
    Base64(#[from] base64::DecodeError),
    #[error("invalid ASN.1 DER structure: {0}")]
    InvalidAsn1(&'static str),
    #[error("invalid P-256 public key point")]
    InvalidPublicKey,
    #[error("invalid P-256 private key scalar")]
    InvalidPrivateKey,
    #[error("invalid TeamSpeak identity format: {0}")]
    InvalidIdentityFormat(&'static str),
    #[error("invalid identity key offset")]
    InvalidKeyOffset,
}

/// Represents a TeamSpeak 3 client identity backed by a NIST P-256 (`secp256r1` / `prime256v1`) keypair
/// and hashcash security level offset (`TsCrypt.cs`, `IdentityData.cs`).
#[derive(Debug, Clone)]
pub struct IdentityData {
    secret_key: SecretKey,
    pub valid_key_offset: u64,
    pub last_checked_key_offset: u64,
}

impl IdentityData {
    /// Creates an [`IdentityData`] from an existing P-256 [`SecretKey`] and key offsets.
    pub fn from_secret_key(
        secret_key: SecretKey,
        valid_key_offset: u64,
        last_checked_key_offset: u64,
    ) -> Self {
        Self {
            secret_key,
            valid_key_offset,
            last_checked_key_offset: last_checked_key_offset.max(valid_key_offset),
        }
    }

    /// Generates a new random NIST P-256 identity with key offsets initialized to `0`.
    pub fn generate() -> Self {
        let secret_key = SecretKey::random(&mut OsRng);
        Self {
            secret_key,
            valid_key_offset: 0,
            last_checked_key_offset: 0,
        }
    }

    /// Generates a new random NIST P-256 identity and improves its hashcash security level to at least `min_level`.
    pub fn generate_with_level(min_level: u32) -> Self {
        let mut identity = Self::generate();
        identity.improve_security_level(min_level);
        identity
    }

    /// Returns a reference to the underlying P-256 [`SecretKey`].
    pub fn secret_key(&self) -> &SecretKey {
        &self.secret_key
    }

    /// Returns the corresponding P-256 [`PublicKey`].
    pub fn public_key(&self) -> PublicKey {
        self.secret_key.public_key()
    }

    /// Returns the ECDSA-P256 [`SigningKey`] for this identity.
    pub fn signing_key(&self) -> SigningKey {
        SigningKey::from(&self.secret_key)
    }

    /// Encodes the P-256 public key into the LibTomCrypt ASN.1 DER base64 `omega` string (`TsCrypt.cs:166-174`).
    pub fn public_key_omega(&self) -> String {
        encode_omega_public_key(&self.public_key())
    }

    /// Encodes the full P-256 keypair (`X`, `Y`, `D`) into LibTomCrypt ASN.1 DER base64 (`TsCrypt.cs:185-194`).
    ///
    /// Layout: `0x30, <seq_len>, 0x03, 0x02, 0x07, 0x80, 0x02, 0x01, 0x20, <INT X>, <INT Y>, <INT D>`.
    pub fn encode_private_key_ltc(&self) -> String {
        let pubkey = self.public_key();
        let point = pubkey.to_encoded_point(false);
        let x = point.x().expect("uncompressed P-256 point has X");
        let y = point.y().expect("uncompressed P-256 point has Y");
        let d = self.secret_key.to_bytes();

        let mut body = Vec::with_capacity(112);
        // BIT STRING: length 2, 7 unused bits, flags = 0x80 (PK_PRIVATE in LibTomCrypt)
        body.extend_from_slice(&[0x03, 0x02, 0x07, 0x80]);
        // INTEGER 32 (0x20)
        body.extend_from_slice(&[0x02, 0x01, 0x20]);
        encode_asn1_positive_int(&mut body, x.as_slice());
        encode_asn1_positive_int(&mut body, y.as_slice());
        encode_asn1_positive_int(&mut body, d.as_slice());

        let der = wrap_asn1_sequence(&body);
        BASE64_STANDARD.encode(der)
    }

    /// Encodes the private-only P-256 scalar (`D`) in TSLib's compact ASN.1 DER base64 format (`TsCrypt.cs:176-183`).
    ///
    /// Layout: `0x30, <seq_len>, 0x03, 0x02, 0x06, 0xC0, 0x02, 0x01, 0x20, <INT D>`.
    pub fn encode_private_key_short_ltc(&self) -> String {
        let d = self.secret_key.to_bytes();
        let mut body = Vec::with_capacity(44);
        body.extend_from_slice(&[0x03, 0x02, 0x06, 0xC0]);
        body.extend_from_slice(&[0x02, 0x01, 0x20]);
        encode_asn1_positive_int(&mut body, d.as_slice());

        let der = wrap_asn1_sequence(&body);
        BASE64_STANDARD.encode(der)
    }

    /// Decodes a LibTomCrypt / TSLib ASN.1 DER base64 private key string into a P-256 [`SecretKey`].
    pub fn decode_private_key_ltc(b64: &str) -> Result<SecretKey, IdentityError> {
        decode_private_key_ltc(b64)
    }

    /// Computes the TeamSpeak 3 client unique identifier (`client_unique_identifier`):
    /// `Base64(SHA1(omega.as_bytes()))` (`TsCrypt.cs:196-201`).
    pub fn uid(&self) -> String {
        uid_from_omega(&self.public_key_omega())
    }

    /// Computes the current hashcash security level at `self.valid_key_offset` (`TsCrypt.cs:834-840`).
    pub fn security_level(&self) -> u32 {
        self.security_level_at(self.valid_key_offset)
    }

    /// Computes the hashcash security level at the given `offset` (`TsCrypt.cs:862-892`).
    pub fn security_level_at(&self, offset: u64) -> u32 {
        security_level_for_omega(&self.public_key_omega(), offset)
    }

    /// Brute-forces `last_checked_key_offset` upwards until `security_level() >= target_level`,
    /// updating `valid_key_offset` whenever a strictly better level is found (`TsCrypt.cs:812-832`).
    pub fn improve_security_level(&mut self, target_level: u32) {
        let omega = self.public_key_omega();
        let omega_bytes = omega.as_bytes();

        self.last_checked_key_offset = self.last_checked_key_offset.max(self.valid_key_offset);
        let mut best = security_level_for_omega_bytes(omega_bytes, self.valid_key_offset);
        if best >= target_level {
            return;
        }

        let mut buf = Vec::with_capacity(omega_bytes.len() + 20);
        buf.extend_from_slice(omega_bytes);

        while best < target_level {
            buf.truncate(omega_bytes.len());
            write_u64_ascii(&mut buf, self.last_checked_key_offset);
            let digest = Sha1::digest(&buf);
            let curr = count_leading_zero_bits_lsb(&digest);
            if curr > best {
                self.valid_key_offset = self.last_checked_key_offset;
                best = curr;
            }
            self.last_checked_key_offset = self.last_checked_key_offset.wrapping_add(1);
        }
    }

    /// Signs `data` with ECDSA-P256-SHA256 and returns the ASN.1 DER encoded signature (`TsCrypt.cs:753-760`).
    pub fn sign_asn1_der(&self, data: &[u8]) -> Vec<u8> {
        let signing_key = self.signing_key();
        let sig: P256Signature = signing_key.sign(data);
        sig.to_der().as_bytes().to_vec()
    }

    /// Verifies an ASN.1 DER encoded ECDSA-P256-SHA256 signature against `pubkey` (`TsCrypt.cs:762-769`).
    pub fn verify_asn1_der(pubkey: &PublicKey, data: &[u8], sig_der: &[u8]) -> bool {
        verify_asn1_der(pubkey, data, sig_der)
    }

    /// Computes the 32-byte big-endian affine X coordinate of the P-256 ECDH shared point `d * peer_pub`
    /// used in legacy `initivexpand` (`TsCrypt.cs:273-285`).
    pub fn ecdh_shared_x(&self, peer_pub: &PublicKey) -> [u8; 32] {
        let shared = p256::ecdh::diffie_hellman(
            self.secret_key.to_nonzero_scalar(),
            peer_pub.as_affine(),
        );
        let mut out = [0u8; 32];
        out.copy_from_slice(shared.raw_secret_bytes().as_slice());
        out
    }

    /// Computes `SHA1(ecdh_shared_x)` (20 bytes) as in `TsCrypt.GetSharedSecret` (`TsCrypt.cs:273-285`).
    pub fn ecdh_shared_key_sha1(&self, peer_pub: &PublicKey) -> [u8; 20] {
        let x = self.ecdh_shared_x(peer_pub);
        let digest = Sha1::digest(x);
        let mut out = [0u8; 20];
        out.copy_from_slice(&digest);
        out
    }

    /// Exports this identity in the obfuscated TeamSpeak 3 INI format `<valid_key_offset>V<obfuscated_b64>`
    /// (`TsCrypt.cs:209-245`, `03-crypto-handshake.md:164-171`).
    pub fn export_ts_identity(&self) -> String {
        let ltc_b64 = self.encode_private_key_ltc();
        let obf_b64 = obfuscate_identity_payload(ltc_b64.as_bytes());
        format!("{}V{}", self.valid_key_offset, obf_b64)
    }

    /// Imports an [`IdentityData`] from either:
    /// 1. The obfuscated TeamSpeak 3 format `<offset>V<obfuscated_b64>` (`TsCrypt.cs:212-245`), or
    /// 2. Raw base64 LibTomCrypt / TSLib ASN.1 DER private key format (`TsCrypt.cs:85-111`), or
    /// 3. Obfuscated base64 without the `<offset>V` prefix.
    pub fn import_ts_identity(s: &str) -> Result<Self, IdentityError> {
        let s = s.trim();
        if s.is_empty() {
            return Err(IdentityError::InvalidIdentityFormat("empty identity string"));
        }

        if let Some(v_idx) = s.find('V') {
            let (level_str, payload_str) = s.split_at(v_idx);
            let payload_str = &payload_str[1..]; // skip 'V'
            if !level_str.is_empty() && level_str.bytes().all(|b| b.is_ascii_digit()) {
                let offset = level_str
                    .parse::<u64>()
                    .map_err(|_| IdentityError::InvalidKeyOffset)?;
                let deobf_b64 = deobfuscate_identity_payload(payload_str)?;
                let secret_key = decode_private_key_ltc(&deobf_b64)?;
                return Ok(Self::from_secret_key(secret_key, offset, offset));
            }
        }

        // Fallback 1: Raw base64 ASN.1 DER key (Matching TsCrypt.LoadIdentityDynamic -> LoadIdentity)
        if let Ok(secret_key) = decode_private_key_ltc(s) {
            return Ok(Self::from_secret_key(secret_key, 0, 0));
        }

        // Fallback 2: Obfuscated base64 payload without `<offset>V` prefix
        if let Ok(deobf_b64) = deobfuscate_identity_payload(s) {
            if let Ok(secret_key) = decode_private_key_ltc(&deobf_b64) {
                return Ok(Self::from_secret_key(secret_key, 0, 0));
            }
        }

        Err(IdentityError::InvalidIdentityFormat(
            "could not parse identity as obfuscated TS3 identity or raw ASN.1 DER key",
        ))
    }
}

/// Encodes a P-256 [`PublicKey`] into the LibTomCrypt ASN.1 DER base64 `omega` format (`TsCrypt.cs:166-174`).
///
/// Layout: `0x30, <seq_len>, 0x03, 0x02, 0x07, 0x00, 0x02, 0x01, 0x20, 0x02, <x_len>, <X_bytes>, 0x02, <y_len>, <Y_bytes>`.
pub fn encode_omega_public_key(pubkey: &PublicKey) -> String {
    let point = pubkey.to_encoded_point(false);
    let x = point.x().expect("uncompressed P-256 point has X");
    let y = point.y().expect("uncompressed P-256 point has Y");

    let mut body = Vec::with_capacity(78);
    // BIT STRING: length 2, 7 unused bits, 0x00 (PK_PUBLIC in LibTomCrypt, TsCrypt.cs:169)
    body.extend_from_slice(&[0x03, 0x02, 0x07, 0x00]);
    // INTEGER 32 (0x20)
    body.extend_from_slice(&[0x02, 0x01, 0x20]);
    encode_asn1_positive_int(&mut body, x.as_slice());
    encode_asn1_positive_int(&mut body, y.as_slice());

    let der = wrap_asn1_sequence(&body);
    BASE64_STANDARD.encode(der)
}

/// Parses a base64-encoded LibTomCrypt ASN.1 DER `omega` string into a P-256 [`PublicKey`] (`TsCrypt.cs:124-136`).
pub fn parse_omega_public_key(omega_b64: &str) -> Result<PublicKey, IdentityError> {
    let der = BASE64_STANDARD.decode(omega_b64.trim())?;
    parse_omega_public_key_der(&der)
}

/// Parses raw LibTomCrypt ASN.1 DER bytes into a P-256 [`PublicKey`].
pub fn parse_omega_public_key_der(der: &[u8]) -> Result<PublicKey, IdentityError> {
    let mut seq = parse_asn1_sequence(der)?;

    // Element 0: BIT STRING (0x03, len 2, unused bits 7, flags 0x00 or 0x80)
    if seq.len() < 4 || seq[0] != 0x03 || seq[1] != 0x02 {
        return Err(IdentityError::InvalidAsn1("expected 2-byte BIT STRING header"));
    }
    let unused_bits = seq[2];
    let bit_flags = seq[3];
    if unused_bits != 0x07 || (bit_flags != 0x00 && bit_flags != 0x80) {
        return Err(IdentityError::InvalidAsn1("unexpected BIT STRING flags for public key"));
    }
    seq = &seq[4..];

    // Element 1: INTEGER 32 (0x02, 0x01, 0x20)
    if seq.len() < 3 || seq[0] != 0x02 || seq[1] != 0x01 || seq[2] != 0x20 {
        return Err(IdentityError::InvalidAsn1("expected curve size INTEGER 32"));
    }
    seq = &seq[3..];

    // Element 2: INTEGER X, Element 3: INTEGER Y
    let (x_bytes, rest) = decode_asn1_u256_be(seq)?;
    let (y_bytes, _rest) = decode_asn1_u256_be(rest)?;

    let encoded_point = EncodedPoint::from_affine_coordinates(&x_bytes.into(), &y_bytes.into(), false);
    Option::<PublicKey>::from(PublicKey::from_encoded_point(&encoded_point))
        .ok_or(IdentityError::InvalidPublicKey)
}

/// Decodes a base64-encoded LibTomCrypt (`X, Y, D`) or TSLib (`D`) ASN.1 DER key into a P-256 [`SecretKey`]
/// (`TsCrypt.cs:138-164`).
pub fn decode_private_key_ltc(b64: &str) -> Result<SecretKey, IdentityError> {
    let der = BASE64_STANDARD.decode(b64.trim())?;
    decode_private_key_ltc_der(&der)
}

/// Decodes raw LibTomCrypt / TSLib ASN.1 DER bytes into a P-256 [`SecretKey`].
pub fn decode_private_key_ltc_der(der: &[u8]) -> Result<SecretKey, IdentityError> {
    let mut seq = parse_asn1_sequence(der)?;

    if seq.len() < 4 || seq[0] != 0x03 || seq[1] != 0x02 {
        return Err(IdentityError::InvalidAsn1("expected 2-byte BIT STRING header"));
    }
    let unused_bits = seq[2];
    let bit_flags = seq[3];
    seq = &seq[4..];

    // INTEGER 32 (0x02, 0x01, 0x20)
    if seq.len() < 3 || seq[0] != 0x02 || seq[1] != 0x01 || seq[2] != 0x20 {
        return Err(IdentityError::InvalidAsn1("expected curve size INTEGER 32"));
    }
    seq = &seq[3..];

    if unused_bits == 0x06 && bit_flags == 0xC0 {
        // TSLib short private key format: [BIT STRING(6, 0xC0), INT(32), INT(D)]
        let (d_bytes, rest) = decode_asn1_u256_be(seq)?;
        if !rest.is_empty() {
            return Err(IdentityError::InvalidAsn1("trailing bytes after private key D"));
        }
        SecretKey::from_slice(&d_bytes).map_err(|_| IdentityError::InvalidPrivateKey)
    } else if unused_bits == 0x07 && (bit_flags == 0x80 || bit_flags == 0x00) {
        // Full LibTomCrypt keypair format: [BIT STRING(7, 0x80), INT(32), INT(X), INT(Y), INT(D)]
        let (x_bytes, rest) = decode_asn1_u256_be(seq)?;
        let (y_bytes, rest) = decode_asn1_u256_be(rest)?;
        if rest.is_empty() {
            return Err(IdentityError::InvalidAsn1("key string did not contain a private key"));
        }
        let (d_bytes, rest) = decode_asn1_u256_be(rest)?;
        if !rest.is_empty() {
            return Err(IdentityError::InvalidAsn1("trailing bytes after private key D"));
        }

        // Validate that (X, Y) is a valid curve point matching D
        let encoded_point =
            EncodedPoint::from_affine_coordinates(&x_bytes.into(), &y_bytes.into(), false);
        let pub_from_xy = Option::<PublicKey>::from(PublicKey::from_encoded_point(&encoded_point))
            .ok_or(IdentityError::InvalidPublicKey)?;
        let secret = SecretKey::from_slice(&d_bytes).map_err(|_| IdentityError::InvalidPrivateKey)?;
        if secret.public_key() != pub_from_xy {
            return Err(IdentityError::InvalidPublicKey);
        }
        Ok(secret)
    } else {
        Err(IdentityError::InvalidAsn1("unsupported BIT STRING key type"))
    }
}

/// Computes `Base64(SHA1(omega.as_bytes()))` (`TsCrypt.cs:196-201`).
pub fn uid_from_omega(omega: &str) -> String {
    let digest = Sha1::digest(omega.as_bytes());
    BASE64_STANDARD.encode(digest)
}

/// Computes the TeamSpeak 3 hashcash security level for an `omega` base64 string and `offset` (`TsCrypt.cs:862-892`).
pub fn security_level_for_omega(omega: &str, offset: u64) -> u32 {
    security_level_for_omega_bytes(omega.as_bytes(), offset)
}

fn security_level_for_omega_bytes(omega_bytes: &[u8], offset: u64) -> u32 {
    let mut hasher = Sha1::new();
    hasher.update(omega_bytes);
    let mut num_buf = Vec::with_capacity(20);
    write_u64_ascii(&mut num_buf, offset);
    hasher.update(&num_buf);
    let digest = hasher.finalize();
    count_leading_zero_bits_lsb(&digest)
}

/// Counts leading zero bits byte-by-byte from index 0, testing bits from LSB (bit 0) to MSB (bit 7)
/// within the first non-zero byte (`TsCrypt.cs:879-892`).
pub fn count_leading_zero_bits_lsb(hash: &[u8]) -> u32 {
    let mut count = 0u32;
    for &b in hash {
        if b == 0 {
            count += 8;
        } else {
            count += b.trailing_zeros();
            break;
        }
    }
    count
}

/// Verifies an ASN.1 DER encoded ECDSA-P256-SHA256 signature against `pubkey` (`TsCrypt.cs:762-769`).
///
/// Normalizes high-S signatures to low-S before verification so signatures produced by
/// LibTomCrypt (`ts3server`) and BouncyCastle (`TSLib`) verify reliably.
pub fn verify_asn1_der(pubkey: &PublicKey, data: &[u8], sig_der: &[u8]) -> bool {
    let verifying_key = VerifyingKey::from(pubkey);
    let sig = match P256Signature::from_der(sig_der).or_else(|_| parse_ecdsa_der_lenient(sig_der)) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let normalized = sig.normalize_s().unwrap_or(sig);
    verifying_key.verify(data, &normalized).is_ok()
}

/// Hashes a password like TeamSpeak 3 (`Base64(SHA1(UTF8(password)))`, `TsCrypt.cs:744-751`).
pub fn hash_password(password: &str) -> String {
    if password.is_empty() {
        return String::new();
    }
    let digest = Sha1::digest(password.as_bytes());
    BASE64_STANDARD.encode(digest)
}

/// TeamSpeak 3 signed client version tuple `(version, platform, sign)` (`TsVersion.cs`, `TsVersion.gen.cs`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionSign {
    pub version: String,
    pub platform: String,
    pub sign: String,
}

impl VersionSign {
    /// Creates a new [`VersionSign`] triple.
    pub fn new(
        version: impl Into<String>,
        platform: impl Into<String>,
        sign: impl Into<String>,
    ) -> Self {
        Self {
            version: version.into(),
            platform: platform.into(),
            sign: sign.into(),
        }
    }

    /// Windows 3.5.3 stable release version tuple (`TsVersion.gen.cs:33`).
    pub fn default_windows() -> Self {
        Self::new(
            "3.5.3 [Build: 1587971024]",
            "Windows",
            "Kvmj7qX6wJCPI5GVT71samfmhz/bvs7M+OTXWB/JWxdQbxDe17xda7dzUWLX7pjvdJTqZmbse1HBmTxThPKvAg==",
        )
    }

    /// Linux 3.5.3 stable release version tuple (`TsVersion.gen.cs:34`).
    pub fn default_linux() -> Self {
        Self::new(
            "3.5.3 [Build: 1587971024]",
            "Linux",
            "59chu1YQ1W4DdVj+yJOVmwhJK7s9p9FatqIjQRxEDmIb0CcDDO/K8CrbVnBHbD67/cExJbC3PjC/o/n0pDbiCg==",
        )
    }

    /// macOS (`OS X`) 3.5.3 stable release version tuple (`TsVersion.gen.cs:36`).
    pub fn default_macos() -> Self {
        Self::new(
            "3.5.3 [Build: 1586955962]",
            "OS X",
            "/9gsbyyJoyHW1okwykGgXrn2j29wvnEfnw6/Hvft2WS12CURxTv5L9uLuu36I5u3TzWOxyB9dEbD1FeZ201uCQ==",
        )
    }

    /// Linux universal debug/wildcard version tuple (`TsVersion.gen.cs:28`).
    pub fn universal_linux() -> Self {
        Self::new(
            "3.?.? [Build: 5680278000]",
            "Linux",
            "Hjd+N58Gv3ENhoKmGYy2bNRBsNNgm5kpiaQWxOj5HN2DXttG6REjymSwJtpJ8muC2gSwRuZi0R+8Laan5ts5CQ==",
        )
    }

    /// Android 3.5.0 stable release version tuple (`TsVersion.gen.cs:47`).
    pub fn default_android() -> Self {
        Self::new(
            "3.5.0 [Build: 1582117478]",
            "Android",
            "vASJQFPECG2TpUh5ATsR2c/gVrvsuZ9QlMpKN51F+HnanYNH3oCSGoievQX6ACjsKH3OZihNFQZ6f5HRU/MAAg==",
        )
    }

    /// iOS 3.5.1 stable release version tuple (`TsVersion.gen.cs:46`).
    pub fn default_ios() -> Self {
        Self::new(
            "3.5.1 [Build: 1582118844]",
            "iOS",
            "3ZiKmiSA9g1xUhb67wgAPz7duV97QCoMQn4OmjqXUffVN5cvztaUhIBm0oZ8n0DEXsgx8hulbqWBFIN8QgAIBA==",
        )
    }

    /// Extracts the build timestamp from `"<semver> [Build: <timestamp>]"` if present (`TsVersion.cs:19-57`).
    pub fn build_timestamp(&self) -> Option<u64> {
        let marker = "[Build: ";
        let start = self.version.find(marker)? + marker.len();
        let rest = &self.version[start..];
        let end = rest.find(']')?;
        rest[..end].parse::<u64>().ok()
    }

    /// Verifies this `(platform, version, sign)` tuple against [`TS_VERSION_SIGN_PUBLIC_KEY`]
    /// using Ed25519 over `ASCII(platform || version)` (`TsCrypt.cs:773-780`).
    pub fn verify_version_sign(&self) -> bool {
        let sig_bytes = match BASE64_STANDARD.decode(self.sign.trim()) {
            Ok(b) => b,
            Err(_) => return false,
        };
        let sig = match Ed25519Signature::from_slice(&sig_bytes) {
            Ok(s) => s,
            Err(_) => return false,
        };
        let pubkey = match Ed25519VerifyingKey::from_bytes(&TS_VERSION_SIGN_PUBLIC_KEY) {
            Ok(pk) => pk,
            Err(_) => return false,
        };

        let mut msg = Vec::with_capacity(self.platform.len() + self.version.len());
        msg.extend_from_slice(self.platform.as_bytes());
        msg.extend_from_slice(self.version.as_bytes());

        pubkey.verify(&msg, &sig).is_ok()
    }
}

impl Default for VersionSign {
    fn default() -> Self {
        Self::default_windows()
    }
}

fn obfuscate_identity_payload(ltc_b64_ascii: &[u8]) -> String {
    let mut ident = ltc_b64_ascii.to_vec();
    let xor_len = ident.len().min(100);
    for i in 0..xor_len {
        ident[i] ^= TS_IDENTITY_OBFUSCATION_KEY[i];
    }

    if ident.len() >= 20 {
        let hash_len = ident[20..]
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(ident.len() - 20);
        let digest = Sha1::digest(&ident[20..20 + hash_len]);
        for i in 0..20 {
            ident[i] ^= digest[i];
        }
    }

    BASE64_STANDARD.encode(&ident)
}

fn deobfuscate_identity_payload(obf_b64: &str) -> Result<String, IdentityError> {
    let raw = BASE64_STANDARD.decode(obf_b64.trim())?;
    if raw.len() < 20 {
        return Err(IdentityError::InvalidIdentityFormat(
            "identity payload shorter than 20 bytes",
        ));
    }

    let null_pos_20 = raw[20..].iter().position(|&b| b == 0);
    let primary_hash_len = null_pos_20.unwrap_or(raw.len() - 20);

    if let Ok(s) = deobfuscate_with_hash_len(&raw, primary_hash_len) {
        if decode_private_key_ltc(&s).is_ok() {
            return Ok(s);
        }
    }

    // Fallback if producer hashed full slice without stopping at an intermediate 0x00 byte
    if primary_hash_len != raw.len() - 20 {
        if let Ok(s) = deobfuscate_with_hash_len(&raw, raw.len() - 20) {
            if decode_private_key_ltc(&s).is_ok() {
                return Ok(s);
            }
        }
    }

    deobfuscate_with_hash_len(&raw, primary_hash_len)
}

fn deobfuscate_with_hash_len(raw: &[u8], hash_len: usize) -> Result<String, IdentityError> {
    let mut ident = raw.to_vec();
    let digest = Sha1::digest(&ident[20..20 + hash_len]);
    for i in 0..20 {
        ident[i] ^= digest[i];
    }

    let xor_len = ident.len().min(100);
    for i in 0..xor_len {
        ident[i] ^= TS_IDENTITY_OBFUSCATION_KEY[i];
    }

    if let Some(null_pos) = ident.iter().position(|&b| b == 0) {
        ident.truncate(null_pos);
    }

    String::from_utf8(ident).map_err(|_| {
        IdentityError::InvalidIdentityFormat("deobfuscated identity is not valid ASCII/UTF-8")
    })
}

fn write_u64_ascii(buf: &mut Vec<u8>, mut val: u64) {
    let mut tmp = [0u8; 20];
    let mut len = 0usize;
    loop {
        tmp[len] = b'0' + (val % 10) as u8;
        val /= 10;
        len += 1;
        if val == 0 {
            break;
        }
    }
    for i in (0..len).rev() {
        buf.push(tmp[i]);
    }
}

fn encode_asn1_positive_int(out: &mut Vec<u8>, bytes_be: &[u8]) {
    let mut stripped = bytes_be;
    while stripped.len() > 1 && stripped[0] == 0 {
        stripped = &stripped[1..];
    }
    if stripped.is_empty() {
        stripped = &[0];
    }
    let prepend_zero = (stripped[0] & 0x80) != 0;
    let total_len = stripped.len() + usize::from(prepend_zero);
    out.push(0x02); // INTEGER tag
    encode_asn1_length(out, total_len);
    if prepend_zero {
        out.push(0x00);
    }
    out.extend_from_slice(stripped);
}

fn wrap_asn1_sequence(body: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(2 + body.len());
    out.push(0x30); // SEQUENCE tag
    encode_asn1_length(&mut out, body.len());
    out.extend_from_slice(body);
    out
}

fn encode_asn1_length(out: &mut Vec<u8>, len: usize) {
    if len < 0x80 {
        out.push(len as u8);
    } else if len <= 0xFF {
        out.push(0x81);
        out.push(len as u8);
    } else {
        out.push(0x82);
        out.push((len >> 8) as u8);
        out.push((len & 0xFF) as u8);
    }
}

fn parse_asn1_length(input: &[u8]) -> Result<(usize, &[u8]), IdentityError> {
    if input.is_empty() {
        return Err(IdentityError::InvalidAsn1("unexpected EOF reading length"));
    }
    let first = input[0];
    if (first & 0x80) == 0 {
        Ok((first as usize, &input[1..]))
    } else {
        let num_bytes = (first & 0x7F) as usize;
        if num_bytes == 0 || num_bytes > 2 || input.len() < 1 + num_bytes {
            return Err(IdentityError::InvalidAsn1("invalid multi-byte DER length"));
        }
        let mut len = 0usize;
        for &b in &input[1..1 + num_bytes] {
            len = (len << 8) | (b as usize);
        }
        Ok((len, &input[1 + num_bytes..]))
    }
}

fn parse_asn1_sequence(der: &[u8]) -> Result<&[u8], IdentityError> {
    if der.is_empty() || der[0] != 0x30 {
        return Err(IdentityError::InvalidAsn1("expected SEQUENCE tag 0x30"));
    }
    let (seq_len, rest) = parse_asn1_length(&der[1..])?;
    if rest.len() != seq_len {
        return Err(IdentityError::InvalidAsn1("SEQUENCE length mismatch"));
    }
    Ok(rest)
}

fn decode_asn1_u256_be(input: &[u8]) -> Result<([u8; 32], &[u8]), IdentityError> {
    if input.is_empty() || input[0] != 0x02 {
        return Err(IdentityError::InvalidAsn1("expected INTEGER tag 0x02"));
    }
    let (int_len, rest) = parse_asn1_length(&input[1..])?;
    if int_len == 0 || rest.len() < int_len {
        return Err(IdentityError::InvalidAsn1("INTEGER length out of bounds"));
    }
    let (mut int_bytes, remaining) = rest.split_at(int_len);
    if (int_bytes[0] & 0x80) != 0 {
        return Err(IdentityError::InvalidAsn1("negative ASN.1 INTEGER not allowed"));
    }
    while !int_bytes.is_empty() && int_bytes[0] == 0 {
        int_bytes = &int_bytes[1..];
    }
    if int_bytes.len() > 32 {
        return Err(IdentityError::InvalidAsn1("INTEGER exceeds 32 bytes"));
    }
    let mut out = [0u8; 32];
    out[32 - int_bytes.len()..].copy_from_slice(int_bytes);
    Ok((out, remaining))
}

fn parse_ecdsa_der_lenient(sig_der: &[u8]) -> Result<P256Signature, IdentityError> {
    let seq = parse_asn1_sequence(sig_der)?;
    let (r, rest) = decode_asn1_u256_be(seq)?;
    let (s, rest) = decode_asn1_u256_be(rest)?;
    if !rest.is_empty() {
        return Err(IdentityError::InvalidAsn1("trailing bytes in ECDSA signature"));
    }
    P256Signature::from_scalars(r, s).map_err(|_| IdentityError::InvalidAsn1("invalid ECDSA scalars"))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Concrete test vector from docs/re/03-crypto-handshake.md Section 7.1 & bot.toml:25-27
    const VECTOR_PRIV_SHORT_B64: &str =
        "MCoDAgbAAgEgAiEApY7dHTX4bMhuKCa5J3z1WyLyMLvnJTWBi8wT7AJtuhA=";
    const VECTOR_PRIV_SCALAR_HEX: &str =
        "a58edd1d35f86cc86e2826b9277cf55b22f230bbe72535818bcc13ec026dba10";
    const VECTOR_OMEGA_B64: &str =
        "MEwDAgcAAgEgAiAUf4CTtPnlwtDyloQqGh4ZsvJqlev5vvndefK2WSzreAIhANEY4CVrn5QI85hTMRftD3hPQnvY1XDfkMiyhPw6UGIz";
    const VECTOR_UID: &str = "tUXzo5QsCNhqcRnaGCA5t3IM3HE=";
    const VECTOR_OBFUSCATED_INI: &str =
        "64VM2F7zoV2btgUFGbpr0NUkAlK6YZ9MmkEW3kJRChyVAAvBE4GZEt7Gy5+EAt/MGJzCg1GZgdzeRIQXXVc";

    #[test]
    fn test_known_vector_omega_uid_and_hashcash() {
        let scalar_bytes = hex::decode(VECTOR_PRIV_SCALAR_HEX).unwrap();
        let secret_key = SecretKey::from_slice(&scalar_bytes).unwrap();
        let identity = IdentityData::from_secret_key(secret_key, 64, 64);

        // Verify omega public key DER base64 matches TsCrypt.cs ExportPublicKey byte-for-byte
        let omega = identity.public_key_omega();
        assert_eq!(omega, VECTOR_OMEGA_B64);

        // Verify short private key DER base64 matches TSLib ExportPrivateKey byte-for-byte
        assert_eq!(identity.encode_private_key_short_ltc(), VECTOR_PRIV_SHORT_B64);

        // Verify omega public key round-trip
        let parsed_pub = parse_omega_public_key(&omega).unwrap();
        assert_eq!(parsed_pub, identity.public_key());

        // Verify Client UID
        assert_eq!(identity.uid(), VECTOR_UID);

        // Verify Security Level at offset 64 is 8 (SHA1 starts with 0x00, 0xF7)
        assert_eq!(identity.security_level(), 8);
        assert_eq!(identity.security_level_at(64), 8);
    }

    #[test]
    fn test_import_known_obfuscated_ts_identity_and_roundtrip() {
        // Import known obfuscated identity from 03-crypto-handshake.md:497
        let imported = IdentityData::import_ts_identity(VECTOR_OBFUSCATED_INI).unwrap();
        assert_eq!(imported.valid_key_offset, 64);
        assert_eq!(imported.last_checked_key_offset, 64);
        assert_eq!(imported.public_key_omega(), VECTOR_OMEGA_B64);
        assert_eq!(imported.uid(), VECTOR_UID);
        assert_eq!(imported.security_level(), 8);

        // Export and re-import full LTC keypair obfuscated identity
        let exported = imported.export_ts_identity();
        assert!(exported.starts_with("64V"));
        let reimported = IdentityData::import_ts_identity(&exported).unwrap();
        assert_eq!(reimported.secret_key().to_bytes(), imported.secret_key().to_bytes());
        assert_eq!(reimported.public_key(), imported.public_key());
        assert_eq!(reimported.valid_key_offset, 64);

        // Also verify importing raw base64 private keys (both full LTC and short TSLib)
        let full_ltc = imported.encode_private_key_ltc();
        let from_full_raw = IdentityData::import_ts_identity(&full_ltc).unwrap();
        assert_eq!(from_full_raw.public_key(), imported.public_key());

        let from_short_raw = IdentityData::import_ts_identity(VECTOR_PRIV_SHORT_B64).unwrap();
        assert_eq!(from_short_raw.public_key(), imported.public_key());
    }

    #[test]
    fn test_hashcash_lsb_bit_counting_and_improve_security() {
        // 0x00 has 8 zero bits; 0xF7 = 0b1111_0111 has 0 trailing (LSB) zeros => total 8
        assert_eq!(count_leading_zero_bits_lsb(&[0x00, 0xF7, 0x00]), 8);
        // 0x00 (8) + 0x48 (0b0100_1000 -> 3 trailing zeros) = 11
        assert_eq!(count_leading_zero_bits_lsb(&[0x00, 0x48, 0x00]), 11);
        // 0x80 (0b1000_0000 -> 7 trailing zeros) = 7
        assert_eq!(count_leading_zero_bits_lsb(&[0x80, 0xFF]), 7);
        // 0x01 (0b0000_0001 -> 0 trailing zeros) = 0
        assert_eq!(count_leading_zero_bits_lsb(&[0x01, 0x00]), 0);

        let id = IdentityData::generate_with_level(8);
        assert!(id.security_level() >= 8);
        assert!(id.last_checked_key_offset >= id.valid_key_offset);
    }

    #[test]
    fn test_ecdsa_asn1_der_sign_and_verify() {
        let id = IdentityData::generate();
        let pubkey = id.public_key();
        let msg = b"TeamSpeak 3 initivexpand2 license & clientek proof payload";

        let sig_der = id.sign_asn1_der(msg);
        assert!(IdentityData::verify_asn1_der(&pubkey, msg, &sig_der));
        assert!(!IdentityData::verify_asn1_der(&pubkey, b"tampered message", &sig_der));

        let other_id = IdentityData::generate();
        assert!(!IdentityData::verify_asn1_der(&other_id.public_key(), msg, &sig_der));
    }

    #[test]
    fn test_ecdh_shared_x_symmetry() {
        let alice = IdentityData::generate();
        let bob = IdentityData::generate();

        let shared_ab = alice.ecdh_shared_x(&bob.public_key());
        let shared_ba = bob.ecdh_shared_x(&alice.public_key());
        assert_eq!(shared_ab, shared_ba);
        assert_ne!(shared_ab, [0u8; 32]);

        let sha1_ab = alice.ecdh_shared_key_sha1(&bob.public_key());
        let sha1_ba = bob.ecdh_shared_key_sha1(&alice.public_key());
        assert_eq!(sha1_ab, sha1_ba);
    }

    #[test]
    fn test_version_sign_verification() {
        let win = VersionSign::default_windows();
        let lin = VersionSign::default_linux();
        let mac = VersionSign::default_macos();
        let uni = VersionSign::universal_linux();
        let and = VersionSign::default_android();
        let ios = VersionSign::default_ios();

        assert!(win.verify_version_sign());
        assert!(lin.verify_version_sign());
        assert!(mac.verify_version_sign());
        assert!(uni.verify_version_sign());
        assert!(and.verify_version_sign());
        assert!(ios.verify_version_sign());

        assert_eq!(win.build_timestamp(), Some(1587971024));
        assert_eq!(uni.build_timestamp(), Some(5680278000));

        // Tampering with platform or version must fail signature check
        let bad = VersionSign::new(&win.version, "Linux", &win.sign);
        assert!(!bad.verify_version_sign());
    }
}
