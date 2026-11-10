# Reverse-Engineered Architecture: TS3 Client Configuration, Chat Logging, BBCode & Emoticons

## 1. Directory Topology & Windows Storage Locations

The Windows TeamSpeak 3 Client (v3.6.x) splits client data across three primary system locations:

### 1.1 `C:\Users\<User>\AppData\Local\TeamSpeak 3` (Ephemeral Web Engine State)
Reserved strictly for the integrated Qt WebEngine / Chromium embedded framework:
- `QtWebEngine/Default/GPUCache/`: Hardware-accelerated OpenGL / Direct3D shader cache binaries (`data_0`..`data_3`, `index`).
- `QtWebEngine/Default/Session Storage/`: LevelDB state logs (`000003.log`, `CURRENT`, `MANIFEST`).
- `QtWebEngine/Default/Platform Notifications/`: Desktop notification dispatch states.
- `QtWebEngine/Default/user_prefs.json`: Chromium renderer preferences.

### 1.2 `C:\Users\<User>\AppData\Roaming\TS3Client` (Persistent Client Configuration)
Houses all user configuration databases, chat logs, cache artifacts, and installed addons:
- `settings.db`: Master SQLite 3 database storing user configuration, profiles, identities metadata, bookmarks metadata, hotkeys, and GUI geometries.
- `urls.db`: SQLite 3 database recording URL history captured by the client's URL Catcher.
- `chats/<server_folder_hash>/`: Channel, server, and private chat transcripts in `.html` and `.txt`.
- `cache/`: Cached server icons, avatars, channel subscriptions, and serialized permissions (`perm.dat`).
- `logs/`: Application execution logs formatted as `ts3client_YYYY-MM-DD__HH_MM_SS.log`.
- `styles/`: User-installed theme `.qss` style sheets and `.tpl` infoframe templates (e.g. `Darcula.qss`, `Darcula_chat.qss`).
- `gfx/`: Custom icon packs and emoticon sets (e.g. `gfx/Ombre Violet/emoticons/`).
- `plugins/`: User-installed dynamic libraries (`.dll`).
- `addons.ini`: Manifest registering installed plugins, styles, soundpacks, and icon packs.
- `clientquery.ini`: Telnet/TCP configuration for the ClientQuery remote automation interface ([`clientquery.ini:1-5`](file:///C:/Users/Sakat/AppData/Roaming/TS3Client/clientquery.ini#L1-L5)).

### 1.3 `C:\Program Files\TeamSpeak 3 Client` (Application Distribution)
Read-only core application assets:
- `ts3client_win64.exe`: Main Qt5-based 64-bit client binary.
- `styles/`: Default Qt style sheets (`default.qss`, `default_chat.qss`, `default_chat_darkmode.qss`) and templates (`channelinfo.tpl`, `clientinfo.tpl`, `serverinfo.tpl`) ([`default_chat.qss:1-100`](file:///C:/Program%20Files/TeamSpeak%203%20Client/styles/default_chat.qss#L1-L100)).
- `gfx/`: Compressed base assets: `default.zip`, `default_colored_2014.zip`, `default_mono_2014.zip`, `countries.zip`. Default emoticons are packaged in `default.zip` under `emoticons/`.
- `translations/`: Compiled Qt translation catalogs (`lagos_<lang>.qm`, `permissions_<lang>.qm`, `ts_core_errors_<lang>.qm`).

---

## 2. SQLite Database Schemas & Configuration Engine

TeamSpeak 3 utilizes SQLite 3 for local state storage. 

> [!IMPORTANT]
> **Security Mandate**: Never export, display, or commit private keys, identity ECC key pairs, server passwords, or personal security tokens. Only the table schemas, structural layouts, and non-sensitive keys are documented below.

### 2.1 `settings.db` Table Architecture
`settings.db` implements a uniform Key-Value table model across 18 distinct domain tables:

```sql
CREATE TABLE <TableName> (
    timestamp INTEGER UNSIGNED NOT NULL,
    key VARCHAR NOT NULL UNIQUE,
    value VARCHAR
);
CREATE INDEX index_<TableName>_key ON <TableName> (key);
```

#### Table Index & Domain Mapping:
| Table Name | Stored Domain | Sample Non-Sensitive Keys | Serialization Format |
|---|---|---|---|
| `General` | Client lifecycle & version state | `DatabaseVersion`, `LicenseVersion`, `LastLaunch`, `CurrentVersions` | Flat values; `CurrentVersions` uses newline-delimited `k=v` |
| `Application` | Core GUI & behavioral toggles | `QtStyleSheet`, `IconPack`, `AdvancedPermissions`, `WarnTalkMuted`, `AutoOpenChannelChat` | String or integer boolean (`0`/`1`) |
| `Profiles` | Audio capture & playback profiles | `DefaultCaptureProfile`, `Capture/Default`, `Capture/Default/PreProcessing` | Multiline INI format for device settings & pre-processing flags |
| `ProtobufItems` | MyTeamSpeak cloud sync cache | Item IDs (`1`, `2`, `3`, ...) | Protobuf binary payload containing UUIDs, bookmarks, badges |
| `Contacts` | Friend / Foe manager database | Contact index (`0`, `1`, `2`, ...) | Multiline INI format with client UID associations |
| `Connecting` | Last connected server connection state | `LastUsedServer`, `LastUsedServerNickName`, `LastUsedServerHotkeyProfile` | Flat string values |
| `Positions3D` | 3D Positional audio user spatial offsets | Server UID folder hash (Base64) | Multiline blocks defining client coordinates `(PosX, PosY, PosZ)` |
| `FileTransfer` | File transfer preferences | `UploadDir`, `SimultaneousDownloads`, `UploadBandwidth`, `DownloadBandwidth` | Strings and integer limits |
| `Global` | Window geometries & splitter states | `MainWindowGeometry`, `ChatAreaHeight`, `InfoAreaWidth`, `BookmarksManagerGeometry` | Qt serialized binary geometry / integer pixels |
| `Permissions` | Permission dialog layout states | `OverView`, `PermServerGroupWidget`, `PermChannelWidget` | Serialized Qt widget states |
| `Tables` | Table header states | `BanListHeaderState`, `FileTransferTreeViewHeader` | Qt serialized QHeaderView state |
| `Addons` | Addon UUID metadata | Installed addon UUIDs | Addon metadata string |
| `Integrations` | Twitch / Discord integration state | `IntegrationsListData`, `IntegrationsListTimestamp` | JSON / binary metadata |
| `Badges` | Overwolf / MyTS badge definitions | `BadgesListData`, `BadgesListTimestamp` | JSON payload of badge GUIDs and SVG endpoints |
| `News` | Client update & news cache | `LastModified`, `NewsClosed`, `NewsText`, `NextCheck` | Plaintext / HTML news announcement |
| `Misc` | Miscellaneous state | `WarnWhenMutedInfoShown`, `BanHighlightOwn`, `LastBanReason` | Flat values |
| `Notifications` | Sound pack configuration | `SoundPack` (e.g. `default`) | String identifier |
| `Ask` | Suppressed confirmation dialogs | `ExternalLinkNoWarning` | Integer boolean (`0`/`1`) |

### 2.2 Domain Value Formats

#### 2.2.1 Audio Preprocessing (`Profiles: Capture/Default/PreProcessing`)
Values are stored as multiline key-value strings:
```ini
typing_suppression=1
echo_reduction_db=10
continous_transmission=0
echo_reduction=0
denoiser_level=0
echo_cancellation=0
vad_over_ptt=0
vad=1
agc=1
voiceactivation_level=-20
vad_mode=0
denoise=1
delay_ptt=0
delay_ptt_msecs=300
```

#### 2.2.2 Contacts Serialization (`Contacts: <index>`)
Each contact is serialized as an INI-style block:
```ini
Nickname=TargetNick
Friend=1
Automute=false
IgnorePublicMessages=false
IgnorePrivateMessages=false
IgnorePokes=false
IgnoreAvatar=false
IgnoreAwayMessage=false
NickShowType=0
HaveVolumeModifier=false
VolumeModifier=0.0
WhisperAllow=false
PhoneticNickname=
LastSeen=1778946440
LastSeenServerName=Community Server
LastSeenServerAddress=ts.example.com
IDS=BUe0crYoEkIlaNtA3zigwRLEsGA=,B9bC7JYHswcVGNZV/rwpDCW7FgE=
```
- `Friend`: `0` = Neutral, `1` = Friend, `2` = Blocked/Foe.
- `IDS`: Comma-separated list of 28-character client Base64 public key hashes.

#### 2.2.3 3D Positional Audio (`Positions3D: <ServerUIDHash>`)
Spatial coordinates for clients in 3D-enabled channels:
```ini
ChannelID=1
Name=akd420
Id=2736
Uid=BUe0crYoEkIlaNtA3zigwRLEsGA=
Type=1
PosX=0.04219409450888634
PosY=1.0
PosZ=-0.02857142873108387
Order=001
```

### 2.3 `urls.db` (URL Catcher Database)
Captured URLs from channel and private chats are recorded in `urls.db`:
```sql
CREATE TABLE urls_table (
    url VARCHAR NOT NULL UNIQUE,
    times_mentioned INTEGER UNSIGNED NOT NULL,
    last_mentioned INTEGER UNSIGNED NOT NULL,
    mentioned_by VARCHAR NOT NULL
);
CREATE INDEX index_urls_table_key ON urls_table (URL);
```
- `url`: Percent-encoded full URL string (e.g. `https%3A%2F%2Fgithub.com%2F...`).
- `times_mentioned`: Running counter of URL frequency in chats.
- `last_mentioned`: Unix epoch timestamp in seconds.
- `mentioned_by`: Last client nickname that posted the link.

---

## 3. Cache Storage & Hash Encoding (`cache/`)

### 3.1 Server UID Folder Hashing
In both `cache/` and `chats/`, server-specific directories are named using a double-Base64 encoding scheme derived from the virtual server's unique identifier (`virtualserver_unique_identifier`):
1. A virtual server identifier is already a 28-character Base64 SHA-1 digest (e.g. `UEB0vIkQ4h4dA6ERA/iabSod6vU=`).
2. Because standard Base64 may contain `/` characters (which are illegal directory separators on Windows and Linux), TS3 runs the ASCII byte representation of that string through Base64 encoding again (`QByteArray::toBase64()`):
   $$\text{Folder Name} = \text{Base64}\big(\text{ASCII}(\text{virtualserver\_unique\_identifier})\big)$$
   Example:
   `UEB0vIkQ4h4dA6ERA/iabSod6vU=` $\rightarrow$ `VUVCMHZJa1E0aDRkQTZFUkEvaWFiU29kNnZVPQ==`

### 3.2 Directory Contents per Server
Inside `cache/<ServerUIDHash>/`:
- `clients/`: Cached user avatars saved as `avatar_<hash>`, where `<hash>` is a hex/base-16 encoding of the user's identity UID.
- `icons/`: Downloaded channel and server group icons (16x16 PNGs named by icon ID, e.g. `12345` or `dummy.png`).
- `channels/cache.dat`: 4-byte channel cache state descriptor (`0x00000000`).
- `perm.dat`: Cached permission list serialized via Qt `QDataStream` using UTF-16BE strings for permission names (e.g. `i_channel_needed_join_power`).

### 3.3 Root Cache Storage
- `subscribed_channels_storage.dat`: Binary Qt stream storing subscribed channel lists per server. Header starts with magic `0xEC 0x90 0x28 0xA1` followed by version `0x00000001`.
- `subscription_mode_storage.dat`: Channel subscription mode flags. Magic `0x8D 0x72 0xC1 0xF3`.
- `servericons.dat`: Index of cached server icons. Magic `0xC7 0xFB 0x28 0xA2`.

---

## 4. Chat Log Storage & Architecture (`chats/`)

TeamSpeak 3 records logs on a per-server basis under `C:\Users\<User>\AppData\Roaming\TS3Client\chats\<ServerUIDHash>/`.

### 4.1 Log File Classification
Each server directory can contain three distinct scopes:
1. `server.html` / `server.txt`: Server-wide events (welcome messages, server broadcasts, connects, disconnects).
2. `channel.html` / `channel.txt`: Active channel chat text and channel join/switch announcements.
3. `<ClientUIDHash>.html` / `<ClientUIDHash>.txt`: Private 1-on-1 direct conversations with a specific client.

### 4.2 Plaintext Format (`.txt`)
Plaintext transcripts preserve raw BBCode formatting and use a standardized timestamp prefix:
```text
<HH:MM:SS> <SenderNick>: <MessageContent>
```

Sample channel transcript ([`channel.txt:1-10`](file:///C:/Users/Sakat/AppData/Roaming/TS3Client/chats/bDZTQzNxTE5seXU5dWhXTEJndE5taWFZUEVjPQ==/channel.txt#L1-L10)):
```text
<14:18:49> Alice: Hello world!
<14:26:40> Bob: Current volume: 50
<16:11:14> Admin: [B][COLOR=red]Error: [/B]You cannot execute that command!
```

### 4.3 Rich HTML Format (`.html`)
HTML transcripts begin with a UTF-8 meta declaration followed by a line break:
```html
<head><meta http-equiv="content-type" content="text/html; charset=UTF-8"></head><br>
```

Every subsequent entry is rendered as an individual `<p class="...">` block wrapped in `<span class="Body">`:
```html
<p class="TextMessage_Message Incoming Public">
  <span class="Body">
    <img src="iconpath:MESSAGE_INCOMING?size=13x13" width="13" height="13">
    <span class="TextMessage_Time">&lt;14:26:40&gt;</span>
    <a class="TextMessage_UserLink" href="client://202/jGKJGPXdhY9wr4MN5zd5ADMjI3A=~Alice">Alice</a>:
    <span class="TextMessage_Text">Hello world!</span>
  </span>
</p>
```

---

## 5. Chat Log CSS & HTML Layout Specification

TeamSpeak 3 renders chat using Qt's `QTextBrowser` rich-text component with CSS defined in `styles/<theme>_chat.qss` (falling back to `styles/default_chat.qss` ([`default_chat.qss:15-88`](file:///C:/Program%20Files/TeamSpeak%203%20Client/styles/default_chat.qss#L15-L88))).

### 5.1 Primary Message Classes (`<p class="...">`)
| CSS Class | Message Type |
|---|---|
| `TextMessage_Normal` | General informational system notice |
| `TextMessage_Welcome` | Virtual server welcome message |
| `TextMessage_HostMsg` | Modal / non-modal server host message |
| `TextMessage_Message` | User chat message (requires secondary modifiers) |
| `TextMessage_Poke` | User poke notification |
| `TextMessage_Connected` | Own client connection event |
| `TextMessage_Disconnected` | Own client disconnection event |
| `TextMessage_ClientConnected` | Remote client connects to server |
| `TextMessage_ClientDisconnected`| Remote client disconnects from server |
| `TextMessage_ClientJoined` | Remote client enters channel |
| `TextMessage_ClientLeft` | Remote client departs channel |
| `TextMessage_ClientSwitched` | Client changes active channel |
| `TextMessage_ClientDropped` | Client connection timeout / dropped |
| `TextMessage_ClientKicked` | Client kicked from server or channel |
| `TextMessage_ClientBanned` | Client banned from server |
| `TextMessage_ChannelCreated` | Channel creation event |
| `TextMessage_ChannelDeleted` | Channel deletion event |
| `TextMessage_ChannelMoved` | Channel hierarchy changed |
| `TextMessage_ChannelEdited` | Channel properties updated |
| `TextMessage_ServerEdited` | Server properties modified |
| `TextMessage_ServerError` | Server-side permission or execution error |
| `TextMessage_GroupAssigned` | Server/channel group assigned |
| `TextMessage_GroupRevoked` | Server/channel group revoked |
| `TextMessage_TalkPowerAssigned` | Talk power granted |
| `TextMessage_TalkPowerRevoked` | Talk power revoked |
| `TextMessage_TalkPowerRequested`| Client requested talk power |
| `TextMessage_BeginHistory` | Start marker of loaded chat history |
| `TextMessage_EndHistory` | End marker of loaded chat history |

### 5.2 Secondary Modifier Classes for `TextMessage_Message`
The `TextMessage_Message` tag combines with multiple secondary class tokens:
- Direction: `Incoming` vs `Outgoing`
- Contact Status: `Friend` vs `Blocked` vs `Neutral`
- Scope: `Private` vs `Public`

Example composite selector:
```css
/* Private text message received from a friend */
.TextMessage_Message.Friend.Private {
    color: #0AA537;
}
```

### 5.3 Inner Structural Classes & Elements
Inside the message paragraph:
- `<span class="Body">`: Container enclosing icon, timestamp, author link, and message text.
- `<span class="TextMessage_Time">`: Timestamp enclosed in escaped angle brackets: `&lt;HH:MM:SS&gt;`.
- `<a class="TextMessage_UserLink" href="client://...">`: Clickable author nickname.
- `<a class="TextMessage_ChannelLink" href="channelid://...">`: Clickable channel jump target.
- `<a class="TextMessage_ServerLink" href="channelid://0">`: Clickable server root target.
- `<a class="TextMessage_OfflineLink" href="offline://...">`: Offline message counter link.
- `<span class="TextMessage_Text">`: Content payload containing rendered BBCode, emoticons, and text.

---

## 6. BBCode & Custom URI Scheme Specification

The client's internal parser (`bbcode::Node` in `ts3client_win64.exe`) translates BBCode into Qt rich-text HTML.

### 6.1 Standard BBCode Tags
| BBCode Syntax | Rendered HTML Output | Notes |
|---|---|---|
| `[b]text[/b]` | `<b>text</b>` | Bold formatting |
| `[i]text[/i]` | `<i>text</i>` | Italic formatting |
| `[u]text[/u]` | `<u>text</u>` | Underline |
| `[s]text[/s]` | `<s>text</s>` | Strike-through |
| `[color=red]text[/color]` | `<font color="red">text</font>` | Named color or `#RRGGBB` hex |
| `[size=10]text[/size]` | `<span style="font-size:10pt;">text</span>` | Absolute pt size |
| `[size=+2]text[/size]` | `<span style="font-size:+2pt;">text</span>` | Relative pt offset |
| `[url]http://...[/url]` | `<a href="http://...">http://...</a>` | Direct URL link |
| `[url=http://...]Link[/url]`| `<a href="http://...">Link</a>` | Link with custom anchor text |
| `[img]http://...[/img]` | `<img src="http://...">` | External HTTP/HTTPS image |

### 6.2 Custom TS3 URI Schemes
TeamSpeak embeds specialized URI schemes inside `[url]` and `[img]` tags to link to internal client objects:

#### 1. Client Mention Scheme (`client://`)
Links directly to a connected user. Clicking triggers the context menu or opens private chat.
```text
BBCode: [URL=client://<clid>/<client_uid>~<encoded_nick>]<display_nick>[/URL]
HTML:   <a class="TextMessage_UserLink" href="client://202/jGKJGPXdhY9wr4MN5zd5ADMjI3A=~Alice">Alice</a>
```
- `<clid>`: Current runtime client ID on the virtual server (e.g. `202`).
- `<client_uid>`: Permanent 28-character Base64 client identity.
- `<encoded_nick>`: Nickname with spaces or special characters percent-encoded.

#### 2. Channel Jump Scheme (`channelid://`)
Navigates the tree view directly to a channel.
```text
BBCode: [URL=channelid://<cid>]<channel_name>[/URL]
HTML:   <a class="TextMessage_ChannelLink" href="channelid://12">&quot;Lobby&quot;</a>
```
- Note on Server Link: `channelid://0` represents the root virtual server:
  `<a class="TextMessage_ServerLink" href="channelid://0">&quot;Server Name&quot;</a>`

#### 3. Remote Channel Image Scheme (`ts3image://`)
Allows inline rendering of images stored in a server channel file directory.
```text
BBCode: [IMG]ts3image://<filename>?channel=<cid>&path=<remote_path>[/IMG]
HTML:   <img src="ts3image://banner.png?channel=1&path=/">
```
Parameters matched by client regex `ts3image://(.+)\?((?:.+=.+&?)+)`:
- `channel`: Channel ID where the file resides.
- `path`: Virtual path inside channel file storage (e.g. `/` or `/subfolder/`).
- `width` / `height`: Optional geometry bounds.

#### 4. File Transfer Scheme (`ts3file://`)
Opens the client's file transfer browser directly to a specific file or directory.
```text
BBCode: [URL=ts3file://<path>]<display_text>[/URL]
```

#### 5. Server Connect Invitation (`ts3server://`)
Encapsulates a full connection string that prompts the client to connect or bookmark.
```text
BBCode: [URL=ts3server://ts.example.com?port=9987&password=secret&channel=Lobby]Join Server[/URL]
```
Supported URL parameters:
- `port`: UDP voice port (default `9987`).
- `password`: Server connection password.
- `channel`: Default channel path to join on connect.
- `channelpassword`: Password for the target channel.
- `token`: Privilege key to redeem upon entry.
- `nickname`: Suggested display nickname.

#### 6. Internal Icon Protocol (`iconpath:`)
Internal pseudo-scheme resolved by the Qt image provider to load icons from the active icon pack:
```html
<img src="iconpath:MESSAGE_INCOMING?size=13x13" width="13" height="13">
```

### 6.3 BBCode Transformation Order & Security
To prevent injection attacks and syntax corruption, the parsing pipeline follows strict ordering:
1. **HTML Entity Escaping**: Input strings are escaped: `&` $\rightarrow$ `&amp;`, `<` $\rightarrow$ `&lt;`, `>` $\rightarrow$ `&gt;`, `"` $\rightarrow$ `&quot;`, `'` $\rightarrow$ `&apos;`.
2. **URL / IMG Isolation**: Blocks matching `\[(URL|IMG)[^\]]*\].*\[/\1\]` are extracted or protected so emoticon shortcuts (e.g. `:)`) within query strings are not replaced.
3. **BBCode Parsing**: Tags are matched case-insensitively (`[b]`, `[B]`) and mapped to corresponding HTML elements.
4. **Smilies Substitution**: Emoticon shortcut replacement runs over remaining unformatted text segments.

---

## 7. Emoticon Engine & Pack Format

Emoticon packs define textual mappings to PNG icon files.

### 7.1 Location & Packaging
- Default Pack: Packaged in `C:\Program Files\TeamSpeak 3 Client\gfx\default.zip` under `emoticons/`.
- Custom / Installed Packs: Located in `C:\Users\<User>\AppData\Roaming\TS3Client\gfx\<PackName>\emoticons/`.

### 7.2 `emoticons.txt` Syntax
The specification uses an INI-like assignment mapping image files to quoted textual triggers:
```ini
<image_file.png> = "<shortcut>"
```

Multiple shortcuts can map to a single image file ([`emoticons.txt:1-21`](file:///C:/Users/Sakat/AppData/Roaming/TS3Client/gfx/Ombre%20Violet/emoticons/emoticons.txt#L1-L21)):
```ini
smile.png = ":)"
smile.png = ":-)"
laugh.png = ":D"
laugh.png = ":-D"
cool.png = "8)"
cool.png = "8-)"
twinkle.png = ";)"
twinkle.png = ";-)"
sad.png = ":("
sad.png = ":-("
angry.png = ":C"
angry.png = ":-C"
scream.png = ":0"
scream.png = ":-0"
skeptical.png = ":/"
skeptical.png = ":-/"
stunned.png = ":x"
stunned.png = ":-x"
tongue.png = ":P"
tongue.png = ":-P"
```

### 7.3 Emoticon HTML Generation
When `ReplaceSmilies` is active in `Application` settings, matching text triggers are converted to `<img>` tags referencing the active emoticon directory:
```html
<img src="iconpath:smile.png?size=16x16" width="16" height="16">
```

---

## 8. Infoframe & Tooltip Template Engine (`.tpl`)

The information panel (lower right info frame) and tree tooltips use a custom templating system implemented in `.tpl` files ([`channelinfo.tpl:1-40`](file:///C:/Program%20Files/TeamSpeak%203%20Client/styles/default/channelinfo.tpl#L1-L40)).

### 8.1 Variable Syntax
- Unconditional Replacement: `%%VARIABLE_NAME%%`
- Conditional Replacement: `%%?VARIABLE_NAME%%` (The entire line is omitted if the variable is empty, undefined, or equals `"0"`).

### 8.2 Common Template Variables
| Template File | Available Variables |
|---|---|
| `channelinfo.tpl` | `CHANNEL_NAME`, `CHANNEL_NAME_PLAIN`, `CHANNEL_ID`, `CHANNEL_TOPIC`, `CHANNEL_MAXCLIENTS`, `CHANNEL_NEEDED_TALK_POWER`, `CHANNEL_CODEC`, `CHANNEL_CODEC_QUALITY` |
| `clientinfo.tpl` | `CLIENT_NAME`, `CLIENT_UNIQUE_ID`, `CLIENT_DATABASE_ID`, `CLIENT_COUNTRY_IMAGE`, `CLIENT_TALK_POWER`, `CLIENT_SERVER_GROUP_NAME`, `CLIENT_PING` |
| `serverinfo.tpl` | `SERVER_NAME`, `SERVER_HOST`, `SERVER_VERSION`, `SERVER_PLATFORM`, `SERVER_CLIENTS_ONLINE`, `SERVER_MAXCLIENTS`, `SERVER_UPTIME` |

---

## 9. Implementation Guide for `ts3-rust`

To interoperate cleanly with the official TeamSpeak 3 file formats and chat display:

### 9.1 Rust SQLite Configuration Trait
Implement `settings.db` access via `rusqlite`:
```rust
pub struct Ts3Settings {
    conn: rusqlite::Connection,
}

impl Ts3Settings {
    pub fn get_value(&self, table: &str, key: &str) -> rusqlite::Result<Option<String>> {
        let query = format!("SELECT value FROM {} WHERE key = ?1", table);
        let mut stmt = self.conn.prepare(&query)?;
        let mut rows = stmt.query([key])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }
}
```

### 9.2 Server Folder Hash Derivation
```rust
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};

pub fn server_folder_name(virtualserver_unique_identifier: &str) -> String {
    // Double-base64 encode to produce safe directory names
    BASE64.encode(virtualserver_unique_identifier.as_bytes())
}
```

### 9.3 BBCode to HTML Regex Lexer Pipeline
```rust
use regex::Regex;

pub fn bbcode_to_html(input: &str) -> String {
    // 1. Escape HTML entities
    let escaped = html_escape::encode_text(input).to_string();

    // 2. Parse standard tags
    let b_re = Regex::new(r"(?i)\[b\](.*?)\[/b\]").unwrap();
    let s1 = b_re.replace_all(&escaped, "<b>$1</b>");

    let i_re = Regex::new(r"(?i)\[i\](.*?)\[/i\]").unwrap();
    let s2 = i_re.replace_all(&s1, "<i>$1</i>");

    let color_re = Regex::new(r"(?i)\[color=([#a-zA-Z0-9]+)\](.*?)\[/color\]").unwrap();
    let s3 = color_re.replace_all(&s2, r#"<font color="$1">$2</font>"#);

    // 3. Parse TeamSpeak custom URIs
    let client_re = Regex::new(r"(?i)\[url=client://(\d+)/([^~]+)~([^\]]+)\](.*?)\[/url\]").unwrap();
    let s4 = client_re.replace_all(&s3, r#"<a class="TextMessage_UserLink" href="client://$1/$2~$3">$4</a>"#);

    let channel_re = Regex::new(r"(?i)\[url=channelid://(\d+)\](.*?)\[/url\]").unwrap();
    channel_re.replace_all(&s4, r#"<a class="TextMessage_ChannelLink" href="channelid://$1">$2</a>"#).to_string()
}
```
