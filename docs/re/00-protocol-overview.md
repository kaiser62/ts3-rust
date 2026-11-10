STATUS: DONE
RESULT: Detailed analysis of TeamSpeak 3 UDP client protocol (port 9987) covering handshake sequences, packet layout, cryptography, and command serialization with byte offsets, field sizes, and Rust implementation examples.

### 1. Handshake Packet Structure and Sequence

TeamSpeak 3 has two handshake architectures: **Old Crypt** (pre-3.1) and **New Crypt** (3.1+ UDP `Init1` handshake). Modern servers require the `Init1` protocol.

#### Handshake Sequence Flow
```
Client                                     Server
  |                                          |
  |--- Init1 Step 0 (Cookie & Timestamp) --->|
  |<-- Init1 Step 1 (Server Cookie Echo) ----|
  |--- Init1 Step 2 (Cookie ACK) ----------->|
  |<-- Init1 Step 3 (RSA Puzzle Challenge) --|
  |--- Init1 Step 4 (Puzzle Sol + clientinitiv) ->|
  |                                          |
  |<-- initivexpand2 (License + Beta + Omega) | [or initivexpand in fallback]
  |--- clientek (Ephemeral Curve25519 PubKey)->| [PacketType::Command, dummy encrypted]
  |--- clientinit (Nick, HWID, Version, etc)->| [PacketType::Command, session encrypted]
  |<-- initserver (Assigned ClientID, etc) ---| [PacketType::Command, session encrypted]
  |--- channelsubscribeall ------------------>|
```

#### Init1 Packet Stages (`Init1` / Type `0x08`, Header Flag `0x80` Unencrypted, MAC = `"TS3INIT1"`)
- **Step 0 (C -> S)**: 21 bytes payload [[TsCrypt.cs:399-404](file:///D:/Dev/TS3AudioBot/TSLib/Full/TsCrypt.cs#L399-L404)]
  - `[0..4]` (4B): Client protocol build timestamp / version (e.g. `1566914096` = `0x5D651030` for TS 3.5.0) [[TsCrypt.cs:41](file:///D:/Dev/TS3AudioBot/TSLib/Full/TsCrypt.cs#L41)].
  - `[4]` (1B): Step identifier = `0x00`.
  - `[5..9]` (4B): Unix timestamp (UInt32 Big-Endian).
  - `[9..13]` (4B): Random integer / client cookie (UInt32 Big-Endian).
  - `[13..21]` (8B): Zero padding.
- **Step 1 (S -> C)**: 21 bytes payload [[TsCrypt.cs:417-433](file:///D:/Dev/TS3AudioBot/TSLib/Full/TsCrypt.cs#L417-L433)]
  - `[0]` (1B): Step identifier = `0x01` (or 5B total if error: `0x01` + 4B Big-Endian error code).
  - `[1..17]` (16B): Server cookie / random bytes.
  - `[17..21]` (4B): Client cookie echoed back (from Step 0 `[9..13]`).
- **Step 2 (C -> S)**: 25 bytes payload [[TsCrypt.cs:421-425](file:///D:/Dev/TS3AudioBot/TSLib/Full/TsCrypt.cs#L421-L425)]
  - `[0..4]` (4B): Client protocol version (UInt32 Big-Endian).
  - `[4]` (1B): Step identifier = `0x02`.
  - `[5..25]` (20B): Server Step 1 payload bytes (`data[1..21]`) echoed verbatim.
- **Step 3 (S -> C) - The Proof-of-Work / RSA Puzzle Challenge**: 233 bytes payload [[TsCrypt.cs:445-464](file:///D:/Dev/TS3AudioBot/TSLib/Full/TsCrypt.cs#L445-L464)]
  - `[0]` (1B): Step identifier = `0x03`.
  - `[1..65]` (64B): `x` - 512-bit unsigned Big-Endian integer (base).
  - `[65..129]` (64B): `n` - 512-bit unsigned Big-Endian integer (modulus).
  - `[129..133]` (4B): `level` - Int32 Big-Endian (number of squarings).
  - `[133..233]` (100B): Server state / verification context bytes.
  - **Puzzle Calculation**: Solves $y = x^{2^{\text{level}}} \pmod n$ [[TsCrypt.cs:495-504](file:///D:/Dev/TS3AudioBot/TSLib/Full/TsCrypt.cs#L495-L504)].
    In Rust/BigInt: start with $acc = x$; perform $acc = (acc \times acc) \pmod n$ for `level` iterations.
- **Step 4 (C -> S)**: $301 + \text{len(clientinitiv)}$ bytes payload [[TsCrypt.cs:466-478](file:///D:/Dev/TS3AudioBot/TSLib/Full/TsCrypt.cs#L466-L478)]
  - `[0..4]` (4B): Client protocol version (UInt32 Big-Endian).
  - `[4]` (1B): Step identifier = `0x04`.
  - `[5..237]` (232B): Direct copy of Step 3 payload (`x`, `n`, `level`, `data`).
  - `[237..301]` (64B): $y$ - Puzzle solution (512-bit unsigned Big-Endian integer).
  - `[301..]` ($N$ bytes): UTF-8 command string `clientinitiv`:
    `clientinitiv alpha=<b64(10 random bytes)> omega=<b64(client_asn1_pubkey)> ot=1 ip=`

#### Post-Init1 Exchange: New Crypt vs Old Crypt
- **New Crypt (`initivexpand2`)** [[TsFullClient.cs:276-302](file:///D:/Dev/TS3AudioBot/TSLib/Full/TsFullClient.cs#L276-L302)]:
  - Server sends `initivexpand2` containing: `beta` (54B base64), `l` (license chain), `omega` (server ECDSA public key), `proof` (ECDSA signature over `l` with `omega`).
  - Client verifies `proof` on `l` with `omega`.
  - Client generates ephemeral Curve25519 keypair: `ek_pub` (32B), `ek_priv` (32B).
  - Client signs `ek_pub (32B) || beta (54B)` (86B total) using its P-256 identity key (ECDSA SHA-256).
  - Client sends `clientek ek=<b64(ek_pub)> proof=<b64(signature)>` as `PacketType::Command` (encrypted with dummy key).
  - Client traverses the license chain to derive the server's public Curve25519 key [[License.cs:55-61](file:///D:/Dev/TS3AudioBot/TSLib/Full/License.cs#L55-L61)].
  - Client computes shared Curve25519 ECDH secret, hashes with SHA-512, and produces 64-byte `ivStruct`:
    - `ivStruct[0..10] = shared[0..10] ^ alpha[0..10]`
    - `ivStruct[10..64] = shared[10..64] ^ beta[0..54]`
- **Old Crypt (`initivexpand` / pre-3.1)** [[TsFullClient.cs:259-274](file:///D:/Dev/TS3AudioBot/TSLib/Full/TsFullClient.cs#L259-L274)]:
  - Server sends `initivexpand` containing `alpha`, `beta` (10B), and `omega` (server P-256 key).
  - ECDH between client P-256 private key and server P-256 public key produces shared X coordinate; `sharedKey = SHA1(X)`.
  - Produces 20-byte `ivStruct`:
    - `ivStruct[0..10] = sharedKey[0..10] ^ alpha[0..10]`
    - `ivStruct[10..20] = sharedKey[10..20] ^ beta[0..10]`
- **Connection Completion (`clientinit` & `initserver`)**:
  - Client sends `clientinit` (command packet encrypted with real session keys derived from `ivStruct`).
  - Server sends `initserver` containing `client_id` (the client's assigned ID on the server) [[TsFullClient.cs:304-316](file:///D:/Dev/TS3AudioBot/TSLib/Full/TsFullClient.cs#L304-L316)].
  - Client saves `client_id` for all future C2S packet headers and transitions to `Connected`.

---

### 2. Packet Header Layout

UDP wire format: `[ MAC (8 bytes) ] [ Header (5 bytes C2S / 3 bytes S2C) ] [ Payload (Data) ]` [[Packet.cs:20-21](file:///D:/Dev/TS3AudioBot/TSLib/Full/Packet.cs#L20-L21), [Packet.cs:83](file:///D:/Dev/TS3AudioBot/TSLib/Full/Packet.cs#L83)].

#### Client-to-Server (C2S) Header Layout (13 bytes total) [[Packet.cs:126-132](file:///D:/Dev/TS3AudioBot/TSLib/Full/Packet.cs#L126-L132), [Packet.cs:182-187](file:///D:/Dev/TS3AudioBot/TSLib/Full/Packet.cs#L182-L187)]
| Byte Offset | Size | Field Name | Description |
|:---|:---|:---|:---|
| `0..8` | 8 bytes | `MAC` | AES-128-EAX tag (or `"TS3INIT1"` / `fakeSignature`) |
| `8..10` | 2 bytes | `PacketId` | Sequential 16-bit packet ID (Big-Endian) |
| `10..12` | 2 bytes | `ClientId` | Assigned by server (`0` prior to `initserver`) (Big-Endian) |
| `12` | 1 byte | `PacketTypeFlagged` | Flags in high 4 bits (`0xF0`), Type in low 4 bits (`0x0F`) |
| `13..` | $N$ bytes | `Payload` | Ciphertext or plaintext payload |

#### Server-to-Client (S2C) Header Layout (11 bytes total) [[Packet.cs:120-124](file:///D:/Dev/TS3AudioBot/TSLib/Full/Packet.cs#L120-L124), [Packet.cs:189-192](file:///D:/Dev/TS3AudioBot/TSLib/Full/Packet.cs#L189-L192)]
| Byte Offset | Size | Field Name | Description |
|:---|:---|:---|:---|
| `0..8` | 8 bytes | `MAC` | AES-128-EAX tag (or `"TS3INIT1"` / `fakeSignature`) |
| `8..10` | 2 bytes | `PacketId` | Sequential 16-bit packet ID (Big-Endian) |
| `10` | 1 byte | `PacketTypeFlagged` | Flags in high 4 bits (`0xF0`), Type in low 4 bits (`0x0F`) |
| `11..` | $N$ bytes | `Payload` | Ciphertext or plaintext payload |

#### Packet Types (`type & 0x0F`) [[PacketType.cs:14-25](file:///D:/Dev/TS3AudioBot/TSLib/Full/PacketType.cs#L14-L25)]
- `0x0`: `Voice`
- `0x1`: `VoiceWhisper`
- `0x2`: `Command`
- `0x3`: `CommandLow`
- `0x4`: `Ping`
- `0x5`: `Pong`
- `0x6`: `Ack`
- `0x7`: `AckLow`
- `0x8`: `Init1`

#### Packet Flags (`flags & 0xF0`) [[PacketType.cs:27-35](file:///D:/Dev/TS3AudioBot/TSLib/Full/PacketType.cs#L27-L35)]
- `0x10`: `Fragmented` (packet split across MTU)
- `0x20`: `Newprotocol` (used on Commands)
- `0x40`: `Compressed` (payload compressed using QuickLZ level 1)
- `0x80`: `Unencrypted` (uses `fakeSignature` MAC)

#### Sequence Counters & Generation ID
- Each packet type maintains an independent 16-bit counter [[PacketHandler.cs:79-80](file:///D:/Dev/TS3AudioBot/TSLib/Full/PacketHandler.cs#L79-L80)].
- For `Init1` packets, `PacketId` is fixed to `101` (`0x0065`) [[PacketHandler.cs:320-322](file:///D:/Dev/TS3AudioBot/TSLib/Full/PacketHandler.cs#L320-L322)].
- When `PacketId` rolls over from `65535` to `0`, `GenerationId` increments by `1` [[PacketHandler.cs:324-329](file:///D:/Dev/TS3AudioBot/TSLib/Full/PacketHandler.cs#L324-L329)].
- Max UDP packet size: 500 bytes (header + payload) [[PacketHandler.cs:797](file:///D:/Dev/TS3AudioBot/TSLib/Full/PacketHandler.cs#L797)]. Max C2S content per fragment: 487 bytes [[PacketHandler.cs:26](file:///D:/Dev/TS3AudioBot/TSLib/Full/PacketHandler.cs#L26)].

---

### 3. Cryptographic Algorithms

#### Symmetric Encryption: AES-128-EAX [[TsCrypt.cs:42](file:///D:/Dev/TS3AudioBot/TSLib/Full/TsCrypt.cs#L42)]
- **Cipher**: AES-128 in EAX mode (not CCM).
- **MAC / Tag size**: 8 bytes (64 bits) [[TsCrypt.cs:44](file:///D:/Dev/TS3AudioBot/TSLib/Full/TsCrypt.cs#L44)].
- **Associated Data (AD)**: The unencrypted header (5 bytes for C2S, 3 bytes for S2C) [[TsCrypt.cs:539](file:///D:/Dev/TS3AudioBot/TSLib/Full/TsCrypt.cs#L539), [TsCrypt.cs:623](file:///D:/Dev/TS3AudioBot/TSLib/Full/TsCrypt.cs#L623)].
- **Pre-session Dummy Key/Nonce**:
  `DummyKey = ASCII("c:\\windows\\system")` (16 bytes) [[TsCrypt.cs:36-39](file:///D:/Dev/TS3AudioBot/TSLib/Full/TsCrypt.cs#L36-L39)]
  `DummyIv  = ASCII("\\firewall32.cpl")` (16 bytes)

#### Per-Packet Key and Nonce Derivation (TomCrypt-style) [[TsCrypt.cs:662-700](file:///D:/Dev/TS3AudioBot/TSLib/Full/TsCrypt.cs#L662-L700)]
For each packet direction, type, and generation, derive base material:
1. Construct hash input buffer (26 bytes for 20B `ivStruct`, 70 bytes for 64B `ivStruct`):
   - `[0]`: `0x31` (`'1'`) for C2S; `0x30` (`'0'`) for S2C.
   - `[1]`: `packetType` (`0x00..0x08`).
   - `[2..6]`: `generationId` (UInt32 Big-Endian).
   - `[6..]`: `ivStruct` (20 or 64 bytes agreed upon in handshake).
2. Hash: `digest = SHA256(buffer)`.
3. Split: `base_key = digest[0..16]`, `base_nonce = digest[16..32]`.
4. Per-packet XOR tweak:
   - `key = base_key.clone()`
   - `key[0] ^= (packet_id >> 8) as u8`
   - `key[1] ^= (packet_id & 0xFF) as u8`
   - `nonce = base_nonce.clone()` (unchanged).

#### Public Key & Identity System
- **Curve**: NIST P-256 (`secp256r1` / `prime256v1`) [[TsCrypt.cs:122](file:///D:/Dev/TS3AudioBot/TSLib/Full/TsCrypt.cs#L122)].
- **Export Format**: LibTomCrypt DER ASN.1 sequence [[TsCrypt.cs:166-174](file:///D:/Dev/TS3AudioBot/TSLib/Full/TsCrypt.cs#L166-L174)]:
  `SEQUENCE { BIT STRING (length 7, 0b0000_0000), INTEGER(32), INTEGER(X), INTEGER(Y) }`
- **Client UID**: `Base64(SHA1(PublicKeyBase64))` [[TsCrypt.cs:196-200](file:///D:/Dev/TS3AudioBot/TSLib/Full/TsCrypt.cs#L196-L200)].
- **Identity Proof-of-Work (`client_key_offset`)**:
  Calculated by finding integer $k$ such that $\text{SHA1}(\text{PublicKeyBase64} \mathbin{\Vert} \text{to\_string}(k))$ has $N$ leading zero bits (e.g. security level 8 requires 8 leading 0 bits) [[TsCrypt.cs:862-892](file:///D:/Dev/TS3AudioBot/TSLib/Full/TsCrypt.cs#L862-L892)].

#### License Chain & Curve25519 Derivation [[License.cs:20-61](file:///D:/Dev/TS3AudioBot/TSLib/Full/License.cs#L20-L61), [License.cs:147-164](file:///D:/Dev/TS3AudioBot/TSLib/Full/License.cs#L147-L164)]
- **License Root Key**: 32-byte constant starting `cd 0d e2 ae ... 40 8a`.
- **Key Derivation per block**:
  $P_{\text{new}} = (P_{\text{pub}} \times \text{clamp}(\text{SHA512}(\text{block})[0..32])) + P_{\text{parent}}$ on Edwards curve / Ed25519.
- **Session ECDH**: `sharedKey = SHA512(ge_scalarmult(ek_priv, derived_server_pub))` [[TsCrypt.cs:347-360](file:///D:/Dev/TS3AudioBot/TSLib/Full/TsCrypt.cs#L347-L360)].

---

### 4. Command Structure and Escaping Mechanism

#### Command Format [[TsCommand.cs:89-148](file:///D:/Dev/TS3AudioBot/TSLib/Commands/TsCommand.cs#L89-L148)]
- Syntax: `<command_name> <key1>=<val1> <key2>=<val2> ...`
- Multiple items/rows are delimited by `|`:
  `notifyclientleftview cfid=1 ctid=2 reasonid=3 reasonmsg=bye clid=5|clid=6`

#### Escaping Rules [[TsString.cs:25-45](file:///D:/Dev/TS3AudioBot/TSLib/Commands/TsString.cs#L25-L45), [TsString.cs:85-100](file:///D:/Dev/TS3AudioBot/TSLib/Commands/TsString.cs#L85-L100)]
| Character | Escaped Sequence | Description |
|:---|:---|:---|
| `\` | `\\` | Backslash |
| `/` | `\/` | Forward Slash |
| ` ` (space) | `\s` | Whitespace |
| `\|` | `\p` | Pipe delimiter |
| `\a` | `\a` | Bell / Alert |
| `\b` | `\b` | Backspace |
| `\f` | `\f` | Formfeed |
| `\n` | `\n` | Newline |
| `\r` | `\r` | Carriage Return |
| `\t` | `\t` | Horizontal Tab |
| `\v` | `\v` | Vertical Tab |

---

### 5. Rust Implementation Examples

#### A. C2S Packet Layout & Parser
```rust
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PacketType {
    Voice = 0x0,
    VoiceWhisper = 0x1,
    Command = 0x2,
    CommandLow = 0x3,
    Ping = 0x4,
    Pong = 0x5,
    Ack = 0x6,
    AckLow = 0x7,
    Init1 = 0x8,
}

pub const FLAG_FRAGMENTED: u8 = 0x10;
pub const FLAG_NEWPROTOCOL: u8 = 0x20;
pub const FLAG_COMPRESSED: u8 = 0x40;
pub const FLAG_UNENCRYPTED: u8 = 0x80;

pub struct C2SPacketHeader {
    pub mac: [u8; 8],
    pub packet_id: u16,
    pub client_id: u16,
    pub flags: u8,
    pub packet_type: PacketType,
}

impl C2SPacketHeader {
    pub const SIZE: usize = 13;

    pub fn serialize_into(&self, buf: &mut [u8; 13]) {
        buf[0..8].copy_from_slice(&self.mac);
        buf[8..10].copy_from_slice(&self.packet_id.to_be_bytes());
        buf[10..12].copy_from_slice(&self.client_id.to_be_bytes());
        buf[12] = (self.flags & 0xF0) | (self.packet_type as u8 & 0x0F);
    }

    pub fn header_ad(&self) -> [u8; 5] {
        let mut ad = [0u8; 5];
        ad[0..2].copy_from_slice(&self.packet_id.to_be_bytes());
        ad[2..4].copy_from_slice(&self.client_id.to_be_bytes());
        ad[4] = (self.flags & 0xF0) | (self.packet_type as u8 & 0x0F);
        ad
    }
}
```

#### B. Per-Packet AES-128-EAX Key & Nonce Derivation
```rust
use sha2::{Digest, Sha256};

pub fn derive_key_nonce(
    is_s2c: bool,
    packet_type: u8,
    generation_id: u32,
    iv_struct: &[u8],
    packet_id: u16,
) -> ([u8; 16], [u8; 16]) {
    let mut hasher = Sha256::new();
    hasher.update([if is_s2c { 0x30 } else { 0x31 }, packet_type]);
    hasher.update(generation_id.to_be_bytes());
    hasher.update(iv_struct);
    let digest = hasher.finalize();

    let mut key = [0u8; 16];
    let mut nonce = [0u8; 16];
    key.copy_from_slice(&digest[0..16]);
    nonce.copy_from_slice(&digest[16..32]);

    // XOR tweak with packet_id
    key[0] ^= (packet_id >> 8) as u8;
    key[1] ^= (packet_id & 0xFF) as u8;

    (key, nonce)
}
```

#### C. RSA Puzzle Solver (Init1 Step 3 -> 4)
```rust
use num_bigint::BigUint;

pub fn solve_ts3_puzzle(x_bytes: &[u8; 64], n_bytes: &[u8; 64], level: i32) -> [u8; 64] {
    let mut acc = BigUint::from_bytes_be(x_bytes);
    let modulus = BigUint::from_bytes_be(n_bytes);

    for _ in 0..level {
        acc = (&acc * &acc) % &modulus;
    }

    let bytes = acc.to_bytes_be();
    let mut out = [0u8; 64];
    out[64 - bytes.len()..].copy_from_slice(&bytes);
    out
}
```

#### D. Command Escaping & Unescaping
```rust
pub fn ts_escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '/' => out.push_str("\\/"),
            ' ' => out.push_str("\\s"),
            '|' => out.push_str("\\p"),
            '\x0c' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\x0b' => out.push_str("\\v"),
            _ => out.push(c),
        }
    }
    out
}

pub fn ts_unescape(input: &str) -> Result<String, &'static str> {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next().ok_or("unexpected end of escape")? {
                '\\' => out.push('\\'),
                '/' => out.push('/'),
                's' => out.push(' '),
                'p' => out.push('|'),
                'f' => out.push('\x0c'),
                'n' => out.push('\n'),
                'r' => out.push('\r'),
                't' => out.push('\t'),
                'v' => out.push('\x0b'),
                _ => return Err("invalid escape sequence"),
            }
        } else {
            out.push(c);
        }
    }
    Ok(out)
}
```

FILES:
D:/Dev/TS3AudioBot/TSLib/Full/Packet.cs:18
D:/Dev/TS3AudioBot/TSLib/Full/Packet.cs:182
D:/Dev/TS3AudioBot/TSLib/Full/Packet.cs:189
D:/Dev/TS3AudioBot/TSLib/Full/PacketType.cs:14
D:/Dev/TS3AudioBot/TSLib/Full/PacketType.cs:27
D:/Dev/TS3AudioBot/TSLib/Full/PacketHandler.cs:26
D:/Dev/TS3AudioBot/TSLib/Full/PacketHandler.cs:85
D:/Dev/TS3AudioBot/TSLib/Full/PacketHandler.cs:319
D:/Dev/TS3AudioBot/TSLib/Full/PacketHandler.cs:797
D:/Dev/TS3AudioBot/TSLib/Full/TsCrypt.cs:36
D:/Dev/TS3AudioBot/TSLib/Full/TsCrypt.cs:40
D:/Dev/TS3AudioBot/TSLib/Full/TsCrypt.cs:42
D:/Dev/TS3AudioBot/TSLib/Full/TsCrypt.cs:312
D:/Dev/TS3AudioBot/TSLib/Full/TsCrypt.cs:363
D:/Dev/TS3AudioBot/TSLib/Full/TsCrypt.cs:495
D:/Dev/TS3AudioBot/TSLib/Full/TsCrypt.cs:537
D:/Dev/TS3AudioBot/TSLib/Full/TsCrypt.cs:662
D:/Dev/TS3AudioBot/TSLib/Full/TsCrypt.cs:862
D:/Dev/TS3AudioBot/TSLib/Full/TsFullClient.cs:259
D:/Dev/TS3AudioBot/TSLib/Full/TsFullClient.cs:276
D:/Dev/TS3AudioBot/TSLib/Full/TsFullClient.cs:304
D:/Dev/TS3AudioBot/TSLib/Full/TsFullClient.cs:539
D:/Dev/TS3AudioBot/TSLib/Full/License.cs:20
D:/Dev/TS3AudioBot/TSLib/Full/License.cs:55
D:/Dev/TS3AudioBot/TSLib/Full/License.cs:147
D:/Dev/TS3AudioBot/TSLib/Commands/TsCommand.cs:89
D:/Dev/TS3AudioBot/TSLib/Commands/TsString.cs:25
D:/Dev/TS3AudioBot/TSLib/Commands/TsString.cs:85
D:/Dev/TS3AudioBot/TSLib/Generated/Messages.cs:6611
D:/Dev/TS3AudioBot/TSLib/Generated/Messages.cs:6652
D:/Dev/TS3AudioBot/TSLib/Generated/Messages.cs:6702

---

### 6. Server Database Schema, Default Permissions, and Virtualserver Properties

- Database Schema:
  - Architecture: Relational core with Entity-Attribute-Value (EAV) tables for extensibility (.scratch/teamspeak3-server_linux_amd64/sql/create_sqlite/create_tables.sql:1-298).
  - Core Entities: `servers` (id, port, autostart, machine_id, bandwidth stats; lines 161-170), `channels` (hierarchy via channel_parent_id, lines 10-15), `clients` (unique_id, nickname, login credentials, traffic stats, lines 29-44).
  - EAV Property Tables: `instance_properties` (lines 229-235), `server_properties` (lines 152-157), `channel_properties` (lines 1-6), `client_properties` (lines 19-24), `custom_fields` (lines 240-246).
  - Group Mappings: `groups_server` (lines 60-66), `groups_channel` (lines 50-56), `group_server_to_client` (lines 70-75), `group_channel_to_client` (channel_id + client_id mapping, lines 80-85).
  - 5-Tier Permission Tables: `perm_server_group` (lines 135-144), `perm_client` (lines 124-133), `perm_channel` (lines 91-100), `perm_channel_groups` (lines 113-122), `perm_channel_clients` (lines 102-111). Schema: (server_id, id1, id2, perm_id, perm_value, perm_negated, perm_skip).
  - Auxiliary Tables: `bindings` (lines 146-150), `tokens` (privilege keys, lines 174-184), `messages` (lines 186-196), `complains` (lines 200-207), `bans` (lines 210-226), `revocations` (lines 250-255), `integrations` & `integration_actions` (lines 257-274), `temporary_passwords` (lines 277-288), `api_keys` (lines 290-298).
- Default Permissions & Evaluation:
  - 5-Tier Resolution: Tier 1 (Server Groups) -> Tier 2 (Client) -> Tier 3 (Channel) -> Tier 4 (Channel Groups) -> Tier 5 (Channel-Client) (.scratch/teamspeak3-server_linux_amd64/doc/permissiondoc.txt:85-89). Higher tiers override lower tiers unless Tier 1/2 has `perm_skip=1` (permissiondoc.txt:129-133); `perm_negated=1` enforces lowest value in Tier 1 (permissiondoc.txt:116-120).
  - Types: Boolean (`b_*`), Integer (`i_*`), Power vs Needed-Power pairs (permissiondoc.txt:20-55).
  - Default Server Groups: ID 1 'Guest Server Query' (type 2), ID 2 'Admin Server Query' (type 2), ID 3 'Server Admin' (type 0), ID 4 'Normal' (type 0), ID 5 'Guest' (type 0) (.scratch/teamspeak3-server_linux_amd64/sql/defaults.sql:18-23).
  - Default Channel Groups: ID 1 'Channel Admin', ID 2 'Operator', ID 3 'Voice', ID 4 'Guest' (defaults.sql:26-30).
  - Instance Template Pointers: guest_query=1, admin_server_group=3, default_server_group=5, channel_admin_group=1, channel_default_group=4, permissions_version=24 (defaults.sql:2-9).
  - Initial Permissions: Guest Query gets login & select (defaults.sql:38-40); Admin Query / Server Admin get administrative powers & modification grants (defaults.sql:55-1017); Channel Admin/Operator get channel management & talk powers (defaults.sql:1033-1085).
- Virtualserver Properties:
  - Storage: Fixed columns in `servers` (create_tables.sql:161-170) plus dynamic rows in `server_properties` (create_tables.sql:152-157).
  - Initial Settings: virtualserver_name='TeamSpeak ]I[ Server', virtualserver_welcomemessage, virtualserver_maxclients=32 (defaults.sql:12-15).
  - Categories (.scratch/teamspeak3-server_linux_amd64/doc/serverquery/serverquery.html:2479-3070):
    - Identity & Net: `VIRTUALSERVER_NAME`, `_WELCOMEMESSAGE`, `_PORT`, `_IP`, `_UNIQUE_IDENTIFIER`, `_ID`, `_MACHINE_ID`, `_AUTOSTART` (html:2490-2520, 3037).
    - Capacity & Group Defaults: `VIRTUALSERVER_MAXCLIENTS`, `_RESERVED_SLOTS`, `_PASSWORD`, `_FLAG_PASSWORD`, `_DEFAULT_SERVER_GROUP`, `_DEFAULT_CHANNEL_GROUP`, `_DEFAULT_CHANNEL_ADMIN_GROUP` (html:2502-2575, 3012).
    - Status & Metrics: `VIRTUALSERVER_STATUS`, `_CLIENTSONLINE`, `_QUERYCLIENTSONLINE`, `_CHANNELSONLINE`, `_UPTIME`, `_CREATED`, `_TOTAL_PING`, `_TOTAL_PACKETLOSS_*` (html:2506-2545, 2980-3030).
    - Traffic & Files: `VIRTUALSERVER_MAX_DOWNLOAD_TOTAL_BANDWIDTH`, `_MAX_UPLOAD_TOTAL_BANDWIDTH`, `_FILEBASE` (html:2585-2605, 3065).
    - UI / Branding: `VIRTUALSERVER_HOSTBANNER_*` (url, gfx_url, interval, mode), `_HOSTBUTTON_*` (tooltip, url, gfx_url), `_HOSTMESSAGE_*` (html:2550-2570, 2700-2720).
    - Security & Policy: `VIRTUALSERVER_NEEDED_IDENTITY_SECURITY_LEVEL`, `_ANTIFLOOD_POINTS_*` (tick_reduce, needed_command_block, needed_ip_block), `_COMPLAIN_AUTOBAN_*`, `_CODEC_ENCRYPTION_MODE`, `_MIN_CLIENT_VERSION`, `_WEBLIST_ENABLED` (html:2720-2780, 2990-3050).
    - Diagnostics / Logging: `VIRTUALSERVER_LOG_*` (client, query, channel, permissions, server, filetransfer) (html:2985-3005).
FILES:
D:/Dev/ts3-rust/.scratch/teamspeak3-server_linux_amd64/doc/permissiondoc.txt:85
D:/Dev/ts3-rust/.scratch/teamspeak3-server_linux_amd64/doc/serverquery/serverquery.html:2479
D:/Dev/ts3-rust/.scratch/teamspeak3-server_linux_amd64/sql/create_sqlite/create_tables.sql:1
D:/Dev/ts3-rust/.scratch/teamspeak3-server_linux_amd64/sql/defaults.sql:1
