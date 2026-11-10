# TeamSpeak 3 Protocol Specification: Cryptography & Handshake

**Document ID:** `docs/re/03-crypto-handshake.md`  
**Target:** Native Rust TeamSpeak 3 Client Implementation  
**Status:** Reverse-Engineered / Verified Specification  
**Reference Codebase:** TSLib (`mx/TSLib/Full/TsCrypt.cs`, `IdentityData.cs`, `License.cs`, `TsFullClient.cs`, `PacketHandler.cs`, `Packet.cs`, `PacketType.cs`)  
**Reference Binary:** TeamSpeak 3 Server `3.13.9` Linux amd64 (`ts3server` ELF, build timestamp `1674480572` / 2023-01-23)

---

## 1. Architectural Overview & Cryptographic States

TeamSpeak 3 client communication transitions across three distinct cryptographic regimes:

```
[Disconnected]
      │
      ▼
[Phase 1: Low-Level Handshake (TS3INIT1)]
  • Transport: UDP, PacketType 0x08 (Init1)
  • MAC: Fixed 8-byte ASCII "TS3INIT1"
  • Encryption: None (Plaintext payload)
  • Steps: 0 (C2S) -> 1 (S2C) -> 2 (C2S) -> 3 (S2C) -> 4 (C2S)
  • Challenge: RSA time-lock puzzle y = x^(2^level) mod n
      │
      ▼
[Phase 2: Key Exchange & Pre-Session (Dummy Crypto)]
  • Transport: UDP Command packets (PacketType 0x02)
  • Cipher: AES-128-EAX with 8-byte MAC tag
  • Key / Nonce: Fixed ASCII "c:\windows\system\firewall32.cpl"
      - Key  (16 B): "c:\windows\syste"
      - Nonce(16 B): "m\firewall32.cpl"
  • Commands: clientinitiv (in Step 4) -> initivexpand / initivexpand2 -> clientek
  • Key derivation: ECDH (P-256 or Ed25519) + License chain validation -> ivStruct (20 or 64 B)
      │
      ▼
[Phase 3: Symmetric Session Encryption]
  • Cipher: AES-128-EAX with 8-byte MAC tag
  • Per-Packet Key/Nonce: Derived via SHA-256(dir || type || generation || ivStruct)
  • Packet ID Masking: Key[0..2] ^= PacketId (Big Endian)
  • Exception: Unencrypted flag (0x80) packets use fakeSignature = SHA-1(ivStruct)[0..8]
```

### Protocol Constants Summary

| Constant Name | Type | Value (Hex / ASCII) | Source Citation |
|---|---|---|---|
| `InitVersion` | `u32` (BE) | `1566914096` (`0x5D6519D0`) | `TsCrypt.cs:41` (TS 3.5.0 build time) |
| `Ts3InitMac` | `[u8; 8]` | `54 53 33 49 4e 49 54 31` (`"TS3INIT1"`) | `TsCrypt.cs:40`, `ts3server:0x9d91a0` |
| `DummyKeyAndNonce` | `&str` | `"c:\windows\system\firewall32.cpl"` (32 B) | `TsCrypt.cs:36`, `ts3server:0x7b28a4` |
| `DummyKey` | `[u8; 16]` | `63 3a 5c 77 69 6e 64 6f 77 73 5c 73 79 73 74 65` (`"c:\windows\syste"`) | `TsCrypt.cs:37` |
| `DummyIv` | `[u8; 16]` | `6d 5c 66 69 72 65 77 61 6c 6c 33 32 2e 63 70 6c` (`"m\firewall32.cpl"`) | `TsCrypt.cs:38` |
| `TsIdentityObfuscationKey` | `[u8; 100]` | `"b9dfaa7bee6ac57ac7b65f1094a1c155e747327bc2fe5d51c512023fe54a280201004e90ad1daaae1075d53b7d571c30e063b5a62a4a017bb394833aa0983e6e"` | `TsCrypt.cs:210` |
| `LicenseRootKey` | `[u8; 32]` | `cd 0d e2 ae d4 63 45 50 9a 7e 3c fd 8f 68 b3 dc 75 55 b2 9d cc ec 73 cd 18 75 0f 99 38 12 40 8a` | `License.cs:20-24`, `ts3server:0x808200` |
| `TsVersionSignPublicKey` | `[u8; 32]` | `52 b3 75 8d 7d 1d 04 4d 6f ba 54 cd 2c 2a 18 c2 b5 69 7c 84 f2 a3 e3 41 ba af ed c1 b7 fd 84 bc` | `TsCrypt.cs:771`, `ts3server:0x7c7591` |

---

## 2. Identity: Curve P-256, DER Formats, UID, Hashcash, Obfuscation

TeamSpeak 3 client identities are built on standard **NIST P-256** (also known as `secp256r1` or `prime256v1`, OID `1.2.840.10045.3.1.7`). The public key is an uncompressed elliptic curve point $(x, y)$, and the private key is a 256-bit scalar integer $d$.

### 2.1 LibTomCrypt ASN.1 DER Export Formats

TeamSpeak 3 utilizes custom ASN.1 DER structures designed by LibTomCrypt (`TsCrypt.cs:124-195`).

#### A. Public Key Export (DER & Base64)
- **Source:** `TsCrypt.cs:166-174`
- **Structure:**
  ```asn1
  Sequence {
      bitInfo    BIT STRING (7 unused bits, value 0b0000_0000 = 0x00)
      curveSize  INTEGER (32)
      xCoord     INTEGER (Affine X coordinate, big-endian)
      yCoord     INTEGER (Affine Y coordinate, big-endian)
  }
  ```
- **Byte Encoding:**
  - `0x30 [seq_len]` (SEQUENCE)
  - `0x03 0x02 0x07 0x00` (BIT STRING: length 2, 7 unused bits, byte `0x00`)
  - `0x02 0x01 0x20` (INTEGER 32)
  - `0x02 [x_len] [x_bytes]` (INTEGER X, unsigned big-endian, leading 0x00 prepended if high bit set)
  - `0x02 [y_len] [y_bytes]` (INTEGER Y, unsigned big-endian, leading 0x00 prepended if high bit set)
- Base64 encoding of this DER sequence produces `PublicKeyString`.

#### B. Private Key Only Export (TSLib format)
- **Source:** `TsCrypt.cs:176-184`
- **Structure:**
  ```asn1
  Sequence {
      bitInfo    BIT STRING (6 unused bits, value 0b1100_0000 = 0xC0)
      curveSize  INTEGER (32)
      dScalar    INTEGER (Private key scalar d, big-endian)
  }
  ```
- **Byte Encoding:**
  - `0x30 [seq_len]`
  - `0x03 0x02 0x06 0xc0` (BIT STRING: length 2, 6 unused bits, byte `0xC0`)
  - `0x02 0x01 0x20` (INTEGER 32)
  - `0x02 [d_len] [d_bytes]` (INTEGER d)

#### C. Full Export (Public + Private Key - LibTomCrypt Native)
- **Source:** `TsCrypt.cs:186-194`
- **Structure:**
  ```asn1
  Sequence {
      bitInfo    BIT STRING (7 unused bits, value 0b1000_0000 = 0x80)
      curveSize  INTEGER (32)
      xCoord     INTEGER (Affine X)
      yCoord     INTEGER (Affine Y)
      dScalar    INTEGER (Private key scalar d)
  }
  ```
- **Byte Encoding:**
  - `0x30 [seq_len]`
  - `0x03 0x02 0x07 0x80` (BIT STRING: length 2, 7 unused bits, byte `0x80`)
  - `0x02 0x01 0x20` (INTEGER 32)
  - `0x02 [x_len] [x_bytes]`
  - `0x02 [y_len] [y_bytes]`
  - `0x02 [d_len] [d_bytes]`

### 2.2 Client UID Derivation
- **Source:** `TsCrypt.cs:196-201`, `IdentityData.cs:43`
- **Formula:**
  $$\text{UID} = \text{Base64}\Big(\text{SHA-1}\big(\text{ASCII}(\text{PublicKeyString})\big)\Big)$$
- **Output:** 28-character base64 string ending with `'='` (20 raw bytes).

### 2.3 Hashcash Security Level Computation
TeamSpeak 3 uses a Proof-of-Work (PoW) hashcash algorithm to mitigate identity spoofing and spam (`TsCrypt.cs:812-892`).

1. **Input:**
   - `PublicKeyString`: Base64 string of the DER public key.
   - `offset`: A 64-bit unsigned integer (`u64`) formatted as an ASCII decimal string (`"0"`, `"1"`, ...).
2. **Buffer:**
   $$\text{Buffer} = \text{ASCII}(\text{PublicKeyString}) \mathbin{\Vert} \text{ASCII}(\text{DecStr}(\text{offset}))$$
3. **Hash:**
   $$H = \text{SHA-1}(\text{Buffer}) \quad (\text{20 bytes})$$
4. **Leading Zero Bit Count:**
   - Scan bytes $H[0], H[1], \dots, H[19]$:
     - If $H[i] == 0$, add `8` to counter.
     - If $H[i] \neq 0$, test bits **LSB-first** (`bit = 0..7`):
       ```c
       for (int bit = 0; bit < 8; bit++) {
           if ((H[i] & (1 << bit)) == 0) count++;
           else break;
       }
       ```
   > [!IMPORTANT]
   > TeamSpeak 3 counts zero bits within the non-zero byte starting from the **Least Significant Bit (bit 0 / 0x01)** up to the Most Significant Bit (bit 7 / 0x80), due to LibTomCrypt/LibTomMath integer bit-testing conventions (`TsCrypt.cs:888-891`).

### 2.4 Identity Key Obfuscation Format (TS3 Client Ini Format)
In standard TeamSpeak 3 client configuration files (`settings.db` or identity export files), identities are stored in an obfuscated format matching the regex `^(?<level>\d+)V(?<identity>[\w\/\+]+={0,2})$` (`TsCrypt.cs:209-245`).

#### Deobfuscation Algorithm
1. Split string at delimiter `'V'` into `level_str` and `b64_payload`.
2. `ident = Base64Decode(b64_payload)` (must be $\ge 20$ bytes).
3. Find first null byte (`0x00`) in `ident[20..]`. If no null byte exists, `hash_len = ident.len - 20`, else `hash_len = index_of_null`.
4. Compute $h = \text{SHA-1}(\text{ident}[20 \dots 20 + \text{hash\_len}])$.
5. XOR first 20 bytes: $\text{ident}[0 \dots 20] \mathbin{\oplus}= h[0 \dots 20]$.
6. XOR up to first 100 bytes with `TsIdentityObfuscationKey` (100-byte ASCII key):
   $$\text{ident}[0 \dots \min(100, \text{len})] \mathbin{\oplus}= \text{TsIdentityObfuscationKey}[0 \dots \min(100, \text{len})]$$
7. `ident` now contains ASCII base64 characters of the LibTomCrypt DER key. Base64-decode `ident` to obtain the raw ASN.1 DER key.

#### Obfuscation Algorithm
1. Start with `ident_ascii = ASCII(Base64(DerKey))`.
2. XOR first $\min(100, \text{len})$ bytes with `TsIdentityObfuscationKey`.
3. Compute $h = \text{SHA-1}(\text{ident\_ascii}[20 \dots])$.
4. XOR first 20 bytes with $h$: $\text{ident\_ascii}[0 \dots 20] \mathbin{\oplus}= h[0 \dots 20]$.
5. Base64-encode `ident_ascii` to produce `b64_payload`.
6. Output formatted string: `"{level}V{b64_payload}"`.

---

## 3. Low-Level Initialization Handshake (TS3INIT1, Steps 0..4)

The initial connection handshake operates over UDP using `PacketType.Init1` (opcode `0x08`). Packets bypass symmetric encryption and use a fixed 8-byte ASCII MAC tag `"TS3INIT1"` (`0x54 0x53 0x33 0x49 0x4e 0x49 0x54 0x31`).

### Packet Framing
- **Client to Server (C2S) Header (5 bytes):**
  - Offset `0..2` (`u16` BE): `PacketId` (starts at `0x0000`)
  - Offset `2..4` (`u16` BE): `ClientId` (`0x0000` during init)
  - Offset `4` (`u8`): `PacketTypeFlagged = 0x08` (`PacketType.Init1`, no flags)
- **Server to Client (S2C) Header (3 bytes):**
  - Offset `0..2` (`u16` BE): `PacketId`
  - Offset `2` (`u8`): `PacketTypeFlagged = 0x08`
- **Wire layout:**
  `[MAC: 8 bytes ("TS3INIT1")] || [Header: 5 or 3 bytes] || [Init1 Data: N bytes]`

```
Client                                           Server
  │                                                │
  │─── Step 0: [21 B] InitVersion + Timestamp ────>│
  │<── Step 1: [21 B] 16 B Cookie + 4 B Echo ──────│
  │─── Step 2: [25 B] InitVersion + Echo 20 B ────>│
  │<── Step 3: [233 B] RSA Challenge (x, n, lvl) ──│
  │─── Step 4: [301+N B] Echo + Solution y + IV ──>│
  │                                                │
  ▼                                                ▼
```

### Detailed Handshake Steps (`TsCrypt.cs:362-488`)

#### Step 0: Client Init Request (C2S)
- **Trigger:** Initiated by client on connect (`PacketHandler.cs:96`).
- **Data Size:** Exactly `21` bytes.
- **Payload Layout:**
  | Offset | Size | Type | Field Description |
  |---|---|---|---|
  | `0x00` | 4 B | `u32` (BE) | `InitVersion`: Client release timestamp (e.g. `1566914096` / `0x5D6519D0`) |
  | `0x04` | 1 B | `u8` | `initType = 0x00` |
  | `0x05` | 4 B | `u32` (BE) | Current Unix timestamp (`Tools.UnixNow`) |
  | `0x09` | 4 B | `u32` (BE) | Client random challenge integer (`Tools.Random.Next()`) |
  | `0x0D` | 8 B | `[u8; 8]` | Zero padding (`0x00`) |

#### Step 1: Server Challenge Response (S2C)
- **Data Size:** `21` bytes (or `5` bytes on server error).
- **Normal Payload Layout (21 B):**
  | Offset | Size | Type | Field Description |
  |---|---|---|---|
  | `0x00` | 1 B | `u8` | `initType = 0x01` |
  | `0x01` | 16 B | `[u8; 16]` | Server random cookie / session bytes |
  | `0x11` | 4 B | `[u8; 4]` | Server state / echo |
- **Error Payload Layout (5 B):**
  | Offset | Size | Type | Field Description |
  |---|---|---|---|
  | `0x00` | 1 B | `u8` | `initType = 0x01` |
  | `0x01` | 4 B | `u32` (BE) | `TsErrorCode` (error number, `TsCrypt.cs:427`) |

#### Step 2: Client Echo (C2S)
- **Data Size:** Exactly `25` bytes.
- **Payload Layout:**
  | Offset | Size | Type | Field Description |
  |---|---|---|---|
  | `0x00` | 4 B | `u32` (BE) | `InitVersion` (`1566914096`) |
  | `0x04` | 1 B | `u8` | `initType = 0x02` |
  | `0x05` | 20 B | `[u8; 20]` | Direct echo of Server Step 1 payload (`Step1_data[1..21]`) |

#### Step 3: Server RSA Puzzle Challenge (S2C)
- **Data Size:** Exactly `233` bytes.
- **Payload Layout:**
  | Offset | Size | Type | Field Description |
  |---|---|---|---|
  | `0x00` | 1 B | `u8` | `initType = 0x03` |
  | `0x01` | 64 B | `[u8; 64]` | Base $x$ (512-bit unsigned big-endian integer) |
  | `0x41` | 64 B | `[u8; 64]` | Modulus $n$ (512-bit unsigned big-endian integer) |
  | `0x81` | 4 B | `i32` (BE) | Puzzle difficulty `level` ($0 \le \text{level} \le 1{,}000{,}000$) |
  | `0x85` | 100 B | `[u8; 100]`| Server random state/context bytes |

#### Step 4: Client Puzzle Solution & clientinitiv (C2S)
- **Calculation:** Solve time-lock puzzle:
  $$y = x^{\left(2^{\text{level}}\right)} \pmod n$$
  Implemented via repeated modular squaring:
  $$y_0 = x, \quad y_{k+1} = (y_k)^2 \pmod n \quad \text{for } k = 0, \dots, \text{level}-1$$
- **Ephemeral Alpha Generation:** Client generates 10 secure random bytes `alphaTmp = [u8; 10]`, base64 encoded into string `alpha`.
- **Command Construction:**
  Client constructs plaintext command string:
  ```
  clientinitiv alpha={alpha} omega={PublicKeyString} ot=1 ip=
  ```
  and UTF-8 encodes it into `textBytes`.
- **Data Size:** $301 + \text{textBytes.len}$ bytes (`4 + 1 + 232 + 64 + textBytes.len`).
- **Payload Layout:**
  | Offset | Size | Type | Field Description |
  |---|---|---|---|
  | `0x00` | 4 B | `u32` (BE) | `InitVersion` (`1566914096`) |
  | `0x04` | 1 B | `u8` | `initType = 0x04` |
  | `0x05` | 232 B | `[u8; 232]`| Direct copy of Server Step 3 payload (`Step3_data[1..233]`) |
  | `0xED` | 64 B | `[u8; 64]` | Solution $y$ (512-bit integer, zero-padded on left to 64 bytes) |
  | `0x12D` | Var | `[u8; N]` | UTF-8 encoded `clientinitiv ...` command string |

> [!NOTE]
> If a server responds with `initType = 0x7F`, the client resets and restarts from Step 0 (`TsCrypt.cs:395-405`).

---

## 4. Key Exchange Protocols: `initivexpand` vs `initivexpand2` + `clientek`

Following Step 4, the server evaluates `clientinitiv` and responds with one of two notification commands (`TsFullClient.cs:259-302`):
- `initivexpand`: Legacy P-256 ECDH key exchange.
- `initivexpand2`: Modern Ed25519 license-chain validated key exchange.

### 4.1 Legacy Path: `initivexpand` (`TsCrypt.cs:255-310`)
1. **Server Notification:** `initivexpand alpha={alpha} beta={beta} omega={omega}`
   - `alpha`: Base64 string of server's alpha (typically echoes client's 10 bytes).
   - `beta`: Base64 string of server's beta (10 bytes).
   - `omega`: Base64 string of server's P-256 public key (LibTomCrypt DER format).
2. **P-256 ECDH Shared Secret:**
   - Server public key point $Q_{\text{server}} = (x_s, y_s)$ is imported.
   - Point multiplication: $P = d_{\text{client}} \cdot Q_{\text{server}} = (x_p, y_p)$.
   - Extract 32-byte big-endian X-coordinate $x_p$.
   - Compute:
     $$\text{sharedKey} = \text{SHA-1}(x_p) \quad (\text{20 bytes})$$
3. **Derive `ivStruct` (20 bytes):**
   $$\text{ivStruct}[0..10] = \text{sharedKey}[0..10] \mathbin{\oplus} \alpha[0..10]$$
   $$\text{ivStruct}[10..20] = \text{sharedKey}[10..20] \mathbin{\oplus} \beta[0..10]$$
4. **Derive `fakeSignature` (8 bytes):**
   $$\text{fakeSignature} = \text{SHA-1}(\text{ivStruct})[0..8]$$
5. `CryptoInitComplete = true`.

---

### 4.2 Modern Path: `initivexpand2` + `clientek` (`TsCrypt.cs:312-360`, `License.cs`, `TsFullClient.cs:276-302`)

```
Client                                                   Server
  │                                                        │
  │<── initivexpand2 license=... omega=... proof=... ──────│
  │                    beta=... (54 B)                     │
  │                                                        │
  │─── clientek ek={ephemeral_pub} proof={proof} ─────────>│ (Dummy Encrypted)
  │                                                        │
  │─── clientinit client_nickname=... ────────────────────>│ (Session Encrypted)
```

#### Step 1: Server Signature Verification
- The server sends:
  - `license`: Base64 encoded binary license chain.
  - `omega`: Base64 encoded server P-256 public key (DER).
  - `proof`: Base64 encoded ECDSA signature over `licenseBytes`.
  - `beta`: Base64 encoded 54-byte string.
- Client verifies `proof` against `licenseBytes` using server public key `omega` under **ECDSA-P256-SHA256**. If verification fails, terminate immediately.

#### Step 2: Ephemeral Client Keypair & `clientek`
- Client generates an ephemeral **Ed25519** keypair (`TsCrypt.cs:506-518`):
  1. Generate 32 secure random bytes: `priv[0..32]`.
  2. Clamp private scalar:
     `priv[0] &= 248; priv[31] &= 127; priv[31] |= 64;`
  3. Derive Ed25519 public key point $A = \text{priv} \cdot B$.
  4. Encode point $A$ to 32 bytes (`ephemeral_pub`).
- Client builds proof to authenticate the ephemeral key:
  $$\text{toSign} = \text{ephemeral\_pub}[0..32] \mathbin{\Vert} \beta[0..54] \quad (\text{86 bytes})$$
  - Sign `toSign` with client's persistent P-256 private key $d_{\text{identity}}$ using **ECDSA-P256-SHA256** (DER format `SEQUENCE { r INTEGER, s INTEGER }`).
  - Base64 encode: `ek = Base64(ephemeral_pub)`, `proof = Base64(signature)`.
- Client transmits command `clientek ek={ek} proof={proof}` (encrypted with dummy firewall key).

#### Step 3: License Chain Binary Structure (`License.cs:28-139`)
- `licenseBytes` starts with 1-byte version header: must be `0x01`.
- Subsequent data consists of contiguous `LicenseBlock` structures:

| Field | Offset | Size | Type | Value / Description |
|---|---|---|---|---|
| `keyKind` | `0x00` | 1 B | `u8` | Must be `0x00` |
| `key` | `0x01` | 32 B | `[u8; 32]` | Ed25519 public key point bytes |
| `blockType` | `0x21` | 1 B | `u8` | `0x00`: Intermediate, `0x02`: Server, `0x20`: Ephemeral |
| `notValidBefore`| `0x22` | 4 B | `u32` (BE) | Unix timestamp $+ 0x50E22700$ offset |
| `notValidAfter` | `0x26` | 4 B | `u32` (BE) | Unix timestamp $+ 0x50E22700$ offset |
| *Type Specific*| `0x2A` | Var | ... | See below |

- **Type-Specific Fields:**
  - `blockType == 0` (Intermediate): Null-terminated UTF-8 string at offset `46` (`data[46..]`). Total block length = $47 + \text{strLen} + 1$.
  - `blockType == 2` (Server):
    - Offset `42` (`u8`): `ServerLicenseType` enum (`0=None, 1=Offline, 2=Sdk, 3=SdkOffline, 4=Npl, 5=Athp, 6=Aal, 7=Default, 8=Gamer, 9=Sponsorship, 10=Commercial`).
    - Offset `47`: Null-terminated UTF-8 issuer string.
    - Total block length = $48 + \text{strLen} + 1$.
  - `blockType == 32` (Ephemeral / `0x20`): No extra data. Total block length = 42 bytes.
- **Block Hash:**
  Each block computes $H_{\text{block}} = \text{SHA-512}(\text{data}[1 \dots \text{blockLen}])[0..32]$ (32 bytes).

#### Step 4: Ed25519 License Key Derivation Loop (`License.cs:55-61, 147-164`)
Starting with the hardcoded TeamSpeak License Root Key:
$$\text{round}_0 = \text{LicenseRootKey} = \text{0xcd0de2aed46345509a7e3cfd8f68b3dc7555b29dccec73cd18750f993812408a}$$
For each block $i$ in the chain:
1. Clamp block hash:
   `hash = block.Hash; hash[0] &= 248; hash[31] &= 127; hash[31] |= 64;`
2. Perform point arithmetic on twisted Edwards curve $-x^2 + y^2 = 1 - \frac{121665}{121666} x^2 y^2$:
   $$\text{round}_{i+1} = \text{round}_i + \text{hash} \cdot \text{block.Key}$$
3. Result after all blocks is `derived_license_pub` (32-byte Ed25519 public key).

#### Step 5: Shared Secret Derivation via Diffie-Hellman (`TsCrypt.cs:347-360`)
1. Client takes `derived_license_pub` and its ephemeral `ephemeral_priv`.
2. Clear high bit: `ephemeral_priv[31] &= 0x7F`.
3. Compute scalar multiplication on Ed25519:
   $$P_{\text{shared}} = \text{ephemeral\_priv} \cdot \text{derived\_license\_pub}$$
4. Encode point to bytes `sharedTmp[0..32]`, flip sign bit: `sharedTmp[31] ^= 0x80`.
5. Hash with SHA-512 to produce a 64-byte shared key:
   $$\text{sharedKey} = \text{SHA-512}(\text{sharedTmp}) \quad (\text{64 bytes})$$

#### Step 6: Derive 64-byte `ivStruct` & `fakeSignature`
- Recall `alphaTmp` is the 10 random bytes generated in Step 4.
- `beta` is 54 bytes from `initivexpand2`.
- Total length of `ivStruct` is $10 + 54 = 64$ bytes:
  $$\text{ivStruct}[0..10] = \text{sharedKey}[0..10] \mathbin{\oplus} \text{alphaTmp}[0..10]$$
  $$\text{ivStruct}[10..64] = \text{sharedKey}[10..64] \mathbin{\oplus} \beta[0..54]$$
- Compute 8-byte `fakeSignature`:
  $$\text{fakeSignature} = \text{SHA-1}(\text{ivStruct})[0..8]$$
- Mark `CryptoInitComplete = true`.

---

## 5. Packet Encryption & Per-Packet Key/Nonce Derivation

TeamSpeak 3 uses **AES-128-EAX** mode (authenticated encryption with associated data) with a **64-bit (8-byte) MAC tag**.

### 5.1 Per-Packet Key and Nonce Derivation (`TsCrypt.cs:656-701`)

For every packet, a 16-byte AES key and 16-byte EAX nonce are generated:

1. **Cache Lookup:**
   - Key generation is cached per `(Direction, PacketType)` pair:
     $$\text{cacheIndex} = \text{PacketType} \times (\text{FromServer} \,?\, 1 : 2)$$
   - Cached value holds `(base_key: [u8; 16], base_nonce: [u8; 16], generation_id: u32)`.
2. **On Cache Miss or Generation Increment:**
   - Determine hash buffer length:
     - 26 bytes if `ivStruct.len == 20` (Legacy P-256)
     - 70 bytes if `ivStruct.len == 64` (Modern Ed25519)
   - Layout of pre-hash buffer:
     | Offset | Size | Type | Field Description |
     |---|---|---|---|
     | `0x00` | 1 B | `u8` | Direction byte: `0x30` (`'0'`) for S2C, `0x31` (`'1'`) for C2S |
     | `0x01` | 1 B | `u8` | `packetTypeRaw = packetType & 0x0F` (lower 4 bits) |
     | `0x02` | 4 B | `u32` (BE) | `generationId` (increments every 65536 packets per type) |
     | `0x06` | 20 or 64 B| Bytes | `ivStruct` |
   - Hash with SHA-256:
     $$H_{\text{deriv}} = \text{SHA-256}(\text{pre\_hash\_buffer}) \quad (\text{32 bytes})$$
   - Base Key: $\text{base\_key} = H_{\text{deriv}}[0..16]$
   - Base Nonce: $\text{base\_nonce} = H_{\text{deriv}}[16..32]$
3. **Per-Packet Transformation:**
   - Nonce is identical to base nonce: $\text{nonce} = \text{base\_nonce}$.
   - Key XORs `PacketId` into its first 2 bytes:
     $$\text{key}[0] = \text{base\_key}[0] \mathbin{\oplus} (\text{PacketId} \gg 8)$$
     $$\text{key}[1] = \text{base\_key}[1] \mathbin{\oplus} (\text{PacketId} \ \& \ \text{0xFF})$$
     $$\text{key}[2..16] = \text{base\_key}[2..16]$$

### 5.2 AES-128-EAX Parameters & Wire Framing (`TsCrypt.cs:524-584`, `Packet.cs:118-160`)
- **Cipher:** AES-128 in EAX Mode.
- **Key:** `key` (16 bytes).
- **Nonce:** `nonce` (16 bytes).
- **Tag Length:** 8 bytes (`MacLen = 8`).
- **Associated Data (AD):** The unencrypted packet header bytes:
  - C2S Header: 5 bytes (`PacketId` [2], `ClientId` [2], `PacketTypeFlagged` [1])
  - S2C Header: 3 bytes (`PacketId` [2], `PacketTypeFlagged` [1])
- **Payload:** Plaintext packet data `packet.Data`.
- **Wire Format (Raw UDP Datagram):**
  ```
  ┌───────────────────┬──────────────────┬────────────────────────┐
  │   MAC / Auth Tag  │  Packet Header   │    Ciphertext Data     │
  │     (8 bytes)     │  (3 or 5 bytes)  │       (N bytes)        │
  └───────────────────┴──────────────────┴────────────────────────┘
  ```
  *(Note: Unlike standard TLS/AEAD which appends the MAC at the end, TeamSpeak places the 8-byte MAC tag at byte offset 0).*

---

## 6. Fixed Dummy Key & Unencrypted Mode Cryptography

TeamSpeak 3 has three exceptions to normal session AES-128-EAX encryption:

### 6.1 Init1 Packets (`PacketType.Init1 = 0x08`)
- Plaintext payload.
- MAC tag replaced by literal ASCII `"TS3INIT1"` (`0x54 0x53 0x33 0x49 0x4e 0x49 0x54 0x31`).

### 6.2 Unencrypted Flag Packets (`PacketFlags.Unencrypted = 0x80`)
- Applies to unencrypted voice packets (`PacketType.Voice = 0x00` without channel encryption).
- Plaintext payload.
- MAC tag replaced by `fakeSignature` = `SHA-1(ivStruct)[0..8]`.

### 6.3 Pre-Session "Dummy" Encryption (`!CryptoInitComplete`)
- Applies to commands sent before `ivStruct` is derived (such as `clientek` and the dummy `clientinitiv`).
- Uses fixed string `"c:\windows\system\firewall32.cpl"` (verified in `ts3server` at `0x7b28a4`):
  - **Dummy Key (16 B):** `"c:\windows\syste"` = `[0x63, 0x3a, 0x5c, 0x77, 0x69, 0x6e, 0x64, 0x6f, 0x77, 0x73, 0x5c, 0x73, 0x79, 0x73, 0x74, 0x65]`
  - **Dummy Nonce (16 B):** `"m\firewall32.cpl"` = `[0x6d, 0x5c, 0x66, 0x69, 0x72, 0x65, 0x77, 0x61, 0x6c, 0x6c, 0x33, 0x32, 0x2e, 0x63, 0x70, 0x6c]`
- Cipher: AES-128-EAX with 8-byte tag, header as AD. `PacketId` is **not** XORed into the dummy key (`TsCrypt.cs:664-665`).

### 6.4 The ACK Decryption Race Condition Workaround (`TsCrypt.cs:597-614`)
During connection startup, the client transmits:
1. `[PacketId: 1]` `clientek` (encrypted with Dummy Key)
2. `[PacketId: 2]` `clientinit` (encrypted with Session Key)
The server returns ACKs (`PacketType.Ack = 0x06`) for both. Due to UDP reordering or processing timing, ACKs can arrive out of order.
- **Rule:** Attempt decryption using the current session key. If authentication fails and `packetType == PacketType.Ack && packetId <= 2`, retry decryption using the **Dummy Key**.

---

## 7. Concrete Test Vectors & Verification Matrix

All test vectors below were verified against `TS3AudioBot` unit test data and primary cryptographic implementations.

### 7.1 Identity & Hashcash Test Vector (`bot.toml:25-27`)
- **Private Key ASN.1 DER (Base64):**
  `MCoDAgbAAgEgAiEApY7dHTX4bMhuKCa5J3z1WyLyMLvnJTWBi8wT7AJtuhA=`
- **Private Key DER Hex:**
  `302a030206c0020120022100a58edd1d35f86cc86e2826b9277cf55b22f230bbe72535818bcc13ec026dba10`
- **Private Scalar $d$:**
  `0xa58edd1d35f86cc86e2826b9277cf55b22f230bbe72535818bcc13ec026dba10`
- **Derived Public Key Affine Coordinates:**
  - $x$: `0x147f8093b4f9e5c2d0f296842a1a1e19b2f26a95ebf9bef9dd79f2b6592ceb78`
  - $y$: `0xd118e0256b9f9408f398533117ed0f784f427bd8d570df90c8b284fc3a506233`
- **Exported Public Key DER (Base64):**
  `MEwDAgcAAgEgAiAUf4CTtPnlwtDyloQqGh4ZsvJqlev5vvndefK2WSzreAIhANEY4CVrn5QI85hTMRftD3hPQnvY1XDfkMiyhPw6UGIz`
- **Client UID:**
  `tUXzo5QsCNhqcRnaGCA5t3IM3HE=`
- **Security Level Proof-of-Work:**
  - `offset`: `64` (`0x40`)
  - Input: `MEwDAgcAAgEgAiAUf4CTtPnlwtDyloQqGh4ZsvJqlev5vvndefK2WSzreAIhANEY4CVrn5QI85hTMRftD3hPQnvY1XDfkMiyhPw6UGIz64`
  - SHA-1 Hash: `00f794913b1e927dff88f76b938f2b0b0b8be849`
  - Leading Zero Bits: `8` (Byte 0 has 8 zeros; Byte 1 is `0xF7` whose LSB is 1)
- **Obfuscated Identity String:**
  `64VM2F7zoV2btgUFGbpr0NUkAlK6YZ9MmkEW3kJRChyVAAvBE4GZEt7Gy5+EAt/MGJzCg1GZgdzeRIQXXVc`

### 7.2 AES-128-EAX Dummy Encryption Test Vector
- **Key:** `63 3a 5c 77 69 6e 64 6f 77 73 5c 73 79 73 74 65` (`"c:\windows\syste"`)
- **Nonce:** `6d 5c 66 69 72 65 77 61 6c 6c 33 32 2e 63 70 6c` (`"m\firewall32.cpl"`)
- **Header (AD):** `00 01 00 00 02` (`PacketId = 1`, `ClientId = 0`, `PacketType = 2 / Command`)
- **Plaintext:** `63 6c 69 65 6e 74 69 6e 69 74` (`"clientinit"`)
- **Ciphertext:** `9d 74 8b 45 aa 7b ef b9 9e fe`
- **MAC Tag (8 B):** `94 43 eb 56 88 bd 38 28`
- **Complete Wire Packet (Hex):**
  `9443eb5688bd382800010000029d748b45aa7befb99efe`

### 7.3 Per-Packet Key Derivation Test Vector
- `ivStruct`: 64 bytes (`0x01, 0x02, ..., 0x40`)
- `Direction`: C2S (`0x31`)
- `PacketType`: `0x02` (Command)
- `generationId`: `0x00000000`
- `PacketId`: `0x1234`
- **Base Key:** `4f 89 2b e8 72 63 b9 6f 2d 92 0c 94 92 d1 cf 3c`
- **Base Nonce:** `98 92 e5 22 e0 13 b5 cb bf 40 18 7e 8f 93 ab 07`
- **Transformed Packet Key ($k[0..2] \mathbin{\oplus}= 0x1234$):**
  `5d bd 2b e8 72 63 b9 6f 2d 92 0c 94 92 d1 cf 3c`
- **Packet Nonce (Unchanged):**
  `98 92 e5 22 e0 13 b5 cb bf 40 18 7e 8f 93 ab 07`

---

## 8. Rust Implementation Blueprint

Below is an architectural template designed for integration with crates `p256`, `curve25519-dalek`, `eax`, `aes`, `sha1`, and `sha2`.

```rust
use aes::Aes128;
use eax::Eax;
use eax::aead::{AeadInPlace, KeyInit, Tag};
use sha1::{Sha1, Digest as Sha1Digest};
use sha2::{Sha256, Sha512, Digest};

pub const DUMMY_KEY: &[u8; 16] = b"c:\\windows\\syste";
pub const DUMMY_IV: &[u8; 16]  = b"m\\firewall32.cpl";
pub const TS3INIT_MAC: &[u8; 8] = b"TS3INIT1";

pub struct TsCryptor {
    pub iv_struct: Option<Vec<u8>>,
    pub fake_signature: [u8; 8],
    pub crypto_init_complete: bool,
    key_cache: [Option<([u8; 16], [u8; 16], u32)>; 18], // 9 types * 2 directions
}

impl TsCryptor {
    pub fn new() -> Self {
        Self {
            iv_struct: None,
            fake_signature: [0u8; 8],
            crypto_init_complete: false,
            key_cache: [None; 18],
        }
    }

    /// Derive per-packet key and nonce
    pub fn get_key_nonce(
        &mut self,
        from_server: bool,
        packet_id: u16,
        generation_id: u32,
        packet_type: u8,
    ) -> ([u8; 16], [u8; 16]) {
        if !self.crypto_init_complete {
            return (*DUMMY_KEY, *DUMMY_IV);
        }

        let p_type = (packet_type & 0x0F) as usize;
        let cache_idx = p_type * (if from_server { 1 } else { 2 });

        let (base_key, base_nonce) = match self.key_cache[cache_idx] {
            Some((k, n, gen)) if gen == generation_id => (k, n),
            _ => {
                let iv = self.iv_struct.as_ref().expect("iv_struct must be initialized");
                let mut hasher = Sha256::new();
                hasher.update([if from_server { 0x30 } else { 0x31 }]);
                hasher.update([p_type as u8]);
                hasher.update(generation_id.to_be_bytes());
                hasher.update(iv);
                let hash = hasher.finalize();

                let mut k = [0u8; 16];
                let mut n = [0u8; 16];
                k.copy_from_slice(&hash[0..16]);
                n.copy_from_slice(&hash[16..32]);

                self.key_cache[cache_idx] = Some((k, n, generation_id));
                (k, n)
            }
        };

        let mut key = base_key;
        key[0] ^= (packet_id >> 8) as u8;
        key[1] ^= (packet_id & 0xFF) as u8;

        (key, base_nonce)
    }

    /// Decrypt an incoming UDP datagram in place
    pub fn decrypt(
        &mut self,
        from_server: bool,
        raw: &[u8],
        header_len: usize,
    ) -> Result<(u16, u8, Vec<u8>), &'static str> {
        if raw.len() < 8 + header_len {
            return Err("Packet too short");
        }

        let (mac, rest) = raw.split_at(8);
        let (header, ciphertext) = rest.split_at(header_len);

        let packet_id = u16::from_be_bytes([header[0], header[1]]);
        let packet_type_flagged = if header_len == 5 { header[4] } else { header[2] };
        let packet_type = packet_type_flagged & 0x0F;
        let unencrypted = (packet_type_flagged & 0x80) != 0;

        if packet_type == 0x08 {
            if mac != TS3INIT_MAC { return Err("Invalid Init1 MAC"); }
            return Ok((packet_id, packet_type_flagged, ciphertext.to_vec()));
        }

        if unencrypted {
            if mac != self.fake_signature { return Err("Invalid fake signature MAC"); }
            return Ok((packet_id, packet_type_flagged, ciphertext.to_vec()));
        }

        let (key, nonce) = self.get_key_nonce(from_server, packet_id, 0, packet_type);
        let mut cipher = Eax::<Aes128>::new(&key.into());
        let mut buffer = ciphertext.to_vec();
        let tag = Tag::from_slice(mac);

        cipher.decrypt_in_place_detached(&nonce.into(), header, &mut buffer, tag)
            .map_err(|_| "EAX decryption failure")?;

        Ok((packet_id, packet_type_flagged, buffer))
    }
}

/// Solves the TS3INIT1 RSA puzzle: y = x^(2^level) mod n via repeated modular squaring
pub fn solve_rsa_puzzle(x_bytes: &[u8], n_bytes: &[u8], level: i32) -> Result<Vec<u8>, &'static str> {
    use num_bigint::BigUint;
    if level < 0 || level > 1_000_000 {
        return Err("Level out of bounds");
    }

    let mut y = BigUint::from_bytes_be(x_bytes);
    let n = BigUint::from_bytes_be(n_bytes);

    for _ in 0..level {
        y = (&y * &y) % &n;
    }

    let mut res = y.to_bytes_be();
    if res.len() < 64 {
        let mut padded = vec![0u8; 64 - res.len()];
        padded.extend_from_slice(&res);
        res = padded;
    }
    Ok(res)
}
```
