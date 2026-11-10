# TeamSpeak 3 GUI Architecture: Info Templates & Styling Engine

This document provides a comprehensive specification of TeamSpeak 3's Info Pane template subsystem, tooltip templates, and Qt Style Sheet (QSS) architecture located in `C:\Program Files\TeamSpeak 3 Client\styles\default\`. It details template token syntax, conditional line evaluation, repetition mechanics, variable bindings, CSS class hierarchies, and a 1:1 Rust implementation reference.

---

## 1. Directory & File Inventory

The standard client styles directory at `C:\Program Files\TeamSpeak 3 Client\styles\` contains the global and chat stylesheets, while the `default\` subdirectory holds HTML/rich-text templates and raster image assets used by the right-hand **Info Pane** and tree view hover tooltips.

### 1.1 Files in `styles/default/`

| File Name | File Size | MIME / Format | Primary Purpose & Rendering Target |
|:---|:---:|:---:|:---|
| [`serverinfo.tpl`](file:///C:/Program%20Files/TeamSpeak%203%20Client/styles/default/serverinfo.tpl) | 3,375 B | HTML 4 / Qt Rich Text | Rendered in `QTextEdit#InfoFrame` when the root virtual server node is selected. |
| [`channelinfo.tpl`](file:///C:/Program%20Files/TeamSpeak%203%20Client/styles/default/channelinfo.tpl) | 3,478 B | HTML 4 / Qt Rich Text | Rendered in `QTextEdit#InfoFrame` when any channel node is selected. |
| [`clientinfo.tpl`](file:///C:/Program%20Files/TeamSpeak%203%20Client/styles/default/clientinfo.tpl) | 5,273 B | HTML 4 / Qt Rich Text | Rendered in `QTextEdit#InfoFrame` when any connected user/client node is selected. |
| [`tree_tooltip_channel.tpl`](file:///C:/Program%20Files/TeamSpeak%203%20Client/styles/default/tree_tooltip_channel.tpl) | 2,464 B | HTML 4 / Inline CSS | Rendered inside a `QToolTip` window when hovering over a channel in `QTreeView#ServerTreeView`. |
| [`tree_tooltip_client.tpl`](file:///C:/Program%20Files/TeamSpeak%203%20Client/styles/default/tree_tooltip_client.tpl) | 1,592 B | HTML 4 / Inline CSS | Rendered inside a `QToolTip` window when hovering over a client in `QTreeView#ServerTreeView`. |
| `logo-128x128.png` | 17,253 B | PNG Image (128x128) | Background watermark in `QTextEdit#InfoFrame` fixed in the top-right corner. |
| `chatlog-128x96.png` | 9,343 B | PNG Image (128x96) | Background watermark in `QTextEdit#ChatTab` fixed in the bottom-right corner. |
| `splitter.png` | 15,530 B | PNG Image | Horizontal splitter bar handle asset. |
| `splitter_vertical.png` | 201 B | PNG Image | Vertical splitter bar handle asset. |

### 1.2 Associated Stylesheets in `styles/`

| File Name | Purpose & Scope |
|:---|:---|
| [`styles/default.qss`](file:///C:/Program%20Files/TeamSpeak%203%20Client/styles/default.qss) | Global application widget styles (Qt widget selectors, palettes, borders, fonts). Lines 68–74 define `QTextEdit#InfoFrame` background properties. |
| [`styles/default_chat.qss`](file:///C:/Program%20Files/TeamSpeak%203%20Client/styles/default_chat.qss) | Stylesheet loaded into every `QTextDocument` instance for chat tabs and `QTextEdit#InfoFrame`. Lines 190–268 define all `.InfoFrame_*` classes. |
| [`styles/default_darkmode.qss`](file:///C:/Program%20Files/TeamSpeak%203%20Client/styles/default_darkmode.qss) | Global dark-theme overrides for Qt widgets. |
| [`styles/default_chat_darkmode.qss`](file:///C:/Program%20Files/TeamSpeak%203%20Client/styles/default_chat_darkmode.qss) | Dark-theme overrides for chat log and InfoFrame elements (e.g. link colors, highlights). |

---

## 2. Template Evaluation Grammar & Processing Engine

TeamSpeak 3 templates are processed using a five-stage textual pipeline prior to being passed to `QTextEdit::setHtml()`.

```
Raw Template (.tpl)
       │
       ▼
[1. Parse Comment Directives] ──► Extracts #%%KEY%%VAL (avatar size, badges max, etc.)
       │
       ▼
[2. Repeat Block Expansion]   ──► Repeats rows for Server Groups & Badges
       │
       ▼
[3. Conditional Line Filter]  ──► Drops entire lines where %%?VAR%% or %%??VAR%% is 0 or empty
       │
       ▼
[4. Variable Substitution]   ──► Replaces %%VAR%% with data or translated strings
       │
       ▼
[5. Leftover Cleanup]         ──► Strips residual %%(.*)%% tokens
       │
       ▼
Rich Text HTML to QTextEdit#InfoFrame
```

### 2.1 Header Directives (`#%%KEY%%VALUE`)

Templates declare configuration options within the initial HTML comment block (`<!-- ... -->`). When prefixed with `#`, options use hardcoded default values. When an administrator or skinner removes `#`, the engine parses the token and overrides default layout metrics before replacing variables:

- [clientinfo.tpl:17-22](file:///C:/Program%20Files/TeamSpeak%203%20Client/styles/default/clientinfo.tpl#L17-L22):
  - `%%AVATAR_MAX_WIDTH%%128`: Maximum displayed avatar pixel width (default: `128`).
  - `%%AVATAR_MAX_HEIGHT%%128`: Maximum displayed avatar pixel height (default: `128`).
  - `%%AVATAR_IMAGE_DONT_APPEND%%1`: If set to `1`, the client will **not** automatically append `<img src="avatar">` at the end of the document if an avatar exists.
  - `%%STATUS_ICONS_DONT_APPEND%%1`: If set to `1`, status icons (muted, away, recording) are not automatically appended at the bottom.
  - `%%CLIENT_SERVER_SHOW_MAX_GROUPS%%3`: Maximum number of server groups rendered in the info pane (default: `3`).
  - `%%CLIENT_SERVER_SHOW_MAX_BADGES%%3`: Maximum number of badges rendered in the showcase table (default: `3`).
- [channelinfo.tpl:17-18](file:///C:/Program%20Files/TeamSpeak%203%20Client/styles/default/channelinfo.tpl#L17-L18):
  - `%%IMAGES_MAX_WIDTH%%256`: Constrains embedded images inside `%%CHANNEL_DESCRIPTION%%` to a maximum width of 256px.
  - `%%IMAGES_MAX_HEIGHT%%256`: Constrains embedded images inside `%%CHANNEL_DESCRIPTION%%` to a maximum height of 256px.

### 2.2 Standard Substitution (`%%VARIABLE%%`)

Direct string substitution without conditions. If the variable is present in the data dictionary, all occurrences of `%%VARIABLE%%` are replaced by its value.

### 2.3 Single-Line Conditional (`%%?VARIABLE%%`)

Defined in [serverinfo.tpl:5-7](file:///C:/Program%20Files/TeamSpeak%203%20Client/styles/default/serverinfo.tpl#L5-L7):
> *"The replaceable variables are embedded in `%%` like `%%SERVER_NAME%%`. At this time you can also use `%%?SERVER_NAME%%` (note the questionmark), which is a tiny 'if'- query. Use it, to remove the whole line, if a variable is empty or just '0'."*

**Rule**:
- If `VARIABLE` is empty (`""`), undefined, or equals `"0"`, the **entire line** containing `%%?VARIABLE%%` is removed from the resulting HTML.
- If `VARIABLE` is non-empty and not `"0"`, `%%?VARIABLE%%` is replaced with the variable's value, and the line is preserved.

**Mutual Exclusion Example** ([serverinfo.tpl:87-88](file:///C:/Program%20Files/TeamSpeak%203%20Client/styles/default/serverinfo.tpl#L87-L88)):
```html
<td>%%SERVER_CLIENTS_ONLINE%% / %%SERVER_MAXCLIENTS%% %%?SERVER_NO_RESERVED_SLOTS%%</td>
<td>%%SERVER_CLIENTS_ONLINE%% / %%SERVER_MAXCLIENTS%% (<span class="Important">-%%?SERVER_RESERVED_SLOTS%% reserved</span>)</td>
```
- When no reserved slots exist: `SERVER_NO_RESERVED_SLOTS` is non-empty (e.g. `""` but evaluated as present/truthy), while `SERVER_RESERVED_SLOTS` is `"0"`. The second line is dropped completely, rendering only line 87.
- When reserved slots exist (e.g. 5): `SERVER_NO_RESERVED_SLOTS` is `"0"` (line 87 is dropped), and line 88 evaluates to `"-5 reserved"`.

### 2.4 Prefix Block Conditional (`%%??VARIABLE%%`)

Used in [clientinfo.tpl:119-126](file:///C:/Program%20Files/TeamSpeak%203%20Client/styles/default/clientinfo.tpl#L119-L126) for the Badges section:
```html
%%??CLIENT_FLAG_BADGES%%<table class="InfoFrame_Table">
%%??CLIENT_FLAG_BADGES%%  <tr>
%%??CLIENT_FLAG_BADGES%%    <td colspan="%%CLIENT_FLAG_BADGES%%" class="Label Space-Top">%%TR_CLIENT_BADGE_SHOWCASE%%:</td>
%%??CLIENT_FLAG_BADGES%%  </tr>
%%??CLIENT_FLAG_BADGES%%  <tr>
%%??CLIENT_FLAG_BADGES%%    <td class="Badge" title="<b>%%CLIENT_BADGE_NAME%%</b><br />%%CLIENT_BADGE_DESCRIPTION%%">%%CLIENT_BADGE_ICON_LARGE%%</td>
%%??CLIENT_FLAG_BADGES%%  </tr>
%%??CLIENT_FLAG_BADGES%%</table>
```
**Rule**:
- Every line prefixed with `%%??VARIABLE%%` is checked against `VARIABLE`.
- If `CLIENT_FLAG_BADGES` is `"0"` or empty, every line beginning with `%%??CLIENT_FLAG_BADGES%%` is completely deleted.
- If `CLIENT_FLAG_BADGES` is `> 0`, the `%%??CLIENT_FLAG_BADGES%%` token is stripped from the beginning of each line, preserving the table and expanding the inner repeating badge cell.

### 2.5 Repeater Expansion Mechanics (Groups and Badges)

In `ts3client_win64.exe` (internal binary references at `.text:0x1401FCCB1` and `.text:0x1401FCDB3`):
1. **Server Groups**: The engine locates the row template containing `%%CLIENT_SERVER_GROUP_ICON%%` and `%%CLIENT_SERVER_GROUP_NAME%%` ([clientinfo.tpl:134](file:///C:/Program%20Files/TeamSpeak%203%20Client/styles/default/clientinfo.tpl#L134)). For each server group assigned to the client (up to `CLIENT_SERVER_SHOW_MAX_GROUPS`), a new `<tr valign="middle">` is generated and injected.
2. **Badges Showcase**: The engine locates the `<td class="Badge">` element containing `%%CLIENT_BADGE_ICON_LARGE%%` ([clientinfo.tpl:124](file:///C:/Program%20Files/TeamSpeak%203%20Client/styles/default/clientinfo.tpl#L124)) and repeats this `<td>` across the columns of the badge row for each active badge up to `CLIENT_SERVER_SHOW_MAX_BADGES`.

### 2.6 Residual Token Cleanup

At the end of processing ([ts3client_win64.exe:.rdata:0xFF2E98](file:///C:/Program%20Files/TeamSpeak%203%20Client/ts3client_win64.exe)), the client runs a regular expression match `%%(.*)%%` to erase any unreplaced placeholder variables, preventing raw tokens from appearing in the user interface.

---

## 3. Comprehensive Variable Inventory

### 3.1 Server Template (`serverinfo.tpl`)

Defined in [serverinfo.tpl:14-38](file:///C:/Program%20Files/TeamSpeak%203%20Client/styles/default/serverinfo.tpl#L14-L38):

| Variable Placeholder | Type | Description |
|:---|:---:|:---|
| `%%SERVER_NAME%%` | String | Virtual server display name (escaped HTML). |
| `%%SERVER_WELCOMEMESSAGE%%` | Rich Text | Server welcome message displayed on connection. |
| `%%SERVER_HOSTMESSAGE%%` | Rich Text | Host message modal text. |
| `%%SERVER_ID%%` | Integer | Server virtual ID. |
| `%%SERVER_UNIQUE_ID%%` | String | Base64-encoded server unique identifier. |
| `%%SERVER_VERSION%%` | String | Full server version string (e.g. `3.13.7 [Build: 1618580000]`). |
| `%%SERVER_VERSION_SHORT%%` | String | Version number only (e.g. `3.13.7`). |
| `%%SERVER_PLATFORM%%` | String | Server operating system (`Linux`, `Windows`, `FreeBSD`, `OS X`). |
| `%%SERVER_CLIENTS_ONLINE%%` | Integer | Current active standard voice clients. |
| `%%SERVER_QUERYCLIENTS_ONLINE%%` | Integer | Current active ServerQuery clients. |
| `%%SERVER_MAXCLIENTS%%` | Integer | Total licensed slot capacity. |
| `%%SERVER_CHANNELS_ONLINE%%` | Integer | Count of channels on the virtual server. |
| `%%SERVER_UPTIME%%` | String | Human-readable uptime (e.g. `14 days 2 hours 12 minutes`). |
| `%%SERVER_CLIENT_CONNECTIONS%%` | Integer | Total voice client connections since server start. |
| `%%SERVER_QUERY_CLIENT_CONNECTIONS%%` | Integer | Total query connections since server start. |
| `%%SERVER_ADDRESS%%` | String | Connected hostname or IP address. |
| `%%SERVER_PORT%%` | Integer | Virtual server UDP voice port. |
| `%%SERVER_NICKNAMES%%` | String | Comma-separated TSDNS names or aliases. |
| `%%SERVER_ICON%%` | HTML `<img>` | Server icon image tag. |
| `%%SERVER_ICON_SCALED%%` | HTML `<img>` | Scaled 16x16 or 32x32 server icon. |
| `%%SERVER_LICENSE%%` | String | License description (`Non-Profit License`, `No License`, `AAL`, `ATHP`). |
| `%%SERVER_NO_RESERVED_SLOTS%%` | Boolean/String | Truthy when no reserved slots are active. |
| `%%SERVER_RESERVED_SLOTS%%` | Integer | Number of slots reserved for privileged clients. |
| `%%SERVER_REFRESH_INACTIVE%%` | URL/String | Anchor URL when refresh action is inactive. |
| `%%SERVER_REFRESH_ACTIVE%%` | URL/String | Anchor action URL for active refresh (`ts3server://refresh`). |
| `%%SERVER_REFRESH_ICON%%` | HTML `<img>` | Refresh button icon. |
| `%%PLUGIN_INFO_DATA%%` | HTML Rows | Arbitrary `<tr><td>...</td></tr>` pairs provided by loaded plugins. |
| `%%TR_SERVER_*%%` | String | Translated label strings (e.g. `TR_SERVER_CLIENTS_ONLINE`, `TR_SERVER_VERSION`). |

### 3.2 Channel Template (`channelinfo.tpl`)

Defined in [channelinfo.tpl:21-54](file:///C:/Program%20Files/TeamSpeak%203%20Client/styles/default/channelinfo.tpl#L21-L54):

| Variable Placeholder | Type | Description |
|:---|:---:|:---|
| `%%CHANNEL_NAME%%` | String | Display name of the channel. |
| `%%CHANNEL_NAME_PLAIN%%` | String | Unformatted raw channel name. |
| `%%CHANNEL_ID%%` | Integer | Channel ID (e.g. `123`). |
| `%%CHANNEL_ICON%%` | HTML `<img>` | Channel icon image element. |
| `%%CHANNEL_ICON_SCALED%%` | HTML `<img>` | Scaled channel icon element. |
| `%%CHANNEL_TOPIC%%` | String | Channel topic line. |
| `%%CHANNEL_MAXCLIENTS%%` | Integer | Max clients allowed (-1 for unlimited). |
| `%%CHANNEL_MAXFAMILYCLIENTS%%` | Integer | Max clients in channel tree/family. |
| `%%CHANNEL_FLAG_MAXCLIENTS%%` | String | Formatted limit string (`"Unlimited"` or number). |
| `%%CHANNEL_FLAG_MAXFAMILYCLIENTS%%` | String | Formatted family limit string. |
| `%%CHANNEL_NEEDED_TALK_POWER%%` | Integer | Talk power required to speak. |
| `%%CHANNEL_MODERATED_ICON%%` | HTML `<img>` | Icon shown when channel is moderated. |
| `%%CHANNEL_ORDER%%` | Integer | Sibling sort order index. |
| `%%CHANNEL_CODEC%%` | String | Codec name (`Opus Voice`, `Opus Music`, `Speex Narrowband`, etc.). |
| `%%CHANNEL_CODEC_QUALITY%%` | Integer | Codec quality index (0–10). |
| `%%CHANNEL_CODEC_BITRATE%%` | String | Codec data rate (e.g. `7.43 KiB/s`). |
| `%%CHANNEL_FLAGS%%` | String | Human-readable flags (`Default, Permanent`, `Password`). |
| `%%CHANNEL_FLAG_PERMANENT%%` | Boolean | True if permanent channel. |
| `%%CHANNEL_FLAG_SEMI_PERMANENT%%` | Boolean | True if semi-permanent channel. |
| `%%CHANNEL_FLAG_TEMPORARY%%` | Boolean | True if temporary channel. |
| `%%CHANNEL_FLAG_PASSWORD%%` | Boolean | True if password protected. |
| `%%CHANNEL_FLAG_DEFAULT%%` | Boolean | True if default server channel. |
| `%%CHANNEL_FLAG_NORMAL%%` | Boolean | True if standard channel. |
| `%%CHANNEL_FLAG_SPACER%%` | Boolean | True if channel name represents a spacer. |
| `%%CHANNEL_FLAG_FORCED_SILENCE%%` | Boolean | True if server enforces silence in channel. |
| `%%CHANNEL_SUBSCRIPTION%%` | String | `"Subscribed"` or `"Unsubscribed"`. |
| `%%CHANNEL_CLIENTS_COUNT%%` | Integer | Current count of clients in channel. |
| `%%CHANNEL_VOICE_DATA_ENCRYPTED%%` | String | `"Globally Enabled"`, `"Globally Disabled"`, `"Per Channel"`. |
| `%%CHANNEL_DESCRIPTION%%` | Rich Text | BBCode-parsed HTML channel description. |
| `%%TEMP_CHANNEL_DELETE_DELAY%%` | Integer | Configured delay before deletion (seconds). |
| `%%TEMP_CHANNEL_TIME_TO_DELETE%%` | String | Remaining countdown string before automatic removal. |
| `%%OPEN_FILE_BROWSER%%` | String | URL link `ts3filebrowser://<channel_id>`. |
| `%%FILE_BROWSER_ICON%%` | HTML `<img>` | File browser icon image. |

### 3.3 Client Template (`clientinfo.tpl`)

Defined in [clientinfo.tpl:25-83](file:///C:/Program%20Files/TeamSpeak%203%20Client/styles/default/clientinfo.tpl#L25-L83):

| Variable Placeholder | Type | Description |
|:---|:---:|:---|
| `%%CLIENT_NAME%%` | String | User nickname. |
| `%%CLIENT_NAME_PERCENT_ENCODED%%` | String | URL percent-encoded nickname for `client://` links. |
| `%%CLIENT_CUSTOM_NICK_NAME%%` | String | Local user-defined nickname override/alias. |
| `%%CLIENT_COUNTRY_TOOLTIP%%` | String | Full country name (e.g. `"Germany"`). |
| `%%CLIENT_COUNTRY_IMAGE%%` | Path | Country flag icon path (e.g. `gfx/countries/de.png`). |
| `%%CLIENT_COUNTRY_IMAGE_SCALED%%` | HTML `<img>` | Scaled flag icon element. |
| `%%CLIENT_ID%%` | Integer | Transient session client ID (1–65535). |
| `%%CLIENT_UNIQUE_ID%%` | String | Public key identity hash (e.g. `bH2r...=`). |
| `%%CLIENT_MYTEAMSPEAK_ID%%` | String | Cloud myTeamSpeak account identifier. |
| `%%CLIENT_DATABASE_ID%%` | Integer | Server database client record ID (cldbid). |
| `%%CLIENT_TALK_POWER%%` | Integer | Current talk power level. |
| `%%CLIENT_AWAY_MESSAGE%%` | String | Custom away message text. |
| `%%CLIENT_PLATFORM%%` | String | Operating system platform (`Windows`, `OS X`, `Linux`, `Android`, `iOS`). |
| `%%CLIENT_VERSION%%` | String | Version number (e.g. `3.5.6`). |
| `%%CLIENT_VERSION_SHORT%%` | String | Short version tag. |
| `%%CLIENT_VERSION_STATE%%` | String | Release channel tag: `[Stable]`, `[Beta]`, `[Alpha]`. |
| `%%CLIENT_VERSION_LONG%%` | String | Complete version with build timestamp and architecture. |
| `%%CLIENT_DESCRIPTION%%` | String | Custom client description text. |
| `%%CLIENT_ICON%%` | HTML `<img>` | Client custom uploaded icon. |
| `%%CLIENT_TOTALCONNECTIONS%%` | Integer | Total lifetime server connection counter. |
| `%%CLIENT_CREATED%%` | String | Formatted date/time of initial connection. |
| `%%CLIENT_LASTCONNECTED%%` | String | Formatted date/time of previous connection. |
| `%%CLIENT_CONNECTED_SINCE%%` | String | Formatted date/time or elapsed time of current session. |
| `%%CLIENT_FLAG_AWAY%%` | Boolean | Away status flag. |
| `%%CLIENT_FLAG_BADGES%%` | Integer | Count of badges to showcase (0 if none). |
| `%%CLIENT_FLAG_INPUT_MUTED%%` | Boolean | Microphone muted flag. |
| `%%CLIENT_FLAG_INPUT_HARDWARE_DISABLED%%`| Boolean | Microphone disabled in hardware/OS. |
| `%%CLIENT_FLAG_OUTPUT_MUTED%%` | Boolean | Audio/speaker muted flag. |
| `%%CLIENT_FLAG_OUTPUT_HARDWARE_DISABLED%%`| Boolean| Audio output disabled in hardware/OS. |
| `%%CLIENT_FLAG_LOCAL_MUTED%%` | Boolean | Locally muted by the current user. |
| `%%CLIENT_FLAG_INPUT_MUTED_OR_DISABLED%%` | Boolean | Input muted or disabled composite flag. |
| `%%CLIENT_FLAG_OUTPUT_MUTED_OR_DISABLED%%`| Boolean | Output muted or disabled composite flag. |
| `%%CLIENT_FLAG_ANY_MUTED_OR_DISABLED%%` | Boolean | Any audio input/output disabled composite flag. |
| `%%CLIENT_FLAG_RECORDING%%` | Boolean | Client active session recording flag. |
| `%%CLIENT_FLAG_CHANNEL_COMMANDER%%` | Boolean | Channel Commander flag. |
| `%%CLIENT_FLAG_PRIORITY_SPEAKER%%` | Boolean | Priority Speaker flag. |
| `%%CLIENT_FLAG_AVATAR%%` | Boolean | Avatar presence flag. |
| `%%CLIENT_PATH_AVATAR%%` | Path | Filesystem path to cached avatar file. |
| `%%CLIENT_IGNORE_AVATAR%%` | Boolean | True if user opted to hide this client's avatar. |
| `%%CLIENT_TALK_REQUEST_TIME%%` | String | Formatted timestamp of talk request. |
| `%%CLIENT_TALK_REQUEST_MSG%%` | String | Message sent with talk power request. |
| `%%CLIENT_VOLUME_MODIFIER%%` | String | Local volume adjustment in dB (e.g. `+3.0`). |
| `%%CLIENT_CHANNEL_GROUP_ID%%` | Integer | Current channel group ID. |
| `%%CLIENT_CHANNEL_GROUP_NAME%%` | String | Current channel group name (`Channel Admin`, etc.). |
| `%%CLIENT_CHANNEL_GROUP_ICON%%` | HTML `<img>` | Channel group icon element. |
| `%%CLIENT_SERVER_GROUP_ID%%` | Integer | Server group ID (repeated per group). |
| `%%CLIENT_SERVER_GROUP_NAME%%` | String | Server group display name (repeated per group). |
| `%%CLIENT_SERVER_GROUP_ICON%%` | HTML `<img>` | Server group icon element (repeated per group). |
| `%%CLIENT_BADGE_NAME%%` | String | Name of active badge (repeated per badge). |
| `%%CLIENT_BADGE_DESCRIPTION%%` | String | Description tooltip of active badge. |
| `%%CLIENT_BADGE_ICON_SMALL%%` | HTML `<img>` | 16x16 badge icon element. |
| `%%CLIENT_BADGE_ICON_LARGE%%` | HTML `<img>` | 64x64 badge icon element. |

### 3.4 Tooltip Templates (`tree_tooltip_*.tpl`)

- **`tree_tooltip_channel.tpl`** ([lines 62-75](file:///C:/Program%20Files/TeamSpeak%203%20Client/styles/default/tree_tooltip_channel.tpl#L62-L75)):
  - Embeds custom `<style>` tag targeting `table#info`, `.blue` (`color: blue;`), and `.red` (`color: red;`).
  - Displays: Channel Name, Topic, Codec, Codec Quality, Flags, Clients Count, Needed Talk Power, Subscription, Voice Encryption, and Temp Channel Countdown.
- **`tree_tooltip_client.tpl`** ([lines 43-63](file:///C:/Program%20Files/TeamSpeak%203%20Client/styles/default/tree_tooltip_client.tpl#L43-L63)):
  - Two-column table: Left cell `.avatar` displays `%%?CLIENT_AVATAR%%` (`<img width="64" height="64" src="...">`), right cell contains 300px table with flag icon, nickname, custom nickname in `[...]`, description, and connected since timestamp.

---

## 4. Layout Tables, Sections, and CSS Hierarchy

### 4.1 Right-Hand Info Pane Structural Layout

```
┌─────────────────────────────────────────────────────────────┐
│ InfoFrame (Container div)                                   │
│  ┌───────────────────────────────────────────────────────┐  │
│  │ InfoFrame_Title: [Icon]  <Link>Name</Link>  [Nick]    │  │
│  └───────────────────────────────────────────────────────┘  │
│  ───────────────────────── <hr /> ────────────────────────  │
│  ┌───────────────────────────────────────────────────────┐  │
│  │ InfoFrame_Table (Basic Info)                          │  │
│  │   Label: Address / Version / Platform                 │  │
│  │   Label: License / Uptime                             │  │
│  └───────────────────────────────────────────────────────┘  │
│  ┌───────────────────────────────────────────────────────┐  │
│  │ InfoFrame_Table (Connection / Status / Codec)         │  │
│  │   Label: Online Clients / Channels Count              │  │
│  │   Plugin Data Injections                              │  │
│  └───────────────────────────────────────────────────────┘  │
│  ┌───────────────────────────────────────────────────────┐  │
│  │ InfoFrame_Table (Badges Showcase - Client Only)       │  │
│  │   [Badge 1] [Badge 2] [Badge 3]                       │  │
│  └───────────────────────────────────────────────────────┘  │
│  ┌───────────────────────────────────────────────────────┐  │
│  │ InfoFrame_Table (Permissions / Groups)                │  │
│  │   Server Groups: [Icon] Name                          │  │
│  │   Channel Group: [Icon] Name                          │  │
│  └───────────────────────────────────────────────────────┘  │
│  ┌───────────────────────────────────────────────────────┐  │
│  │ Description / Action Bar Table                        │  │
│  │   Channel Description (BBCode HTML) / Refresh Action  │  │
│  └───────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

### 4.2 CSS Rules and Class Definitions

Extracted from [styles/default_chat.qss:190-268](file:///C:/Program%20Files/TeamSpeak%203%20Client/styles/default_chat.qss#L190-L268):

```css
/* Container & Title */
.InfoFrame_Title {
  font-size: x-large;
}

.InfoFrame_Title .TextMessage_UserLink,
.InfoFrame_Title .TextMessage_ChannelLink,
.InfoFrame_Title .TextMessage_ServerLink {
  color: midnightblue;
  font-weight: normal;
}

.InfoFrame_Title .InfoFrame_CustomNick {
  color: palette(link);
}

/* Tables and Cells */
.InfoFrame_Table {
  border-collapse: collapse;
  border-spacing: 0px;
}

.InfoFrame_Table td {
  padding: 0px 4px 0px 1px;
}

.InfoFrame_Table td.Label {
  font-weight: bold;
}

.InfoFrame_Table td.Bottom {
  vertical-align: bottom;
}

.InfoFrame_Table td.List {
  padding: 0px 1px 0px 8px;
}

.InfoFrame_Table td.Badge {
  padding: 5px 5px 0px 5px;
}

/* Margins and Padding Utilities */
.InfoFrame_Table td.Space-Top,
.InfoFrame_Table.Space-Top {
  padding-top: 10px;
  margin-top: 10px;
}

.InfoFrame_Table td.Space-Bot,
.InfoFrame_Table.Space-Bot {
  padding-bottom: 10px;
  margin-bottom: 10px;
}

/* Semantic Status Colors */
.Highlight,
.InfoFrame_Table td.Highlight {
  color: palette(link);
}

.Important,
.InfoFrame_Table td.Important {
  color: red;
}

.Active,
.InfoFrame_Table td.Active {
  color: green;
}

.Inactive,
.InfoFrame_Table td.Inactive {
  color: gray;
}
```

### 4.3 Chat System Classes and Internal Link Protocols

TeamSpeak 3's chat log subsystem ([default_chat.qss:32-85](file:///C:/Program%20Files/TeamSpeak%203%20Client/styles/default_chat.qss#L32-L85)) uses the same HTML 4 subset and URL schemes:

- **Custom Link Schemes**:
  - `channelid://<id>`: Selects channel node in server tree view ([serverinfo.tpl:50](file:///C:/Program%20Files/TeamSpeak%203%20Client/styles/default/serverinfo.tpl#L50), `channelid://0` represents the root server).
  - `client://<client-id>/<unique-id>~<url-encoded-nick>`: Opens client context menu or poke dialog ([clientinfo.tpl:94](file:///C:/Program%20Files/TeamSpeak%203%20Client/styles/default/clientinfo.tpl#L94)).
  - `ts3filebrowser://<channel-id>`: Opens file transfer browser modal.
  - `ts3server://refresh`: Invokes server statistics update request.

---

## 5. Concrete 1:1 Rust Implementation Guide

To implement the template engine and info pane rendering in Rust, the engine must handle:
1. Directive parsing from HTML comments.
2. Repeating elements for groups and badges.
3. Line filtering based on `%%?KEY%%` and `%%??KEY%%`.
4. BBCode subset transformation into HTML.
5. Placeholder substitution and residue cleanup.

### 5.1 Data Binding Models

```rust
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct ServerGroupItem {
    pub id: u64,
    pub name: String,
    pub icon_html: String,
}

#[derive(Debug, Clone, Default)]
pub struct BadgeItem {
    pub name: String,
    pub description: String,
    pub icon_small_html: String,
    pub icon_large_html: String,
}

#[derive(Debug, Clone, Default)]
pub struct TemplateContext {
    pub options: HashMap<String, String>,
    pub variables: HashMap<String, String>,
    pub server_groups: Vec<ServerGroupItem>,
    pub badges: Vec<BadgeItem>,
}
```

### 5.2 Template Engine Pipeline Implementation

```rust
use regex::Regex;

pub struct Ts3TemplateEngine {
    token_re: Regex,
    cleanup_re: Regex,
}

impl Ts3TemplateEngine {
    pub fn new() -> Self {
        Self {
            token_re: Regex::new(r"%%(\??\??[A-Z0-9_]+)%%").unwrap(),
            cleanup_re: Regex::new(r"%%(.*?)%%").unwrap(),
        }
    }

    /// Evaluates a template string against a data context
    pub fn render(&self, template: &str, ctx: &TemplateContext) -> String {
        // Stage 1: Parse directive overrides inside HTML comments
        let mut max_groups = ctx.options.get("CLIENT_SERVER_SHOW_MAX_GROUPS")
            .and_then(|v| v.parse::<usize>().ok()).unwrap_or(3);
        let mut max_badges = ctx.options.get("CLIENT_SERVER_SHOW_MAX_BADGES")
            .and_then(|v| v.parse::<usize>().ok()).unwrap_or(3);

        // Stage 2: Line-by-line processing
        let mut output_lines = Vec::new();

        for line in template.lines() {
            // Stage 2a: Prefix conditional (%%??VAR%%)
            if line.contains("%%??") {
                let mut should_drop = false;
                let mut stripped_line = line.to_string();

                for cap in self.token_re.captures_iter(line) {
                    let token = &cap[1];
                    if let Some(var_name) = token.strip_prefix("??") {
                        let is_falsy = match ctx.variables.get(var_name) {
                            None => true,
                            Some(val) => val.trim().is_empty() || val.trim() == "0",
                        };
                        if is_falsy {
                            should_drop = true;
                            break;
                        } else {
                            stripped_line = stripped_line.replace(&cap[0], "");
                        }
                    }
                }

                if should_drop {
                    continue;
                }

                // If badge row within preserved prefix block, expand badges
                if stripped_line.contains("%%CLIENT_BADGE_ICON_LARGE%%") {
                    let mut expanded_cells = Vec::new();
                    let badges_to_show = ctx.badges.iter().take(max_badges);
                    for badge in badges_to_show {
                        let mut cell = stripped_line.clone();
                        cell = cell.replace("%%CLIENT_BADGE_NAME%%", &badge.name);
                        cell = cell.replace("%%CLIENT_BADGE_DESCRIPTION%%", &badge.description);
                        cell = cell.replace("%%CLIENT_BADGE_ICON_LARGE%%", &badge.icon_large_html);
                        cell = cell.replace("%%CLIENT_BADGE_ICON_SMALL%%", &badge.icon_small_html);
                        expanded_cells.push(cell);
                    }
                    if !expanded_cells.is_empty() {
                        output_lines.push(expanded_cells.join(""));
                    }
                    continue;
                }

                output_lines.push(stripped_line);
                continue;
            }

            // Stage 2b: Inline conditional (%%?VAR%%)
            if line.contains("%%?") {
                let mut should_drop = false;
                for cap in self.token_re.captures_iter(line) {
                    let token = &cap[1];
                    if let Some(var_name) = token.strip_prefix('?') {
                        let is_falsy = match ctx.variables.get(var_name) {
                            None => true,
                            Some(val) => val.trim().is_empty() || val.trim() == "0",
                        };
                        if is_falsy {
                            should_drop = true;
                            break;
                        }
                    }
                }
                if should_drop {
                    continue;
                }
            }

            // Stage 2c: Repeater row expansion for Server Groups
            if line.contains("%%CLIENT_SERVER_GROUP_NAME%%") {
                let groups_to_show = ctx.server_groups.iter().take(max_groups);
                for group in groups_to_show {
                    let mut group_line = line.to_string();
                    group_line = group_line.replace("%%CLIENT_SERVER_GROUP_ID%%", &group.id.to_string());
                    group_line = group_line.replace("%%CLIENT_SERVER_GROUP_NAME%%", &group.name);
                    group_line = group_line.replace("%%CLIENT_SERVER_GROUP_ICON%%", &group.icon_html);
                    output_lines.push(group_line);
                }
                continue;
            }

            output_lines.push(line.to_string());
        }

        let intermediate = output_lines.join("\n");

        // Stage 3: Standard Variable Substitutions
        let mut substituted = intermediate;
        for (key, val) in &ctx.variables {
            let exact_token = format!("%%{}%%", key);
            let cond_token = format!("%%?{}%%", key);
            substituted = substituted.replace(&exact_token, val);
            substituted = substituted.replace(&cond_token, val);
        }

        // Stage 4: Residual Cleanup (TS3 %%(.*)%% rule)
        let cleaned = self.cleanup_re.replace_all(&substituted, "");
        cleaned.into_owned()
    }
}
```

### 5.3 BBCode to HTML Conversion Subset

Channel and client descriptions are stored as TeamSpeak BBCode. The Rust client must transform BBCode tags into HTML 4 before rendering:

| BBCode Syntax | Generated HTML Equivalent | Notes |
|:---|:---|:---|
| `[b]...[/b]` | `<b>...</b>` | Bold text. |
| `[i]...[/i]` | `<i>...</i>` | Italic text. |
| `[u]...[/u]` | `<u>...</u>` | Underlined text. |
| `[color=#RRGGBB]...[/color]` | `<span style="color:#RRGGBB">...</span>` | Hex color styling. |
| `[size=N]...[/size]` | `<span style="font-size:Npx">...</span>` | Text size scaling. |
| `[url=TARGET]LABEL[/url]` | `<a href="TARGET">LABEL</a>` | Hyperlink (auto-sanitizes JavaScript). |
| `[img]SRC[/img]` | `<img src="SRC" max-width="256" />` | Constrained by `%%IMAGES_MAX_WIDTH%%`. |

### 5.4 Native Rust UI Mapping (egui / iced)

If not using an embedded web browser (`wry`/WebKit) or `QTextBrowser`, the parsed HTML structure maps directly into native declarative GUI trees:

```rust
// Example: Rendering the Info Frame in egui
pub fn ui_info_frame(ui: &mut egui::Ui, ctx: &TemplateContext) {
    ui.vertical(|ui| {
        // Title Row
        ui.horizontal(|ui| {
            if let Some(icon) = ctx.variables.get("CHANNEL_ICON") {
                // Render icon texture
            }
            let title = ctx.variables.get("CHANNEL_NAME").map(|s| s.as_str()).unwrap_or("");
            ui.heading(title);
        });
        ui.separator();

        // Info Table Grid
        egui::Grid::new("info_grid").striped(true).show(ui, |ui| {
            if let Some(topic) = ctx.variables.get("CHANNEL_TOPIC") {
                ui.label(egui::RichText::new("Topic:").strong());
                ui.label(topic);
                ui.end_row();
            }
            if let Some(codec) = ctx.variables.get("CHANNEL_CODEC") {
                ui.label(egui::RichText::new("Codec:").strong());
                ui.label(codec);
                ui.end_row();
            }
            if let Some(clients) = ctx.variables.get("CHANNEL_CLIENTS_COUNT") {
                let max = ctx.variables.get("CHANNEL_FLAG_MAXCLIENTS").unwrap();
                ui.label(egui::RichText::new("Current Clients:").strong());
                ui.label(format!("{} / {}", clients, max));
                ui.end_row();
            }
        });
    });
}
```

---

## 6. Verification and Binary Cross-References

| Component | Disk Location / Binary Offset | Observation / Verification Detail |
|:---|:---|:---|
| InfoFrame Background | [`styles/default.qss:68-74`](file:///C:/Program%20Files/TeamSpeak%203%20Client/styles/default.qss#L68-L74) | Sets `background-image: url('styles:default/logo-128x128.png')` with `top right fixed`. |
| ChatTab Background | [`styles/default.qss:76-82`](file:///C:/Program%20Files/TeamSpeak%203%20Client/styles/default.qss#L76-L82) | Sets `background-image: url('styles:default/chatlog-128x96.png')` with `bottom right fixed`. |
| Server Template Hook | `ts3client_win64.exe:.text:0x140200598` | Loads `styles/default/serverinfo.tpl` and registers translation strings. |
| Channel Template Hook | `ts3client_win64.exe:.text:0x1401F4BAA` | Loads `styles/default/channelinfo.tpl` and validates `CHANNEL_CODEC` enum strings. |
| Client Template Hook | `ts3client_win64.exe:.text:0x1401F889A` | Loads `styles/default/clientinfo.tpl`, evaluates `CLIENT_FLAG_BADGES` showcase block. |
| Channel Tooltip Hook | `ts3client_win64.exe:.text:0x140569FB8` | Loads `styles/default/tree_tooltip_channel.tpl` on hover event. |
| Client Tooltip Hook | `ts3client_win64.exe:.text:0x14056BE6C` | Loads `styles/default/tree_tooltip_client.tpl` on hover event. |
| Server Group Repeater | `ts3client_win64.exe:.text:0x1401FCCB1` | Expands server group items up to `CLIENT_SERVER_SHOW_MAX_GROUPS`. |
| Badge Showcase Repeater| `ts3client_win64.exe:.text:0x1401FCDB3` | Expands badge cells up to `CLIENT_SERVER_SHOW_MAX_BADGES`. |
| Residual Token Eraser | `ts3client_win64.exe:.rdata:0xFF2E98` | Compiles `%%(.*)%%` regular expression to strip residual placeholders. |
