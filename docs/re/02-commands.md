# TeamSpeak 3 Voice-Client Command & Notification Catalog and Escaping Specification

**Target Server Version**: TeamSpeak 3 Server `3.13.9` (Linux `amd64` binary at `D:\Dev\ts3-rust\.scratch\teamspeak3-server_linux_amd64\ts3server`)  
**Primary Reference Implementation**: `TSLib` (`D:\Dev\TS3AudioBot\mx\TSLib\`)  
**Scope**: Complete wire-level command/notification syntax, escaping and unescaping rules, primitive/composite serialization, pipe-separated record compression (`Expand`), `return_code` correlation, full voice-client connection lifecycle, and the exhaustive C->S command and S->C notification catalog.

---

## 1. Command Layer Framing & Transport Integration

Voice-client commands and server notifications ride inside the TeamSpeak 3 UDP packet layer (`D:\Dev\TS3AudioBot\mx\TSLib\Full\PacketHandler.cs:25-32`, `D:\Dev\TS3AudioBot\mx\TSLib\Full\TsFullClient.cs:449-473`):

1. **Packet Types (`PacketType`)**:
   - `PacketType.Command` (`0x02`): Standard reliable command/notification channel (`Full\PacketHandler.cs:196-206`).
   - `PacketType.CommandLow` (`0x03`): Low-priority reliable command/notification channel (`Full\PacketHandler.cs:196-206`).
   - Header sizes: **13 bytes** for Client -> Server (`C->S`) packets (`HeaderLenClient = 13`), **11 bytes** for Server -> Client (`S->C`) packets (`HeaderLenServer = 11`) (`Full\PacketHandler.cs:25-26`).
2. **Packet Flags (`PacketFlags`)**:
   - Every C->S `Command` and `CommandLow` packet sets `PacketFlags.Newprotocol` (`0x08`) (`Full\PacketHandler.cs:204`).
   - **QuickLZ Compression (`PacketFlags.Compressed` = `0x20`)**:
     - For `Command` and `CommandLow`, if `payload.Length > QuickLZ.HeaderLen` (9 bytes), the client compresses the UTF-8 command bytes using **QuickLZ Level 1** (`QuickLZ.Compress(data, 1)`). If the compressed output is strictly smaller than the raw payload, the compressed buffer replaces the payload and `PacketFlags.Compressed` (`0x20`) is set (`Full\PacketHandler.cs:238-248`).
     - On receive (`S->C`), if `PacketFlags.Compressed` (`0x20`) is set on a reassembled packet, the client validates `QuickLZ.SizeCompressed(data) == data.Length`, checks `QuickLZ.SizeDecompressed(data) <= MaxDecompressedSize` (`409,600` bytes = `400 * 1024`), and decompresses via `QuickLZ.Decompress` (`Full\PacketHandler.cs:465-481`).
   - **Fragmentation (`PacketFlags.Fragmented` = `0x10`)**:
     - Maximum UDP datagram size is `MaxPacketLen = 500` bytes (`Full\PacketHandler.cs:27`). Maximum C->S payload per UDP packet is `500 - 13 = 487` bytes (`Full\PacketHandler.cs:276`).
     - After optional compression, if `data.Length + 13 > 500`, the payload is split into chunks of `487` bytes (`splitCount = Ceil(data.Length / 487.0)`). The **first** and **last** fragments have `PacketFlags.Fragmented` (`0x10`) toggled ON; middle fragments (if `splitCount > 2`) do **not** set `PacketFlags.Fragmented` (`Full\PacketHandler.cs:278-302`).
     - On receive (`S->C`), when a packet in stream `Command` or `CommandLow` has `PacketFlags.Fragmented` (`0x10`) set and the fragment queue is empty, buffering begins; subsequent packets in that stream are appended until a second packet with `PacketFlags.Fragmented` (`0x10`) arrives, at which point all fragments are concatenated and then decompressed if the first fragment had `PacketFlags.Compressed` (`Full\PacketHandler.cs:438-463`).
3. **Wire Character Encoding**:
   - All command and notification payloads are UTF-8 encoded without BOM (`new UTF8Encoding(false, true)`, `Commands\TsString.cs:104`, `Full\TsFullClient.cs:459`, `MessageProcessor.cs:26`).
   -Unlike TCP `ServerQuery`, UDP voice-client commands do **not** terminate with `\n\r` on the wire (`Full\TsFullClient.cs:459` vs `Query\TsQueryClient.cs:185`).

---

## 2. Command Text Syntax, Escaping Rules & Serialization

### 2.1 Wire Grammar (ABNF-style)

```abnf
; Client -> Server Command
cs-command       = cmd-name *( " " single-param ) [ " " multi-param-list ] *( " -" option-name )
cmd-name         = 1*( %x61-7A / %x30-39 / "_" ) ; regex: ^[a-z0-9_]+$
option-name      = 1*( %x61-7A / %x30-39 / "_" )
single-param     = param-key [ "=" escaped-val ]
param-key        = 1*( %x61-7A / %x30-39 / "_" )
multi-param-list = param-set *( "|" param-set )
param-set        = single-param *( " " single-param )

; Server -> Client Message
sc-message       = sc-notification / sc-error / sc-data-response
sc-notification  = notify-name [ " " multi-param-list ]
sc-error         = "error" *( " " single-param )
sc-data-response = multi-param-list ; First token contains '=' (no leading command/notify name)
```
*Citations*: `D:\Dev\TS3AudioBot\mx\TSLib\Commands\TsCommand.cs:24, 89-148`, `D:\Dev\TS3AudioBot\mx\TSLib\MessageProcessor.cs:36-74`, `D:\Dev\TS3AudioBot\mx\TSLib\Messages\Deserializer.cs:21-151`.

### 2.2 Character Escaping & Unescaping Table

Every parameter value (`escaped-val`) is escaped so that raw spaces (`0x20`), pipes (`0x7C`), control characters, slashes, and backslashes never appear unescaped on the wire (`D:\Dev\TS3AudioBot\mx\TSLib\Commands\TsString.cs:25-102`).

| Raw Character | ASCII / Hex | Wire Escape Sequence | Escape Bytes (Hex) | Source Citation & Implementation Notes |
|---|---|---|---|---|
| `\` (Backslash) | `0x5C` | `\\` | `5C 5C` | `Commands\TsString.cs:32, 60` — Must be escaped before other replacements or in single-pass scanner. |
| `/` (Forward Slash) | `0x2F` | `\/` | `5C 2F` | `Commands\TsString.cs:33, 61` |
| ` ` (Space) | `0x20` | `\s` | `5C 73` | `Commands\TsString.cs:34, 62` — Separates key-value tokens (` `) when unescaped. |
| `|` (Vertical Pipe) | `0x7C` | `\p` | `5C 70` | `Commands\TsString.cs:35, 63` — Separates multi-item records (`|`) when unescaped. |
| `\f` (Form Feed) | `0x0C` | `\f` | `5C 66` | `Commands\TsString.cs:36, 64` |
| `\n` (Line Feed / LF) | `0x0A` | `\n` | `5C 6E` | `Commands\TsString.cs:37, 65` |
| `\r` (Carriage Return / CR) | `0x0D` | `\r` | `5C 72` | `Commands\TsString.cs:38, 66` |
| `\t` (Horizontal Tab) | `0x09` | `\t` | `5C 74` | `Commands\TsString.cs:39, 67` |
| `\v` (Vertical Tab) | `0x0B` | `\v` | `5C 76` | `Commands\TsString.cs:40, 68` |
| `\a` (Bell / Alert) | `0x07` | `\a` | `5C 61` | Standard TS3 protocol escape (verified in `ts3server` 3.13.9 & official TS3 SDK). *Note*: `TsString.cs:58-70` omits `\a` and throws `FormatException`; Rust implementation **must** handle `\a` <-> `0x07`. |
| `\b` (Backspace) | `0x08` | `\b` | `5C 62` | Standard TS3 protocol escape (verified in `ts3server` 3.13.9 & official TS3 SDK). *Note*: `TsString.cs:58-70` omits `\b`; Rust implementation **must** handle `\b` <-> `0x08`. |

**Unescaping edge cases (`Commands\TsString.cs:50-101`, `Messages\Deserializer.cs:111-151`)**:
- Key-value splitting searches for the **first** `=` in a space-delimited token: `key = token[..eq_idx]`, `raw_val = token[eq_idx+1..]` (`Messages\Deserializer.cs:126-143`).
- If a token has no `=` or ends immediately after `=` (`key=` with empty value), the value is the empty string `""` (`Messages\Deserializer.cs:128-137`).
- Unescaping operates on the raw UTF-8 slice `raw_val` **after** splitting by unescaped `|` and unescaped ` ` (`Messages\Deserializer.cs:47-51, 91-125`).
- In a Rust decoder, if a trailing single `\` or unknown `\x` sequence is encountered in untrusted server/client strings, either return a protocol parse error or preserve `x` gracefully (configurable strict/lenient mode).

### 2.3 Primitive, ID, Enum, Array & Password Serialization

1. **Booleans (`bool`)**:
   - **Serialize (`C->S`)**: `true` -> `"1"`, `false` -> `"0"` (`Commands\CommandParameter.cs:33-34`, `TsBaseFunctions.cs:58`).
   - **Deserialize (`S->C`)**: `val.len() > 0 && val.as_bytes()[0] != b'0'` (`Generated\Messages.tt:56`).
2. **Integers (`i8`, `u8`, `i16`, `u16`, `i32`, `u32`, `i64`, `u64`) & Floats (`f32`, `f64`)**:
   - Base-10 ASCII invariant representation (`NumberFormatInfo.InvariantInfo`, `Commands\CommandParameter.cs:29-31`, `Generated\Messages.tt:46-55`).
3. **Domain Identifier Types (`Messages\BaseTypes.cs:1-30`, `Generated\Messages.tt:96-118`)**:
   - `ClientId` (`clid`, `aclid`, `invokerid`, `tclid`, `fclid`): `u16` (`ushort`).
   - `ClientDbId` (`cldbid`, `tcldbid`, `fcldbid`): `u64` (`ulong`).
   - `ChannelId` (`cid`, `cpid`, `ctid`, `cfid`, `tcid`, `order`, `channel_order`): `u64` (`ulong`).
   - `ServerGroupId` (`sgid`, `ssgid`, `tsgid`): `u64` (`ulong`).
   - `ChannelGroupId` (`cgid`, `scgid`, `tcgid`): `u64` (`ulong`).
   - `IconHash` (`channel_icon_id`, `client_icon_id`, `virtualserver_icon_id`, `iconid`): `i32` (`int`), parsed specially because the TS3 server sometimes formats 32-bit unsigned icon CRC32 hashes as positive 64-bit decimals (e.g. `3054198960`) and sometimes as signed negative 32-bit decimals:
     ```rust
     fn parse_icon_hash(s: &str) -> Result<i32, ParseIntError> {
         if s.starts_with('-') {
             s.parse::<i32>()
         } else {
             s.parse::<u64>().map(|v| v as i32)
         }
     }
     ```
     (`Generated\Messages.tt:111-112`).
4. **Timestamps & Durations (`Generated\Messages.tt:102-110`)**:
   - `DateTime` (e.g. `client_created`, `client_lastconnected`, `created`): Unix epoch timestamp in integer seconds (`u32` or `i64`).
   - `DurationSeconds`: `f64` seconds (`TimeSpan.FromSeconds(double.Parse(val))`).
   - `DurationMilliseconds`: `f64` milliseconds (`TimeSpan.FromMilliseconds(double.Parse(val))`).
5. **Comma-Separated Lists Within a Single Value (`Generated\Messages.tt:34-39`)**:
   - Attributes typed as arrays inside a single record (e.g. `client_servergroups="6,7,9"`, `virtualserver_ip="0.0.0.0,::"`) are split on `,` (`0x2C`) before parsing each element; empty strings produce an empty `Vec<T>`.
6. **Voice-Client Password Hashing (`TsBaseFunctions.cs:64-70`, `Full\TsFullClient.cs:546-547`)**:
   - On a Full Voice Client (`ClientType.Full`), **all** password parameters (`client_server_password`, `client_default_channel_password`, `channel_password`, `cpw`, `tcpw`) are **never** sent in plaintext if non-empty. They are hashed with SHA-1 over the UTF-8 password bytes and Base64-encoded:
     $$\text{wire\_password} = \begin{cases} \text{""} & \text{if password is empty} \\ \text{Base64}(\text{SHA1}(\text{UTF8}(\text{password}))) & \text{otherwise} \end{cases}$$
   - (In contrast, TCP `ServerQuery` sends plaintext passwords, `TsBaseFunctions.cs:69`.)
7. **Command Multi-Parameters (`CommandMultiParameter`, `Commands\TsCommand.cs:111-139`)**:
   - When sending a batch of parameter tuples in a single C->S command (e.g. `channelsubscribe cid=1|cid=2|cid=3` or `servergroupaddperm sgid=6 permid=10 permvalue=75 permnegated=0 permskip=0|permid=11 permvalue=50 permnegated=0 permskip=0`), each tuple's parameters are joined by `" "` and the tuples are joined by `"|"` with **no spaces** immediately before or after `"|"`.
   - In `TsCommand.CacheCommand()`, parameter groups are emitted in this exact order (`Commands\TsCommand.cs:96-147`):
     1. `<CommandName>`
     2. All single `CommandParameter`s (`<key>=<escaped_val>`), preceded by `" "`
     3. All `CommandMultiParameter`s (`<k1>=<v1> <k2>=<v2>|<k1>=<v1b> <k2>=<v2b>`), preceded by `" "`
     4. All `CommandOption`s (`-<option>`), preceded by `" "`

### 2.4 S->C Multi-Record Pipe Lists & Header-Field Inheritance (`Expand`)

When the server sends a pipe-delimited list of records (`record_0|record_1|...|record_{N-1}`), such as `notifycliententerview`, `channellist`, `notifyclientmoved`, or `notifyservergrouplist`, **the TS3 server elides fields in `record_1..record_{N-1}` whose values are identical across the batch, sending those "header" fields only in `record_0`** (`D:\Dev\TS3AudioBot\mx\TSLib\Messages\Deserializer.cs:47-86`).

For example, when two clients enter view simultaneously from the same channel:
```text
notifycliententerview cfid=0 ctid=1 reasonid=0 clid=5 client_nickname=Alice ...|clid=6 client_nickname=Bob ...
```
Here, `cfid=0`, `ctid=1`, and `reasonid=0` appear **only** in `record_0`, but logically apply to `record_1` (`Bob`) as well!

**Exact Deserialization & `Expand` Algorithm (`Messages\Deserializer.cs:45-86`, `Generated\Messages.tt:176-188`)**:
1. Split the message body (everything after `<notify_name> `) on `"|"` into `N` slices: `slices[0..N]`.
2. Allocate `N` default-initialized instances of the target notification struct: `arr[0..N]`.
3. If `N == 1`: parse all `key=value` tokens of `slices[0]` into `arr[0]` and return `arr`.
4. If `N > 1`:
   - Maintain a set `arr_items: HashSet<FieldId>` of field keys seen in `slices[1..N]`.
   - For `i` from `1` to `N - 1` (note: `TSLib` parses `slices[N - 1]` first, then `1..N - 1`):
     - For each space-separated `key=value` token in `slices[i]`:
       - Set the field `key` on `arr[i]`.
       - Insert `key` into `arr_items`.
   - Maintain a list `single: Vec<FieldId>` of header fields present **only** in `slices[0]`.
   - For each space-separated `key=value` token in `slices[0]`:
     - Set the field `key` on `arr[0]`.
     - If `!arr_items.contains(&key)`, push `key` to `single`.
   - **Expand step (`arr[0].Expand(arr, single)`)**:
     - For each `key` in `single`, copy the field value of `key` from `arr[0]` into `arr[1]`, `arr[2]`, ..., `arr[N - 1]`.

---

## 3. Request / Response Correlation & `return_code` Dispatch

### 3.1 `return_code` Generation (`Full\TsFullClient.cs:33, 449-455`)
- The client maintains a monotonically incrementing `u32` counter `return_code` (initialized to `0`, incremented via wrapping/checked `++returnCode` before each command where `ExpectResponse == true`).
- Before serializing a command that expects a response, the client attaches `return_code=<u32>` as a single `CommandParameter`:
  ```text
  channelsubscribeall return_code=1
  ```
- **Commands Sent WITHOUT `return_code` (`ExpectResponse == false`, via `SendNoResponsed`)**:
  Only 6 C->S commands in `TsFullClient` are fire-and-forget and do **not** attach `return_code`:
  1. `clientinitiv` (`Full\TsFullClient.cs:530-534`)
  2. `clientek` (`Full\TsFullClient.cs:288-292`)
  3. `clientinit` (`Full\TsFullClient.cs:536-554`)
  4. `clientdisconnect` (`Full\TsFullClient.cs:556-560`)
  5. `setconnectioninfo` (`Full\TsFullClient.cs:357`)
  6. `clientpoke` (`Full\TsFullClient.cs:568-572`) — *Note*: `clientpoke` in `TsFullClient.cs:571` calls `SendNoResponsed(com)`, though if a `return_code` is attached the server will still reply with `error id=0 msg=ok return_code=...`.

### 3.2 `error` Packet Structure (`Generated\Messages.cs:5398-5443`)
Every command sent with `return_code=<rc>` (and handshake errors even without `return_code`) terminates with an S->C `error` message (`NotificationType.CommandError`):
```text
error id=0 msg=ok return_code=1
error id=2568 msg=insufficient\sclient\spermissions failed_permid=139 return_code=4
```
| Wire Key | Rust Type | C# Property (`CommandError`) | Description |
|---|---|---|---|
| `id` | `u32` (`TsErrorCode`) | `ErrorFormat` (`TsErrorCode`) | `0` = `TsErrorCode.ok`. Non-zero indicates command failure (`Generated\Messages.cs:5401`). |
| `msg` | `String` | `Message` | Human-readable status string, e.g. `"ok"` or `"insufficient client permissions"` (`Generated\Messages.cs:5402`). |
| `extra_msg` | `Option<String>` | `ExtraMessage` | Optional additional error detail from the server (`Generated\Messages.cs:5403`). |
| `failed_permid` | `Option<u16>` (`TsPermission`) | `FailedPermissionId` | Transformed via `PermissionTransform` to identify which permission check failed (`Generated\Messages.cs:5404, 5416`). |
| `return_code` | `Option<String>` | `ReturnCode` | Echoes the `return_code` sent by the client (`Generated\Messages.cs:5405`). |

### 3.3 The Three C->S Command Response Patterns (`MessageProcessor.cs:34-125`, `WaitBlock.cs:60-131`, `Full\TsFullClient.cs:475-526`)

Incoming S->C command-stream packets are classified by inspecting the **first space-delimited token** (`MessageProcessor.cs:36-74`):

1. **First token contains `'='` (Unnamed Direct Data Line)**:
   - Example: `cid=1 clid=5 client_database_id=2 ...`
   - The server only sends unnamed lines as the direct response payload to an active request (e.g. `clientinfo`, `channelinfo`, `clientdbinfo`, `whoami`, explicit `channellist`, explicit `clientlist`).
   - `BaseMessageProcessor.PushMessage` appends this raw string to `cmdLineBuffer: Vec<String>` (`MessageProcessor.cs:44-52`) and waits for the trailing `error id=0 msg=ok return_code=<rc>`.
2. **First token is `"error"` (`NotificationType.CommandError`)**:
   - Deserialized into `CommandError`.
   - If `return_code` is empty, it is treated as a general/unsolicited server error (`GeneralErrorEvent`, `MessageProcessor.cs:81-87`).
   - If `return_code` matches a pending `WaitBlock` in `requestDict` (`MessageProcessor.cs:88-96`):
     - Any accumulated lines in `cmdLineBuffer` are moved into the `WaitBlock` (`wb.SetData(cmdLineBuffer)`) and `cmdLineBuffer` is cleared (`MessageProcessor.cs:91-94`).
     - `wb.Resolve(cmdError)` completes the command waiter (`WaitBlock.cs:60-85`).
3. **First token does NOT contain `'='` and is NOT `"error"` (Named Notification)**:
   - Lookup token in `MessageHelper.GetToClientNotificationType(notifyName)` (`MessageProcessor.cs:65`, `Generated\Messages.cs:10786-10864`).
   - **Coupled Notification vs Asynchronous Event (`MessageProcessor.cs:100-125`, `WaitBlock.cs:92-131`)**:
     - Many voice-client commands return their payload via a **named notification** right before `error id=0 msg=ok return_code=<rc>` (indicated by `DependsOn` in `Messages.cs`). For example, `permissionlist` (`return_code=2`) causes the server to send `notifypermissionlist ...` followed by `error id=0 msg=ok return_code=2`.
     - `BaseMessageProcessor` checks if any active `WaitBlock` in `dependingBlocks` is waiting for `notifyType` (`MessageProcessor.cs:103-111`). If found, the deserialized `INotification[]` array is attached to that `WaitBlock` (`wb.SetNotification(...)`), which returns once **both** the notification and the `error id=0 msg=ok return_code=<rc>` have arrived (`WaitBlock.cs:111-131`).
     - Additionally, the notification is dispatched to internal state handlers (`OnInternalNotification`) and user event subscribers (`NotificationEvent`) (`MessageProcessor.cs:115-124`).

### 3.4 Dynamic Permission ID Mapping (`PermissionTransform`, `Messages\PermissionTransform.cs:1-46`, `Full\TsFullClient.cs:360-371`)
- On the wire, permission IDs (`permid`, `p`, `failed_permid`, `cid` in `notifypermissionlist`) are compact `u16` integers (`1..~300`) assigned sequentially by the server version.
- During connection setup, the client sends `permissionlist` and receives `notifypermissionlist`, which contains entries with `permid=<u16> permname=<string> permdesc=<string>` (and group headers `group_id_end=<u16>` where `permid == 0`).
- `TsFullClient.ProcessPermList` (`Full\TsFullClient.cs:360-371`) iterates through all entries with `permid != 0`, parses `permname` into the canonical `TsPermission` enum, and stores `map[permid] = perm_enum` in `TablePermissionTransform` (`Messages\PermissionTransform.cs:25-45`).

---

## 4. Complete Voice-Client Connection Lifecycle Sequence

Verified from `D:\Dev\TS3AudioBot\mx\TSLib\Full\PacketHandler.cs:85-177` and `D:\Dev\TS3AudioBot\mx\TSLib\Full\TsFullClient.cs:259-384, 530-560`.

```mermaid
sequenceDiagram
    participant C as TS3 Voice Client (Rust/TSLib)
    participant S as TS3 Server (3.13.9)

    Note over C,S: Phase 1: Low-Level UDP Init1 Handshake (PacketType.Init1 = 0x08)
    C->>S: Init1 Step 0 (Version + Timestamp + Random4)
    S->>C: Init1 Step 1 (ServerData16 + Echoed Random4)
    C->>S: Init1 Step 2 (Echoed ServerData16 + Echoed Random4)
    S->>C: Init1 Step 3 (Y = X - level, N, omega, alpha, beta...)
    C->>S: Init1 Step 4 (Hashcash Solution + embedded clientinitiv alpha/omega/ip)

    Note over C,S: Phase 2: Cryptographic Key Exchange (PacketType.Command = 0x02)
    S->>C: initivexpand2 l=... beta=... omega=... proof=... ot=1 tvd=... (PacketId 0)
    Note right of C: Verify omega/proof, generate ECDH P-256 ek,<br/>sign (ek || beta) with identity key,<br/>derive AES key/IV via CryptoInit2
    C->>S: clientek ek=<base64(32B)> proof=<base64(sig)> (PacketId 1, no return_code)
    C->>S: clientinit client_nickname=... client_version=... hwid=... (PacketId 2, no return_code)

    Note over C,S: Phase 3: Server Initialization & World State Push
    S->>C: initserver aclid=<clid> virtualserver_name=... pv=6 ...
    Note right of C: Set PacketHandler.ClientId = aclid;<br/>State = Connected
    S->>C: notifyservergrouplist sgid=... name=...|...
    S->>C: notifychannelgrouplist cgid=... name=...|...
    S->>C: channellist cid=... cpid=... channel_name=...|... (1 or more batches)
    S->>C: channellistfinished
    S->>C: notifycliententerview cfid=0 ctid=<cid> reasonid=0 clid=... (own client + channel peers)
    S->>C: notifyclientneededpermissions permid=... permvalue=...|...
    S->>C: notifyplugincmd name=cliententerview data=version invokerid=0 ...

    Note over C,S: Phase 4: Post-ChannelList Client Setup
    C->>S: channelsubscribeall return_code=1
    C->>S: permissionlist return_code=2
    C->>S: plugincmd name=cliententerview data=TAB targetmode=0 return_code=3
    S->>C: notifychannelsubscribed cid=... es=...|...
    S->>C: notifycliententerview ... (clients in newly subscribed channels)
    S->>C: error id=0 msg=ok return_code=1
    S->>C: notifypermissionlist group_id_end=...|permid=1 permname=...|...
    S->>C: error id=0 msg=ok return_code=2
    S->>C: error id=0 msg=ok return_code=3

    Note over C,S: Phase 5: Active Session & Periodic Telemetry
    S->>C: notifyconnectioninforequest invokerids=...
    C->>S: setconnectioninfo connection_ping=... (30 stats fields, no return_code)

    Note over C,S: Phase 6: Graceful Disconnect
    C->>S: clientdisconnect reasonid=8 reasonmsg=leaving (no return_code)
    S->>C: notifyclientleftview cfid=<cid> ctid=0 reasonid=8 clid=<aclid>
    Note right of C: clid == own ClientId -> Disconnect()
```

### Step-by-Step Lifecycle Details

1. **Packet ID Counter Offset During `Init1` (`Full\PacketHandler.cs:89-95`)**:
   - Because step 4 of `Init1` embeds the `clientinitiv` command inside the `Init1` packet rather than sending it as `Command` PacketId `0`, `PacketHandler.StartHandshake()` immediately increments the outgoing `PacketType.Command` packet counter by `1` (`outStreams[(int)PacketType.Command].GetNextPacketId()`). Thus, the first actual `PacketType.Command` packet sent by the client (`clientek`) has **PacketId = 1**, and `clientinit` has **PacketId = 2**.
2. **`initivexpand2` vs Legacy `initivexpand` (`Full\TsFullClient.cs:259-302`)**:
   - **Modern (`initivexpand2`, `3.1.0+` / server `3.13.9`)**:
     - Server sends `initivexpand2 l=<base64> beta=<base64(54B)> omega=<base64(asn1_pub)> proof=<base64(ecdsa_sig)> ot=1 tvd=<base64>` (`Generated\Messages.cs:6649-6700`).
     - Client calls `PacketHandler.ReceivedFinalInitAck()` to stop repeating the `Init1` step 4 packet (`Full\TsFullClient.cs:279`).
     - Client generates a fresh ephemeral ECDH keypair (`publicKey` = 32 bytes, `privateKey`), constructs an 86-byte buffer `sign_buf[0..32] = publicKey`, `sign_buf[32..86] = base64_decode(beta)` (54 bytes), and signs `sign_buf` with the client's persistent identity ECDSA key (`Full\TsFullClient.cs:283-287`).
     - Client sends `clientek ek=<base64(publicKey)> proof=<base64(signature)>` (`ExpectResponse = false`) (`Full\TsFullClient.cs:288-292`).
     - Client calls `PacketHandler.CryptoInit2(l, omega, proof, beta, privateKey)` to verify the server's `proof` and activate AES-GCM/CTR session encryption (`Full\TsFullClient.cs:294-295`).
     - Client immediately sends `clientinit` (`Full\TsFullClient.cs:296, 373-384`).
   - **Legacy (`initivexpand`, pre-3.1 fallback)**:
     - Server sends `initivexpand alpha=<base64(10B)> beta=<base64(10B)> omega=<base64(asn1_pub)>` (`Generated\Messages.cs:6611-6647`).
     - Client calls `PacketHandler.ReceivedFinalInitAck()`, derives keys via `CryptoInit(alpha, beta, omega)`, and sends `clientinit` (`Full\TsFullClient.cs:264-273`).
3. **`clientinit` Parameters (`Full\TsFullClient.cs:373-384, 536-554`, `Generated\Messages.cs:3398-3467`)**:
   - Sent via `SendNoResponsed` (`ExpectResponse = false`).
   - Required/standard fields populated by `TsFullClient.SendClientInit`:
     - `client_nickname`: User display name (`String`, min 3 chars).
     - `client_version`: e.g. `"3.?.? [Build: 1568021092]"` (`String`).
     - `client_platform`: e.g. `"Windows"`, `"Linux"`, `"OS X"` (`String`).
     - `client_input_hardware`: `"1"` (`bool`).
     - `client_output_hardware`: `"1"` (`bool`).
     - `client_default_channel`: Optional default channel path or `"/" + cid` (`String`, empty if none).
     - `client_default_channel_password`: `base64(sha1(utf8(channel_pw)))` or `""`.
     - `client_server_password`: `base64(sha1(utf8(server_pw)))` or `""`.
     - `client_meta_data`: `""` (`String`).
     - `client_version_sign`: Base64 signature corresponding to `(client_version, client_platform)` (`String`).
     - `client_key_offset`: Identity key offset (`u64`, from `IdentityData.ValidKeyOffset`).
     - `client_nickname_phonetic`: Phonetic nickname (`String`).
     - `client_default_token`: Optional privilege key token (`String`).
     - `hwid`: Hardware ID string, e.g. `"123f13431e227922110796c29a2f63f3,123f13431e227922110796c29a2f63f3"` (`String`).
4. **`initserver` & Initial State Burst (`Full\TsFullClient.cs:304-351`)**:
   - Upon receiving `initserver`, the client extracts `aclid` (`InitServer.ClientId`), assigns `PacketHandler.ClientId = aclid`, and transitions `ClientState` to `Connected` (`Full\TsFullClient.cs:306-314`).
   - When `channellistfinished` arrives, the client sends `channelsubscribeall` (if `AutoSubscribeAllChannels` is enabled) and `permissionlist` (`Full\TsFullClient.cs:347-351`).
   - When `notifyplugincmd` arrives with `name="cliententerview"` and `data="version"`, `TsFullClient` responds with `plugincmd name=cliententerview data=TAB targetmode=0` (`Full\TsFullClient.cs:318-322`).
5. **Periodic Connection Info Telemetry (`Full\TsFullClient.cs:353-358`, `Full\NetworkStats.cs:147-178`)**:
   - When the server sends `notifyconnectioninforequest`, the client replies with `setconnectioninfo` (no `return_code`) containing 30 `connection_*` metrics (`connection_ping`, `connection_ping_deviation`, `connection_packets_sent_speech`, `connection_bytes_sent_speech`, etc.).

---

## 5. Complete C->S Voice-Client Command Catalog

All C->S commands recognized by `MessageHelper.GetToServerNotificationType` (`D:\Dev\TS3AudioBot\mx\TSLib\Generated\Messages.cs:10866-11016`) plus the two special full-client commands constructed directly in `TsFullClient`/`NetworkStats` (`clientek`, `setconnectioninfo`).

### Response Pattern Legend
- **`NoResponse`**: Sent without `return_code` (`ExpectResponse == false`); server sends no `error` ack for this command.
- **`Void`**: Sent with `return_code`; server responds only with `error id=0 msg=ok return_code=<rc>` (plus any broadcast event notifications to affected clients).
- **`Direct(<Struct>)`**: Sent with `return_code`; server responds with an **unnamed** pipe-delimited line of `<Struct>` fields (`cmdLineBuffer`), followed by `error id=0 msg=ok return_code=<rc>`.
- **`Notify(<notifyname> -> <Struct>)`**: Sent with `return_code`; server responds with a **named** notification `<notifyname> ...` coupled to the request (`DependsOn` in `Messages.cs`), followed by `error id=0 msg=ok return_code=<rc>`.

### 5.1 Handshake, Authentication & Connection Lifecycle Commands

| Wire Command | C# Class (`Messages.cs`) | Response Pattern | Wire Parameters (`key: RustType`) & Options (`-flag`) | Source Citations |
|---|---|---|---|---|
| `clientinitiv` | `ClientInitIv` | `NoResponse` (embedded in `Init1` step 4 or `Command` pkt 0) | `alpha: String` (10B base64 random), `omega: String` (client ASN.1 public key base64), `ot: i32` (`1`), `ip: Option<String>` | `Messages.cs:3469-3504`, `Full\TsFullClient.cs:530-534`, `Full\PacketHandler.cs:163-171` |
| `clientek` | *(inline `TsCommand`)* | `NoResponse` | `ek: String` (32B ECDH public key base64), `proof: String` (ECDSA signature of `ek \|\| beta` base64) | `Full\TsFullClient.cs:288-292` |
| `clientinit` | `ClientInit` | `NoResponse` | `client_nickname: String`, `client_version: String`, `client_platform: String`, `client_input_hardware: bool`, `client_output_hardware: bool`, `client_default_channel: String`, `client_default_channel_password: String` (SHA1-B64), `client_server_password: String` (SHA1-B64), `client_meta_data: String`, `client_version_sign: String`, `client_key_offset: u64`, `client_nickname_phonetic: String`, `client_default_token: String`, `hwid: String`, `client_away: Option<bool>`, `client_away_message: Option<String>`, `client_flag_talking: Option<bool>`, `client_input_muted: Option<bool>`, `client_output_muted: Option<bool>`, `client_outputonly_muted: Option<bool>`, `client_is_channel_commander: Option<bool>` | `Messages.cs:3398-3467`, `Full\TsFullClient.cs:373-384, 536-554` |
| `clientdisconnect` | `Disconnect` | `NoResponse` | `reasonid: u8` (`Reason`, `8` = `LeftServer`), `reasonmsg: Option<String>` | `Messages.cs:5595-5625`, `Full\TsFullClient.cs:152, 556-560` |
| `setconnectioninfo` | *(inline `TsCommand`)* | `NoResponse` | `connection_ping: u32`, `connection_ping_deviation: f64`, `connection_packets_sent_speech: u64`, `connection_packets_sent_keepalive: u64`, `connection_packets_sent_control: u64`, `connection_bytes_sent_speech: u64`, `connection_bytes_sent_keepalive: u64`, `connection_bytes_sent_control: u64`, `connection_packets_received_speech: u64`, `connection_packets_received_keepalive: u64`, `connection_packets_received_control: u64`, `connection_bytes_received_speech: u64`, `connection_bytes_received_keepalive: u64`, `connection_bytes_received_control: u64`, `connection_server2client_packetloss_speech: f32`, `connection_server2client_packetloss_keepalive: f32`, `connection_server2client_packetloss_control: f32`, `connection_server2client_packetloss_total: f32`, `connection_bandwidth_sent_last_second_speech: u64`, `connection_bandwidth_sent_last_second_keepalive: u64`, `connection_bandwidth_sent_last_second_control: u64`, `connection_bandwidth_sent_last_minute_speech: u64`, `connection_bandwidth_sent_last_minute_keepalive: u64`, `connection_bandwidth_sent_last_minute_control: u64`, `connection_bandwidth_received_last_second_speech: u64`, `connection_bandwidth_received_last_second_keepalive: u64`, `connection_bandwidth_received_last_second_control: u64`, `connection_bandwidth_received_last_minute_speech: u64`, `connection_bandwidth_received_last_minute_keepalive: u64`, `connection_bandwidth_received_last_minute_control: u64` | `Full\TsFullClient.cs:353-358`, `Full\NetworkStats.cs:147-178` |
| `getconnectioninfo` | `ClientConnectionInfoRequest` | `Notify(notifyconnectioninfo -> ClientConnectionInfo)` | `clid: ClientId (u16)` | `Messages.cs:2437-2462`, `Full\TsFullClient.gen.cs:343-350` |
| `setconnectioninfoupdate` | `ClientConnectionInfoUpdateRequest` | `Void` | `client_connection_info_update: bool` | `Messages.cs:2464-2489` |

### 5.2 Client State, Movement, Lookup & Database Commands

| Wire Command | C# Class (`Messages.cs`) | Response Pattern | Wire Parameters (`key: RustType`) & Options (`-flag`) | Source Citations |
|---|---|---|---|---|
| `clientupdate` | `ClientUpdate` | `Void` | Optional properties to update on self: `client_nickname: Option<String>`, `client_away: Option<bool>`, `client_away_message: Option<String>`, `client_flag_talking: Option<bool>`, `client_input_muted: Option<bool>`, `client_output_muted: Option<bool>`, `client_outputonly_muted: Option<bool>`, `client_input_hardware: Option<bool>`, `client_output_hardware: Option<bool>`, `client_is_channel_commander: Option<bool>`, `client_nickname_phonetic: Option<String>`, `client_meta_data: Option<String>`, `client_ default_token: Option<String>`, `client_talk_request: Option<bool>`, `client_talk_request_msg: Option<String>`, `client_description: Option<String>`, `client_is_talker: Option<bool>`, `client_icon_id: Option<IconHash (i32)>`, `client_badges: Option<String>`, `client_flag_avatar: Option<String>` | `Messages.cs:4920-5005`, `TsBaseFunctions.cs:96-119`, `Commands\TsCommand.gen.cs:62-97` |
| `clientedit` | `ClientEdit` | `Void` | `clid: ClientId (u16)`, plus any editable client property (`client_nickname`, `client_description`, `client_is_talker`, `client_icon_id`, `client_is_channel_commander`, etc.) | `Messages.cs:2865-2945`, `TsBaseFunctions.cs:121-145` |
| `clientmove` | `ClientMove` | `Void` (triggers S->C `notifyclientmoved`) | `clid: ClientId (u16)` (supports multi-param `clid=1\|clid=2`), `cid: ChannelId (u64)`, `cpw: Option<String>` (SHA1-B64 password) | `Messages.cs:4193-4223`, `TsBaseFunctions.cs:171-193` |
| `clientkick` | `ClientKick` | `Void` (triggers S->C `notifyclientleftview`) | `clid: ClientId (u16)` (supports multi-param `clid=1\|clid=2`), `reasonid: u8` (`4` = `KickChannel`, `5` = `KickServer`), `reasonmsg: Option<String>` | `Messages.cs:3506-3536`, `TsBaseFunctions.cs:195-217` |
| `clientpoke` | `ClientPokeRequest` | `NoResponse` / `Void` | `clid: ClientId (u16)`, `msg: String` | `Messages.cs:4621-4648`, `Full\TsFullClient.cs:568-572` |
| `clientinfo` | `ClientInfoRequest` | `Direct(ClientInfo)` (Query) / `clientgetvariables` on Full | `clid: ClientId (u16)` (multi-param supported) | `Messages.cs:3108-3133`, `TsBaseFunctions.gen.cs:55-61` |
| `clientgetvariables` | `ClientVariablesRequest` | `Void` (triggers S->C `notifyclientupdated`) | `clid: ClientId (u16)` | `Messages.cs:5371-5396`, `Full\TsFullClient.cs:562-566`, `Full\TsFullClient.gen.cs:448-455` |
| `clientlist` | `ClientListRequest` | `Direct(ClientList)` | Options: `-uid`, `-away`, `-voice`, `-times`, `-groups`, `-info`, `-icon`, `-country`, `-ip`, `-badges` | `Messages.cs:3864-3887`, `TsBaseFunctions.cs:147-169` |
| `clientfind` | `ClientFindRequest` | `Direct(ClientFind)` | `pattern: String` | `Messages.cs:3025-3050`, `TsBaseFunctions.gen.cs:47-53` |
| `clientgetids` | `ClientIdsRequest` | `Notify(notifyclientids -> ClientIds)` | `cluid: String` (Base64 client UID) | `Messages.cs:3081-3106`, `Full\TsFullClient.gen.cs:395-402` |
| `clientgetdbidfromuid` | `ClientDbIdFromUidRequest` | `Notify(notifyclientdbidfromuid -> ClientDbIdFromUid)` | `cluid: String` | `Messages.cs:2602-2627`, `Full\TsFullClient.gen.cs:367-374` |
| `clientgetnamefromuid` | `ClientNameFromUidRequest` | `Notify(notifyclientnamefromuid -> ClientNameFromUid)` | `cluid: String` | `Messages.cs:4339-4364`, `Full\TsFullClient.gen.cs:419-426` |
| `clientgetnamefromdbid` | `ClientNameFromDbIdRequest` | `Notify(notifyclientnamefromdbid -> ClientNameFromDbId)` | `cldbid: ClientDbId (u64)` | `Messages.cs:4283-4308`, `Full\TsFullClient.gen.cs:411-418` |
| `clientgetuidfromclid` | `ClientUidFromClidRequest` | `Notify(notifyclientuidfromclid -> ClientUidFromClid)` | `clid: ClientId (u16)` | `Messages.cs:4893-4918`, `Full\TsFullClient.gen.cs:440-447` |
| `clientdblist` | `ClientDbListRequest` | `Notify(notifyclientdblist -> ClientDbList)` | `start: Option<u64>`, `duration: Option<u64>`, Option: `-count` | `Messages.cs:2803-2832`, `Full\TsFullClient.gen.cs:384-394` |
| `clientdbinfo` | `ClientDbInfoRequest` | `Direct(ClientDbInfo)` | `cldbid: ClientDbId (u64)` (multi-param supported) | `Messages.cs:2629-2654`, `TsBaseFunctions.gen.cs:39-45` |
| `clientdbfind` | `ClientDbFindRequest` | `Notify(notifyclientdbfind -> ClientDbFind)` | `pattern: String`, Option: `-uid` | `Messages.cs:2547-2572`, `Full\TsFullClient.gen.cs:358-366` |
| `clientdbedit` | `ClientDbEdit` | `Void` | `cldbid: ClientDbId (u64)`, `client_description: String` | `Messages.cs:2491-2518`, `Full\TsFullClient.gen.cs:351-357` |
| `clientdbdelete` | `ClientDbDelete` | `Void` | `cldbid: ClientDbId (u64)` | `Messages.cs:2410-2435`, `TsBaseFunctions.gen.cs:31-37` |
| `clientsetserverquerylogin` | `ClientSetServerQueryLoginRequest` | `Notify(notifyclientsetserverquerylogin -> ClientSetServerQueryLogin)` | `client_login_name: String` | `Messages.cs:4836-4861`, `Full\TsFullClient.gen.cs:432-439` |
| `clientchatcomposing` | `ClientChatComposing` | `Void` | `clid: ClientId (u16)` | `Messages.cs:2383-2408`, `Full\TsFullClient.gen.cs:336-342` |
| `clientchatclosed` | `ClientChatClose` | `Void` | `clid: ClientId (u16)` | `Messages.cs:2327-2352`, `Full\TsFullClient.gen.cs:329-335` |

### 5.3 Channel Management & Subscription Commands

| Wire Command | C# Class (`Messages.cs`) | Response Pattern | Wire Parameters (`key: RustType`) & Options (`-flag`) | Source Citations |
|---|---|---|---|---|
| `channellist` | `ChannelListRequest` | `Direct(ChannelList)` | Options: `-topic`, `-flags`, `-voice`, `-limits`, `-icon`, `-secondempty`, `-banners` | `Messages.cs:1901-1924`, `TsBaseFunctions.cs:219-241` |
| `channelinfo` | `ChannelInfoRequest` | `Direct(ChannelInfo)` | `cid: ChannelId (u64)` | `Messages.cs:1639-1664`, `TsBaseFunctions.gen.cs:23-29` |
| `channelfind` | `ChannelFindRequest` | `Direct(ChannelFind)` | `pattern: String` | `Messages.cs:962-987`, `TsBaseFunctions.gen.cs:15-21` |
| `channelcreate` | `ChannelCreate` | `Void` (triggers S->C `notifychannelcreated`) | `channel_name: String`, plus optional `cpid: ChannelId (u64)`, `channel_topic: String`, `channel_description: String`, `channel_password: String` (SHA1-B64), `channel_codec: Codec (u8)`, `channel_codec_quality: u8`, `channel_maxclients: i32`, `channel_maxfamilyclients: i32`, `channel_order: ChannelId (u64)`, `channel_flag_permanent: bool`, `channel_flag_semi_permanent: bool`, `channel_flag_temporary: bool`, `channel_flag_default: bool`, `channel_flag_maxclients_unlimited: bool`, `channel_flag_maxfamilyclients_unlimited: bool`, `channel_flag_maxfamilyclients_inherited: bool`, `channel_needed_talk_power: i32`, `channel_name_phonetic: String`, `channel_icon_id: IconHash (i32)`, `channel_codec_is_unencrypted: bool`, `channel_delete_delay: u32`, `channel_banner_gfx_url: String`, `channel_banner_mode: BannerMode (u8)` | `Messages.cs:518-625`, `TsBaseFunctions.cs:243-286`, `Commands\TsCommand.gen.cs:12-60` |
| `channeledit` | `ChannelEdit` | `Void` (triggers S->C `notifychanneledited` / `notifychannelchanged`) | `cid: ChannelId (u64)`, plus any editable `channel_*` property from `channelcreate` | `Messages.cs:763-872`, `TsBaseFunctions.cs:288-330` |
| `channeldelete` | `ChannelDelete` | `Void` (triggers S->C `notifychanneldeleted`) | `cid: ChannelId (u64)`, `force: bool` | `Messages.cs:655-682`, `TsBaseFunctions.cs:332-343` |
| `channelmove` | `ChannelMove` | `Void` (triggers S->C `notifychannelmoved`) | `cid: ChannelId (u64)`, `cpid: ChannelId (u64)`, `order: Option<ChannelId (u64)>` | `Messages.cs:1926-1955`, `TsBaseFunctions.cs:345-359` |
| `channelgetdescription` | `ChannelDescriptionRequest` | `Void` (triggers S->C `notifychanneledited` with `channel_description`) | `cid: ChannelId (u64)` | `Messages.cs:736-761`, `Full\TsFullClient.gen.cs:211-217` |
| `channelsubscribe` | `ChannelSubscribe` | `Notify(notifychannelsubscribed -> ChannelSubscribed)` | `cid: ChannelId (u64)` (multi-param `cid=1\|cid=2` supported) | `Messages.cs:2092-2117`, `Full\TsFullClient.gen.cs:294-301` |
| `channelsubscribeall` | `ChannelSubscribeAll` | `Notify(notifychannelsubscribed -> ChannelSubscribed)` | *(no parameters)* | `Messages.cs:2119-2142`, `Full\TsFullClient.gen.cs:302-308` |
| `channelunsubscribe` | `ChannelUnsubscribe` | `Notify(notifychannelunsubscribed -> ChannelUnsubscribed)` | `cid: ChannelId (u64)` (multi-param `cid=1\|cid=2` supported) | `Messages.cs:2174-2199`, `Full\TsFullClient.gen.cs:309-316` |
| `channelunsubscribeall` | `ChannelUnsubscribeAll` | `Notify(notifychannelunsubscribed -> ChannelUnsubscribed)` | *(no parameters)* | `Messages.cs:2201-2224`, `Full\TsFullClient.gen.cs:317-323` |

### 5.4 Messaging, Plugin Commands & Offline Messages

| Wire Command | C# Class (`Messages.cs`) | Response Pattern | Wire Parameters (`key: RustType`) & Options (`-flag`) | Source Citations |
|---|---|---|---|---|
| `sendtextmessage` | `SendTextMessage` | `Void` (triggers S->C `notifytextmessage`) | `targetmode: u8` (`TextMessageTargetMode`: `1` = Client, `2` = Channel, `3` = Server), `msg: String`, `target: Option<ClientId (u16)>` (required when `targetmode=1`) | `Messages.cs:8300-8329`, `TsBaseFunctions.cs:73-94` |
| `plugincmd` | `PluginCommandRequest` | `Void` (triggers S->C `notifyplugincmd`) | `name: String`, `data: String`, `targetmode: u8` (`PluginTargetMode`: `0` = CurrentChannel, `1` = Server, `2` = Client, `3` = SubscribedClients), `target: Option<u16[]>` (comma/multi `ClientId`s when `targetmode=2`) | `Messages.cs:7986-8017`, `Full\TsFullClient.cs:318-322, 574-592` |
| `gm` | `GlobalMessage` | `Void` | `msg: String` | `Messages.cs:6483-6508`, `TsBaseFunctions.gen.cs:87-93` |
| `messageadd` | `OfflineMessageAdd` | `Void` | `cluid: String`, `subject: String`, `message: String` | `Messages.cs:7186-7215`, `Full\TsFullClient.gen.cs:544-550` |
| `messagedel` | `OfflineMessageDel` | `Void` | `msgid: u64` | `Messages.cs:7217-7242`, `Full\TsFullClient.gen.cs:551-557` |
| `messageget` | `OfflineMessageGet` | `Notify(notifyofflinemessage -> OfflineMessage)` | `msgid: u64` | `Messages.cs:7244-7269`, `Full\TsFullClient.gen.cs:558-565` |
| `messagelist` | `OfflineMessageListRequest` | `Notify(notifyofflinemessagelist -> OfflineMessageList)` | *(no parameters)* | `Messages.cs:7331-7354`, `Full\TsFullClient.gen.cs:566-573` |
| `messageupdateflag` | `OfflineMessageUpdateFlag` | `Void` | `msgid: u64`, `flag: bool` | `Messages.cs:7356-7383`, `Full\TsFullClient.gen.cs:574-580` |

### 5.5 Server Groups, Channel Groups & Permission Commands

| Wire Command | C# Class (`Messages.cs`) | Response Pattern | Wire Parameters (`key: RustType`) & Options (`-flag`) | Source Citations |
|---|---|---|---|---|
| `permissionlist` | `PermListRequest` | `Notify(notifypermissionlist -> PermList)` | Option: `-new` (optional) | `Messages.cs:7760-7783`, `Full\TsFullClient.cs:349`, `Full\TsFullClient.gen.cs:597-604` |
| `permfind` | `PermFindRequest` | `Notify(notifypermfind -> PermFind)` | `permid: u16` (`TsPermission`, multi-param) or `permsid: String` | `Messages.cs:7421-7448`, `Full\TsFullClient.gen.cs:581-588` |
| `permget` | `PermRequest` | `Direct(Perm)` | `permid: u16` or `permsid: String` | `Messages.cs:7871-7898`, `TsBaseFunctions.gen.cs:111-119` |
| `permidgetbyname` | `PermIdByNameRequest` | `Direct(PermIdByName)` | `permsid: String` (multi-param supported) | `Messages.cs:7479-7504`, `TsBaseFunctions.gen.cs:103-109` |
| `permoverview` | `PermOverviewRequest` | `Notify(notifypermoverview -> PermOverview)` | `cid: ChannelId (u64)`, `cldbid: ClientDbId (u64)`, `permid: u16` (or `permsid: String`) | `Messages.cs:7838-7869`, `Full\TsFullClient.gen.cs:605-612` |
| `permreset` | `PermReset` | `Direct(Token)` | *(no parameters)* | `Messages.cs:7900-7923`, `TsBaseFunctions.gen.cs:121-127` |
| `servergrouplist` | `ServerGroupListRequest` | `Notify(notifyservergrouplist -> ServerGroupList)` | *(no parameters)* | `Messages.cs:9257-9280`, `Full\TsFullClient.gen.cs:718-725` |
| `servergroupadd` | `ServerGroupAdd` | `Direct(ServerGroupId)` | `name: String`, `type: Option<GroupDatabaseType (u8)>` | `Messages.cs:8827-8854`, `TsBaseFunctions.gen.cs:137-143` |
| `servergroupdel` | `ServerGroupDel` | `Void` | `sgid: ServerGroupId (u64)`, `force: bool` | `Messages.cs:9099-9126`, `Full\TsFullClient.gen.cs:704-710` |
| `servergrouprename` | `ServerGroupRename` | `Void` | `sgid: ServerGroupId (u64)`, `name: String` | `Messages.cs:9343-9370`, `Full\TsFullClient.gen.cs:734-740` |
| `servergroupcopy` | `ServerGroupCopy` | `Direct(ServerGroupId)` | `ssgid: ServerGroupId (u64)`, `tsgid: ServerGroupId (u64)`, `name: String`, `type: GroupDatabaseType (u8)` | `Messages.cs:9066-9097`, `Full\TsFullClient.gen.cs:696-703` |
| `servergroupaddclient` | `ServerGroupAddClient` | `Void` (triggers S->C `notifyclientservergroupadded`) | `sgid: ServerGroupId (u64)`, `cldbid: ClientDbId (u64)` (multi-param supported) | `Messages.cs:8856-8883`, `Full\TsFullClient.gen.cs:657-663` |
| `servergroupdelclient` | `ServerGroupDelClient` | `Void` (triggers S->C `notifyclientservergroupremoved`) | `sgid: ServerGroupId (u64)`, `cldbid: ClientDbId (u64)` (multi-param supported) | `Messages.cs:9128-9155`, `Full\TsFullClient.gen.cs:711-717` |
| `servergroupclientlist` | `ServerGroupClientListRequest` | `Notify(notifyservergroupclientlist -> ServerGroupClientList)` | `sgid: ServerGroupId (u64)`, Option: `-names` | `Messages.cs:9037-9064`, `Full\TsFullClient.gen.cs:687-695` |
| `servergroupsbyclientid` | `ServerGroupsByClientIdRequest` | `Notify(notifyservergroupsbyclientid -> ServerGroupsByClientId)` | `cldbid: ClientDbId (u64)` | `Messages.cs:9407-9432`, `Full\TsFullClient.gen.cs:741-748` |
| `servergrouppermlist` | `ServerGroupPermListRequest` | `Notify(notifyservergrouppermlist -> ServerGroupPermList)` | `sgid: ServerGroupId (u64)`, Option: `-permsid` | `Messages.cs:9314-9341`, `Full\TsFullClient.gen.cs:726-733` |
| `servergroupaddperm` | `ServerGroupAddPerm` | `Void` | `sgid: ServerGroupId (u64)`, multi-param: `permid: u16` (or `permsid: String`), `permvalue: i32`, `permnegated: bool`, `permskip: bool` | `Messages.cs:8885-8920`, `Full\TsFullClient.gen.cs:664-671` |
| `servergroupdelperm` | `ServerGroupDelPerm` | `Void` | `sgid: ServerGroupId (u64)`, multi-param: `permid: u16` (or `permsid: String`) | `Messages.cs:9157-9186`, `Full\TsFullClient.gen.cs:672-679` |
| `servergroupautoaddperm` | `ServerGroupAutoAddPerm` | `Void` | `sgtype: GroupType (u8)`, multi-param: `permid: u16` (or `permsid: String`), `permvalue: i32`, `permnegated: bool`, `permskip: bool` | `Messages.cs:8922-8957`, `Full\TsFullClient.gen.cs:680-686` |
| `servergroupautodelperm` | `ServerGroupAutoDelPerm` | `Void` | `sgtype: GroupType (u8)`, multi-param: `permid: u16` (or `permsid: String`) | `Messages.cs:8959-8988` |
| `channelgrouplist` | `ChannelGroupListRequest` | `Notify(notifychannelgrouplist -> ChannelGroupList)` | *(no parameters)* | `Messages.cs:1524-1547`, `Full\TsFullClient.gen.cs:256-263` |
| `channelgroupadd` | `ChannelGroupAdd` | `Direct(ChannelGroupId)` | `name: String`, `type: Option<GroupDatabaseType (u8)>` | `Messages.cs:1224-1251`, `Full\TsFullClient.gen.cs:218-225` |
| `channelgroupdel` | `ChannelGroupDel` | `Void` | `cgid: ChannelGroupId (u64)`, `force: bool` | `Messages.cs:1399-1426`, `Full\TsFullClient.gen.cs:242-248` |
| `channelgrouprename` | `ChannelGroupRename` | `Void` | `cgid: ChannelGroupId (u64)`, `name: String` | `Messages.cs:1610-1637`, `Full\TsFullClient.gen.cs:272-278` |
| `channelgroupcopy` | `ChannelGroupCopy` | `Direct(ChannelGroupId)` | `scgid: ChannelGroupId (u64)`, `tcgid: ChannelGroupId (u64)`, `name: String`, `type: GroupDatabaseType (u8)` | `Messages.cs:1366-1397`, `Full\TsFullClient.gen.cs:234-241` |
| `channelgroupclientlist` | `ChannelGroupClientListRequest` | `Notify(notifychannelgroupclientlist -> ChannelGroupClientList)` | `cid: Option<ChannelId (u64)>`, `cldbid: Option<ClientDbId (u64)>`, `cgid: Option<ChannelGroupId (u64)>` | `Messages.cs:1333-1364`, `Full\TsFullClient.gen.cs:226-233` |
| `channelgrouppermlist` | `ChannelGroupPermListRequest` | `Notify(notifychannelgrouppermlist -> ChannelGroupPermList)` | `cgid: ChannelGroupId (u64)`, Option: `-permsid` | `Messages.cs:1581-1608`, `Full\TsFullClient.gen.cs:264-271` |
| `channelgroupaddperm` | `ChannelGroupAddPerm` | `Void` | `cgid: ChannelGroupId (u64)`, multi-param: `permid: u16` (or `permsid: String`), `permvalue: i32`, `permnegated: bool`, `permskip: bool` | `Messages.cs:1253-1288`, `Full\TsFullClient.gen.cs:249-255` |
| `channelgroupdelperm` | `ChannelGroupDelPerm` | `Void` | `cgid: ChannelGroupId (u64)`, multi-param: `permid: u16` (or `permsid: String`) | `Messages.cs:1428-1457` |
| `setclientchannelgroup` | `SetClientChannelGroup` | `Void` (triggers S->C `notifyclientchannelgroupchanged`) | `cgid: ChannelGroupId (u64)`, `cid: ChannelId (u64)`, `cldbid: ClientDbId (u64)` | `Messages.cs:10220-10249`, `Full\TsFullClient.gen.cs:769-775` |
| `channelpermlist` | `ChannelPermListRequest` | `Notify(notifychannelpermlist -> ChannelPermList)` | `cid: ChannelId (u64)`, Option: `-permsid` | `Messages.cs:2063-2090`, `Full\TsFullClient.gen.cs:286-293` |
| `channeladdperm` | `ChannelAddPerm` | `Void` | `cid: ChannelId (u64)`, multi-param: `permid: u16` (or `permsid: String`), `permvalue: i32` | `Messages.cs:270-301`, `Full\TsFullClient.gen.cs:182-188` |
| `channeldelperm` | `ChannelDelPerm` | `Void` | `cid: ChannelId (u64)`, multi-param: `permid: u16` (or `permsid: String`) | `Messages.cs:684-713`, `Full\TsFullClient.gen.cs:204-210` |
| `clientpermlist` | `ClientPermListRequest` | `Notify(notifyclientpermlist -> ClientPermList)` | `cldbid: ClientDbId (u64)`, Option: `-permsid` | `Messages.cs:4564-4591`, `Full\TsFullClient.gen.cs:427-431` |
| `clientaddperm` | `ClientAddPerm` | `Void` | `cldbid: ClientDbId (u64)`, multi-param: `permid: u16` (or `permsid: String`), `permvalue: i32`, `permskip: bool` | `Messages.cs:2259-2292`, `Full\TsFullClient.gen.cs:324-328` |
| `clientdelperm` | `ClientDelPerm` | `Void` | `cldbid: ClientDbId (u64)`, multi-param: `permid: u16` (or `permsid: String`) | `Messages.cs:2834-2863`, `Full\TsFullClient.gen.cs:375-383` |
| `channelclientpermlist` | `ChannelClientPermListRequest` | `Notify(notifychannelclientpermlist -> ChannelClientPermList)` | `cid: ChannelId (u64)`, `cldbid: ClientDbId (u64)`, Option: `-permsid` | `Messages.cs:487-516`, `Full\TsFullClient.gen.cs:196-203` |
| `channelclientaddperm` | `ChannelClientAddPerm` | `Void` | `cid: ChannelId (u64)`, `cldbid: ClientDbId (u64)`, multi-param: `permid: u16` (or `permsid: String`), `permvalue: i32` | `Messages.cs:393-426`, `Full\TsFullClient.gen.cs:189-195` |
| `channelclientdelperm` | `ChannelClientDelPerm` | `Void` | `cid: ChannelId (u64)`, `cldbid: ClientDbId (u64)`, multi-param: `permid: u16` (or `permsid: String`) | `Messages.cs:428-459` |

### 5.6 File Transfer Commands

| Wire Command | C# Class (`Messages.cs`) | Response Pattern | Wire Parameters (`key: RustType`) & Options (`-flag`) | Source Citations |
|---|---|---|---|---|
| `ftinitupload` | `FtInitUpload` | `Notify(notifyftinitupload -> FileUpload)` | `clientftfid: u16`, `name: String` (starts with `/`), `cid: ChannelId (u64)`, `cpw: String` (SHA1-B64), `size: u64`, `overwrite: bool`, `resume: bool`, `proto: Option<i32>` | `Messages.cs:6255-6298`, `TsBaseFunctions.FileTransfer.cs:48-115` |
| `ftinitdownload` | `FtInitDownload` | `Notify(notifyftinitdownload -> FileDownload)` | `clientftfid: u16`, `name: String` (starts with `/`), `cid: ChannelId (u64)`, `cpw: String` (SHA1-B64), `seekpos: u64`, `proto: Option<i32>` | `Messages.cs:6218-6253`, `TsBaseFunctions.FileTransfer.cs:183-242` |
| `ftgetfilelist` | `FtFileListRequest` | `Notify(notifyfilelist -> FileList)` + `notifyfilelistfinished` | `cid: ChannelId (u64)`, `cpw: String` (SHA1-B64), `path: String` (e.g. `"/"`) | `Messages.cs:6189-6216`, `Full\TsFullClient.gen.cs:496-503` |
| `ftgetfileinfo` | `FtFileInfoRequest` | `Notify(notifyfileinfo -> FileInfo)` | Multi-param: `cid: ChannelId (u64)`, `cpw: String` (SHA1-B64), `name: String` | `Messages.cs:6160-6187`, `Full\TsFullClient.gen.cs:488-495` |
| `ftcreatedir` | `FtCreateDir` | `Void` | `cid: ChannelId (u64)`, `cpw: String` (SHA1-B64), `dirname: String` | `Messages.cs:6102-6129`, `Full\TsFullClient.gen.cs:474-480` |
| `ftdeletefile` | `FtDeleteFile` | `Void` | `cid: ChannelId (u64)`, `cpw: String` (SHA1-B64), multi-param: `name: String` | `Messages.cs:6131-6158`, `Full\TsFullClient.gen.cs:481-487` |
| `ftrenamefile` | `FtRenameFile` | `Void` | `cid: ChannelId (u64)`, `cpw: String` (SHA1-B64), `oldname: String`, `newname: String`, `tcid: Option<ChannelId (u64)>`, `tcpw: Option<String>` (SHA1-B64) | `Messages.cs:6417-6454`, `Full\TsFullClient.gen.cs:511-517` |
| `ftlist` | `FtList` | `Notify(notifyftlist -> FileTransfer)` | *(no parameters)* | `Messages.cs:6393-6415`, `Full\TsFullClient.gen.cs:504-510` |
| `ftstop` | `FtStop` | `Void` | `serverftfid: u16`, `delete: bool` | `Messages.cs:6456-6481`, `Full\TsFullClient.gen.cs:518-524` |

### 5.7 Bans, Complains, Privilege Keys (Tokens) & Custom Properties

| Wire Command | C# Class (`Messages.cs`) | Response Pattern | Wire Parameters (`key: RustType`) & Options (`-flag`) | Source Citations |
|---|---|---|---|---|
| `banadd` | `BanAdd` | `Direct(BanId)` | `ip: Option<String>`, `name: Option<String>`, `uid: Option<String>`, `mytsid: Option<String>`, `time: Option<DurationSeconds (f64)>`, `banreason: Option<String>` | `Messages.cs:17-54`, `TsBaseFunctions.gen.cs:7-13` |
| `banclient` | `BanClient` | `Direct(BanId[])` | `clid: ClientId (u16)` (multi-param), `time: Option<DurationSeconds (f64)>`, `banreason: Option<String>`, Option: `-continueonerror` | `Messages.cs:56-89`, `Full\TsFullClient.gen.cs:153-160` |
| `bandel` | `BanDel` | `Void` | `banid: u64` | `Messages.cs:91-116`, `Full\TsFullClient.gen.cs:161-167` |
| `bandelall` | `BanDelAll` | `Void` | *(no parameters)* | `Messages.cs:118-141`, `Full\TsFullClient.gen.cs:168-173` |
| `banlist` | `BanListRequest` | `Notify(notifybanlist -> BanList)` | `start: Option<u64>`, `duration: Option<u64>`, Option: `-count` | `Messages.cs:215-242`, `Full\TsFullClient.gen.cs:174-181` |
| `complainadd` | `ComplainAdd` | `Void` | `tcldbid: ClientDbId (u64)`, `message: String` | `Messages.cs:5445-5472`, `Full\TsFullClient.gen.cs:456-462` |
| `complaindel` | `ComplainDel` | `Void` | `tcldbid: ClientDbId (u64)`, `fcldbid: ClientDbId (u64)` | `Messages.cs:5474-5501`, `Full\TsFullClient.gen.cs:463-469` |
| `complaindelall` | `ComplainDelAll` | `Void` | `tcldbid: ClientDbId (u64)` | `Messages.cs:5503-5528`, `Full\TsFullClient.gen.cs:470-473` |
| `complainlist` | `ComplainListRequest` | `Notify(notifycomplainlist -> ComplainList)` | `tcldbid: Option<ClientDbId (u64)>` | `Messages.cs:5568-5593`, `TsBaseFunctions.gen.cs:63-69` |
| `tokenadd` / `privilegekeyadd` | `TokenAddRequest` / `PrivilegeKeyAddRequest` | `Notify(notifytokenadd -> TokenAdd)` | `tokentype: u8` (`0` = ServerGroup, `1` = ChannelGroup), `tokenid1: u64` (`sgid` or `cgid`), `tokenid2: u64` (`0` for ServerGroup, `cid` for ChannelGroup), `tokendescription: Option<String>`, `tokencustomset: Option<String>` | `Messages.cs:8019-8054, 10315-10350`, `Full\TsFullClient.gen.cs:613-620, 776-783` |
| `tokendelete` / `privilegekeydelete` | `TokenDelete` / `PrivilegeKeyDelete` | `Void` | `token: String` | `Messages.cs:8056-8081, 10352-10377`, `Full\TsFullClient.gen.cs:621-627, 784-790` |
| `tokenlist` / `privilegekeylist` | `TokenListRequest` / `PrivilegeKeyListRequest` | `Notify(notifytokenlist -> TokenList)` | *(no parameters)* | `Messages.cs:8083-8106, 10418-10441`, `Full\TsFullClient.gen.cs:628-635, 791-798` |
| `tokenuse` / `privilegekeyuse` | `TokenUse` / `PrivilegeKeyUse` | `Void` (triggers S->C `notifytokenused`) | `token: String` | `Messages.cs:8108-8133, 10443-10468`, `Full\TsFullClient.gen.cs:636-642, 799-805` |
| `custominfo` | `CustomInfoRequest` | `Direct(CustomInfo)` | `cldbid: ClientDbId (u64)` | `Messages.cs:5652-5677`, `TsBaseFunctions.gen.cs:71-77` |
| `customsearch` | `CustomSearch` | `Direct(CustomSearch)` | `ident: String`, `pattern: String` | `Messages.cs:5679-5710`, `TsBaseFunctions.gen.cs:79-85` |
| `customset` | `CustomSet` | `Void` | `cldbid: ClientDbId (u64)`, `ident: String`, `value: String` | `Messages.cs:5712-5741` |
| `customdelete` | `CustomDelete` | `Void` | `cldbid: ClientDbId (u64)`, `ident: String` | `Messages.cs:5627-5650` |

### 5.8 Virtual Server Inspection, Management & Query-Overlap Commands

| Wire Command | C# Class (`Messages.cs`) | Response Pattern | Wire Parameters (`key: RustType`) & Options (`-flag`) | Source Citations |
|---|---|---|---|---|
| `servergetvariables` | `ServerVariablesRequest` | `Void` (triggers S->C `notifyserverupdated`) | *(no parameters)* | `Messages.cs:10196-10218`, `Full\TsFullClient.gen.cs:762-768` |
| `serverrequestconnectioninfo` | `ServerConnectionInfoRequest` | `Notify(notifyserverconnectioninfo -> ServerConnectionInfo)` | *(no parameters)* | `Messages.cs:8411-8434`, `Full\TsFullClient.gen.cs:643-650` |
| `serveredit` | `ServerEdit` | `Void` (triggers S->C `notifyserveredited`) | Editable `virtualserver_*` properties (`virtualserver_name`, `virtualserver_welcomemessage`, `virtualserver_maxclients`, `virtualserver_password`, `virtualserver_default_server_group`, `virtualserver_default_channel_group`, `virtualserver_default_channel_admin_group`, `virtualserver_hostbanner_url`, `virtualserver_hostbanner_gfx_url`, `virtualserver_hostbanner_gfx_interval`, `virtualserver_hostbanner_mode`, `virtualserver_priority_speaker_dimm_modificator`, `virtualserver_hostbutton_tooltip`, `virtualserver_hostbutton_url`, `virtualserver_hostbutton_gfx_url`, `virtualserver_name_phonetic`, `virtualserver_icon_id`, `virtualserver_codec_encryption_mode`, etc.) | `Messages.cs:8489-8697`, `Full\TsFullClient.gen.cs:651-656`, `Commands\TsCommand.gen.cs:99-208` |
| `servertemppasswordadd` | `ServerTempPasswordAdd` | `Void` | `pw: String` (SHA1-B64), `desc: String`, `duration: DurationSeconds (f64)`, `tcid: Option<ChannelId (u64)>`, `tcpw: Option<String>` (SHA1-B64) | `Messages.cs:9882-9919`, `Full\TsFullClient.gen.cs:749-755` |
| `servertemppassworddel` | `ServerTempPasswordDel` | `Void` | `pw: String` (SHA1-B64) | `Messages.cs:9921-9946`, `Full\TsFullClient.gen.cs:756-761` |
| `servertemppasswordlist` | `ServerTempPasswordListRequest` | `Notify(notifyservertemppasswordlist -> ServerTempPasswordList)` | *(no parameters)* | `Messages.cs:9996-10019` |
| `logview` | `LogView` | `Notify(notifyserverlog -> ServerLog)` | `lines: Option<u8>` (`1..100`), `reverse: Option<bool>`, `instance: Option<bool>`, `begin_pos: Option<u64>` | `Messages.cs:7103-7138`, `Full\TsFullClient.gen.cs:537-543` |
| `logadd` | `LogAdd` | `Void` | `loglevel: u8` (`LogLevel`: `1`=Error..`4`=Info), `logmsg: String` | `Messages.cs:7023-7050`, `Full\TsFullClient.gen.cs:530-536` |
| `whoami` | `WhoAmIRequest` | `Direct(WhoAmI)` | *(no parameters; primary on ServerQuery, `initserver.aclid` used on Full)* | `Messages.cs:10539-10562`, `TsBaseFunctions.cs:379-384` |
| `version` | `VersionRequest` | `Direct(Version)` | *(no parameters)* | `Messages.cs:10515-10537`, `TsBaseFunctions.gen.cs:129-135` |
| `hostinfo` | `HostInfoRequest` | `Direct(HostInfo)` | *(no parameters)* | `Messages.cs:6510-6533`, `TsBaseFunctions.gen.cs:95-101` |
| `instanceinfo` | `InstanceInfo` | `Direct(InstanceInfo)` | *(no parameters)* | `Messages.cs:6980-7021` |
| `instanceedit` | `InstanceEdit` | `Void` | `serverinstance_*` properties | `Messages.cs:6924-6978`, `Full\TsFullClient.gen.cs:525-529` |
| `bindinglist` | `BindingList` | `Direct(BindingList)` | `subsystem: Option<BindingSubsystem>` | `Messages.cs:244-268` |
| `serverinfo` | `ServerInfo` | `Direct(ServerInfo)` | *(ServerQuery; Full client uses `servergetvariables`)* | `Messages.cs:9460-9682` |
| `serverlist` | `ServerListRequest` | `Direct(ServerList)` | Options: `-uid`, `-short`, `-all`, `-onlyoffline` (ServerQuery) | `Messages.cs:9684-9707` |
| `serveridgetbyport` | `ServerIdGetByPort` | `Direct(ServerId)` | `virtualserver_port: u16` (ServerQuery) | `Messages.cs:9434-9458` |
| `servercreate` | `ServerCreate` | `Direct(ServerCreated)` | `virtualserver_name: String`, plus optional `virtualserver_*` | `Messages.cs:8436-8461` |
| `serverdelete` | `ServerDelete` | `Void` | `sid: u64` | `Messages.cs:8463-8487` |
| `serverstart` | `ServerStart` | `Void` | `sid: u64` | `Messages.cs:9830-9854` |
| `serverstop` | `ServerStop` | `Void` | `sid: u64`, `reasonmsg: Option<String>` | `Messages.cs:9856-9880` |
| `serverprocessstop` | `ServerProcessStop` | `Void` | `reasonmsg: Option<String>` | `Messages.cs:9778-9803` |
| `serversnapshotcreate` | `ServerSnapshotCreate` | `Direct(Snapshot)` | `password: Option<String>` | `Messages.cs:9805-9816` |
| `serversnapshotdeploy` | `ServerSnapshotDeploy` | `Void` | `password: Option<String>`, Options: `-keepfiles`, `-mapping` | `Messages.cs:9818-9828` |
| `servernotifyregister` | `ServerNotifyRegister` | `Void` | `event: NotifyRegisterType`, `id: Option<ChannelId (u64)>` (ServerQuery only) | `Messages.cs:9729-9754` |
| `servernotifyunregister` | `ServerNotifyUnregister` | `Void` | *(ServerQuery only)* | `Messages.cs:9756-9776` |
| `login` / `logout` / `use` / `quit` | `Login` / `Logout` / `Use` / `Quit` | `Void` | *(ServerQuery only)* | `Messages.cs:7052-7101, 8135-8158, 10490-10513` |

---

## 6. Complete S->C Notification & Response Catalog

All 72 S->C notification types dispatched by `MessageHelper.GetToClientNotificationType` (`D:\Dev\TS3AudioBot\mx\TSLib\Generated\Messages.cs:10786-10864`). Every multi-record notification supports the `Expand` header-inheritance rule described in Section 2.4.

### 6.1 Handshake & Initial World State Notifications

| Wire Notification Name | C# Class (`Messages.cs`) | Trigger / Context | Wire Attributes (`key: RustType`) | Source Citations |
|---|---|---|---|---|
| `initivexpand` | `InitIvExpand` | Legacy S->C handshake step after `Init1` | `alpha: String` (10B base64), `beta: String` (10B base64), `omega: String` (server ASN.1 pubkey base64) | `Messages.cs:6611-6647`, `Full\TsFullClient.cs:261-274` |
| `initivexpand2` | `InitIvExpand2` | Modern S->C handshake step after `Init1` (`3.13.9`) | `l: String` (license block base64), `beta: String` (54B random challenge base64), `omega: String` (server ASN.1 pubkey base64), `proof: String` (ECDSA sig base64), `ot: i32`, `tvd: Option<String>` (TeamSpeak validation data base64) | `Messages.cs:6649-6700`, `Full\TsFullClient.cs:276-302` |
| `initserver` | `InitServer` | Sent after `clientinit` is accepted | `aclid: ClientId (u16)` (**assigned client ID**), `acn: String` (assigned nickname), `pv: u32` (protocol version, `6`), `lt: LicenseType (u8)`, `virtualserver_id: u64`, `virtualserver_unique_identifier: String`, `virtualserver_name: String`, `virtualserver_welcomemessage: String`, `virtualserver_platform: String`, `virtualserver_version: String`, `virtualserver_maxclients: u32`, `virtualserver_created: DateTime (u64)`, `virtualserver_codec_encryption_mode: CodecEncryptionMode (u8)`, `virtualserver_default_server_group: ServerGroupId (u64)`, `virtualserver_default_channel_group: ChannelGroupId (u64)`, `virtualserver_default_channel_admin_group: ChannelGroupId (u64)`, `virtualserver_hostbanner_url: String`, `virtualserver_hostbanner_gfx_url: String`, `virtualserver_hostbanner_gfx_interval: DurationSeconds (f64)`, `virtualserver_hostbanner_mode: BannerMode (u8)`, `virtualserver_priority_speaker_dimm_modificator: f32`, `virtualserver_hostbutton_tooltip: String`, `virtualserver_hostbutton_url: String`, `virtualserver_hostbutton_gfx_url: String`, `virtualserver_name_phonetic: String`, `virtualserver_icon_id: IconHash (i32)`, `virtualserver_ip: String[]` (comma-sep), `virtualserver_ask_for_privilegekey: bool`, `virtualserver_channel_temp_delete_delay_default: DurationSeconds (f64)`, `virtualserver_protocol_verify_keypair: String`, `virtualserver_voice_data_encryption: i32`, `client_talk_power: i32`, `client_needed_serverquery_view_power: i32`, `client_myteamspeak_id: String`, `client_integrations: String`, `TempPassword: Option<String>` (`temppw`) | `Messages.cs:6702-6922`, `Full\TsFullClient.cs:304-316` |
| `channellist` | `ChannelList` | Streamed during connect (1+ packets) or response to `channellist` | `cid: ChannelId (u64)`, `cpid: ChannelId (u64)`, `channel_name: String`, `channel_topic: String`, `channel_codec: Codec (u8)`, `channel_codec_quality: u8`, `channel_maxclients: i32`, `channel_maxfamilyclients: i32`, `channel_order: ChannelId (u64)`, `channel_flag_permanent: bool`, `channel_flag_semi_permanent: bool`, `channel_flag_default: bool`, `channel_flag_password: bool`, `channel_codec_latency_factor: i32`, `channel_codec_is_unencrypted: bool`, `channel_delete_delay: u32`, `channel_flag_maxclients_unlimited: bool`, `channel_flag_maxfamilyclients_unlimited: bool`, `channel_flag_maxfamilyclients_inherited: bool`, `channel_filepath: Option<String>`, `channel_needed_talk_power: i32`, `channel_forced_silence: bool`, `channel_name_phonetic: String`, `channel_icon_id: IconHash (i32)`, `channel_banner_gfx_url: String`, `channel_banner_mode: BannerMode (u8)`, `total_clients_family: Option<i32>`, `total_clients: Option<i32>`, `seconds_empty: Option<i32>` | `Messages.cs:1750-1878`, `Full\TsFullClient.cs:347-351` |
| `channellistfinished` | `ChannelListFinished` | Marks end of initial `channellist` stream | *(no attributes)* | `Messages.cs:1880-1899`, `Full\TsFullClient.cs:347-351` |
| `error` | `CommandError` | Acknowledges every command with `return_code` (or handshake errors) | `id: TsErrorCode (u32)`, `msg: String`, `extra_msg: Option<String>`, `failed_permid: Option<TsPermission (u16)>`, `return_code: Option<String>` | `Messages.cs:5398-5443`, `MessageProcessor.cs:76-98` |

### 6.2 Client View, Movement & State Update Notifications

| Wire Notification Name | C# Class (`Messages.cs`) | Trigger / Context | Wire Attributes (`key: RustType`) | Source Citations |
|---|---|---|---|---|
| `notifycliententerview` | `ClientEnterView` | Client connects, moves into subscribed channel, or channel is subscribed | `cfid: ChannelId (u64)` (`0` on connect), `ctid: ChannelId (u64)`, `reasonid: Reason (u8)`, `clid: ClientId (u16)`, `client_unique_identifier: String`, `client_nickname: String`, `client_input_muted: bool`, `client_output_muted: bool`, `client_outputonly_muted: bool`, `client_input_hardware: bool`, `client_output_hardware: bool`, `client_meta_data: String`, `client_is_recording: bool`, `client_database_id: ClientDbId (u64)`, `client_channel_group_id: ChannelGroupId (u64)`, `client_servergroups: ServerGroupId[]` (comma-sep `u64`), `client_away: bool`, `client_away_message: String`, `client_type: ClientType (u8)` (`0`=Full, `1`=Query), `client_flag_avatar: String`, `client_talk_power: i32`, `client_talk_request: bool`, `client_talk_request_msg: String`, `client_description: String`, `client_is_talker: bool`, `client_is_priority_speaker: bool`, `client_unread_messages: u32`, `client_nickname_phonetic: String`, `client_needed_serverquery_view_power: i32`, `client_icon_id: IconHash (i32)`, `client_is_channel_commander: bool`, `client_country: String`, `client_channel_group_inherited_channel_id: ChannelId (u64)`, `client_badges: String`, `client_myteamspeak_id: String`, `client_integrations: String`, `client_active_integrations_info: String`, `invokerid: Option<ClientId (u16)>`, `invokername: Option<String>`, `invokeruid: Option<String>` | `Messages.cs:2947-3023` |
| `notifyclientleftview` | `ClientLeftView` | Client disconnects, is kicked/banned, or moves to unsubscribed channel | `cfid: ChannelId (u64)`, `ctid: ChannelId (u64)` (`0` on disconnect/kick/ban), `reasonid: Reason (u8)` (`3`=LostConnection, `4`=KickChannel, `5`=KickServer, `6`=Ban, `8`=LeftServer, `10`=ServerStopped), `clid: ClientId (u16)`, `reasonmsg: Option<String>`, `invokerid: Option<ClientId (u16)>`, `invokername: Option<String>`, `invokeruid: Option<String>`, `bantime: Option<DurationSeconds (f64)>` | `Messages.cs:3538-3607`, `Full\TsFullClient.cs:335-345` |
| `notifyclientmoved` | `ClientMoved` | Client changes channel (both channels in view) | `ctid: ChannelId (u64)`, `reasonid: Reason (u8)` (`0`=Self, `1`=MovedByOther, `4`=KickChannel), `clid: ClientId (u16)`, `cfid: Option<ChannelId (u64)>`, `reasonmsg: Option<String>`, `invokerid: Option<ClientId (u16)>`, `invokername: Option<String>`, `invokeruid: Option<String>` | `Messages.cs:4225-4281` |
| `notifyclientupdated` | `ClientUpdated` | Client properties changed (`clientupdate`, `clientedit`, or `clientgetvariables` response) | `clid: ClientId (u16)`, plus any subset of `ClientEnterView` / `ClientInfo` properties (`client_nickname`, `client_input_muted`, `client_output_muted`, `client_away`, `client_away_message`, `client_talk_power`, `client_description`, `client_icon_id`, `client_badges`, `client_version`, `client_platform`, `client_created`, `client_lastconnected`, `client_totalconnections`, `client_month_bytes_uploaded`, `client_month_bytes_downloaded`, `client_total_bytes_uploaded`, `client_total_bytes_downloaded`, `connection_client_ip`, etc.) | `Messages.cs:5007-5369`, `Full\TsFullClient.cs:562-566` |
| `notifyclientpoke` | `ClientPoke` | Another client pokes us | `invokerid: ClientId (u16)`, `invokername: String`, `invokeruid: String`, `msg: String` | `Messages.cs:4593-4619` |
| `notifyclientchatcomposing` | `ClientChatComposing` | Remote client is typing in private chat | `clid: ClientId (u16)`, `cluid: String` | `Messages.cs:2383-2408` |
| `notifyclientchatclosed` | `ClientChatClosed` | Remote client closed private chat tab | `clid: ClientId (u16)`, `cluid: String` | `Messages.cs:2354-2381` |
| `notifyconnectioninforequest` | `ClientConnectionInfoRequest` | Server requests `setconnectioninfo` telemetry from us | `invokerids: ClientId[]` (comma-sep `u16`) | `Messages.cs:2437-2462`, `Full\TsFullClient.cs:353-358` |
| `notifyconnectioninfo` | `ClientConnectionInfo` | Response to `getconnectioninfo` | `clid: ClientId (u16)`, `connection_filetransfer_bandwidth_sent: u64`, `connection_filetransfer_bandwidth_received: u64`, `connection_connected_time: DurationMilliseconds (f64)`, `connection_idle_time: DurationMilliseconds (f64)`, `connection_client_ip: String`, plus all 30 `connection_*` packet/byte/bandwidth/ping/loss fields from `setconnectioninfo` | `Messages.cs:2410-2435` |

### 6.3 Client Lookup & Database Notifications

| Wire Notification Name | C# Class (`Messages.cs`) | Trigger / Context | Wire Attributes (`key: RustType`) | Source Citations |
|---|---|---|---|---|
| `notifyclientids` | `ClientIds` | Response to `clientgetids` | `cluid: String`, `clid: ClientId (u16)`, `name: String` | `Messages.cs:3052-3079` |
| `notifyclientdbidfromuid` | `ClientDbIdFromUid` | Response to `clientgetdbidfromuid` | `cluid: String`, `cldbid: ClientDbId (u64)` | `Messages.cs:2574-2600` |
| `notifyclientnamefromuid` | `ClientNameFromUid` | Response to `clientgetnamefromuid` | `cluid: String`, `cldbid: ClientDbId (u64)`, `name: String` | `Messages.cs:4310-4337` |
| `notifyclientnamefromdbid` | `ClientNameFromDbId` | Response to `clientgetnamefromdbid` | `cluid: String`, `cldbid: ClientDbId (u64)`, `name: String` | `Messages.cs:4283-4308` |
| `notifyclientuidfromclid` | `ClientUidFromClid` | Response to `clientgetuidfromclid` | `clid: ClientId (u16)`, `cluid: String`, `nickname: String` | `Messages.cs:4863-4891` |
| `notifyclientdblist` | `ClientDbList` | Response to `clientdblist` | `cldbid: ClientDbId (u64)`, `client_unique_identifier: String`, `client_nickname: String`, `client_created: DateTime (u64)`, `client_lastconnected: DateTime (u64)`, `client_totalconnections: u32`, `client_description: String`, `client_lastip: Option<String>`, ` client_login_name: Option<String>`, `count: Option<u64>` | `Messages.cs:2745-2801` |
| `notifyclientdbfind` | `ClientDbFind` | Response to `clientdbfind` | `cldbid: ClientDbId (u64)` | `Messages.cs:2520-2545` |
| `notifyclientsetserverquerylogin` | `ClientSetServerQueryLogin` | Response to `clientsetserverquerylogin` | `client_login_password: String` | `Messages.cs:4809-4834` |

### 6.4 Channel Event & Subscription Notifications

| Wire Notification Name | C# Class (`Messages.cs`) | Trigger / Context | Wire Attributes (`key: RustType`) | Source Citations |
|---|---|---|---|---|
| `notifychannelcreated` | `ChannelCreated` | Channel created on server | `cid: ChannelId (u64)`, `cpid: ChannelId (u64)`, `invokerid: ClientId (u16)`, `invokername: String`, `invokeruid: String`, plus all `channel_*` properties (`channel_name`, `channel_topic`, `channel_codec`, `channel_order`, etc.) | `Messages.cs:627-653` |
| `notifychanneledited` | `ChannelEdited` | Channel properties edited (or response to `channelgetdescription`) | `cid: ChannelId (u64)`, `reasonid: Reason (u8)`, `invokerid: Option<ClientId (u16)>`, `invokername: Option<String>`, `invokeruid: Option<String>`, plus modified `channel_*` properties (e.g. `channel_description`, `channel_name`, `channel_topic`, etc.) | `Messages.cs:874-960` |
| `notifychannelchanged` | `ChannelChanged` | Channel flags/settings changed by server | `cid: ChannelId (u64)`, plus updated `channel_*` properties | `Messages.cs:303-391` |
| `notifychanneldeleted` | `ChannelDeleted` | Channel deleted | `cid: ChannelId (u64)`, `invokerid: ClientId (u16)`, `invokername: String`, `invokeruid: String` | `Messages.cs:655-682` |
| `notifychannelmoved` | `ChannelMoved` | Channel parent or sort order changed | `cid: ChannelId (u64)`, `cpid: ChannelId (u64)`, `order: ChannelId (u64)`, `reasonid: Reason (u8)`, `invokerid: ClientId (u16)`, `invokername: String`, `invokeruid: String` | `Messages.cs:1957-1997` |
| `notifychanneldescriptionchanged` | `ChannelDescriptionChanged` | Channel description was modified (client should call `channelgetdescription` when viewing) | `cid: ChannelId (u64)` | `Messages.cs:715-734` |
| `notifychannelpasswordchanged` | `ChannelPasswordChanged` | Channel password state changed | `cid: ChannelId (u64)` | `Messages.cs:1999-2018` |
| `notifychannelsubscribed` | `ChannelSubscribed` | Response to `channelsubscribe` / `channelsubscribeall` | `cid: ChannelId (u64)`, `es: u64` (empty seconds / event sequence) | `Messages.cs:2144-2172` |
| `notifychannelunsubscribed` | `ChannelUnsubscribed` | Response to `channelunsubscribe` / `channelunsubscribeall` | `cid: ChannelId (u64)` | `Messages.cs:2226-2257` |

### 6.5 Text Message, Plugin & Offline Message Notifications

| Wire Notification Name | C# Class (`Messages.cs`) | Trigger / Context | Wire Attributes (`key: RustType`) | Source Citations |
|---|---|---|---|---|
| `notifytextmessage` | `TextMessage` | Chat message received | `targetmode: TextMessageTargetMode (u8)` (`1`=Client, `2`=Channel, `3`=Server), `msg: String`, `target: Option<ClientId (u16)>`, `invokerid: ClientId (u16)`, `invokername: String`, `invokeruid: String` | `Messages.cs:10251-10287` |
| `notifyplugincmd` | `PluginCommand` | Plugin message received | `name: String`, `data: String`, `invokerid: ClientId (u16)`, `invokername: String`, `invokeruid: String` | `Messages.cs:7951-7984`, `Full\TsFullClient.cs:318-322` |
| `notifyofflinemessage` | `OfflineMessage` | Response to `messageget` | `msgid: u64`, `cluid: String`, `subject: String`, `message: String`, `timestamp: DateTime (u64)`, `flag_read: bool` | `Messages.cs:7140-7184` |
| `notifyofflinemessagelist` | `OfflineMessageList` | Response to `messagelist` | `msgid: u64`, `cluid: String`, `subject: String`, `timestamp: DateTime (u64)`, `flag_read: bool` | `Messages.cs:7271-7329` |

### 6.6 Group, Permission & Hint Notifications

| Wire Notification Name | C# Class (`Messages.cs`) | Trigger / Context | Wire Attributes (`key: RustType`) | Source Citations |
|---|---|---|---|---|
| `notifypermissionlist` | `PermList` | Response to `permissionlist` | Group entries: `group_id_end: u16`; Permission entries: `permid: u16`, `permname: String`, `permdesc: String` | `Messages.cs:7506-7758`, `Full\TsFullClient.cs:360-371` |
| `notifyclientneededpermissions` | `ClientNeededPermissions` | Sent on connect and when effective permissions change | `permid: TsPermission (u16)` (`p`), `permvalue: i32` (`v`) | `Messages.cs:4366-4399` |
| `notifyclientpermhints` | `ClientPermissionHints` | Permission hints for visible client | `clid: ClientId (u16)`, `permid: TsPermission (u16)` (`p`), `permvalue: i32` (`v`) | `Messages.cs:4401-4436` |
| `notifychannelpermhints` | `ChannelPermissionHints` | Permission hints for channel | `cid: ChannelId (u64)`, `permid: TsPermission (u16)` (`p`), `permvalue: i32` (`v`) | `Messages.cs:2020-2061` |
| `notifyservergrouplist` | `ServerGroupList` | Sent on connect or response to `servergrouplist` | `sgid: ServerGroupId (u64)`, `name: String`, `type: GroupDatabaseType (u8)`, `iconid: IconHash (i32)`, `savedb: bool`, `sortid: i32`, `namemode: GroupNamingMode (u8)`, `n_modifyp: i32`, `n_member_addp: i32`, `n_member_removep: i32` | `Messages.cs:9188-9255` |
| `notifychannelgrouplist` | `ChannelGroupList` | Sent on connect or response to `channelgrouplist` | `cgid: ChannelGroupId (u64)`, `name: String`, `type: GroupDatabaseType (u8)`, `iconid: IconHash (i32)`, `savedb: bool`, `sortid: i32`, `namemode: GroupNamingMode (u8)`, `n_modifyp: i32`, `n_member_addp: i32`, `n_member_removep: i32` | `Messages.cs:1459-1522` |
| `notifyclientservergroupadded` | `ClientServerGroupAdded` | Client added to server group | `sgid: ServerGroupId (u64)`, `clid: ClientId (u16)`, `name: String`, `cluid: String`, `cldbid: ClientDbId (u64)`, `invokerid: ClientId (u16)`, `invokername: String`, `invokeruid: String` | `Messages.cs:4650-4728` |
| `notifyclientservergroupremoved` | `ClientServerGroupRemoved` | Client removed from server group | `sgid: ServerGroupId (u64)`, `clid: ClientId (u16)`, `name: String`, `cluid: String`, `cldbid: ClientDbId (u64)`, `invokerid: ClientId (u16)`, `invokername: String`, `invokeruid: String` | `Messages.cs:4730-4807` |
| `notifyclientchannelgroupchanged` | `ClientChannelGroupChanged` | Client's channel group changed in a channel | `cgid: ChannelGroupId (u64)`, `clid: ClientId (u16)`, `cldbid: ClientDbId (u64)`, `cid: ChannelId (u64)`, `cgci: ChannelId (u64)` (inherited channel ID), `invokerid: ClientId (u16)`, `invokername: String`, `invokeruid: String` | `Messages.cs:2294-2325` |
| `notifyservergroupclientlist` | `ServerGroupClientList` | Response to `servergroupclientlist` | `sgid: Option<ServerGroupId (u64)>`, `cldbid: ClientDbId (u64)`, `client_nickname: Option<String>`, `client_unique_identifier: Option<String>` | `Messages.cs:8990-9035` |
| `notifyservergroupsbyclientid` | `ServerGroupsByClientId` | Response to `servergroupsbyclientid` | `name: String`, `sgid: ServerGroupId (u64)`, `cldbid: ClientDbId (u64)` | `Messages.cs:9372-9405` |
| `notifychannelgroupclientlist` | `ChannelGroupClientList` | Response to `channelgroupclientlist` | `cid: ChannelId (u64)`, `cldbid: ClientDbId (u64)`, `cgid: ChannelGroupId (u64)` | `Messages.cs:1290-1331` |
| `notifyservergrouppermlist` | `ServerGroupPermList` | Response to `servergrouppermlist` | `sgid: Option<ServerGroupId (u64)>`, `permid: TsPermission (u16)`, `permsid: Option<String>`, `permvalue: i32`, `permnegated: bool`, `permskip: bool` | `Messages.cs:9282-9312` |
| `notifychannelgrouppermlist` | `ChannelGroupPermList` | Response to `channelgrouppermlist` | `cgid: Option<ChannelGroupId (u64)>`, `permid: TsPermission (u16)`, `permsid: Option<String>`, `permvalue: i32`, `permnegated: bool`, `permskip: bool` | `Messages.cs:1549-1579` |
| `notifychannelpermlist` | `ChannelPermList` | Response to `channelpermlist` | `cid: Option<ChannelId (u64)>`, `permid: TsPermission (u16)`, `permsid: Option<String>`, `permvalue: i32`, `permnegated: bool`, `permskip: bool` | `Messages.cs:2020-2061` |
| `notifyclientpermlist` | `ClientPermList` | Response to `clientpermlist` | `cldbid: Option<ClientDbId (u64)>`, `permid: TsPermission (u16)`, `permsid: Option<String>`, `permvalue: i32`, `permnegated: bool`, `permskip: bool` | `Messages.cs:4438-4562` |
| `notifychannelclientpermlist` | `ChannelClientPermList` | Response to `channelclientpermlist` | `cid: Option<ChannelId (u64)>`, `cldbid: Option<ClientDbId (u64)>`, `permid: TsPermission (u16)`, `permsid: Option<String>`, `permvalue: i32`, `permnegated: bool`, `permskip: bool` | `Messages.cs:461-485` |
| `notifypermfind` | `PermFind` | Response to `permfind` | `t: PermissionType (u8)`, `id1: u64`, `id2: u64`, `p: TsPermission (u16)`, `v: i32` | `Messages.cs:7385-7419` |
| `notifypermoverview` | `PermOverview` | Response to `permoverview` | `t: PermissionType (u8)`, `id1: u64`, `id2: u64`, `p: TsPermission (u16)`, `v: i32`, `n: bool`, `s: bool` | `Messages.cs:7785-7836` |

### 6.7 File Transfer Notifications

| Wire Notification Name | C# Class (`Messages.cs`) | Trigger / Context | Wire Attributes (`key: RustType`) | Source Citations |
|---|---|---|---|---|
| `notifyftinitupload` | `FileUpload` | Response to `ftinitupload` | `clientftfid: u16`, `serverftfid: u16`, `ftkey: String` (32-char ASCII token sent over TCP), `port: u16`, `ip: String[]` (comma-sep), `seekpos: u64`, `proto: i32`, `status: Option<TsErrorCode (u32)>`, `msg: Option<String>` | `Messages.cs:6040-6100`, `TsBaseFunctions.FileTransfer.cs:48-115` |
| `notifyftinitdownload` | `FileDownload` | Response to `ftinitdownload` | `clientftfid: u16`, `serverftfid: u16`, `ftkey: String`, `port: u16`, `ip: String[]` (comma-sep), `size: u64`, `proto: i32`, `status: Option<TsErrorCode (u32)>`, `msg: Option<String>` | `Messages.cs:5743-5803`, `TsBaseFunctions.FileTransfer.cs:183-242` |
| `notifyallfilestransferred` / `notifyscript` / `notifystatusfiletransfer` | `FileTransferStatus` | Sent when a file transfer finishes or aborts | `clientftfid: u16`, `status: TsErrorCode (u32)` (`0` or `file_transfer_complete = 0x0800`), `msg: String`, `size: u64` | `Messages.cs:5999-6038`, `TsBaseFunctions.FileTransfer.cs:295-324` |
| `notifyfilelist` | `FileList` | Streamed in response to `ftgetfilelist` | `cid: ChannelId (u64)`, `path: String`, `name: String`, `size: u64`, `datetime: DateTime (u64)`, `type: FileType (u8)` (`0`=Directory, `1`=Regular) | `Messages.cs:5855-5918` |
| `notifyfilelistfinished` | `FileListFinished` | Marks end of `notifyfilelist` stream | `cid: ChannelId (u64)`, `path: String` | `Messages.cs:5920-5947` |
| `notifyfileinfo` | `FileInfo` | Response to `ftgetfileinfo` | `cid: ChannelId (u64)`, `name: String`, `size: u64`, `datetime: DateTime (u64)` | `Messages.cs:5805-5853` |
| `notifyftlist` | `FileTransfer` | Response to `ftlist` | `clid: ClientId (u16)`, `cldbid: ClientDbId (u64)`, `path: String`, `name: String`, `size: u64`, `sizedone: u64`, `clientftfid: u16`, `serverftfid: u16`, `sender: u8`, `status: u8`, `current_speed: f64`, `average_speed: f64`, `runtime: DurationMilliseconds (f64)` | `Messages.cs:5949-5997` |

### 6.8 Server, Ban, Complain, Token & Log Notifications

| Wire Notification Name | C# Class (`Messages.cs`) | Trigger / Context | Wire Attributes (`key: RustType`) | Source Citations |
|---|---|---|---|---|
| `notifyserveredited` | `ServerEdited` | Server settings edited | `reasonid: Reason (u8)`, `invokerid: ClientId (u16)`, `invokername: String`, `invokeruid: String`, plus modified `virtualserver_*` fields | `Messages.cs:8699-8825` |
| `notifyserverupdated` | `ServerUpdated` | Response to `servergetvariables` | Full `virtualserver_*` runtime statistics and properties (`virtualserver_clientsonline`, `virtualserver_channelsonline`, `virtualserver_uptime`, `connection_*`, etc.) | `Messages.cs:10021-10194` |
| `notifyserverconnectioninfo` | `ServerConnectionInfo` | Response to `serverrequestconnectioninfo` | `connection_filetransfer_bandwidth_sent: f64`, `connection_filetransfer_bandwidth_received: f64`, `connection_filetransfer_bytes_sent_total: u64`, `connection_filetransfer_bytes_received_total: u64`, `connection_packets_sent_total: u64`, `connection_bytes_sent_total: u64`, `connection_packets_received_total: u64`, `connection_bytes_received_total: u64`, `connection_bandwidth_sent_last_second_total: f64`, `connection_bandwidth_sent_last_minute_total: f64`, `connection_bandwidth_received_last_second_total: f64`, `connection_bandwidth_received_last_minute_total: f64`, `connection_connected_time: DurationSeconds (f64)`, `connection_packetloss_total: f64`, `connection_ping: f64` | `Messages.cs:8331-8409` |
| `notifybanlist` | `BanList` | Response to `banlist` | `banid: u64`, `ip: String`, `name: String`, `uid: String`, `mytsid: String`, `lastnickname: String`, `created: DateTime (u64)`, `duration: DurationSeconds (f64)`, `invokername: String`, `invokercldbid: ClientDbId (u64)`, `invokeruid: String`, `reason: String`, `enforcements: u32` | `Messages.cs:143-213` |
| `notifycomplainlist` | `ComplainList` | Response to `complainlist` | `tcldbid: ClientDbId (u64)`, `tname: String`, `fcldbid: ClientDbId (u64)`, `fname: String`, `message: String`, `timestamp: DateTime (u64)` | `Messages.cs:5530-5566` |
| `notifytokenadd` | `TokenAdd` | Response to `tokenadd` / `privilegekeyadd` | `token: String` | `Messages.cs:10289-10313` |
| `notifytokenlist` | `TokenList` | Response to `tokenlist` / `privilegekeylist` | `token: String`, `token_type: TokenType (u8)`, `token_id1: u64`, `token_id2: u64`, `token_created: DateTime (u64)`, `token_description: String`, `token_customset: String` | `Messages.cs:10379-10416` |
| `notifytokenused` | `TokenUsed` | Privilege key used by client | `clid: ClientId (u16)`, `cldbid: ClientDbId (u64)`, `cluid: String`, `token: String`, `tokencustomset: String`, `token1: u64`, `token2: u64` | `Messages.cs:10470-10488` |
| `notifyservertemppasswordlist` | `ServerTempPasswordList` | Response to `servertemppasswordlist` | `nickname: String`, `uid: String`, `desc: String`, `pw_clear: String`, `start: DateTime (u64)`, `end: DateTime (u64)`, `tcid: ChannelId (u64)` | `Messages.cs:9948-9994` |
| `notifyserverlog` | `ServerLog` | Response to `logview` | `last_pos: Option<u64>`, `file_size: Option<u64>`, `l: String` (log line) | `Messages.cs:9709-9727` |
| `notifychannelcreated`..*(see 6.4)* | *(covered in 6.1–6.7)* | — | — | `Messages.cs:10786-10864` |
