# 01 — `ts3server` 3.13.9 Binary Strings, Constants & ELF Internals

**Target Binary:** `D:\Dev\ts3-rust\.scratch\teamspeak3-server_linux_amd64\ts3server` (13,701,328 bytes, Linux x86-64 ELF)  
**Reference Implementation:** `D:\Dev\TS3AudioBot\mx\TSLib\`  
**Verification Status:** All binary file offsets, virtual addresses (VAs), byte sequences, and C# source cross-references below are **verified** directly against the `ts3server` 3.13.9 binary and `TSLib` source unless explicitly marked `[Inferred]`.

---

## 1. ELF Binary Metadata, Sections & Dynamic Linkage

### 1.1 ELF Header & Build Metadata (`[Verified]`)

| Property | Value | Notes |
|---|---|---|
| **File Size** | `13,701,328` bytes (`0x00d11150`) | Linux `amd64` server executable |
| **ELF Class / Endianness** | `ELFCLASS64` / `ELFDATA2LSB` (Little-Endian) | 64-bit x86-64 (`EM_X86_64`), `ELFOSABI_SYSV` |
| **ELF Type** | `ET_EXEC` (Non-PIE executable) | Fixed load base `0x00400000` |
| **Entry Point** | `0x004230f0` (File offset `0x000230f0`) | `_start` in `.text` |
| **Symbol Stripping** | **Stripped** (`.symtab` absent) | `.dynsym` (`16,560` bytes) and C++ RTTI typeinfo strings in `.rodata` remain intact |
| **GNU Build ID** | `51f58ddfc2878aa2d2238393364471e43d32a8f7` | `.note.gnu.build-id` at file offset `0x000002ac` |
| **Compilers (`.comment`)** | `GCC: (Ubuntu 5.4.0-6ubuntu1~16.04.12) 5.4.0 20160609`<br>`clang version 7.1.0-svn353565-1~exp1~20190408084827.60 (branches/release_70)` | File offset `0x00d10680`, size `0x82` (130 bytes); uses `libc++` (`std::__1`) |
| **Debug Link** | `.gnu_debuglink` at `0x00d10702` (`20` bytes) | Points to external debug symbol file |

### 1.2 Section Table & Address Mapping (`[Verified]`)

Key address-to-offset conversion formulas for static analysis:
- **Code / Read-only sections (`.text`, `.rodata`, `.eh_frame`):** $\text{VA} = \text{file\_offset} + \text{0x00400000}$
- **Relocated / Writable data sections (`.data.rel.ro`, `.dynamic`, `.got`, `.data`):** $\text{VA} = \text{file\_offset} + \text{0x00600000}$

| Section | Virtual Address (VA) | File Offset | Size (Hex) | Size (Bytes) | Flags | Purpose |
|---|---|---|---|---|---|---|
| `.interp` | `0x00400270` | `0x00000270` | `0x0000001c` | 28 | `ALLOC` | `/lib64/ld-linux-x86-64.so.2` |
| `.note.ABI-tag` | `0x0040028c` | `0x0000028c` | `0x00000020` | 32 | `ALLOC` | Linux ABI tag |
| `.note.gnu.build-id` | `0x004002ac` | `0x000002ac` | `0x00000024` | 36 | `ALLOC` | SHA-1 GNU Build ID |
| `.dynsym` | `0x00400d60` | `0x00000d60` | `0x000040b0` | 16,560 | `ALLOC` | Dynamic symbol table |
| `.dynstr` | `0x00404e10` | `0x00004e10` | `0x0000292d` | 10,541 | `ALLOC` | Dynamic string table |
| `.plt` | `0x00409f90` | `0x00009f90` | `0x00001520` | 5,408 | `ALLOC\|EXECINSTR` | Procedure Linkage Table |
| `.text` | `0x0040b4d0` | `0x0000b4d0` | `0x00797b22` | 7,961,378 | `ALLOC\|EXECINSTR` | Server, Boost.Asio, LibTomCrypt, Protobuf code |
| `.rodata` | `0x00ba3000` | `0x007a3000` | `0x00095268` | 610,920 | `ALLOC` | Protocol strings, crypto tables, keys, RTTI names |
| `.eh_frame_hdr` | `0x00c38268` | `0x00838268` | `0x0002eaec` | 191,212 | `ALLOC` | Exception frame header |
| `.eh_frame` | `0x00c66d58` | `0x00866d58` | `0x00112ef4` | 1,126,132 | `ALLOC` | Unwind info |
| `.gcc_except_table` | `0x00d79c4c` | `0x00979c4c` | `0x000343e4` | 213,988 | `ALLOC` | C++ exception tables |
| `.tdata` / `.tbss` | `0x00fae030` | `0x009ae030` | `0x00000030` | 48 | `WRITE\|ALLOC` | Thread-local storage (incl. QuickLZ state) |
| `.init_array` | `0x00fae040` | `0x009ae040` | `0x00000420` | 1,056 | `WRITE\|ALLOC` | Global C++ constructors (132 pointers) |
| `.data.rel.ro` | `0x00fae470` | `0x009ae470` | `0x000277a8` | 161,704 | `WRITE\|ALLOC` | Vtables, master error/permission group index table |
| `.dynamic` | `0x00fd5c18` | `0x009d5c18` | `0x00000250` | 592 | `WRITE\|ALLOC` | Dynamic linker tags |
| `.got` / `.got.plt` | `0x00fd5e68` | `0x009d5e68` | `0x00000c30` | 3,120 | `WRITE\|ALLOC` | Global Offset Table |
| `.data` | `0x00fd6ac0` | `0x009d6ac0` | `0x00339bc0` | 3,382,208 | `WRITE\|ALLOC` | Initialized globals, error string pointer tables, `TS3INIT1` struct |
| `.bss` | `0x01310680` | `0x00d10680` | `0x0000f060` | 61,536 | `WRITE\|ALLOC` | Zero-initialized globals |

### 1.3 Dynamic Libraries (`DT_NEEDED`) & Companion Plugins (`[Verified]`)

Extracted from `.dynamic` (`0x009d5c18`):
1. `libts3_ssh.so` (bundled SSH ServerQuery transport, `1,104,336` bytes)
2. `libdl.so.2` (dynamic loader for `libts3db_*.so` database backends: `libts3db_sqlite3.so`, `libts3db_mariadb.so`, `libts3db_postgresql.so`)
3. `librt.so.1`
4. `libm.so.6`
5. `libpthread.so.0`
6. `libstdc++.so.6`
7. `libc.so.6`
8. `ld-linux-x86-64.so.2`

---

## 2. Client-Protocol Command Names, Notifications & Parameter Keys

`.rodata` (`0x007a3000`–`0x00838268`, VA `0x00ba3000`–`0x00c38268`) contains **7,879** null-terminated printable ASCII strings (`len >= 3`), organized into contiguous functional clusters corresponding to translation units.

### 2.1 Handshake & Authentication Commands (`Server_Packet_Handler.cpp` cluster, `0x007c01a0`–`0x007c03e0`) (`[Verified]`)

Located adjacent to assertion string `../../../../deps/teamspeak_server_lib/src/ts_server/net/Server_Packet_Handler.cpp` (`0x007c01a8`) and RTTI symbol `com::teamspeak::server::net::Server_Packet_Handler` (`0x007c03e0`):

| String | File Offset | Virtual Address | Direction | Protocol Role & Parameters | TSLib Cross-Reference |
|---|---|---|---|---|---|
| `clientinitiv` | `0x007c030f` | `0x00bc030f` | C2S | Sent inside Init1 step 4 packet payload (`alpha`, `omega`, `ot=1`, `ip`) | [TsCrypt.cs](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/TsCrypt.cs#L451-L456) |
| `initivexpand` | *(legacy)* | *(N/A in 3.13.9)* | S2C | Legacy pre-3.1 handshake command (`alpha`, `beta`, `omega`); superseded in 3.13.9 by `initivexpand2` | [TsFullClient.cs](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/TsFullClient.cs#L261-L269), [Messages.tt](file:///D:/Dev/TS3AudioBot/mx/TSLib/Generated/Messages.tt#L26-L31) |
| `initivexpand2` | `0x007c022f` | `0x00bc022f` | S2C | Modern license-chain ECDH handshake command (`l`, `beta`, `omega`, `proof`, `tvd`, `ot`) | [TsFullClient.cs](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/TsFullClient.cs#L271-L290), [TsCrypt.cs](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/TsCrypt.cs#L312-L345) |
| `clientek` | `0x007c01fb` | `0x00bc01fb` | C2S | Sent as Command packet ID 1 (dummy-encrypted) with ephemeral Curve25519/Ed25519 key `ek` and `proof` | [TsFullClient.cs](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/TsFullClient.cs#L275-L280), [TsCrypt.cs](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/TsCrypt.cs#L599) |
| `clientinit` | `0x007c0204` | `0x00bc0204` | C2S | Sent as Command packet ID 2 (session-encrypted) with client identity, version, nickname, password, and security offset | [TsFullClient.cs](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/TsFullClient.cs#L292-L335), [TsCrypt.cs](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/TsCrypt.cs#L600) |
| `initserver` | `0x007b594c` | `0x00bb594c` | S2C | Server session-established packet containing `virtualserver_*` properties, `aclid`, `acn`, `pv` | [TsFullClient.cs](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/TsFullClient.cs#L338-L355), [Messages.tt](file:///D:/Dev/TS3AudioBot/mx/TSLib/Generated/Messages.tt#L38-L120) |
| `dummy_connectfailed` | `0x007c020f` | `0x00bc020f` | Internal | Dummy command marker on failed handshake | Verified in binary `.rodata` |

**Handshake Parameter Keys in `.rodata` (`[Verified]`):**
- `alpha` (`0x007c0223`, VA `0x00bc0223`) — 10-byte client random nonce (base64)
- `omega` (`0x007c0229`, VA `0x00bc0229`) — ASN.1 DER-encoded NIST P-256 public key (base64)
- `beta` (`0x007c023d`, VA `0x00bc023d`) — Server random nonce: 10 bytes (`initivexpand`) or 54 bytes (`initivexpand2`, base64)
- `proof` (`0x007c0242`, VA `0x00bc0242`) — ECDSA-SHA256 signature (DER-encoded, base64) over `l` (in `initivexpand2`) or `ek` (in `clientek`)
- `tvd` (`0x007c0248`, VA `0x00bc0248`) — Optional token validation data (`Token_Validation_Data`)
- `acn` (`0x007b5957`, VA `0x00bb5957`) — Assigned client nickname in `initserver`
- `aclid` (`0x007b595b`, VA `0x00bb595b`) — Assigned `ClientId` (`u16`) in `initserver`
- `pv` (`0x007b5961`, VA `0x00bb5961`) — Protocol version in `initserver`
- `pubSign` (`0x007b58fa`, VA `0x00bb58fa`), `pubSignCert` (`0x007b5902`, VA `0x00bb5902`) — myTeamSpeak ID certificate fields

### 2.2 Core Client-to-Server Commands (`0x007c3b70`–`0x007c4200`) (`[Verified]`)

| Command Name | File Offset | Virtual Address | Purpose |
|---|---|---|---|
| `clientmove` | `0x007c3b75` | `0x00bc3b75` | Move client(s) to target channel (`clid`, `cid`, `cpw`) |
| `clientdisconnect` | `0x007c3b98` | `0x00bc3b98` | Gracefully disconnect from server (`reasonid`, `reasonmsg`) |
| `clientupdate` | `0x007c3ba9` | `0x00bc3ba9` | Update own client properties (`client_nickname`, `client_input_muted`, etc.) |
| `clientkick` | `0x007c3bb6` | `0x00bc3bb6` | Kick client from channel (`reasonid=4`) or server (`reasonid=5`) |
| `channelcreate` | `0x007c3cd9` | `0x00bc3cd9` | Create channel (`channel_name`, flags, codec) |
| `channeledit` | `0x007c3ce7` | `0x00bc3ce7` | Modify channel properties (`cid`, `channel_*`) |
| `channelmove` | `0x007c3cf3` | `0x00bc3cf3` | Change channel parent/order (`cid`, `cpid`, `order`) |
| `channeldelete` | `0x007c3cff` | `0x00bc3cff` | Delete channel (`cid`, `force`) |
| `channelsubscribe` | `0x007c3d23` | `0x00bc3d23` | Subscribe to channel client lists (`cid`) |
| `channelunsubscribe` | `0x007c3d48` | `0x00bc3d48` | Unsubscribe from channel(s) (`cid`) |
| `serverrequestconnectioninfo` | `0x007c3d84` | `0x00bc3d84` | Request server connection statistics |
| `sendtextmessage` | `0x007c3da0` | `0x00bc3da0` | Send text message (`targetmode`, `target`, `msg`) |
| `ftinitupload` | `0x007c3db0` | `0x00bc3db0` | Initialize file upload (`clientftfid`, `name`, `cid`, `cpw`, `size`, `overwrite`, `resume`) |
| `ftinitdownload` | `0x007c3dbd` | `0x00bc3dbd` | Initialize file download (`clientftfid`, `name`, `cid`, `cpw`, `seekpos`) |
| `permget` | `0x007c4020` | `0x00bc4020` | Query own effective permission value (`permid` / `permsid`) |
| `privilegekeyuse` | `0x007c41df` | `0x00bc41df` | Redeem privilege key / token (`token`) |

### 2.3 Server-to-Client Notifications (`virtualserverbase.cpp` cluster, `0x007b51e1`–`0x007c4e2b`) (`[Verified]`)

78 notification command strings exist in `.rodata`. Key client-facing notifications:

| Notification Name | File Offset | Virtual Address | Trigger / Associated Parameter Keys |
|---|---|---|---|
| `channellist` | `0x007b51e1` | `0x00bb51e1` | Initial channel tree stream sent during `initserver` sequence (`cid`, `cpid`, `channel_name`, etc.) |
| `channellistfinished` | `0x007b51ed` | `0x00bb51ed` | Marks completion of initial `channellist` burst |
| `notifychanneledited` | `0x007b5201` | `0x00bb5201` | Channel properties modified (`cid`, `reasonid`, `invokerid`, `invokername`, `invokeruid`) |
| `notifyclientupdated` | `0x007b5271` | `0x00bb5271` | Client state/metadata changed (`clid`, `client_*`) |
| `notifymutedclientdisconnected` | `0x007b5361` | `0x00bb5361` | Muted client left |
| `notifychannelmoved` | `0x007b537f` | `0x00bb537f` | Channel reparented (`cid`, `cpid`, `order`, `reasonid`) |
| `notifytextmessage` | `0x007b5397` | `0x00bb5397` | Incoming chat message (`targetmode`, `msg`, `target`, `invokerid`, `invokername`, `invokeruid`) |
| `notifyserverconnectioninfo` | `0x007b53b4` | `0x00bb53b4` | Response to `serverrequestconnectioninfo` |
| `notifyconnectioninforequest` | `0x007b5415` | `0x00bb5415` | Server asks client to report connection info (`invokerids`) |
| `notifyconnectioninfo` | `0x007b5431` | `0x00bb5431` | Connection stats for a specific client (`clid`, `connection_*`) |
| `notifychannelcreated` | `0x007b5446` | `0x00bb5446` | New channel created (`cid`, `cpid`, `channel_*`) |
| `notifychannelpasswordchanged` | `0x007b5484` | `0x00bb5484` | Channel password updated (`cid`) |
| `notifychanneldeleted` | `0x007b54a1` | `0x00bb54a1` | Channel removed (`cid`, `invokerid`, `invokername`, `invokeruid`) |
| `notifychannelsubscribed` | `0x007b54c6` | `0x00bb54c6` | Channel subscription confirmed (`cid`, `es`) |
| `notifychannelunsubscribed` | `0x007b54de` | `0x00bb54de` | Channel unsubscription confirmed (`cid`, `es`) |
| `notifyclientleftview` | `0x007b59b0` | `0x00bb59b0` | Client disconnected or moved out of view (`cfid`, `ctid`, `reasonid`, `reasonmsg`, `clid`, `bantime`) |
| `notifycliententerview` | `0x007b5a15` | `0x00bb5a15` | Client connected or moved into view (`cfid`, `ctid`, `reasonid`, `clid`, `client_*`) |
| `notifychanneldescriptionchanged` | `0x007b5a2b` | `0x00bb5a2b` | Channel description edited (`cid`) |
| `notifystartupload` | `0x007b5aaa` | `0x00bb5aaa` | File upload ready (`clientftfid`, `serverftfid`, `ftkey`, `port`, `seekpos`) |
| `notifystartdownload` | `0x007b5afa` | `0x00bb5afa` | File download ready (`clientftfid`, `serverftfid`, `ftkey`, `port`, `size`) |
| `notifyfileinfo` | `0x007b5b26` | `0x00bb5b26` | File metadata (`cid`, `name`, `size`, `datetime`) |
| `notifyfilelist` | `0x007b5b3e` | `0x00bb5b3e` | Directory listing chunk (`cid`, `path`, `name`, `size`, `datetime`, `type`) |
| `notifyfilelistfinished` | `0x007b5b68` | `0x00bb5b68` | End of directory listing (`cid`, `path`) |
| `notifyfiletransferlist` | `0x007b5be0` | `0x00bb5be0` | Active file transfers (`clientftfid`, `serverftfid`, `sizedone`, `sender`, `current_speed`) |
| `notifyserverupdated` | `0x007b5c9b` | `0x00bb5c9b` | Virtual server properties updated (`virtualserver_*`) |
| `notifyclientmoved` | `0x007b5d11` | `0x00bb5d11` | Client moved between channels (`clid`, `ctid`, `reasonid`, `invokerid`, `invokername`, `invokeruid`) |

### 2.4 Property & Parameter Key Tables in `.rodata` (`[Verified]`)

1. **`connection_*` keys (`0x007a6fa3`–`0x007a7280`, VA `0x00ba6fa3`–`0x00ba7280`, 22 keys):**
   - `connection_ping` (`0x007a6fa3`), `connection_ping_deviation` (`0x007a6fb3`), `connection_packets_sent` (`0x007a6fcd`), `connection_bytes_sent` (`0x007a6fe5`), `connection_packets_received` (`0x007a6ffb`), `connection_bytes_received` (`0x007a7017`), `connection_server2client_packetloss` (`0x007a7031`), `connection_client_ip` (`0x007a70f5`), `connection_client_port` (`0x007a710a`), `connection_idle_time` (`0x007a71d0`).
2. **`virtualserver_*` keys (`0x007a7f3a`–`0x007a8975`, VA `0x00ba7f3a`–`0x00ba8975`, 82 keys):**
   - `virtualserver_unique_identifier` (`0x007a7f3a`), `virtualserver_name` (`0x007a7f5a`), `virtualserver_welcomemessage` (`0x007a7f6d`), `virtualserver_platform` (`0x007a7f8a`), `virtualserver_version` (`0x007a7fa1`), `virtualserver_maxclients` (`0x007a7fb7`), `virtualserver_password` (`0x007a7fd0`), `virtualserver_clientsonline` (`0x007a7fe7`), `virtualserver_channelsonline` (`0x007a8002`), `virtualserver_created` (`0x007a801e`), `virtualserver_uptime` (`0x007a8034`), `virtualserver_codec_encryption_mode` (`0x007a8049`), `virtualserver_encryption_ciphers` (`0x007a806d`, default `"AES-256,AES-128"` at `0x007a8099`), `virtualserver_keypair` (`0x007a80a9`), `virtualserver_protocol_verify_keypair` (`0x007a80bf`), `virtualserver_min_client_version` (`0x007a8559`), `virtualserver_min_android_version`, `virtualserver_min_ios_version`, `virtualserver_security_salt`.
3. **`channel_*` keys (`0x007a8976`–`0x007a8bed`, VA `0x00ba8976`–`0x00ba8bed`, 38 keys):**
   - `channel_name`, `channel_topic`, `channel_description`, `channel_password`, `channel_codec`, `channel_codec_quality`, `channel_maxclients`, `channel_maxfamilyclients`, `channel_order` (`0x007a89f5`), `channel_flag_permanent` (`0x007a8a03`), `channel_flag_semi_permanent` (`0x007a8a1a`), `channel_flag_default` (`0x007a8a36`), `channel_flag_password` (`0x007a8a4b`), `channel_codec_latency_factor` (`0x007a8a61`), `channel_codec_is_unencrypted` (`0x007a8a7e`), `channel_security_salt` (`0x007a8a9b`), `channel_unique_identifier` (`0x007a8ab1`), `channel_needed_talk_power` (`0x007a8b6a`).
4. **`client_*` keys (`0x007a8bee`–`0x007a9480`, VA `0x00ba8bee`–`0x00ba9480`, 92 keys):**
   - `client_unique_identifier` (`0x007a8bee`), `client_nickname` (`0x007a8c07`), `client_platform` (`0x007a8c17`), `client_flag_talking` (`0x007a8c27`), `client_input_muted` (`0x007a8c3b`), `client_output_muted` (`0x007a8c4e`), `client_outputonly_muted` (`0x007a8c62`), `client_input_hardware` (`0x007a8c7a`), `client_output_hardware` (`0x007a8c90`), `client_input_deactivated` (`0x007a8ca7`), `client_idle_time` (`0x007a8cc0`), `client_default_channel` (`0x007a8cd1`), `client_default_channel_password` (`0x007a8ce8`), `client_server_password` (`0x007a8d08`), `client_meta_data` (`0x007a8d1f`), `client_is_muted` (`0x007a8d30`), `client_is_recording` (`0x007a8d40`), `client_volume_modificator` (`0x007a8d54`), `client_version_sign` (`0x007a8d6e`), `client_security_hash` (`0x007a8d82`), `client_encryption_ciphers` (`0x007a8d97`), `client_key_offset` (`0x007a8db1`), `client_database_id` (`0x007a8e03`), `client_channel_group_id` (`0x007a8e16`), `client_servergroups` (`0x007a8e2e`).

---

## 3. Cryptographic Constants, Curves & Library Markers

### 3.1 Embedded Crypto Libraries (`[Verified]`)

The binary statically embeds two cryptographic layers:
1. **`ts_tomcrypt` (Vendored LibTomCrypt fork):** 105 source path strings in `.rodata` under `./deps/ts_tomcrypt/src/`:
   - **AES / Rijndael:** `./deps/ts_tomcrypt/src/ciphers/aes/aes.c` (`0x007d50d4`), `cpurijndael` (`0x00808828`, AES-NI hardware acceleration alongside software Rijndael).
   - **EAX Authenticated Encryption:** `./deps/ts_tomcrypt/src/encauth/eax/eax_decrypt_verify_memory.c` (`0x007d964d`), `eax_done.c` (`0x007d96ae`), `eax_init.c`, `eax_encrypt_authenticate_memory.c`.
   - **Hashes:** `./deps/ts_tomcrypt/src/hashes/sha1.c` (`0x007d97e3`), `./deps/ts_tomcrypt/src/hashes/sha2/sha256.c` (`0x007d993d`), SHA-512 (`id-sha512` at `0x007ebea4`). Also includes hardware SHA extensions (`SHA1 - SHA-1 hardware acceleration` at `0x007e6b95`, `SHA256` at `0x007e6bbe`, `SHA512` at `0x007e6beb`).
   - **ECC (Short Weierstrass):** `./deps/ts_tomcrypt/src/pk/ecc/ecc_free.c` (`0x007d9d59`), `ecc_import.c` (`0x007d9d8c`), `ecc_verify_hash.c` (`0x007d9de1`), `ltc_ecc_map.c` (`0x007d9e29`), `ltc_ecc_mul2add.c` (`0x007d9e7a`), `ecc_make_key.c` (`0x00821579`).
2. **`com::teamspeak3::teamnative_crypto` (C++ wrapper & Ed25519/Curve25519 engine):**
   - Version banner at `0x007df45d` (VA `0x00bdf45d`): `"Version: 0.4 Nov  9 2026 17:16:59"`
   - Error strings at `0x007df400`: `INVALID_SIZE`, `unknown ERROR`, `RNG_FAIL`, `SIGNATURE_INVALID`, `INVALID_KEY`, `UNABLE_TO_CREATE_VALID_KEY`.
   - C++ RTTI types (`0x007df480`+): `Sha<160>`, `Sha<224>`, `Sha<256>`, `Sha<384>`, `Sha<512>`, `Hasher<20>`, `Hasher<28>`, `Hasher<32>`, `Hasher<48>`, `Hasher<64>`, `Aes256_Impl_CBC`, `Aes256_Impl_CTR`, `Aes256_Impl_ECB`, `Aes256_Impl_CFB8`, and `ed25519::Public_Key_t` (`0x007b4e7c`).

### 3.2 NIST P-256 (`secp256r1` / `prime256v1`) Binary Parameters (`[Verified]`)

Curve names table in `.rodata` around `0x007e2370`–`0x007e23d0` (`secp521r1`, `brainpoolP512r1`, `secp384r1`, `brainpoolP384r1`, `secp256r1` at `0x007e239a`, `secp256k1`, `brainpoolP256r1`, `secp224r1`, `secp224k1`, `secp192r1`, `secp192k1`).

The 256-bit `secp256r1` domain parameters are stored as **32-byte Little-Endian integers** in `.rodata` starting at `0x007e2570` (VA `0x00be2570`), cross-checked with `X9ObjectIdentifiers.Prime256v1` in [TsCrypt.cs](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/TsCrypt.cs#L122):

| Parameter | File Offset | Virtual Address | Size | Endianness in Binary | Value (Big-Endian Hex) |
|---|---|---|---|---|---|
| **Prime $p$** | `0x007e2570` | `0x00be2570` | 32 B | Little-Endian | `FFFFFFFF00000001000000000000000000000000FFFFFFFFFFFFFFFFFFFFFFFF` |
| **Coefficient $b$** | `0x007e2590` | `0x00be2590` | 32 B | Little-Endian | `5AC635D8AA3A93E7B3EBBD55769886BC651D06B0CC53B0F63BCE3C3E27D2604B` |
| **Generator $G_x$** | `0x007e25b0` | `0x00be25b0` | 32 B | Little-Endian | `6B17D1F2E12C4247F8BCE6E563A440F277037D812DEB33A0F4A13945D898C296` |
| **Generator $G_y$** | `0x007e25d0` | `0x00be25d0` | 32 B | Little-Endian | `4FE342E2FE1A7F9B8EE7EB4A7C0F9E162BCE33576B315ECECBB6406837BF51F5` |
| **Order $n$** | `0x007e25f0` | `0x00be25f0` | 32 B | Little-Endian | `FFFFFFFF00000000FFFFFFFFFFFFFFFFBCE6FAADA7179E84F3B9CAC2FC632551` |

**ASN.1 DER Public/Private Key Encoding (`omega` & Client Identity) (`[Verified]`, [TsCrypt.cs](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/TsCrypt.cs#L124-L194)):**
- LibTomCrypt `ecc_export` / `ecc_import` format wrapped in Base64:
  - `SEQUENCE` of:
    1. `BIT STRING`: `0x00` (7 unused bits) for Public Key (`omega`), `0x80` (7 unused bits) for Public+Private Key, `0xC0` (6 unused bits) for Private-Only Key.
    2. `INTEGER`: `32` (`0x20`, key size in bytes).
    3. `INTEGER`: $X$ coordinate (big-endian signed ASN.1 integer).
    4. `INTEGER`: $Y$ coordinate (big-endian signed ASN.1 integer).
    5. *(Optional, if `bitInfo == 0x80`)* `INTEGER`: Private scalar $d$.
- **Client Unique Identifier (`client_unique_identifier`):** `Base64(SHA1(ASCII(ExportPublicKey(Q))))` ([TsCrypt.cs](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/TsCrypt.cs#L196-L201)).

### 3.3 Ed25519 / Curve25519 Ephemeral Key Exchange (`CryptoInit2`) (`[Verified]`)

Used in modern handshake (`initivexpand2` + `clientek`, [TsCrypt.cs](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/TsCrypt.cs#L312-L360) and [License.cs](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/License.cs#L55-L61)):
1. **Temporary Client Key (`ek`):**
   - Generate 32 random bytes `priv`, clamp via `sc_clamp(priv)` (`priv[0] &= 248; priv[31] &= 127; priv[31] |= 64`).
   - Compute Ed25519 base point multiplication $A = \text{priv} \cdot B$, serialize via `ge_p3_tobytes(pub, A)` ([TsCrypt.cs](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/TsCrypt.cs#L506-L518)).
   - Sign `pub` with client's P-256 identity private key using `ECDSA-with-SHA256` (`1.2.840.10045.4.3.2`, string `"ecdsa-with-SHA256"` at binary offset `0x007ebcd9`) to produce `proof` ([TsCrypt.cs](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/TsCrypt.cs#L753-L760)).
2. **License Chain Key Derivation (`Licenses.DeriveKey`, [License.cs](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/License.cs#L55-L164)):**
   - Starts with 32-byte `LicenseRootKey` (`parent`).
   - License binary blob starts with version byte `0x01`, followed by one or more `LicenseBlock` entries (minimum 42 bytes each: `[0]=0x00`, `[1..33]=Key (32B)`, `[33]=BlockType (0=Intermediate, 2=Server, 32=Ephemeral)`, `[34..38]=NotValidBefore (BE u32 + 0x50e22700)`, `[38..42]=NotValidAfter (BE u32 + 0x50e22700)`).
   - For each block:
     - `hash = SHA512(block_bytes[1..allLen])[0..32]`
     - `sc_clamp(hash)`
     - Unpack Ed25519 points `pubkey = ge_frombytes_negate_vartime(block.Key)` and `parkey = ge_frombytes_negate_vartime(parent)`
     - Compute `new_parent = ge_p3_tobytes(hash * pubkey + parkey)` with `new_parent[31] ^= 0x80`.
3. **Shared Secret (`GetSharedSecret2`, [TsCrypt.cs](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/TsCrypt.cs#L347-L360)):**
   - Given derived 32-byte license leaf public key $K_{\text{lic}}$ and client temporary private key $\text{priv}$:
   - Copy $\text{priv}$ and clear bit 255 (`priv_cpy[31] &= 0x7F`).
   - Decode point $P = \text{ge\_frombytes\_negate\_vartime}(K_{\text{lic}})$.
   - Multiply $M = \text{priv\_cpy} \cdot P$, serialize `sharedTmp = ge_tobytes(M)`, flip sign bit `sharedTmp[31] ^= 0x80`.
   - `sharedKey = SHA512(sharedTmp)` (64 bytes).
4. **`ivStruct` & `fakeSignature` Initialization (`SetSharedSecret`, [TsCrypt.cs](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/TsCrypt.cs#L291-L310)):**
   - `alpha` is 10 bytes; `beta` is 54 bytes (in `initivexpand2`) or 10 bytes (in legacy `initivexpand`).
   - `ivStruct` (`64` bytes for `initivexpand2`, `20` bytes for `initivexpand`):
     - `ivStruct[0..10] = sharedKey[0..10] ^ alpha[0..10]`
     - `ivStruct[10..10+beta.len] = sharedKey[10..10+beta.len] ^ beta[0..beta.len]`
   - `fakeSignature` (8-byte MAC used when `UnencryptedFlag == 1`):
     - `fakeSignature[0..8] = SHA1(ivStruct)[0..8]`

### 3.4 AES-128-EAX Packet Encryption & Per-Packet Key/Nonce Derivation (`[Verified]`, [TsCrypt.cs](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/TsCrypt.cs#L524-L700))

- **Cipher:** `AES-128-EAX` with 16-byte Key, 16-byte Nonce, 8-byte MAC tag (`MacLen = 8`, 64-bit tag), and packet header as Associated Data (AAD).
- **Wire Layout:** `[MAC (8B)] || [Header (5B C2S / 3B S2C)] || [Ciphertext (N B)]`
  - **C2S Header (5 bytes, Big-Endian):** `PacketId (u16 BE) || ClientId (u16 BE) || PacketTypeFlagged (u8)` ([Packet.cs](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/Packet.cs#L126-L133))
  - **S2C Header (3 bytes, Big-Endian):** `PacketId (u16 BE) || PacketTypeFlagged (u8)` ([Packet.cs](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/Packet.cs#L121-L125))
- **Per-Packet Key & Nonce (`GetKeyNonce`, [TsCrypt.cs](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/TsCrypt.cs#L662-L700)):**
  - If dummy encryption (`!CryptoInitComplete`, e.g., `clientek`): uses fixed `DummyKey` and `DummyIv` (Section 4.2).
  - Otherwise, construct `tmpToHash` (`70` bytes for 64-byte `ivStruct`, or `26` bytes for 20-byte `ivStruct`):
    - `tmpToHash[0] = fromServer ? 0x30 ('0') : 0x31 ('1')`
    - `tmpToHash[1] = (byte)(packetType & 0x0F)`
    - `tmpToHash[2..6] = BigEndian_u32(generationId)`
    - `tmpToHash[6..6+ivStruct.len] = ivStruct`
  - Compute `digest = SHA256(tmpToHash)` (32 bytes):
    - `baseKey = digest[0..16]`
    - `nonce = digest[16..32]`
    - `key = baseKey` with `key[0] ^= (byte)(packetId >> 8)` and `key[1] ^= (byte)(packetId & 0xFF)`.

### 3.5 Hashcash / Identity Security Level Algorithm (`[Verified]`, [TsCrypt.cs](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/TsCrypt.cs#L803-L893))

Server config keys at `0x007a85fb` (`virtualserver_min_client_identity_security_level`), `0x007ab4cc`, `0x007ad7f5`.
- Given base64-encoded public key string `omega` (ASCII bytes) and unsigned 64-bit decimal integer `keyOffset` (`client_key_offset`):
- Compute `hash = SHA1(ASCII(omega) || ASCII_Decimal(keyOffset))` (20 bytes).
- Count leading zero bits of `hash` scanning byte-by-byte from index 0, and within the first non-zero byte scanning from **LSB to MSB** (`(hash[i] & (1 << bit)) == 0` for `bit` in `0..7`, [TsCrypt.cs](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/TsCrypt.cs#L879-L892)).

---

## 4. Hardcoded Keys, Fixed IVs & Protocol Magic Constants

All constants below are cross-checked between `ts3server` 3.13.9 and `TSLib`.

| Constant Name | Binary File Offset | Binary VA (Section) | Size | Exact Value (Hex / ASCII) | TSLib Citation |
|---|---|---|---|---|---|
| **`LicenseRootKey`** (Ed25519 Root Public Key) | `0x00808200` | `0x00c08200` (`.rodata`) | 32 B | `cd0de2aed46345509a7e3cfd8f68b3dc7555b29dccec73cd18750f993812408a` | [License.cs:20-24](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/License.cs#L20-L24) |
| **`DummyKeyAndNonceString`** | `0x007b28a4` | `0x00bb28a4` (`.rodata`) | 32 B (+NUL) | ASCII: `"c:\\windows\\system\\firewall32.cpl"`<br>• `DummyKey` (`[0..16]`): `633a5c77696e646f77735c7379737465` (`"c:\\windows\\syste"`)<br>• `DummyIv` (`[16..32]`): `6d5c6669726577616c6c33322e63706c` (`"m\\firewall32.cpl"`) | [TsCrypt.cs:36-39](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/TsCrypt.cs#L36-L39) |
| **100-Byte Hex Prefix before `DummyKeyAndNonceString`** | `0x007b2840` | `0x00bb2840` (`.rodata`) | 100 B | ASCII: `"ee2b91580364c35183fe95432c1a53d8908e744e8a79d4d3e1074b6d1585aa0323ef2645fa7e702c417a1ad4f2622454d494"` (contiguous with `c:\windows\system\firewall32.cpl` in `.rodata`, referenced via pointer at `0x009d91a8` right after `TS3INIT1`) | Verified in binary `.rodata` & `.data` |
| **`Ts3InitMac` (`"TS3INIT1"`)** | `0x009d91a0`<br>`0x000a0eed`<br>`0x000a0f76`<br>`0x000a1077`<br>`0x000a11ba` | `0x00fd91a0` (`.data`)<br>`0x004a0eed` (`.text`)<br>`0x004a0f76` (`.text`)<br>`0x004a1077` (`.text`)<br>`0x004a11ba` (`.text`) | 8 B | ASCII: `"TS3INIT1"` (`545333494e495431`, little-endian `u64` immediate `0x3154494e49335354` in `movabs` instructions building `Init1` packets with S2C packet ID `0x0065` = 101 and flags `0x88`) | [TsCrypt.cs:40](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/TsCrypt.cs#L40) |
| **`TsVersionSignPublicKey`** (Ed25519 Version Sign Key) | `0x007c7591` | `0x00bc7591` (`.rodata`) | 32 B | Hex: `52b3758d7d1d044d6fba54cd2c2a18c2b5697c84f2a3e341baafedc1b7fd84bc`<br>Base64: `"UrN1jX0dBE1vulTNLCoYwrVpfITyo+NBuq/twbf9hLw="` | [TsCrypt.cs:771](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/TsCrypt.cs#L771) |
| **`TsIdentityObfuscationKey`** | *(Client-only)* | *(Absent in server)* | 128 B | ASCII: `"b9dfaa7bee6ac57ac7b65f1094a1c155e747327bc2fe5d51c512023fe54a280201004e90ad1daaae1075d53b7d571c30e063b5a62a4a017bb394833aa0983e6e"` (used only by clients to import/export `<level>V<base64>` INI identities) | [TsCrypt.cs:209-245](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/TsCrypt.cs#L209-L245) |
| **`InitVersion`** (Client `Init1` Header Timestamp) | *(Client-sent)* | *(Checked in `Init1`)* | 4 B | Big-Endian `u32`: `1566914096` (`0x5d653e30`, corresponds to client 3.5.0) prepended to C2S `Init1` steps 0, 2, 4 | [TsCrypt.cs:41, 400](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/TsCrypt.cs#L41) |
| **License Epoch Offset** | *(In code)* | `.text` | 4 B | `0x50e22700` (`1356998400`, Jan 1 2013 00:00:00 UTC) added to 32-bit big-endian timestamps at license block offsets `+34` and `+38` | [License.cs:118-119](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/License.cs#L118-L119) |

---

## 5. Error IDs, Two-Level Lookup Table & Client Error Messages

### 5.1 Binary Error Table Architecture (`[Verified]`)

In `ts3server`, `TsErrorCode` (`u16`, formatted as `error id=<u16> msg=<escaped_str>`) is resolved via a **two-level pointer table**:
- **Master Group Table:** Located in `.data.rel.ro` at file offset `0x009b1020` (VA `0x00fb1020`), containing **21 `u64` pointers** for high-byte error groups `0x00` through `0x14` (`err_group = (error_id >> 8) & 0xFF`, `err_sub = error_id & 0xFF`).
- **Group Sub-Tables:** Located contiguously in `.data` from file offset `0x009d6b80` (VA `0x00fd6b80`) to `0x009d7298` (VA `0x00fd7298`), containing **238 `u64` pointers** to null-terminated error strings in `.rodata` (`0x007a9736`–`0x007aacd0`, VA `0x00ba9736`–`0x00baacd0`).

| Group (`error_id >> 8`) | Master Entry Offset (VA) | Sub-Table `.data` VA (File Offset) | Count | Category |
|---|---|---|---|---|
| `0x00` (General) | `0x009b1020` (`0x00fb1020`) | `0x00fd6b80` (`0x009d6b80`) | 8 | General (`ok`, `undefined error`, `not implemented`) |
| `0x01` (Command/Net) | `0x009b1028` (`0x00fb1028`) | `0x00fd6bc0` (`0x009d6bc0`) | 4 | Command / Network port binding |
| `0x02` (Client) | `0x009b1030` (`0x00fb1030`) | `0x00fd6be0` (`0x009d6be0`) | 18 | Client ID, nickname, login, identity, version, flood |
| `0x03` (Channel) | `0x009b1038` (`0x00fb1038`) | `0x00fd6c70` (`0x009d6c70`) | 16 | Channel ID, flags, limits, passwords |
| `0x04` (Server) | `0x009b1040` (`0x00fb1040`) | `0x00fd6cf0` (`0x009d6cf0`) | 18 | Virtual server status, maxclients, modal quit |
| `0x05` (Database) | `0x009b1048` (`0x00fb1048`) | `0x00fd6d80` (`0x009d6d80`) | 6 | Database error, empty resultset |
| `0x08` (File Transfer) | `0x009b1060` (`0x00fb1060`) | `0x00fd6e70` (`0x009d6e70`) | 26 | File transfer path, quota, transfer key |
| `0x0a` (Permissions) | `0x009b1070` (`0x00fb1070`) | `0x00fd7010` (`0x009d7010`) | 14 | Insufficient client permissions |
| `0x0d` ( Accounting) | `0x009b1088` (`0x00fb1088`) | `0x00fd7100` (`0x009d7100`) | 4 | Accounting / Privilege key (`token`) |
| `0x13` (Certificates) | `0x009b10b8` (`0x00fb10b8`) | `0x00fd71e0` (`0x009d71e0`) | 18 | myTeamSpeak ID / Ed25519 certificate validation |
| `0x14` (Crypto/API) | `0x009b10c0` (`0x00fb10c0`) | `0x00fd7270` (`0x009d7270`) | 6 | `out of scope`, `crypto error`, `invalid apikey` |

### 5.2 Key Client-Relevant Error Codes (`[Verified]`, cross-checked with [TsErrorCode.cs](file:///D:/Dev/TS3AudioBot/mx/TSLib/Generated/TsErrorCode.cs#L24-L400))

| Error ID (Hex) | Error ID (Dec) | Message String in `.rodata` | `.rodata` Offset (VA) | `TSLib.TsErrorCode` Enum Name |
|---|---|---|---|---|
| `0x0000` | `0` | `ok` | `0x007aa6ec` (`0x00baa6ec`) | `ok` |
| `0x0001` | `1` | `undefined error` | `0x007a9736` (`0x00ba9736`) | `undefined` |
| `0x0002` | `2` | `not implemented` | `0x00834ebd` (`0x00c34ebd`) | `not_implemented` |
| `0x0005` | `5` | `library time limit reached` | `0x007a9746` (`0x00ba9746`) | `lib_time_limit_reached` |
| `0x0100` | `256` | `command not found` | `0x007a976f` (`0x00ba976f`) | `command_not_found` |
| `0x0200` | `512` | `invalid clientID` | `0x007b5a5f` (`0x00bb5a5f`) | `client_invalid_id` |
| `0x0201` | `513` | `nickname is already in use` | `0x007a97cb` (`0x00ba97cb`) | `client_nickname_inuse` |
| `0x0203` | `515` | `max clients protocol limit reached` | `0x007a97f9` (`0x00ba97f9`) | `client_protocol_limit_reached` |
| `0x0204` | `516` | `invalid client type` | `0x007a981c` (`0x00ba981c`) | `client_invalid_type` |
| `0x0205` | `517` | `already subscribed` | `0x007a9830` (`0x00ba9830`) | `client_already_subscribed` |
| `0x0206` | `518` | `not logged in` | `0x007a9843` (`0x00ba9843`) | `client_not_logged_in` |
| `0x0207` | `519` | `could not validate client identity` | `0x007a9851` (`0x00ba9851`) | `client_could_not_validate_identity` |
| `0x0208` | `520` | `invalid loginname or password` | `0x007a9874` (`0x00ba9874`) | `client_invalid_password` |
| `0x0209` | `521` | `too many clones already connected` | `0x007a9892` (`0x00ba9892`) | `client_too_many_clones_connected` |
| `0x020a` | `522` | `client version outdated, please update` | `0x007a98b4` (`0x00ba98b4`) | `client_version_outdated` |
| `0x020c` | `524` | `client is flooding` | `0x007a98ec` (`0x00ba98ec`) | `client_is_flooding` |
| `0x020d` | `525` | `client is modified` | `0x007a98ff` (`0x00ba98ff`) | `client_hacked` |
| `0x020e` | `526` | `can not verify client at this moment` | `0x007a9912` (`0x00ba9912`) | `client_cannot_verify_now` |
| `0x020f` | `527` | `client is not permitted to log in` | `0x007a9937` (`0x00ba9937`) | `client_login_not_permitted` |
| `0x0210` | `528` | `client is not subscribed to the channel` | `0x007a9959` (`0x00ba9959`) | `client_not_subscribed` |
| `0x0300` | `768` | `invalid channelID` | `0x007b5239` (`0x00bb5239`) | `channel_invalid_id` |
| `0x0302` | `770` | `already member of channel` | `0x007a99a5` (`0x00ba99a5`) | `channel_already_in` |
| `0x0304` | `772` | `channel not empty` | `0x007a99dd` (`0x00ba99dd`) | `channel_not_empty` |
| `0x0309` | `777` | `channel maxclient reached` | `0x007a9a7d` (`0x00ba9a7d`) | `channel_maxclients_reached` |
| `0x030b` | `779` | `invalid channel password` | `0x007a9ab9` (`0x00ba9ab9`) | `channel_invalid_password` |
| `0x0401` | `1025` | `server is not running` | `0x007a9b1b` (`0x00ba9b1b`) | `server_is_not_running` |
| `0x0402` | `1026` | `server is shutting down` | `0x007a9b31` (`0x00ba9b31`) | `server_is_shutting_down` |
| `0x0403` | `1027` | `max serverquery clients reached` | `0x007a9b49` (`0x00ba9b49`) | `server_maxclients_reached` |
| `0x0404` | `1028` | `invalid server password` | `0x007a9b69` (`0x00ba9b69`) | `server_invalid_password` |
| `0x0a08` | `2568` | `insufficient client permissions` | `0x007aa1f5` (`0x00baa1f5`) | `permissions_client_insufficient` |
| `0x0d00` | `3328` | `virtualserver limit reached` | `0x007aa3d1` (`0x00baa3d1`) | `accounting_virtualserver_limit_reached` |
| `0x130e` | `4878` | `certificate not valid yet` | `0x007aa874` (`0x00baa874`) | *(Server 3.13+ cert check)* |
| `0x130f` | `4879` | `certificate expired` | `0x007aa88e` (`0x00baa88e`) | *(Server 3.13+ cert check)* |
| `0x1401` | `5121` | `crypto error` | `0x007aa8d8` (`0x00baa8d8`) | *(Server 3.13+ handshake crypto error)* |

---

## 6. Version, Build & Platform Sign Strings

### 6.1 Server Build & Version Markers (`[Verified]`)

- **Release Version:** `3.13.9` (`09 Nov 2026`, verified in `CHANGELOG:12` — patches CVE-2026-4391 ECC key parsing crash during connection handshake).
- **Embedded Build Timestamp:** `"Version: 0.4 Nov  9 2026 17:16:59"` at `.rodata` offset `0x007df45d` (VA `0x00bdf45d`).
- **Server Banners in `.rodata`:**
  - `"TeamSpeak 3 Server "` at `0x00808711` (VA `0x00c08711`)
  - `"TeamSpeak Server "` at `0x007cd5a3` (VA `0x00bcd5a3`)
  - `"Welcome to the TeamSpeak 3 ServerQuery interface, type \"help\" for a list of commands and \"help <command>\" for information on a specific command.\n\r"` at `0x007c103b` (VA `0x00bc103b`)

### 6.2 Client Version & Platform Signature Verification (`EdCheck`) (`[Verified]`)

During `clientinit`, the server verifies the `(client_platform, client_version, client_version_sign)` triple using Ed25519 (`TsCrypt.EdCheck`, [TsCrypt.cs](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/TsCrypt.cs#L771-L780), [TsVersion.cs](file:///D:/Dev/TS3AudioBot/mx/TSLib/TsVersion.cs#L19-L91)):
- **Verification Formula:**
  $$\text{Ed25519\_Verify}\big(\text{pubkey} = \text{TsVersionSignPublicKey}, \text{msg} = \text{ASCII}(\text{Platform} \mathbin{\Vert} \text{Version}), \text{sig} = \text{Base64Decode}(\text{Sign})\big)$$
  where `TsVersionSignPublicKey` (`0x007c7591` in `.rodata`) is `52b3758d7d1d044d6fba54cd2c2a18c2b5697c84f2a3e341baafedc1b7fd84bc` (`UrN1jX0dBE1vulTNLCoYwrVpfITyo+NBuq/twbf9hLw=`).
- **Failure Behavior:** If signature verification fails, the server rejects `clientinit` with error `0x020d` (`525`, `"client is modified"`). If the build timestamp inside `Version` (`<semver> [Build: <unix_ts>]`) is below `virtualserver_min_client_version`, the server returns `0x020a` (`522`, `"client version outdated, please update"`).
- **Recommended Signed Version Tuples for Rust Client (`[Verified]`, [TsVersion.gen.cs](file:///D:/Dev/TS3AudioBot/mx/TSLib/Generated/TsVersion.gen.cs#L26-L36)):**

| Platform | `client_platform` | `client_version` | `client_version_sign` (64-byte Ed25519 signature, Base64) |
|---|---|---|---|
| **Linux (3.5.3 Stable)** | `Linux` | `3.5.3 [Build: 1587971024]` | `59chu1YQ1W4DdVj+yJOVmwhJK7s9p9FatqIjQRxEDmIb0CcDDO/K8CrbVnBHbD67/cExJbC3PjC/o/n0pDbiCg==` |
| **Windows (3.5.3 Stable)** | `Windows` | `3.5.3 [Build: 1587971024]` | `Kvmj7qX6wJCPI5GVT71samfmhz/bvs7M+OTXWB/JWxdQbxDe17xda7dzUWLX7pjvdJTqZmbse1HBmTxThPKvAg==` |
| **macOS (3.5.3 Stable)** | `OS X` | `3.5.3 [Build: 1586955962]` | `/9gsbyyJoyHW1okwykGgXrn2j29wvnEfnw6/Hvft2WS12CURxTv5L9uLuu36I5u3TzWOxyB9dEbD1FeZ201uCQ==` |
| **Linux (3.?.? Universal)** | `Linux` | `3.?.? [Build: 5680278000]` | `Hjd+N58Gv3ENhoKmGYy2bNRBsNNgm5kpiaQWxOj5HN2DXttG6REjymSwJtpJ8muC2gSwRuZi0R+8Laan5ts5CQ==` |
