// Owned by Pool Agent 5 (arthoroy28@gmail.com)

use std::time::{SystemTime, UNIX_EPOCH};

use aes::Aes128;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use cipher::consts::U8;
use curve25519_dalek::edwards::{CompressedEdwardsY, EdwardsPoint};
use curve25519_dalek::scalar::Scalar;
use eax::aead::{AeadInPlace, KeyInit, Tag};
use eax::Eax;
use rand::RngCore;
use sha1::Sha1;
use sha2::{Digest, Sha256, Sha512};

use crate::command::escape as escape_ts3_param;
use crate::identity::{
    parse_omega_public_key_der, verify_asn1_der, IdentityData, IdentityError,
};
use crate::license::{LicenseChain, LicenseError};
pub use crate::packet::{
    C2SPacket, PacketFlags, PacketType, S2CPacket, INIT_VERSION_BYTES, MAC_LEN, TS3_INIT_MAC,
};
use crate::puzzle::{solve_rsa_puzzle, MAX_RSA_PUZZLE_LEVEL, RSA_PUZZLE_LEN};

/// Fixed 16-byte dummy AES key (`"c:\\windows\\syste"`, `TsCrypt.cs:37`).
pub const DUMMY_KEY_NONCE: [u8; 16] = *b"c:\\windows\\syste";

/// Fixed 16-byte dummy AES key alias (`"c:\\windows\\syste"`, `TsCrypt.cs:37`).
pub const DUMMY_KEY: [u8; 16] = DUMMY_KEY_NONCE;

/// Fixed 16-byte dummy EAX nonce (`"m\\firewall32.cpl"`, `TsCrypt.cs:38`).
pub const DUMMY_NONCE: [u8; 16] = *b"m\\firewall32.cpl";

/// Fixed 32-byte dummy key + nonce string (`"c:\\windows\\system\\firewall32.cpl"`, `TsCrypt.cs:36`).
pub const DUMMY_KEY_AND_NONCE_FULL: [u8; 32] = *b"c:\\windows\\system\\firewall32.cpl";

/// `InitVersion` integer constant from `TsCrypt.cs:41` (`1566914096` = `0x5d6519d0`).
pub const TSLIB_INIT_VERSION: u32 = 1_566_914_096;

/// Number of packet types (`TsCrypt.cs:45`).
pub const PACKET_TYPE_KINDS: usize = 9;

/// Errors produced by [`TsCrypt`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CryptError {
    #[error("packet too short")]
    PacketTooShort,
    #[error("invalid packet type: {0}")]
    InvalidPacketType(u8),
    #[error("invalid Init1 packet (too short)")]
    Init1TooShort,
    #[error("invalid Init1 step: {0}")]
    Init1InvalidStep(u8),
    #[error("invalid Init1 packet length: {0}")]
    Init1InvalidLength(usize),
    #[error("Init1 step 1 server error code: {0}")]
    Init1ServerError(u32),
    #[error("Init1 step 1 random nonce (a0_reversed) mismatch")]
    Init1NonceMismatch,
    #[error("RSA puzzle level {0} is out of acceptable range (0..=1_000_000)")]
    InvalidPuzzleLevel(i64),
    #[error("RSA puzzle modulus n cannot be zero or exceed 64 bytes")]
    InvalidPuzzleModulus,
    #[error("missing alpha nonce from Init1 step 4")]
    MissingAlphaNonce,
    #[error("invalid base64 parameter: {0}")]
    InvalidBase64(&'static str),
    #[error("invalid beta size: {0} (expected 10 or 54)")]
    InvalidBetaSize(usize),
    #[error("invalid alpha size: {0} (expected 10)")]
    InvalidAlphaSize(usize),
    #[error("identity error: {0}")]
    Identity(#[from] IdentityError),
    #[error("invalid server P-256 public key (omega)")]
    InvalidServerPublicKey,
    #[error("server initivexpand2 signature verification failed")]
    InvalidInitProofSignature,
    #[error("license chain error: {0}")]
    License(#[from] LicenseError),
    #[error("invalid server Ed25519 ephemeral public key derived from license")]
    InvalidDerivedEd25519Key,
    #[error("EAX authentication tag verification failed")]
    AuthenticationFailed,
    #[error("invalid Init1 MAC (expected TS3INIT1)")]
    InvalidInit1Mac,
    #[error("invalid unencrypted fake signature MAC")]
    InvalidFakeSignatureMac,
}

/// Action returned after processing an incoming `Init1` S2C packet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Init1Next {
    /// Transmit the returned `Init1` C2S payload (step 0, step 2, or step 4).
    SendInit1Packet(Vec<u8>),
    /// Emitted if the caller handles `clientinitiv` separately.
    SendClientInitIv {
        alpha_b64: String,
        omega_b64: String,
        ip: String,
    },
}

/// Parameters returned by [`TsCrypt::process_initivexpand2`] to build the `clientek` command (`TsFullClient.cs:283-292`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientEkParams {
    /// Base64-encoded 32-byte client ephemeral Ed25519 public key (`ek`).
    pub ek_b64: String,
    /// Base64-encoded ECDSA-P256-SHA256 DER signature over `[ek_bytes(32B) || beta_bytes(54B)]` (`proof`).
    pub proof_b64: String,
}

/// Cached per-(direction, packet_type, generation_id) base key and nonce (`TsCrypt.cs:53, 670-688`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CachedKeyNonce {
    base_key: [u8; 16],
    base_nonce: [u8; 16],
    generation_id: u32,
}

/// TeamSpeak 3 cryptographic state machine (`TsCrypt.cs:33-702`).
#[derive(Debug, Clone)]
pub struct TsCrypt {
    identity: IdentityData,
    init_version: [u8; 4],
    last_a0: Option<[u8; 4]>,
    alpha_tmp: Option<[u8; 10]>,
    iv_struct: Option<Vec<u8>>,
    fake_signature: [u8; MAC_LEN],
    crypto_init_complete: bool,
    key_cache: [Option<CachedKeyNonce>; PACKET_TYPE_KINDS * 2],
}

impl TsCrypt {
    /// Creates a new [`TsCrypt`] state machine initialized with `identity`.
    pub fn new(identity: IdentityData) -> Self {
        Self {
            identity,
            init_version: INIT_VERSION_BYTES,
            last_a0: None,
            alpha_tmp: None,
            iv_struct: None,
            fake_signature: [0u8; MAC_LEN],
            crypto_init_complete: false,
            key_cache: [None; PACKET_TYPE_KINDS * 2],
        }
    }

    /// Resets the session cryptographic state back to pre-handshake (`TsCrypt.cs:61-67`).
    pub fn reset(&mut self) {
        self.crypto_init_complete = false;
        self.last_a0 = None;
        self.alpha_tmp = None;
        self.iv_struct = None;
        self.fake_signature = [0u8; MAC_LEN];
        self.key_cache = [None; PACKET_TYPE_KINDS * 2];
    }

    /// Overrides the 4-byte `Init1` client version header (defaults to [`INIT_VERSION_BYTES`] = `[0x06, 0x3b, 0xec, 0xe9]`).
    pub fn set_init_version(&mut self, version_bytes: [u8; 4]) {
        self.init_version = version_bytes;
    }

    /// Returns a reference to the client [`IdentityData`].
    pub fn identity(&self) -> &IdentityData {
        &self.identity
    }

    /// Returns `true` once `process_initivexpand2` or `process_initivexpand` has completed.
    pub fn crypto_init_complete(&self) -> bool {
        self.crypto_init_complete
    }

    /// Returns the 8-byte `fake_signature` (`SHA1(iv_struct)[0..8]`) used as MAC for unencrypted packets.
    pub fn fake_signature(&self) -> &[u8; MAC_LEN] {
        &self.fake_signature
    }

    /// Returns the derived `iv_struct` (20 bytes for legacy, 64 bytes for modern), if initialized.
    pub fn iv_struct(&self) -> Option<&[u8]> {
        self.iv_struct.as_deref()
    }

    /// Returns the 10-byte `alpha` nonce generated during `Init1` Step 4 (if still pending key expansion).
    pub fn alpha_nonce(&self) -> Option<&[u8; 10]> {
        self.alpha_tmp.as_ref()
    }

    /// Explicitly sets the 10-byte `alpha` nonce (useful for deterministic testing).
    pub fn set_alpha_nonce(&mut self, alpha: [u8; 10]) {
        self.alpha_tmp = Some(alpha);
    }

    /// Creates the 21-byte `Init1` Step 0 C2S packet payload (`TsCrypt.cs:399-404`):
    /// `[version(4B: 0x06,0x3b,0xec,0xe9), step=0, unix_time_be(4B), random_a0(4B), reserved(8B=0)]`.
    pub fn create_init1_step0(&mut self) -> Vec<u8> {
        let unix_now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as u32)
            .unwrap_or(0);
        let mut a0 = [0u8; 4];
        rand::rngs::OsRng.fill_bytes(&mut a0);
        self.create_init1_step0_with(unix_now, a0)
    }

    /// Deterministic helper for `create_init1_step0` with explicit `unix_time` and `random_a0`.
    pub fn create_init1_step0_with(&mut self, unix_time: u32, random_a0: [u8; 4]) -> Vec<u8> {
        self.last_a0 = Some(random_a0);
        let mut buf = Vec::with_capacity(21);
        buf.extend_from_slice(&self.init_version); // [0..4] version (4B)
        buf.push(0x00); // [4] step = 0 (1B)
        buf.extend_from_slice(&unix_time.to_be_bytes()); // [5..9] unix_time_be (4B)
        buf.extend_from_slice(&random_a0); // [9..13] random_a0 (4B)
        buf.extend_from_slice(&[0u8; 8]); // [13..21] reserved (8B = 0)
        buf
    }

    /// Processes an incoming S2C `Init1` response payload (Step 1, Step 3, or `0x7F` reset)
    /// and returns the next step action (`TsCrypt.cs:362-488`).
    pub fn process_init1_response(&mut self, payload: &[u8]) -> Result<Init1Next, CryptError> {
        if payload.is_empty() {
            return Err(CryptError::Init1TooShort);
        }

        let step = payload[0];
        match step {
            0x7F => {
                // Server requested handshake restart from Step 0 (`TsCrypt.cs:395-405`)
                Ok(Init1Next::SendInit1Packet(self.create_init1_step0()))
            }
            1 => {
                if payload.len() == 5 {
                    let err_code = u32::from_be_bytes([payload[1], payload[2], payload[3], payload[4]]);
                    return Err(CryptError::Init1ServerError(err_code));
                }
                if payload.len() != 21 {
                    return Err(CryptError::Init1InvalidLength(payload.len()));
                }

                // Validate `a0_reversed` against `self.last_a0` (`TsCrypt.cs:414`: server reverses the 4 bytes of `a0`)
                let a0_reversed = [payload[17], payload[18], payload[19], payload[20]];
                if let Some(expected_a0) = self.last_a0 {
                    let expected_rev = [expected_a0[3], expected_a0[2], expected_a0[1], expected_a0[0]];
                    if a0_reversed != expected_rev {
                        return Err(CryptError::Init1NonceMismatch);
                    }
                }

                // Build Step 2 C2S (25 bytes: `[version(4B), step=2(1B), server_stuff(16B), a0_reversed(4B)]`)
                let mut send_data = Vec::with_capacity(25);
                send_data.extend_from_slice(&self.init_version);
                send_data.push(0x02);
                send_data.extend_from_slice(&payload[1..21]);
                Ok(Init1Next::SendInit1Packet(send_data))
            }
            3 => {
                // Step 3 response (`1 + 64 + 64 + 4 + N` bytes): `[step=3, x(64B), n(64B), level(4B BE), server_data(N bytes)]`
                if payload.len() < 1 + 64 + 64 + 4 {
                    return Err(CryptError::Init1InvalidLength(payload.len()));
                }

                let x = &payload[1..65];
                let n = &payload[65..129];
                let level_i32 = i32::from_be_bytes([payload[129], payload[130], payload[131], payload[132]]);
                if level_i32 < 0 || (level_i32 as u32) > MAX_RSA_PUZZLE_LEVEL {
                    return Err(CryptError::InvalidPuzzleLevel(level_i32 as i64));
                }
                if n.iter().all(|&b| b == 0) {
                    return Err(CryptError::InvalidPuzzleModulus);
                }

                let y = solve_rsa_puzzle(x, n, level_i32 as u32);
                debug_assert_eq!(y.len(), RSA_PUZZLE_LEN);

                // Generate 10-byte random `alpha` nonce (`TsCrypt.cs:448-450`)
                let mut alpha = [0u8; 10];
                rand::rngs::OsRng.fill_bytes(&mut alpha);
                self.alpha_tmp = Some(alpha);
                let alpha_b64 = BASE64.encode(alpha);

                // Format `clientinitiv` command string (`TsCrypt.cs:451-457`)
                let omega_raw = self.identity.public_key_omega();
                let omega_escaped = escape_ts3_param(&omega_raw);
                let alpha_escaped = escape_ts3_param(&alpha_b64);
                let cmd_str = format!(
                    "clientinitiv alpha={} omega={} ot=1 ip=",
                    alpha_escaped, omega_escaped
                );
                let cmd_bytes = cmd_str.as_bytes();

                // Step 4 layout (`TsCrypt.cs:466-477`):
                // `[version(4B), step=4(1B), x(64B), n(64B), level(4B), server_data(N B), y(64B), clientinitiv_utf8]`
                // When server_data is 100B (standard TS3), prefix before cmd_bytes is 4 + 1 + 64 + 64 + 4 + 100 + 64 = 301 bytes,
                // or 301 + cmd_bytes.len() total.
                let mut send_data = Vec::with_capacity(4 + 1 + (payload.len() - 1) + 64 + cmd_bytes.len());
                send_data.extend_from_slice(&self.init_version);
                send_data.push(0x04);
                send_data.extend_from_slice(&payload[1..]); // x(64) + n(64) + level(4) + server_data(N)
                send_data.extend_from_slice(&y); // y(64)
                send_data.extend_from_slice(cmd_bytes);

                Ok(Init1Next::SendInit1Packet(send_data))
            }
            other => Err(CryptError::Init1InvalidStep(other)),
        }
    }

    /// Processes the modern `initivexpand2` server command (`TsFullClient.cs:276-300`, `TsCrypt.cs:312-360`):
    /// 1. Base64-decodes `license`, `beta` (54B), `proof`, `omega` (server P-256 DER public key).
    /// 2. Verifies `proof` (ECDSA-P256-SHA256 DER signature over `license` bytes using `omega`).
    /// 3. Derives 32-byte server ephemeral Ed25519 public key `pub_server_ed` via [`LicenseChain::parse_and_derive_key`].
    /// 4. Generates a 32-byte client ephemeral Ed25519 seed/scalar (`d_client` + compressed Edwards `ek_bytes`),
    ///    signs `[ek_bytes (32B) || beta_bytes (54B)]` with client P-256 identity ECDSA-SHA256 (`proof_b64`).
    /// 5. Computes shared secret matching `TsCrypt.cs:347-360` and sets 64-byte `iv_struct`, `fake_signature`, and `crypto_init_complete = true`.
    pub fn process_initivexpand2(
        &mut self,
        license_b64: &str,
        beta_b64: &str,
        omega_b64: &str,
        proof_b64: &str,
    ) -> Result<ClientEkParams, CryptError> {
        let mut ephemeral_seed = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut ephemeral_seed);
        self.process_initivexpand2_with_seed(license_b64, beta_b64, omega_b64, proof_b64, ephemeral_seed)
    }

    /// Deterministic helper for [`Self::process_initivexpand2`] with an explicit 32-byte ephemeral random buffer.
    pub fn process_initivexpand2_with_seed(
        &mut self,
        license_b64: &str,
        beta_b64: &str,
        omega_b64: &str,
        proof_b64: &str,
        mut ephemeral_priv: [u8; 32],
    ) -> Result<ClientEkParams, CryptError> {
        let alpha = self.alpha_tmp.ok_or(CryptError::MissingAlphaNonce)?;

        let license_bytes = BASE64
            .decode(license_b64.trim())
            .map_err(|_| CryptError::InvalidBase64("license"))?;
        let beta_bytes = BASE64
            .decode(beta_b64.trim())
            .map_err(|_| CryptError::InvalidBase64("beta"))?;
        if beta_bytes.len() != 54 {
            return Err(CryptError::InvalidBetaSize(beta_bytes.len()));
        }
        let omega_bytes = BASE64
            .decode(omega_b64.trim())
            .map_err(|_| CryptError::InvalidBase64("omega"))?;
        let proof_bytes = BASE64
            .decode(proof_b64.trim())
            .map_err(|_| CryptError::InvalidBase64("proof"))?;

        // 1 & 2. Import server P-256 public key (`omega`) and verify `proof` over `license_bytes` (`TsCrypt.cs:322-327`)
        let server_pub = parse_omega_public_key_der(&omega_bytes)
            .map_err(|_| CryptError::InvalidServerPublicKey)?;
        if !verify_asn1_der(&server_pub, &license_bytes, &proof_bytes) {
            return Err(CryptError::InvalidInitProofSignature);
        }

        // 3. Parse license chain and derive 32-byte server ephemeral Ed25519 public key (`TsCrypt.cs:330-338`)
        let pub_server_ed = LicenseChain::parse_and_derive_key(&license_bytes)?;

        // 4. Clamp client ephemeral private scalar and compute `ek_bytes` (`TsCrypt.cs:506-518`)
        // ScalarOperations.sc_clamp(privateKey):
        ephemeral_priv[0] &= 248;
        ephemeral_priv[31] &= 63;
        ephemeral_priv[31] |= 64;
        let client_scalar = Scalar::from_bytes_mod_order(ephemeral_priv);
        let ek_point = &curve25519_dalek::constants::ED25519_BASEPOINT_POINT * &client_scalar;
        let ek_bytes: [u8; 32] = ek_point.compress().to_bytes();

        // Sign `[ek_bytes (32B) || beta_bytes (54B)]` (86 bytes) with client P-256 identity (`TsFullClient.cs:286-291`)
        let mut to_sign = Vec::with_capacity(86);
        to_sign.extend_from_slice(&ek_bytes);
        to_sign.extend_from_slice(&beta_bytes);
        let client_sig_der = self.identity.sign_asn1_der(&to_sign);

        let ek_out_b64 = BASE64.encode(ek_bytes);
        let proof_out_b64 = BASE64.encode(client_sig_der);

        // 5. Compute shared secret matching `TsCrypt.cs:347-360`:
        // In `TsCrypt.cs:350-356`:
        //   privateKeyCpy[31] &= 0x7F; (already <= 127 after sc_clamp)
        //   ge_frombytes_negate_vartime(out var pub1, publicKey); -> decompresses P_server and negates X (-P_server)
        //   ge_scalarmult_vartime(out GroupElementP2 mul, privateKeyCpy, pub1); -> computes -(d_client * P_server)
        //   ge_tobytes(sharedTmp, mul);
        //   sharedTmp[31] ^= 0x80; -> flips X sign bit back, yielding compressed (d_client * P_server)!
        //   Sha512.Hash(sharedTmp, bytes);
        let shared_key = compute_shared_secret2(&pub_server_ed, &ephemeral_priv)?;
        self.set_shared_secret(&alpha, &beta_bytes, &shared_key)?;

        Ok(ClientEkParams {
            ek_b64: ek_out_b64,
            proof_b64: proof_out_b64,
        })
    }

    /// Legacy `initivexpand` key derivation (`TsCrypt.cs:255-310`):
    /// - 10-byte `alpha`, 10-byte `beta`, server P-256 DER public key `omega`.
    /// - Computes ECDH P-256 shared secret `SHA1(shared_x)` (20 bytes).
    /// - Derives 20-byte `iv_struct` XORed with `alpha` and `beta`, sets `fake_signature = SHA1(iv_struct)[0..8]`,
    ///   and sets `crypto_init_complete = true`.
    pub fn process_initivexpand(
        &mut self,
        alpha_b64: &str,
        beta_b64: &str,
        omega_b64: &str,
    ) -> Result<(), CryptError> {
        let alpha_bytes = BASE64
            .decode(alpha_b64.trim())
            .map_err(|_| CryptError::InvalidBase64("alpha"))?;
        if alpha_bytes.len() != 10 {
            return Err(CryptError::InvalidAlphaSize(alpha_bytes.len()));
        }
        let beta_bytes = BASE64
            .decode(beta_b64.trim())
            .map_err(|_| CryptError::InvalidBase64("beta"))?;
        if beta_bytes.len() != 10 {
            return Err(CryptError::InvalidBetaSize(beta_bytes.len()));
        }
        let omega_bytes = BASE64
            .decode(omega_b64.trim())
            .map_err(|_| CryptError::InvalidBase64("omega"))?;

        let server_pub = parse_omega_public_key_der(&omega_bytes)
            .map_err(|_| CryptError::InvalidServerPublicKey)?;
        let shared_key = self.identity.ecdh_shared_key_sha1(&server_pub);

        let mut alpha_arr = [0u8; 10];
        alpha_arr.copy_from_slice(&alpha_bytes);
        self.set_shared_secret(&alpha_arr, &beta_bytes, &shared_key)
    }

    /// Initializes `iv_struct`, `fake_signature`, and marks `crypto_init_complete = true` (`TsCrypt.cs:291-310`).
    fn set_shared_secret(
        &mut self,
        alpha: &[u8; 10],
        beta: &[u8],
        shared_key: &[u8],
    ) -> Result<(), CryptError> {
        if beta.len() != 10 && beta.len() != 54 {
            return Err(CryptError::InvalidBetaSize(beta.len()));
        }
        let total_len = 10 + beta.len();
        let mut iv = vec![0u8; total_len];
        for i in 0..10 {
            iv[i] = shared_key[i] ^ alpha[i];
        }
        for i in 0..beta.len() {
            iv[10 + i] = shared_key[10 + i] ^ beta[i];
        }

        let sha1_iv = Sha1::digest(&iv);
        self.fake_signature.copy_from_slice(&sha1_iv[0..8]);
        self.iv_struct = Some(iv);
        self.alpha_tmp = None;
        self.key_cache = [None; PACKET_TYPE_KINDS * 2];
        self.crypto_init_complete = true;
        Ok(())
    }

    /// Directly initializes `iv_struct` (20 or 64 bytes) for testing or resumed sessions.
    pub fn set_iv_struct_for_test(&mut self, iv: Vec<u8>) {
        let sha1_iv = Sha1::digest(&iv);
        self.fake_signature.copy_from_slice(&sha1_iv[0..8]);
        self.iv_struct = Some(iv);
        self.key_cache = [None; PACKET_TYPE_KINDS * 2];
        self.crypto_init_complete = true;
    }

    /// Derives the 16-byte AES-128 key and 16-byte EAX nonce for a packet (`TsCrypt.cs:662-700`).
    ///
    /// When `dummy` is `true` or `!self.crypto_init_complete`, returns `(DUMMY_KEY_NONCE, DUMMY_NONCE)`.
    /// Otherwise computes `SHA256([if from_server { 0x30 } else { 0x31 }, (packet_type as u8) & 0x0F, gen_be[0..4], iv_struct])`
    /// with `key[0] ^= (packet_id >> 8) as u8; key[1] ^= (packet_id & 0xff) as u8`.
    pub fn derive_key_nonce(
        &self,
        from_server: bool,
        packet_type: PacketType,
        packet_id: u16,
        generation_id: u32,
        dummy: bool,
    ) -> ([u8; 16], [u8; 16]) {
        if dummy || !self.crypto_init_complete {
            return (DUMMY_KEY_NONCE, DUMMY_NONCE);
        }

        let iv = match &self.iv_struct {
            Some(iv) => iv,
            None => return (DUMMY_KEY_NONCE, DUMMY_NONCE),
        };

        let pt_raw = (packet_type as u8) & 0x0F;
        let dir_byte = if from_server { 0x30u8 } else { 0x31u8 };

        let mut hasher = Sha256::new();
        hasher.update([dir_byte, pt_raw]);
        hasher.update(generation_id.to_be_bytes());
        hasher.update(iv);
        let digest = hasher.finalize();

        let mut key = [0u8; 16];
        let mut nonce = [0u8; 16];
        key.copy_from_slice(&digest[0..16]);
        nonce.copy_from_slice(&digest[16..32]);

        key[0] ^= (packet_id >> 8) as u8;
        key[1] ^= (packet_id & 0xFF) as u8;

        (key, nonce)
    }

    /// Cached version of [`Self::derive_key_nonce`] matching `TsCrypt.cs:670-688`.
    pub fn get_key_nonce_cached(
        &mut self,
        from_server: bool,
        packet_type: PacketType,
        packet_id: u16,
        generation_id: u32,
        dummy: bool,
    ) -> ([u8; 16], [u8; 16]) {
        if dummy || !self.crypto_init_complete {
            return (DUMMY_KEY_NONCE, DUMMY_NONCE);
        }

        let pt_raw = ((packet_type as u8) & 0x0F) as usize;
        let cache_idx = pt_raw * if from_server { 1 } else { 2 };

        let (base_key, base_nonce) = match self.key_cache.get(cache_idx).copied().flatten() {
            Some(entry) if entry.generation_id == generation_id => (entry.base_key, entry.base_nonce),
            _ => {
                let iv = match &self.iv_struct {
                    Some(iv) => iv,
                    None => return (DUMMY_KEY_NONCE, DUMMY_NONCE),
                };
                let dir_byte = if from_server { 0x30u8 } else { 0x31u8 };
                let mut hasher = Sha256::new();
                hasher.update([dir_byte, pt_raw as u8]);
                hasher.update(generation_id.to_be_bytes());
                hasher.update(iv);
                let digest = hasher.finalize();

                let mut bk = [0u8; 16];
                let mut bn = [0u8; 16];
                bk.copy_from_slice(&digest[0..16]);
                bn.copy_from_slice(&digest[16..32]);

                if let Some(slot) = self.key_cache.get_mut(cache_idx) {
                    *slot = Some(CachedKeyNonce {
                        base_key: bk,
                        base_nonce: bn,
                        generation_id,
                    });
                }
                (bk, bn)
            }
        };

        let mut key = base_key;
        key[0] ^= (packet_id >> 8) as u8;
        key[1] ^= (packet_id & 0xFF) as u8;
        (key, base_nonce)
    }

    /// Encrypts an outgoing [`C2SPacket`] in place (`TsCrypt.cs:524-571`).
    ///
    /// - `PacketType::Init1`: sets `pkt.mac = *b"TS3INIT1"`, leaves `pkt.payload` unencrypted.
    /// - `pkt.flags.contains(PacketFlags::UNENCRYPTED)` (`0x80` flag): sets `pkt.mac = self.fake_signature`, leaves `pkt.payload` unencrypted.
    /// - Otherwise encrypts `pkt.payload` in place using `Eax<Aes128, U8>` with the 5-byte C2S header as AAD
    ///   and writes the 8-byte authentication tag into `pkt.mac`.
    pub fn encrypt_c2s(&self, pkt: &mut C2SPacket, force_dummy: bool) -> Result<(), CryptError> {
        if pkt.packet_type == PacketType::Init1 {
            pkt.mac = *TS3_INIT_MAC;
            return Ok(());
        }
        if pkt.flags.contains(PacketFlags::UNENCRYPTED) {
            pkt.mac = self.fake_signature;
            return Ok(());
        }

        let dummy = force_dummy || !self.crypto_init_complete;
        let (key, nonce) = self.derive_key_nonce(
            false,
            pkt.packet_type,
            pkt.packet_id,
            pkt.generation_id,
            dummy,
        );
        let header = pkt.header_aad();

        let cipher = Eax::<Aes128, U8>::new((&key).into());
        let tag = cipher
            .encrypt_in_place_detached((&nonce).into(), &header, &mut pkt.payload)
            .map_err(|_| CryptError::AuthenticationFailed)?;
        pkt.mac.copy_from_slice(tag.as_slice());
        Ok(())
    }

    /// Decrypts an incoming [`S2CPacket`] in place (`TsCrypt.cs:585-654`).
    ///
    /// - `PacketType::Init1`: verifies `pkt.mac == *b"TS3INIT1"`.
    /// - `pkt.flags.contains(PacketFlags::UNENCRYPTED)` (`0x80` flag): verifies `pkt.mac == self.fake_signature`.
    /// - Otherwise attempts `Eax<Aes128, U8>` decryption using the active key (`!self.crypto_init_complete`);
    ///   if that fails and `pkt.packet_type == PacketType::Ack && pkt.packet_id <= 2`, retries with the dummy key
    ///   (`TsCrypt.cs:597-614`).
    pub fn decrypt_s2c(&self, pkt: &mut S2CPacket) -> Result<(), CryptError> {
        if pkt.packet_type == PacketType::Init1 {
            if !constant_time_eq_8(&pkt.mac, TS3_INIT_MAC) {
                return Err(CryptError::InvalidInit1Mac);
            }
            return Ok(());
        }
        if pkt.flags.contains(PacketFlags::UNENCRYPTED) {
            if !constant_time_eq_8(&pkt.mac, &self.fake_signature) {
                return Err(CryptError::InvalidFakeSignatureMac);
            }
            return Ok(());
        }

        let primary_dummy = !self.crypto_init_complete;
        if self.try_decrypt_s2c_data(pkt, primary_dummy).is_ok() {
            return Ok(());
        }

        // Shady ACK workaround (`TsCrypt.cs:606-610`):
        // When `clientek` (dummy-encrypted, id 1) and `clientinit` (session-encrypted, id 2) are sent back-to-back,
        // the server's ACK for `clientek` or early commands can arrive after `crypto_init_complete` is already true.
        if !primary_dummy
            && (pkt.packet_type == PacketType::Ack || pkt.packet_type == PacketType::Command)
            && pkt.packet_id <= 2
        {
            return self.try_decrypt_s2c_data(pkt, true);
        }

        Err(CryptError::AuthenticationFailed)
    }

    fn try_decrypt_s2c_data(&self, pkt: &mut S2CPacket, dummy: bool) -> Result<(), CryptError> {
        let (key, nonce) = self.derive_key_nonce(
            true,
            pkt.packet_type,
            pkt.packet_id,
            pkt.generation_id,
            dummy,
        );
        let header = pkt.header_aad();
        let cipher = Eax::<Aes128, U8>::new((&key).into());
        let tag = Tag::<Eax<Aes128, U8>>::from_slice(&pkt.mac);

        // Decrypt in a temporary copy first so `pkt.payload` is untouched if we need the ACK<=2 retry!
        let mut buf = pkt.payload.clone();
        cipher
            .decrypt_in_place_detached((&nonce).into(), &header, &mut buf, tag)
            .map_err(|_| CryptError::AuthenticationFailed)?;
        pkt.payload = buf;
        Ok(())
    }

    /// Encrypts an S2C packet (useful for testing [`Self::decrypt_s2c`]).
    pub fn encrypt_s2c_for_test(&self, pkt: &mut S2CPacket, force_dummy: bool) -> Result<(), CryptError> {
        if pkt.packet_type == PacketType::Init1 {
            pkt.mac = *TS3_INIT_MAC;
            return Ok(());
        }
        if pkt.flags.contains(PacketFlags::UNENCRYPTED) {
            pkt.mac = self.fake_signature;
            return Ok(());
        }

        let dummy = force_dummy || !self.crypto_init_complete;
        let (key, nonce) = self.derive_key_nonce(
            true,
            pkt.packet_type,
            pkt.packet_id,
            pkt.generation_id,
            dummy,
        );
        let header = pkt.header_aad();
        let cipher = Eax::<Aes128, U8>::new((&key).into());
        let tag = cipher
            .encrypt_in_place_detached((&nonce).into(), &header, &mut pkt.payload)
            .map_err(|_| CryptError::AuthenticationFailed)?;
        pkt.mac.copy_from_slice(tag.as_slice());
        Ok(())
    }

    /// Decrypts a C2S packet (useful for testing [`Self::encrypt_c2s`]).
    pub fn decrypt_c2s_for_test(&self, pkt: &mut C2SPacket, force_dummy: bool) -> Result<(), CryptError> {
        if pkt.packet_type == PacketType::Init1 {
            if !constant_time_eq_8(&pkt.mac, TS3_INIT_MAC) {
                return Err(CryptError::InvalidInit1Mac);
            }
            return Ok(());
        }
        if pkt.flags.contains(PacketFlags::UNENCRYPTED) {
            if !constant_time_eq_8(&pkt.mac, &self.fake_signature) {
                return Err(CryptError::InvalidFakeSignatureMac);
            }
            return Ok(());
        }

        let dummy = force_dummy || !self.crypto_init_complete;
        let (key, nonce) = self.derive_key_nonce(
            false,
            pkt.packet_type,
            pkt.packet_id,
            pkt.generation_id,
            dummy,
        );
        let header = pkt.header_aad();
        let cipher = Eax::<Aes128, U8>::new((&key).into());
        let tag = Tag::<Eax<Aes128, U8>>::from_slice(&pkt.mac);
        cipher
            .decrypt_in_place_detached((&nonce).into(), &header, &mut pkt.payload, tag)
            .map_err(|_| CryptError::AuthenticationFailed)
    }
}

/// Computes the 64-byte `SHA512` shared secret matching `TsCrypt.GetSharedSecret2` (`TsCrypt.cs:347-360`).
///
/// Exact TSLib behavior:
/// 1. `privateKeyCpy[31] &= 0x7F`
/// 2. `ge_frombytes_negate_vartime(out var pub1, publicKey)` -> decompresses Edwards point `P_pub` and negates X (`-P_pub`).
/// 3. `ge_scalarmult_vartime(out GroupElementP2 mul, privateKeyCpy, pub1)` -> computes `(-P_pub) * scalar = -(P_pub * scalar)`.
/// 4. `ge_tobytes(sharedTmp, mul)` -> compresses `-(P_pub * scalar)`.
/// 5. `sharedTmp[31] ^= 0x80` -> flips the X sign bit, which negates the point back to `P_pub * scalar`!
///    (Note: if the X coordinate of `P_pub * scalar` is 0, `ge_tobytes` sets bit 31 to 0, whereas `sharedTmp[31] ^= 0x80`
///    unconditionally flips bit 31; doing `(-P_pub * scalar)` followed by `^= 0x80` reproduces `TsCrypt.cs:352-356` with 100% bit-identity).
/// 6. `Sha512.Hash(sharedTmp, bytes)`.
pub fn compute_shared_secret2(
    public_key: &[u8; 32],
    private_key: &[u8; 32],
) -> Result<[u8; 64], CryptError> {
    let mut priv_cpy = *private_key;
    priv_cpy[31] &= 0x7F;
    let scalar = Scalar::from_bytes_mod_order(priv_cpy);

    let pub_point: EdwardsPoint = CompressedEdwardsY(*public_key)
        .decompress()
        .ok_or(CryptError::InvalidDerivedEd25519Key)?;

    let neg_pub = -pub_point;
    let mul_point = neg_pub * scalar;
    let mut shared_tmp = mul_point.compress().to_bytes();
    shared_tmp[31] ^= 0x80;

    let digest = Sha512::digest(shared_tmp);
    let mut out = [0u8; 64];
    out.copy_from_slice(&digest);
    Ok(out)
}

#[inline]
fn constant_time_eq_8(a: &[u8; 8], b: &[u8; 8]) -> bool {
    let mut diff = 0u8;
    for i in 0..8 {
        diff |= a[i] ^ b[i];
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::encode_omega_public_key;
    use crate::license::LICENSE_ROOT_KEY;
    use p256::ecdsa::signature::Verifier;
    use p256::ecdsa::{Signature as P256Signature, VerifyingKey as P256VerifyingKey};
    use p256::SecretKey;

    #[test]
    fn test_dummy_encryption_vector_matches_spec() {
        // Test vector from `docs/re/03-crypto-handshake.md` Section 7.2:
        // Key: "c:\windows\syste", Nonce: "m\firewall32.cpl"
        // Header (AD): 00 01 00 00 02 (PacketId = 1, ClientId = 0, PacketType = 2 / Command, Flags = 0)
        // Plaintext: "clientinit" (63 6c 69 65 6e 74 69 6e 69 74)
        // Expected Ciphertext: 9d 74 8b 45 aa 7b ef b9 9e fe
        // Expected MAC Tag (8 B): 94 43 eb 56 88 bd 38 28
        let ident = IdentityData::generate();
        let crypt = TsCrypt::new(ident);

        let mut pkt = C2SPacket::new(
            1,
            0,
            PacketType::Command,
            PacketFlags::NONE,
            b"clientinit".to_vec(),
        );
        crypt.encrypt_c2s(&mut pkt, true).unwrap();

        assert_eq!(
            pkt.mac,
            [0x94, 0x43, 0xeb, 0x56, 0x88, 0xbd, 0x38, 0x28]
        );
        assert_eq!(
            pkt.payload,
            vec![0x9d, 0x74, 0x8b, 0x45, 0xaa, 0x7b, 0xef, 0xb9, 0x9e, 0xfe]
        );
        assert_eq!(
            hex::encode(pkt.encode()),
            "9443eb5688bd382800010000029d748b45aa7befb99efe"
        );

        // Roundtrip decrypt
        crypt.decrypt_c2s_for_test(&mut pkt, true).unwrap();
        assert_eq!(pkt.payload, b"clientinit");
    }

    #[test]
    fn test_per_packet_key_derivation_vector_matches_spec() {
        // Test vector from `docs/re/03-crypto-handshake.md` Section 7.3:
        // ivStruct: 64 bytes (0x01, 0x02, ..., 0x40)
        // Direction: C2S (0x31), PacketType: 0x02 (Command), generationId: 0, PacketId: 0x1234
        let ident = IdentityData::generate();
        let mut crypt = TsCrypt::new(ident);
        let iv: Vec<u8> = (1u8..=0x40).collect();
        crypt.set_iv_struct_for_test(iv);

        let (key, nonce) = crypt.derive_key_nonce(false, PacketType::Command, 0x1234, 0, false);
        assert_eq!(
            hex::encode(key),
            "5dbd2be87263b96f2d920c9492d1cf3c"
        );
        assert_eq!(
            hex::encode(nonce),
            "9892e522e013b5cbbf40187e8f93ab07"
        );

        // Also verify cached method returns identical output
        let (key_c, nonce_c) =
            crypt.get_key_nonce_cached(false, PacketType::Command, 0x1234, 0, false);
        assert_eq!(key, key_c);
        assert_eq!(nonce, nonce_c);
    }

    #[test]
    fn test_ack_packet_id_le_2_dummy_fallback_and_unencrypted_fake_signature() {
        let ident = IdentityData::generate();
        let mut crypt = TsCrypt::new(ident);
        crypt.set_iv_struct_for_test(vec![0x55; 64]);

        // 1. Create an ACK with packet_id = 1 encrypted with the DUMMY key (simulating server ACK for clientek)
        let mut ack1 = S2CPacket::new(
            1,
            PacketType::Ack,
            PacketFlags::NONE,
            1u16.to_be_bytes().to_vec(),
        );
        crypt.encrypt_s2c_for_test(&mut ack1, true).unwrap();

        // Even though `crypto_init_complete == true`, `decrypt_s2c` falls back to dummy key for Ack with packet_id <= 2
        crypt.decrypt_s2c(&mut ack1).unwrap();
        assert_eq!(ack1.payload, 1u16.to_be_bytes());

        // 2. ACK with packet_id = 3 encrypted with dummy key MUST fail when crypto_init_complete == true
        let mut ack3 = S2CPacket::new(
            3,
            PacketType::Ack,
            PacketFlags::NONE,
            3u16.to_be_bytes().to_vec(),
        );
        crypt.encrypt_s2c_for_test(&mut ack3, true).unwrap();
        assert_eq!(
            crypt.decrypt_s2c(&mut ack3),
            Err(CryptError::AuthenticationFailed)
        );

        // 3. Unencrypted Voice / Ping packet uses `fake_signature`
        let mut ping_c2s = C2SPacket::new(
            10,
            1,
            PacketType::Ping,
            PacketFlags::UNENCRYPTED,
            vec![],
        );
        crypt.encrypt_c2s(&mut ping_c2s, false).unwrap();
        assert_eq!(&ping_c2s.mac, crypt.fake_signature());

        let mut voice_s2c = S2CPacket::new(
            7,
            PacketType::Voice,
            PacketFlags::UNENCRYPTED,
            vec![0x00, 0x07, 0x04, 0xde, 0xad, 0xbe, 0xef],
        );
        crypt.encrypt_s2c_for_test(&mut voice_s2c, false).unwrap();
        assert_eq!(&voice_s2c.mac, crypt.fake_signature());
        crypt.decrypt_s2c(&mut voice_s2c).unwrap();
        assert_eq!(voice_s2c.payload, vec![0x00, 0x07, 0x04, 0xde, 0xad, 0xbe, 0xef]);

        // Tampering with unencrypted MAC fails verification
        voice_s2c.mac[0] ^= 0xFF;
        assert_eq!(
            crypt.decrypt_s2c(&mut voice_s2c),
            Err(CryptError::InvalidFakeSignatureMac)
        );
    }

    #[test]
    fn test_init1_steps_0_1_2_3_4_transitions() {
        let ident = IdentityData::generate();
        let mut crypt = TsCrypt::new(ident);

        // Step 0: 21 bytes `[version(4B: 0x06,0x3b,0xec,0xe9), step=0, unix_time_be(4B), random_a0(4B), reserved(8B=0)]`
        let step0 = crypt.create_init1_step0_with(0x11223344, [0xAA, 0xBB, 0xCC, 0xDD]);
        assert_eq!(step0.len(), 21);
        assert_eq!(&step0[0..4], &[0x06, 0x3b, 0xec, 0xe9]);
        assert_eq!(step0[4], 0x00);
        assert_eq!(&step0[5..9], &[0x11, 0x22, 0x33, 0x44]);
        assert_eq!(&step0[9..13], &[0xAA, 0xBB, 0xCC, 0xDD]);
        assert_eq!(&step0[13..21], &[0u8; 8]);

        // Step 1 S2C (21B): `[step=1, server_stuff(16B), a0_reversed(4B)]`
        let mut step1_s2c = vec![0x01];
        step1_s2c.extend_from_slice(&[0x42u8; 16]); // server_stuff
        step1_s2c.extend_from_slice(&[0xDD, 0xCC, 0xBB, 0xAA]); // a0_reversed
        assert_eq!(step1_s2c.len(), 21);

        let step2_res = crypt.process_init1_response(&step1_s2c).unwrap();
        let step2 = match step2_res {
            Init1Next::SendInit1Packet(pkt) => pkt,
            _ => panic!("expected SendInit1Packet for step 2"),
        };
        assert_eq!(step2.len(), 25);
        assert_eq!(&step2[0..4], &[0x06, 0x3b, 0xec, 0xe9]);
        assert_eq!(step2[4], 0x02);
        assert_eq!(&step2[5..25], &step1_s2c[1..21]);

        // Verify wrong `a0_reversed` is rejected
        let mut bad_step1 = step1_s2c.clone();
        bad_step1[20] ^= 0x01;
        assert_eq!(
            crypt.process_init1_response(&bad_step1),
            Err(CryptError::Init1NonceMismatch)
        );

        // Step 3 S2C (233B): `[step=3, x(64B), n(64B), level(4B BE), server_data(100B)]`
        // Let x = 2, n = 15, level = 3 -> y = 2^(2^3) mod 15 = 256 mod 15 = 1
        let mut step3_s2c = vec![0x03];
        let mut x = [0u8; 64];
        x[63] = 2;
        let mut n = [0u8; 64];
        n[63] = 15;
        step3_s2c.extend_from_slice(&x);
        step3_s2c.extend_from_slice(&n);
        step3_s2c.extend_from_slice(&3i32.to_be_bytes());
        step3_s2c.extend_from_slice(&[0x77u8; 100]);
        assert_eq!(step3_s2c.len(), 233);

        let step4_res = crypt.process_init1_response(&step3_s2c).unwrap();
        let step4 = match step4_res {
            Init1Next::SendInit1Packet(pkt) => pkt,
            _ => panic!("expected SendInit1Packet for step 4"),
        };
        assert!(step4.len() > 301);
        assert_eq!(&step4[0..4], &[0x06, 0x3b, 0xec, 0xe9]);
        assert_eq!(step4[4], 0x04);
        assert_eq!(&step4[5..237], &step3_s2c[1..233]);
        // y at [237..301] must be 1 (padded to 64B)
        let mut expected_y = [0u8; 64];
        expected_y[63] = 1;
        assert_eq!(&step4[237..301], &expected_y);
        let cmd_tail = std::str::from_utf8(&step4[301..]).unwrap();
        assert!(cmd_tail.starts_with("clientinitiv alpha="));
        assert!(cmd_tail.contains(" omega="));
        assert!(cmd_tail.ends_with(" ot=1 ip="));
        assert!(crypt.alpha_nonce().is_some());
    }

    #[test]
    fn test_process_initivexpand2_and_legacy_initivexpand() {
        // 1. Test `process_initivexpand2` end-to-end
        let client_ident = IdentityData::generate();
        let server_ident = IdentityData::generate();
        let mut crypt = TsCrypt::new(client_ident.clone());

        let alpha = [0x10, 0x20, 0x30, 0x40, 0x50, 0x60, 0x70, 0x80, 0x90, 0xA0];
        crypt.set_alpha_nonce(alpha);

        // Build a valid 1-block Ephemeral license chain (1 + 42 = 43 bytes)
        let base = curve25519_dalek::constants::ED25519_BASEPOINT_POINT;
        let block_key = (&base * &Scalar::from(9u64)).compress().to_bytes();
        let mut license_bytes = Vec::with_capacity(43);
        license_bytes.push(0x01); // version 1
        license_bytes.push(0x00); // keyKind = 0
        license_bytes.extend_from_slice(&block_key);
        license_bytes.push(32); // Ephemeral block type
        license_bytes.extend_from_slice(&100u32.to_be_bytes());
        license_bytes.extend_from_slice(&200u32.to_be_bytes());

        // Sign `license_bytes` with `server_ident` P-256 key
        let proof_bytes = server_ident.sign_asn1_der(&license_bytes);
        let beta_bytes = [0x33u8; 54];

        let license_b64 = BASE64.encode(&license_bytes);
        let beta_b64 = BASE64.encode(beta_bytes);
        let omega_b64 = server_ident.public_key_omega();
        let proof_b64 = BASE64.encode(&proof_bytes);

        let seed = [0x07u8; 32];
        let ek_params = crypt
            .process_initivexpand2_with_seed(&license_b64, &beta_b64, &omega_b64, &proof_b64, seed)
            .expect("process_initivexpand2 should succeed");

        assert!(crypt.crypto_init_complete());
        assert_eq!(crypt.iv_struct().unwrap().len(), 64);

        // Verify the client's `proof_b64` signature over `[ek_bytes || beta_bytes]` using `client_ident`'s public key
        let ek_raw = BASE64.decode(&ek_params.ek_b64).unwrap();
        assert_eq!(ek_raw.len(), 32);
        let client_proof_raw = BASE64.decode(&ek_params.proof_b64).unwrap();
        let mut signed_msg = Vec::with_capacity(86);
        signed_msg.extend_from_slice(&ek_raw);
        signed_msg.extend_from_slice(&beta_bytes);
        let client_vk = P256VerifyingKey::from(&client_ident.public_key());
        let client_sig = P256Signature::from_der(&client_proof_raw).unwrap();
        client_vk
            .verify(&signed_msg, &client_sig)
            .expect("clientek proof signature must verify");

        // Verify tampered server proof is rejected
        let mut bad_proof = proof_bytes.clone();
        let last_idx = bad_proof.len() - 1;
        bad_proof[last_idx] ^= 0x01;
        let mut crypt_bad = TsCrypt::new(client_ident.clone());
        crypt_bad.set_alpha_nonce(alpha);
        assert_eq!(
            crypt_bad.process_initivexpand2(
                &license_b64,
                &beta_b64,
                &omega_b64,
                &BASE64.encode(&bad_proof),
            ),
            Err(CryptError::InvalidInitProofSignature)
        );

        // 2. Test legacy `process_initivexpand` (P-256 ECDH, 10B alpha, 10B beta -> 20B iv_struct)
        let mut crypt_legacy = TsCrypt::new(client_ident.clone());
        let beta_legacy = [0xCCu8; 10];
        crypt_legacy
            .process_initivexpand(
                &BASE64.encode(alpha),
                &BASE64.encode(beta_legacy),
                &omega_b64,
            )
            .expect("legacy process_initivexpand should succeed");
        assert!(crypt_legacy.crypto_init_complete());
        assert_eq!(crypt_legacy.iv_struct().unwrap().len(), 20);

        // Verify symmetric ECDH: server with `server_ident` and `client_ident.public_key_omega()` derives the exact same iv_struct!
        let mut server_crypt_legacy = TsCrypt::new(server_ident);
        server_crypt_legacy
            .process_initivexpand(
                &BASE64.encode(alpha),
                &BASE64.encode(beta_legacy),
                &client_ident.public_key_omega(),
            )
            .unwrap();
        assert_eq!(crypt_legacy.iv_struct(), server_crypt_legacy.iv_struct());
        assert_eq!(crypt_legacy.fake_signature(), server_crypt_legacy.fake_signature());
    }

    #[test]
    fn test_p256_public_key_der_import_export_matches_spec_vector() {
        let d_hex = "a58edd1d35f86cc86e2826b9277cf55b22f230bbe72535818bcc13ec026dba10";
        let d_bytes = hex::decode(d_hex).unwrap();
        let secret_key = SecretKey::from_slice(&d_bytes).unwrap();
        let ident = IdentityData::from_secret_key(secret_key, 64, 64);

        assert_eq!(
            encode_omega_public_key(&ident.public_key()),
            "MEwDAgcAAgEgAiAUf4CTtPnlwtDyloQqGh4ZsvJqlev5vvndefK2WSzreAIhANEY4CVrn5QI85hTMRftD3hPQnvY1XDfkMiyhPw6UGIz"
        );
        assert_eq!(ident.uid(), "tUXzo5QsCNhqcRnaGCA5t3IM3HE=");

        let imported =
            parse_omega_public_key_der(&BASE64.decode(ident.public_key_omega()).unwrap()).unwrap();
        assert_eq!(imported, ident.public_key());
    }

    #[test]
    fn test_shared_secret2_matches_tslib() {
        let mut priv_key = [0x42u8; 32];
        priv_key[0] &= 248;
        priv_key[31] &= 63;
        priv_key[31] |= 64;

        let shared = compute_shared_secret2(&LICENSE_ROOT_KEY, &priv_key).unwrap();
        assert_eq!(shared.len(), 64);

        // Verify direct scalar mult on Edwards point matches
        let root_pt = CompressedEdwardsY(LICENSE_ROOT_KEY).decompress().unwrap();
        let scalar = Scalar::from_bytes_mod_order(priv_key);
        let direct_bytes = (root_pt * scalar).compress().to_bytes();
        let expected_hash: [u8; 64] = Sha512::digest(direct_bytes).into();
        assert_eq!(shared, expected_hash);
    }
}
