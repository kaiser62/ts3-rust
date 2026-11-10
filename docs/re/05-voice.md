# TeamSpeak 3 Protocol Specification: Voice & Whisper Subsystem

**Document Version:** 1.0.0  
**Scope:** Voice send/receive transmission, whisper routing, codec configuration, packet layouts, encryption rules, jitter/playback management, and native Rust implementation architecture.  
**Primary References:**
- TSLib C# Reference: `D:\Dev\TS3AudioBot\mx\TSLib\`
  - Audio: [`AudioPacketReader.cs`](file:///D:/Dev/TS3AudioBot/mx/TSLib/Audio/AudioPacketReader.cs), [`EncoderPipe.cs`](file:///D:/Dev/TS3AudioBot/mx/TSLib/Audio/EncoderPipe.cs), [`DecoderPipe.cs`](file:///D:/Dev/TS3AudioBot/mx/TSLib/Audio/DecoderPipe.cs), [`PreciseTimedPipe.cs`](file:///D:/Dev/TS3AudioBot/mx/TSLib/Audio/PreciseTimedPipe.cs), [`AudioMeta.cs`](file:///D:/Dev/TS3AudioBot/mx/TSLib/Audio/AudioMeta.cs)
  - Full Protocol: [`TsFullClient.cs`](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/TsFullClient.cs), [`PacketHandler.cs`](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/PacketHandler.cs), [`Packet.cs`](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/Packet.cs), [`PacketType.cs`](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/PacketType.cs), [`TsCrypt.cs`](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/TsCrypt.cs), [`GenerationWindow.cs`](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/GenerationWindow.cs)
  - Enums: [`TsEnums.cs`](file:///D:/Dev/TS3AudioBot/mx/TSLib/TsEnums.cs)
- TS3 Server Linux amd64 3.13.9: `D:\Dev\ts3-rust\.scratch\teamspeak3-server_linux_amd64\`
  - Binary strings / assertions (`ts3server`): `sound/decode.cpp`, `sound/decodemanager.cpp`
  - Docs: [`serverquery.html`](file:///D:/Dev/ts3-rust/.scratch/teamspeak3-server_linux_amd64/doc/serverquery/serverquery.html), [`channeledit.txt`](file:///D:/Dev/ts3-rust/.scratch/teamspeak3-server_linux_amd64/serverquerydocs/channeledit.txt)

---

## 1. Protocol Overview & Wire Transport

Voice transmission in TeamSpeak 3 operates over UDP (default port 9987). Unlike control commands, voice packets are **unreliable datagrams**:
1. **No ACKs**: The receiver does **not** acknowledge voice packets ([`PacketHandler.cs:399-406`](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/PacketHandler.cs#L399-L406)).
2. **No Resends**: Lost voice frames are discarded; latency is prioritized over reliability ([`PacketHandler.cs:196-200`](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/PacketHandler.cs#L196-L200)).
3. **No Splitting/Fragmentation for Standard Voice**: Voice packets must fit within the MTU. If a voice packet exceeds allowed size, transmission fails immediately (`"Voice packet too big"`, [`PacketHandler.cs:199-200`](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/PacketHandler.cs#L199-L200)).

### 1.1 Complete UDP Packet Envelopes

Every UDP datagram consists of an 8-byte Message Authentication Code (MAC), a transport header, and the voice payload.

#### Client to Server (C -> S) Outer Packet Layout
Total Header Size = 13 bytes (`8B MAC + 5B C2S Header`) [[`Packet.cs:186`](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/Packet.cs#L186)]:
```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                            MAC (0..3)                         |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                            MAC (4..7)                         |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|        Packet ID (BE)         |        Client ID (BE)         |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| PacketType/Fl |            Voice Payload ...                  |
+-+-+-+-+-+-+-+-+                                               |
```
- **Byte 0..7 (8 bytes)**: MAC.
  - If unencrypted: `fakeSignature` (8 bytes derived during crypto handshake from `ivStruct` hash, [`TsCrypt.cs:303-306, 531-534`](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/TsCrypt.cs#L303-L306)).
  - If encrypted: 8-byte AES-128-EAX tag ([`TsCrypt.cs:537-570`](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/TsCrypt.cs#L537-L570)).
- **Byte 8..9 (2 bytes)**: `PacketId` (`u16`, Big-Endian network byte order). Increments per outgoing packet of this type ([`PacketHandler.cs:255-258`](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/PacketHandler.cs#L255-L258)).
- **Byte 10..11 (2 bytes)**: `ClientId` (`u16`, Big-Endian network byte order). The sender client ID assigned by the server in `initserver` ([`Packet.cs:128-132, 154-158`](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/Packet.cs#L128-L132), [`PacketHandler.cs:259-264`](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/PacketHandler.cs#L259-L264)).
- **Byte 12 (1 byte)**: `PacketTypeFlagged` (`u8`):
  - Bits 0..3: `PacketType` (`Voice = 0x0`, `VoiceWhisper = 0x1`, [`PacketType.cs:16-17`](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/PacketType.cs#L16-L17)).
  - Bits 4..7: `PacketFlags` (`Newprotocol = 0x20`, `Compressed = 0x40`, `Unencrypted = 0x80`, [`PacketType.cs:28-32`](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/PacketType.cs#L28-L32)).
- **Byte 13..End**: Voice payload (decrypted/plaintext).

#### Server to Client (S -> C) Outer Packet Layout
Total Header Size = 11 bytes (`8B MAC + 3B S2C Header`) [[`Packet.cs:193`](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/Packet.cs#L193)]:
```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                            MAC (0..3)                         |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                            MAC (4..7)                         |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|        Packet ID (BE)         | PacketType/Fl | Voice Payload |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+               |
|                           ...                                 |
```
- **Byte 0..7 (8 bytes)**: MAC (EAX tag or `fakeSignature`).
- **Byte 8..9 (2 bytes)**: `PacketId` (`u16`, Big-Endian network byte order).
- **Byte 10 (1 byte)**: `PacketTypeFlagged` (`u8`):
  - Bits 0..3: `PacketType` (`Voice = 0x0`, `VoiceWhisper = 0x1`).
  - Bits 4..7: `PacketFlags` (`Unencrypted = 0x80`, etc.).
- **Byte 11..End**: Voice payload (decrypted/plaintext).

---

## 2. Voice Payload Formats

The voice payload resides inside the decrypted `Packet.Data` buffer.

### 2.1 C -> S Standard Voice Payload (`PacketType::Voice = 0x0`)
Defined in [`TsFullClient.cs:581-595`](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/TsFullClient.cs#L581-L595) and [`PacketHandler.cs:272`](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/PacketHandler.cs#L272):

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|        Voice ID (BE)          |   Codec (u8)  | Audio Frame...|
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                           ...                                 |
```

| Offset (bytes) | Field Name | Type | Description |
|---|---|---|---|
| `0..2` (2B) | `Voice ID` | `u16` (Big-Endian) | Audio sequence counter. In TSLib, matches outgoing `PacketId` ([`PacketHandler.cs:272`](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/PacketHandler.cs#L272)). |
| `2` (1B) | `Codec ID` | `u8` | Codec enum value (e.g. `0x04` for OpusVoice, `0x05` for OpusMusic). |
| `3..End` | `Audio Data` | `[u8]` | Encoded audio frame (e.g., Opus packet). |

### 2.2 S -> C Voice & Whisper Payload (`PacketType::Voice` / `PacketType::VoiceWhisper`)
Defined in [`AudioPacketReader.cs:20-32`](file:///D:/Dev/TS3AudioBot/mx/TSLib/Audio/AudioPacketReader.cs#L20-L32):

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|        Voice ID (BE)          |       Sender Client ID (BE)   |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|   Codec (u8)  |                 Audio Frame Data ...          |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                           ...                                 |
```

| Offset (bytes) | Field Name | Type | Description |
|---|---|---|---|
| `0..2` (2B) | `Voice ID` | `u16` (Big-Endian) | Audio packet counter assigned by the original speaker. |
| `2..4` (2B) | `Sender Client ID` | `u16` (Big-Endian) | The server-assigned `ClientId` of the speaker. |
| `4` (1B) | `Codec ID` | `u8` | Codec identifier (e.g., `0x04` OpusVoice, `0x05` OpusMusic). |
| `5..End` | `Audio Data` | `[u8]` | Encoded audio payload. |

---

## 3. Whisper Packet Specifications

Whisper enables transmitting audio directly to specified channels, individual clients, or logical groups rather than broadcasting only to the current channel.

### 3.1 C -> S Directed Whisper (`PacketType::VoiceWhisper = 0x1`, Flags: Standard)
Defined in [`TsFullClient.cs:597-622`](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/TsFullClient.cs#L597-L622):

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|        Voice ID (BE)          |   Codec (u8)  |  NumChan (N)  |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|  NumCli (M)   | Channel ID 0 (u64 BE) ...                     |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| ... Channel ID (N-1) (u64 BE)                                 |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| Client ID 0 (u16 BE)          | Client ID 1 (u16 BE) ...      |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| ... Client ID (M-1) (u16 BE)  | Audio Frame Data ...          |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

#### Field Layout
1. `[0..2]` (2B): `Voice ID` (`u16`, Big-Endian).
2. `[2]` (1B): `Codec ID` (`u8`).
3. `[3]` (1B): `channel_count` ($N$, `u8`): Count of target channel IDs.
4. `[4]` (1B): `client_count` ($M$, `u8`): Count of target client IDs.
5. `[5 .. 5 + N*8]` ($N \times 8$ bytes): Array of targeted `ChannelId` values, each `u64` Big-Endian ([`TsFullClient.cs:615-616`](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/TsFullClient.cs#L615-L616)).
6. `[5 + N*8 .. 5 + N*8 + M*2]` ($M \times 2$ bytes): Array of targeted `ClientId` values, each `u16` Big-Endian ([`TsFullClient.cs:617-618`](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/TsFullClient.cs#L617-L618)).
7. `[5 + N*8 + M*2 .. End]`: Encoded audio frame data.

*Header Flags:* Does **not** set `PacketFlags::Newprotocol`.

### 3.2 C -> S New-Style Group Whisper (`PacketType::VoiceWhisper = 0x1`, Flags: `Newprotocol = 0x20`)
Defined in [`TsFullClient.cs:624-644`](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/TsFullClient.cs#L624-L644):

Targeting logical roles (e.g., all Channel Commanders, Server Groups) uses a fixed 13-byte routing header and sets `PacketFlags::Newprotocol` (`0x20`) on the outer transport header ([`TsFullClient.cs:643`](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/TsFullClient.cs#L643)).

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|        Voice ID (BE)          |   Codec (u8)  | GroupType(u8) |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| GrpTarget(u8) |           Target ID (u64 BE) (0..3)           |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|           Target ID (u64 BE) (4..7)           | Audio Data... |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                           ...                                 |
```

#### Field Layout
1. `[0..2]` (2B): `Voice ID` (`u16`, Big-Endian).
2. `[2]` (1B): `Codec ID` (`u8`).
3. `[3]` (1B): `GroupWhisperType` (`u8`) [[`TsEnums.cs:89-101`](file:///D:/Dev/TS3AudioBot/mx/TSLib/TsEnums.cs#L89-L101)]:
   - `0x00` = `ServerGroup`: Targets users in the specified server group (requires `Target ID` = `ServerGroupId`).
   - `0x01` = `ChannelGroup`: Targets users in the specified channel group (requires `Target ID` = `ChannelGroupId`).
   - `0x02` = `ChannelCommander`: Targets all clients with Channel Commander status.
   - `0x03` = `AllClients`: Broadcast whisper to all clients on virtual server.
4. `[4]` (1B): `GroupWhisperTarget` (`u8`) [[`TsEnums.cs:103-112`](file:///D:/Dev/TS3AudioBot/mx/TSLib/TsEnums.cs#L103-L112)]:
   - `0x00` = `AllChannels`
   - `0x01` = `CurrentChannel`
   - `0x02` = `ParentChannel`
   - `0x03` = `AllParentChannel`
   - `0x04` = `ChannelFamily`
   - `0x05` = `CompleteChannelFamily`
   - `0x06` = `Subchannels`
5. `[5..13]` (8B): `Target ID` (`u64`, Big-Endian): Group ID or Channel ID associated with the whisper target, or `0` if not applicable ([`TsFullClient.cs:640`](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/TsFullClient.cs#L640)).
6. `[13..End]`: Encoded audio frame data.

*Header Flags:* `PacketFlags::Newprotocol` (`0x20`) is mandatory ([`TsFullClient.cs:643`](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/TsFullClient.cs#L643)).

### 3.3 S -> C Whisper Delivery
When a whisper reaches an eligible listener, the server sends a standard S -> C Voice packet where `PacketType` is set to `0x01` (`VoiceWhisper`) instead of `0x00` (`Voice`). The payload layout is identical to section 2.2 (`[VoiceId (2B)][ClientId (2B)][Codec (1B)][Audio (NB)]`). The recipient client uses `PacketType == VoiceWhisper` to display visual whisper indicators (e.g. red talk icon) and route whisper sound cues ([`TsFullClient.cs:231-240`](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/TsFullClient.cs#L231-L240)).

---

## 4. Codec Identifiers & Opus Audio Specifications

### 4.1 Codec Identifier Enumeration
Source: [`TsEnums.cs:154-171`](file:///D:/Dev/TS3AudioBot/mx/TSLib/TsEnums.cs#L154-L171) & `ts3server` (`sound/decode.cpp`):

```rust
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TsCodec {
    /// Mono, 16-bit, 8 kHz Speex (Legacy)
    SpeexNarrowband = 0,
    /// Mono, 16-bit, 16 kHz Speex (Legacy)
    SpeexWideband = 1,
    /// Mono, 16-bit, 32 kHz Speex (Legacy)
    SpeexUltraWideband = 2,
    /// Mono, 16-bit, 48 kHz CELT (Deprecated)
    CeltMono = 3,
    /// Mono, 16-bit, 48 kHz Opus VoIP
    OpusVoice = 4,
    /// Stereo, 16-bit, 48 kHz Opus Fullband Audio
    OpusMusic = 5,
    /// Raw PCM S16LE (TSLib extension only; rejected by official clients)
    Raw = 127,
}
```

### 4.2 Opus Audio Parameters

Modern TeamSpeak 3 servers and clients use Opus (`OpusVoice` and `OpusMusic`).

| Parameter | `OpusVoice` (`0x04`) | `OpusMusic` (`0x05`) | Reference |
|---|---|---|---|
| **Sample Rate** | 48,000 Hz | 48,000 Hz | [`EncoderPipe.cs:56, 63`](file:///D:/Dev/TS3AudioBot/mx/TSLib/Audio/EncoderPipe.cs#L56) |
| **Channels** | 1 (Mono) | 2 (Stereo Interleaved) | [`EncoderPipe.cs:57, 64`](file:///D:/Dev/TS3AudioBot/mx/TSLib/Audio/EncoderPipe.cs#L57) |
| **Sample Format** | 16-bit Signed Little-Endian (`i16`) | 16-bit Signed Little-Endian (`i16`) | [`EncoderPipe.cs:75`](file:///D:/Dev/TS3AudioBot/mx/TSLib/Audio/EncoderPipe.cs#L75) |
| **Frame Duration** | 20 ms (0.020 s) | 20 ms (0.020 s) | [`EncoderPipe.cs:33`](file:///D:/Dev/TS3AudioBot/mx/TSLib/Audio/EncoderPipe.cs#L33) |
| **Samples per Frame** | 960 samples | 960 samples per channel | `48000 * 0.020 = 960` |
| **Uncompressed Frame Bytes**| 1,920 bytes (`960 * 1 * 2`) | 3,840 bytes (`960 * 2 * 2`) | [`EncoderPipe.cs:76`](file:///D:/Dev/TS3AudioBot/mx/TSLib/Audio/EncoderPipe.cs#L76) |
| **Opus Application** | `OPUS_APPLICATION_VOIP` (`2048`) | `OPUS_APPLICATION_AUDIO` (`2049`) | [`EncoderPipe.cs:58, 65`](file:///D:/Dev/TS3AudioBot/mx/TSLib/Audio/EncoderPipe.cs#L58) |
| **Default Bitrate** | 16,384 – 32,000 bps | 32,768 – 96,000+ bps | [`EncoderPipe.cs:59, 66`](file:///D:/Dev/TS3AudioBot/mx/TSLib/Audio/EncoderPipe.cs#L59) |
| **Max Encoded Size** | 1,275 bytes (RFC 6716 §3.2.1) | 1,275 bytes (RFC 6716 §3.2.1) | [`EncoderPipe.cs:38`](file:///D:/Dev/TS3AudioBot/mx/TSLib/Audio/EncoderPipe.cs#L38) |

---

## 5. End-of-Transmission (EOT) Empty Packet

When a user stops speaking (e.g. releases Push-to-Talk or voice activation threshold falls below silence cutoff):
1. **Empty Audio Packet**: The client dispatches an audio packet where the audio data payload is **0 bytes** length ([`AudioPacketReader.cs:23-26`](file:///D:/Dev/TS3AudioBot/mx/TSLib/Audio/AudioPacketReader.cs#L23-L26)).
2. **C -> S EOT Layout**: Exactly **3 bytes** payload:
   ```
   [Voice ID (2B BE)][Codec ID (1B)]
   ```
3. **S -> C EOT Layout**: Exactly **5 bytes** payload:
   ```
   [Voice ID (2B BE)][Sender Client ID (2B BE)][Codec ID (1B)]
   ```
4. **Reader Detection Condition**:
   ```csharp
   // AudioPacketReader.cs:23-26
   // End of stream is signalled with no data or a single byte.
   // The header has 5 bytes, so check for 6.
   if (data.Length < 6)
       return; // Handle end-of-stream / silence transition
   ```
5. **Receiver Action**:
   - Resets talking state for `Sender Client ID` in the UI (turns off blue/glow icon).
   - Flushes remaining jitter buffer frames for that client.
   - Cleans up or resets the client's `OpusDecoder` state.

---

## 6. Voice Encryption Architecture

Voice payload encryption is governed by both virtual server settings and channel configuration.

### 6.1 Server & Channel Encryption Configuration

Sources: [`TsEnums.cs:173-181`](file:///D:/Dev/TS3AudioBot/mx/TSLib/TsEnums.cs#L173-L181), [`serverquery.html:3057`](file:///D:/Dev/ts3-rust/.scratch/teamspeak3-server_linux_amd64/doc/serverquery/serverquery.html#L3057), [`channeledit.txt`](file:///D:/Dev/ts3-rust/.scratch/teamspeak3-server_linux_amd64/serverquerydocs/channeledit.txt).

The virtual server sets `virtualserver_codec_encryption_mode`:
```rust
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodecEncryptionMode {
    /// 0 = Individual: Encryption decided per channel by channel_codec_is_unencrypted
    Individual = 0,
    /// 1 = Disabled: Voice encryption is globally disabled on the server
    Disabled = 1,
    /// 2 = Enabled: Voice encryption is globally enforced on all channels
    Enabled = 2,
}
```

The active channel sets `channel_codec_is_unencrypted` (boolean `0` or `1`).

#### Resolution Logic
```rust
pub fn is_voice_encrypted(server_mode: CodecEncryptionMode, channel_is_unencrypted: bool) -> bool {
    match server_mode {
        CodecEncryptionMode::Disabled => false,
        CodecEncryptionMode::Enabled => true,
        CodecEncryptionMode::Individual => !channel_is_unencrypted,
    }
}
```

### 6.2 Wire Differences: Encrypted vs. Unencrypted Voice

#### Case A: Unencrypted Voice (`PacketFlags::Unencrypted = 0x80`)
1. **Header Flag**: Bit 7 of `PacketTypeFlagged` is set (`flags |= 0x80`, [`PacketHandler.cs:271`](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/PacketHandler.cs#L271)).
2. **MAC Field (8 bytes)**: Contains `fakeSignature`.
   - Computed during handshake completion:
     $$\text{fakeSignature} = \text{Hash1It}(\text{ivStruct})[0..8]$$
     ([`TsCrypt.cs:303-306`](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/TsCrypt.cs#L303-L306)).
3. **Payload**: Plaintext bytes ([`TsCrypt.cs:531-534, 573-583`](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/TsCrypt.cs#L531-L534)).

#### Case B: Encrypted Voice (`PacketFlags::Unencrypted` Cleared)
1. **Header Flag**: Bit 7 of `PacketTypeFlagged` is 0.
2. **Cipher Algorithm**: AES-128-EAX.
3. **Key and Nonce Derivation**:
   `GetKeyNonce(fromServer, packetId, generationId, PacketType::Voice, ...)` derives a 16-byte key and 16-byte nonce from the master `sharedKey` and `ivStruct` ([`TsCrypt.cs:537`](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/TsCrypt.cs#L537)).
4. **Associated Data (AAD)**: The transport header (5 bytes for C2S, 3 bytes for S2C) is passed as AAD to the EAX cipher ([`TsCrypt.cs:539`](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/TsCrypt.cs#L539)).
5. **MAC Field (8 bytes)**: The 8-byte EAX tag emitted by `DoFinal` ([`TsCrypt.cs:565`](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/TsCrypt.cs#L565)).
6. **Payload**: AES-128-EAX ciphertext.

---

## 7. Generation Window & Jitter Management

### 7.1 Generation Window (`GenerationWindow`)
Source: [`GenerationWindow.cs:12-101`](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/GenerationWindow.cs#L12-L101).

Because UDP packets can arrive out of order and `PacketId` is a 16-bit integer (`0..65535`), the client maintains a generation sliding window:
- **Modulus**: $65536$ (`ushort.MaxValue + 1`).
- **Window Size**: $32768$ (`Mod / 2`).
- **Generation ID**: Incremented when `PacketId` wraps around 65535.
- Used to construct the 64-bit sequence counter required for AES-EAX nonce calculation and replay prevention.

```rust
pub struct GenerationWindow {
    base_offset: u32,
    generation: u32,
    modulus: u32,       // 65536
    window_size: u32,   // 32768
}

impl GenerationWindow {
    pub fn new() -> Self {
        Self { base_offset: 0, generation: 0, modulus: 65536, window_size: 32768 }
    }

    pub fn get_generation(&self, packet_id: u16) -> u32 {
        let mapped = packet_id as u32;
        let is_next_gen = self.base_offset > (self.modulus - self.window_size)
            && mapped < (self.base_offset + self.window_size) - self.modulus;
        self.generation + if is_next_gen { 1 } else { 0 }
    }

    pub fn is_in_window(&self, packet_id: u16) -> bool {
        let mapped = packet_id as u32;
        let max_offset = self.base_offset + self.window_size;
        if max_offset < self.modulus {
            mapped >= self.base_offset && mapped < max_offset
        } else {
            mapped >= self.base_offset || mapped < (max_offset - self.modulus)
        }
    }

    pub fn advance_to(&mut self, packet_id: u16) {
        let mapped = packet_id as u32;
        let move_dist = (mapped as i64 - self.base_offset as i64) + 1;
        if move_dist <= 0 { return; }
        let new_base = self.base_offset + move_dist as u32;
        if new_base >= self.modulus {
            self.generation += new_base / self.modulus;
            self.base_offset = new_base % self.modulus;
        } else {
            self.base_offset = new_base;
        }
    }
}
```

### 7.2 Jitter Buffer & Packet Loss Concealment (PLC)

1. **Jitter Target**: Fixed or adaptive 40–60 ms buffer (2 to 3 packets of 20 ms).
2. **Reordering**: Incoming voice packets are sorted by `VoiceId`.
3. **Opus PLC**: When a packet is dropped or arrives past the playback deadline, invoke the Opus decoder with `None` or `NULL` data. Opus internally reconstructs the missing 20 ms frame using pitch prediction and energy interpolation.
4. **Per-Client State**: Keep separate decoder and jitter instances per `Sender Client ID` ([`DecoderPipe.cs:25`](file:///D:/Dev/TS3AudioBot/mx/TSLib/Audio/DecoderPipe.cs#L25)). Clean up idle decoders after 10–30 seconds of silence.

---

## 8. Rust Implementation Architecture

### 8.1 Recommended Crates

```toml
[dependencies]
# Opus codec bindings
audiopus = "0.2"            # Pure safe Rust bindings to libopus
# Alternatively: opus = "0.3"

# Audio hardware I/O
cpal = "0.15"               # Cross-platform audio capture and playback

# Concurrency & Lock-free Ringbuffers
ringbuf = "0.4"             # SPSC lock-free buffer between audio thread and network
crossbeam-channel = "0.5"

# Resampling (if sound card does not support native 48kHz)
rubato = "0.15"

# Cryptography (AES-EAX & SHA1)
eax = "0.5"
aes = "0.8"
sha1 = "0.10"
byteorder = "1.5"
```

### 8.2 Voice Pipeline Architecture Diagram

```
+-----------------------------------------------------------------------------+
|                                TX PIPELINE                                  |
|                                                                             |
|  [ Microphone (cpal) ] (48 kHz, Mono S16LE)                                 |
|            |                                                                |
|            v                                                                |
|  [ Frame Accumulator ] (Accumulate 960 samples = 20ms = 1920 bytes)         |
|            |                                                                |
|            v                                                                |
|  [ Opus Encoder (audiopus) ] -> Encoded Frame (e.g. 40-120 bytes)           |
|            |                                                                |
|            v                                                                |
|  [ Payload Builder ] -> [VoiceId: u16][Codec: 4][Opus Data]                 |
|            |                                                                |
|            v                                                                |
|  [ Encryptor (AES-EAX) / Plaintext ] -> [MAC: 8B][Header: 5B][Payload]      |
|            |                                                                |
|            v                                                                |
|  [ UDP Socket send_to ] (Port 9987)                                         |
+-----------------------------------------------------------------------------+

+-----------------------------------------------------------------------------+
|                                RX PIPELINE                                  |
|                                                                             |
|  [ UDP Socket recv_from ]                                                   |
|            |                                                                |
|            v                                                                |
|  [ Decryptor / FakeSig Verify ] -> PacketType (0=Voice, 1=Whisper)          |
|            |                                                                |
|            v                                                                |
|  [ S->C Payload Demux ]                                                     |
|       VoiceId: u16, ClientId: u16, Codec: u8, Audio: &[u8]                  |
|            |                                                                |
|       len < 6? ---> (Yes: Signal Talk Stopped / Flush Client Decoder)       |
|            | No                                                             |
|            v                                                                |
|  [ Per-Client Jitter Buffer ] (Order by VoiceId, Detect Loss)               |
|            |                                                                |
|            v                                                                |
|  [ Opus Decoder (audiopus) ] (Decode frame or pass None for PLC)            |
|            |                                                                |
|            v (Mono to Stereo Expansion if OpusVoice)                        |
|  [ Software Audio Mixer ] (Sum PCM samples with clipping protection)        |
|            |                                                                |
|            v                                                                |
|  [ Ring Buffer (ringbuf) ]                                                  |
|            |                                                                |
|            v                                                                |
|  [ Speaker Playback (cpal) ] (48 kHz, Stereo S16LE)                         |
+-----------------------------------------------------------------------------+
```

### 8.3 Minimal Rust Implementation

#### A. Voice Payload Definitions
```rust
use byteorder::{BigEndian, ByteOrder};

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TsCodec {
    SpeexNarrowband = 0,
    SpeexWideband = 1,
    SpeexUltraWideband = 2,
    CeltMono = 3,
    OpusVoice = 4,
    OpusMusic = 5,
}

pub struct VoicePacketOut<'a> {
    pub voice_id: u16,
    pub codec: TsCodec,
    pub data: &'a [u8],
}

impl<'a> VoicePacketOut<'a> {
    pub fn serialize(&self, buf: &mut [u8]) -> usize {
        assert!(buf.len() >= 3 + self.data.len());
        BigEndian::write_u16(&mut buf[0..2], self.voice_id);
        buf[2] = self.codec as u8;
        buf[3..3 + self.data.len()].copy_from_slice(self.data);
        3 + self.data.len()
    }
}

pub struct WhisperPacketOut<'a> {
    pub voice_id: u16,
    pub codec: TsCodec,
    pub channel_ids: &'a [u64],
    pub client_ids: &'a [u16],
    pub data: &'a [u8],
}

impl<'a> WhisperPacketOut<'a> {
    pub fn serialize(&self, buf: &mut [u8]) -> usize {
        let header_len = 5 + self.channel_ids.len() * 8 + self.client_ids.len() * 2;
        assert!(buf.len() >= header_len + self.data.len());
        
        BigEndian::write_u16(&mut buf[0..2], self.voice_id);
        buf[2] = self.codec as u8;
        buf[3] = self.channel_ids.len() as u8;
        buf[4] = self.client_ids.len() as u8;

        let mut offset = 5;
        for &cid in self.channel_ids {
            BigEndian::write_u64(&mut buf[offset..offset + 8], cid);
            offset += 8;
        }
        for &clid in self.client_ids {
            BigEndian::write_u16(&mut buf[offset..offset + 2], clid);
            offset += 2;
        }
        buf[offset..offset + self.data.len()].copy_from_slice(self.data);
        offset + self.data.len()
    }
}

pub struct VoicePacketIn<'a> {
    pub voice_id: u16,
    pub sender_client_id: u16,
    pub codec: TsCodec,
    pub audio_data: &'a [u8],
    pub is_eot: bool,
}

impl<'a> VoicePacketIn<'a> {
    pub fn parse(payload: &'a [u8]) -> Option<Self> {
        if payload.len() < 5 {
            // End of transmission or malformed
            return None;
        }
        let voice_id = BigEndian::read_u16(&payload[0..2]);
        let sender_client_id = BigEndian::read_u16(&payload[2..4]);
        let codec_byte = payload[4];
        let codec = match codec_byte {
            0 => TsCodec::SpeexNarrowband,
            1 => TsCodec::SpeexWideband,
            2 => TsCodec::SpeexUltraWideband,
            3 => TsCodec::CeltMono,
            4 => TsCodec::OpusVoice,
            5 => TsCodec::OpusMusic,
            _ => return None,
        };

        let is_eot = payload.len() < 6;
        let audio_data = if is_eot { &[] } else { &payload[5..] };

        Some(Self {
            voice_id,
            sender_client_id,
            codec,
            audio_data,
            is_eot,
        })
    }
}
```

#### B. 20ms Frame Timing & Audio Dispatch
```rust
use std::time::{Duration, Instant};

pub struct PreciseAudioTimer {
    interval: Duration, // 20ms
    next_tick: Instant,
}

impl PreciseAudioTimer {
    pub fn new() -> Self {
        Self {
            interval: Duration::from_millis(20),
            next_tick: Instant::now(),
        }
    }

    pub fn wait_next_frame(&mut self) {
        let now = Instant::now();
        if now < self.next_tick {
            std::thread::sleep(self.next_tick - now);
        }
        self.next_tick += self.interval;
        if Instant::now() > self.next_tick + Duration::from_millis(100) {
            // Prevent drift catch-up storm if paused
            self.next_tick = Instant::now() + self.interval;
        }
    }
}
```

---

## 9. Verification Summary & Field Provenance

| Component | Observation / Value | Primary Source Reference | Verification Status |
|---|---|---|---|
| **C2S Outer Header** | 13 bytes (`8B MAC + 2B ID + 2B Clid + 1B Type/Flags`) | [`Packet.cs:128-132, 186`](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/Packet.cs#L128-L132) | **Verified** |
| **S2C Outer Header** | 11 bytes (`8B MAC + 2B ID + 1B Type/Flags`) | [`Packet.cs:123-126, 193`](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/Packet.cs#L123-L126) | **Verified** |
| **C->S Voice Payload** | `[VoiceId: 2B BE][Codec: 1B][Data: NB]` | [`TsFullClient.cs:586-594`](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/TsFullClient.cs#L586-L594) | **Verified** |
| **S->C Voice Payload** | `[VoiceId: 2B BE][ClientId: 2B BE][Codec: 1B][Data: NB]` | [`AudioPacketReader.cs:27-31`](file:///D:/Dev/TS3AudioBot/mx/TSLib/Audio/AudioPacketReader.cs#L27-L31) | **Verified** |
| **C->S Whisper Payload**| `[VoiceId: 2B][Codec: 1B][N: 1B][M: 1B][N*8B Cids][M*2B Clids][Data]` | [`TsFullClient.cs:602-621`](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/TsFullClient.cs#L602-L621) | **Verified** |
| **Group Whisper Payload**| `[VoiceId: 2B][Codec: 1B][Type: 1B][Target: 1B][TargetId: 8B][Data]` | [`TsFullClient.cs:629-644`](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/TsFullClient.cs#L629-L644) | **Verified** |
| **Group Whisper Flag** | Outer `PacketFlags::Newprotocol` (`0x20`) is mandatory | [`TsFullClient.cs:643`](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/TsFullClient.cs#L643) | **Verified** |
| **End of Transmission** | Payload size `< 6` bytes (3 bytes C->S, 5 bytes S->C) | [`AudioPacketReader.cs:23-26`](file:///D:/Dev/TS3AudioBot/mx/TSLib/Audio/AudioPacketReader.cs#L23-L26) | **Verified** |
| **Opus Voice Specs** | 48 kHz, Mono, 20 ms frames (960 samples, 1920B raw S16LE) | [`EncoderPipe.cs:56-61`](file:///D:/Dev/TS3AudioBot/mx/TSLib/Audio/EncoderPipe.cs#L56-L61) | **Verified** |
| **Opus Music Specs** | 48 kHz, Stereo, 20 ms frames (960 samples/ch, 3840B raw S16LE) | [`EncoderPipe.cs:63-68`](file:///D:/Dev/TS3AudioBot/mx/TSLib/Audio/EncoderPipe.cs#L63-L68) | **Verified** |
| **Voice Encryption** | Disabled (`1`), Enabled (`2`), Individual (`0` via channel flag) | [`TsEnums.cs:173-181`](file:///D:/Dev/TS3AudioBot/mx/TSLib/TsEnums.cs#L173-L181), `ts3server` | **Verified** |
| **Fake Signature** | 8 bytes: `Hash1It(ivStruct)[0..8]` for unencrypted packets | [`TsCrypt.cs:303-306, 531-534`](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/TsCrypt.cs#L303-L306) | **Verified** |
| **Jitter Window** | `GenerationWindow` mod 65536, win 32768, no ACKs | [`GenerationWindow.cs:20`](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/GenerationWindow.cs#L20), [`PacketHandler.cs:401`](file:///D:/Dev/TS3AudioBot/mx/TSLib/Full/PacketHandler.cs#L401) | **Verified** |
