# 02 — TeamSpeak 3 Client GUI Reverse-Engineering: Main Window Layout & Widgets

**Target Executable:** `C:\Program Files\TeamSpeak 3 Client\ts3client_win64.exe` (23,464,048 bytes, Qt 5.15.2, MSVC 2019 x64)  
**Reference Source Tree:** `C:\builds\hxz19fiJ\0\teamspeak\teamspeak_client_ui\src\UI_qt\`  
**Key C++ Source Files:**
- `mainwindow.cpp` (lines 426–5809)
- `serverview.cpp` (lines 725–9746) & `serverviewmanager.cpp`
- `treeview.cpp`, `treemodel.cpp`, `treeitem.cpp`
- `infoframe.cpp` & `searchframe.cpp`
- `improvedtoolbar.cpp` & `customizetoolbar.cpp`
- `improvedtabwidget.cpp`, `improvedtabbar.cpp`, `improvedtabbarclosebutton.cpp`
- `serverstatuswidget.cpp`
- `styles/default.qss`

---

## 1. Main Window & 3-Pane Splitter Layout

The TeamSpeak 3 Client main interface (`MainWindow` inheriting `QMainWindow`) is built around a nested `QSplitter` layout hosting server views and chat workspaces.

```
+---------------------------------------------------------------------------------------+
| QMenuBar: &Connections | &Bookmarks | S&elf | Per&missions | &Tools | P&lugins | &Help|
+---------------------------------------------------------------------------------------+
| QToolBar#ImprovedToolBar: [Conn] [BM] | [Mic] [Spk] | [Away v] [SubAll] [Whisp] [Vol] |
+---------------------------------------------------------------------------------------+
| MainWindowServerTabsWidget (ImprovedTabWidget / ImprovedTabBar)                      |
| [Tab 1: Voice Server A] [Tab 2: Community B] [+]                                     |
+---------------------------------------------------------------------------------------+
| ServerView (per server tab):                                                          |
| +-----------------------------------------------+-----------------------------------+ |
| | Vertical Sub-Layout:                          | QTextEdit#InfoFrame               | |
| |  SearchFrame (collapsible search bar)         |                                   | |
| |  QTreeView#ServerTreeView                     | (Rich HTML display of server,     | |
| |   - Channels (with Spacers)                   |  channel, or selected client      | |
| |   - Clients in channels                       |  details, banner, & avatars)      | |
| |                                               |                                   | |
| | <==================== Horizontal Splitter (InfoAreaWidth) ======================> | |
| +-----------------------------------------------+-----------------------------------+ |
| <========================= Vertical Splitter (ChatAreaHeight) ======================> |
| MainWindowChatWidget / ChatTabWidget (ImprovedTabWidget):                            |
| [Server Chat] [Channel Chat: Lounge] [Private: Alice]                                 |
| +-----------------------------------------------------------------------------------+ |
| | QTextEdit#ChatTab (Rich text chat history with clickable URLs, timestamps, BBCode)| |
| +-----------------------------------------------------------------------------------+ |
| | QTextEdit#ChatLineEdit [helpTextVisible="true|false"]                             | |
+---------------------------------------------------------------------------------------+
| QStatusBar: [PingWidget] [ServerStatusWidget] [Error] [CharCount] [Sync] [Hoster]   |
+---------------------------------------------------------------------------------------+
```

### 1.1 Architecture & Object Hierarchy

| Object Name | Qt Class | Source File & Context | Purpose |
|---|---|---|---|
| `MainWindow` | `MainWindow` (`QMainWindow`) | `src/UI_qt/mainwindow.cpp:426` | Top-level application shell, manages menus, toolbars, global shortcuts, and tabs. |
| `MainWindowServerTabsWidget` | `ImprovedTabWidget` | `src/UI_qt/mainwindow.cpp:382` | Tab widget hosting individual `ServerView` instances across multiple connections. |
| `ServerViewManager` | `QObject` | `src/UI_qt/serverviewmanager.cpp` | Coordinates active/inactive server tab life cycles, routing events to `activeServerChanged`. |
| `ServerView` | `QWidget` | `src/UI_qt/serverview.cpp:725` | Composite container for one active or disconnected virtual server connection. |
| `ServerTreeView` | `QTreeView` (`ImprovedTreeView`) | `src/UI_qt/treeview.cpp`, `default.qss:88` | Tree widget displaying channels, sub-channels, spacers, and connected clients. |
| `SearchFrame` | `SearchFrame` (`QWidget`) | `src/UI_qt/searchframe.cpp:1716` | Filter bar docked above channel tree (`Ctrl+F`), with search flags and incremental find. |
| `InfoFrame` | `QTextEdit` (`InfoFrame`) | `src/UI_qt/infoframe.cpp:1708`, `default.qss:68` | Read-only HTML viewer showing server summary, channel info/topic, or client details. |
| `MainWindowChatWidget` | `QWidget` | `src/UI_qt/mainwindow.cpp:378` | Bottom docking container holding the chat tab bar and input controls. |
| `ChatTabWidget` | `ImprovedTabWidget` | `src/UI_qt/mainwindow.cpp:366` | Tab container separating Server chat, Channel chat, and 1-to-1 Private queries. |
| `ChatTab` | `QTextEdit` (`ChatTab`) | `src/UI_qt/mainwindow.cpp:264`, `default.qss:76` | Formatted chat transcript area rendering timestamps, nicknames, and chat payloads. |
| `ChatLineEdit` | `QTextEdit` / `QLineEdit` | `src/UI_qt/mainwindow.cpp:263`, `default.qss:55` | Multi-line text input field supporting history navigation (`Up`/`Down`) and BBCode. |

### 1.2 Splitters & Geometry Persistence

1. **Horizontal Splitter (Channel Tree vs. Info Frame):**
   - Splits `ServerTreeView` (left) and `InfoFrame` (right).
   - Saved setting: `InfoAreaWidth` in configuration (`mainwindow.cpp:362`).
   - Default QSS rule (`styles/default.qss:33-34`):
     ```qss
     QSplitter::handle:horizontal {
         background-image: url('styles:default/splitter_vertical.png');
     }
     ```
2. **Vertical Splitter (Upper Workspaces vs. Chat Area):**
   - Splits upper tree/info panes from the bottom `MainWindowChatWidget`.
   - Saved setting: `ChatAreaHeight` in configuration (`mainwindow.cpp:262`).
   - Saved window states: `MainWindowGeometry`, `MainWindowMaximized`, `MainWindowMaximizedScreen` (`mainwindow.cpp:379-381`).

### 1.3 Default QSS Styling Rules (`default.qss`)

```qss
/* InfoFrame styling */
QTextEdit#InfoFrame {
    background-color: palette(base);
    border: 1px solid palette(mid);
    padding: 4px;
}

/* ChatTab background watermark */
QTextEdit#ChatTab {
    background-image: url('styles:default/chatlog-128x96.png');
    background-repeat: no-repeat;
    background-position: bottom right;
    background-attachment: fixed;
}

/* ChatLineEdit placeholder state */
QTextEdit#ChatLineEdit[helpTextVisible="true"] {
    color: #808080;
    font-style: italic;
}
QTextEdit#ChatLineEdit[helpTextVisible="false"] {
    color: palette(text);
    font-style: normal;
}

/* ServerTreeView styling */
QTreeView#ServerTreeView {
    show-decoration-selected: 1;
    selection-background-color: palette(highlight);
    selection-color: palette(highlighted-text);
}
```

---

## 2. Complete Menu Bar & Action Matrix

The menu bar is configured in `MainWindow::setupMenus()` (`mainwindow.cpp:5200-5620`).

| Top-Level Menu | Action Label | Shortcut | Slot / Signal | Icon / Action Key |
|---|---|---|---|---|
| **&Connections** | &Connect | `Ctrl+S` | `1onConnectRequest()` | `CONNECT` |
| | &Disconnect from Current | `Ctrl+D` | `1onDisconnectCurrentRequest()` | `DISCONNECT` |
| | &Disconnect from All | `Ctrl+Shift+D` | `1onDisconnectAllRequest()` | `DISCONNECT_ALL` |
| | *Separator* | | | |
| | &ServerQuery Login | — | `1onServerQueryLoginRequest()` | `SERVERQUERY_LOGIN` |
| | *Separator* | | | |
| | &Quit | `Ctrl+Q` | `1onQuit()` | `QUIT` |
| **&Bookmarks** | &Bookmarks Manager | `Ctrl+B` | `1onBookmarksManagerRequest(QString)` | `BOOKMARK_MANAGER` |
| | &Add to Bookmarks | `Ctrl+Shift+B` | `1onAddBookmarkRequest()` | `BOOKMARK_ADD` |
| | *Dynamic List of Bookmarks*| — | `connectToBookmarkRequest()` | (Custom Server Icons) |
| **S&elf** | &Set Away | `Alt+A` | `1onToggleAway()` | `AWAY` |
| | &Set Away Status... | — | `1onAwayMenuTriggered(QAction*)` | `AWAY_STATUS` |
| | &Set Globally Away | — | `1onTakeAwayGlobally()` | `AWAY_GLOBAL` |
| | *Separator* | | | |
| | &Activate Microphone | `Ctrl+M` | `1onActivateCurrentInputRequest()` | `INPUT_MUTED` |
| | &Mute Microphone | `Ctrl+Shift+M` | `1onMuteCurrentInputRequest()` | `MICROPHONE_MUTED` |
| | &Mute Speakers/Headphones | `Ctrl+Shift+D` | `1onMuteCurrentOutputRequest()` | `HEADPHONES_MUTED` |
| | Mute Speakers on All Servers | `Ctrl+Shift+S` | `1onMuteGlobalOutputRequest()` | `HEADPHONES_MUTED_ALL` |
| | *Separator* | | | |
| | &Connection Info | `Ctrl+Shift+I` | `1onSelfClientConnectionInfoAction()`| `CONNECTION_INFO` |
| | &Set Phonetic Nickname | — | `1onSetPhoneticsNickname()` | `PHONETIC_NICKNAME` |
| **Per&missions** | &Server Groups | `Ctrl+F1` | `1onPermServerGroups()` | `PERMISSIONS_SERVER_GROUPS` |
| | &Channel Groups | `Ctrl+F2` | `1onPermChannelGroups()` | `PERMISSIONS_CHANNEL_GROUPS` |
| | &Channel Permissions | `Ctrl+F3` | `1onPermChannel()` | `PERMISSIONS_CHANNELS` |
| | &Client Permissions | `Ctrl+F4` | `1onPermClient()` | `PERMISSIONS_CLIENT_PERMISSIONS` |
| | &Channel Client Permissions | `Ctrl+F5` | `1onPermChannelClient()` | `PERMISSIONS_CHANNEL_CLIENT` |
| | *Separator* | | | |
| | &Privilege Keys | `Ctrl+Shift+K` | `1onUseToken()` | `TOKEN` |
| **&Tools** | &Contacts | `Ctrl+Shift+O` | `1onFriendFoeManagerRequest()` | `CONTACTS` |
| | &List All Clients | `Ctrl+Shift+L` | `1onClientListRequest()` | `CLIENT_LIST` |
| | &Ban List | `Ctrl+Shift+B` | `1onBanListRequest()` | `BAN_LIST` |
| | Complaints List | `Ctrl+Shift+C` | `1onComplainListRequest()` | `COMPLAINT_LIST` |
| | *Separator* | | | |
| | &File Browser | `Ctrl+T` | `1onFileTransferRequest()` | `FILE_BROWSER` |
| | &Server Log | `Ctrl+L` | `1onServerLogRequest()` | `SERVER_LOG` |
| | &Client Log | `Ctrl+Shift+C` | `1onClientLogRequest()` | `CLIENT_LOG` |
| | *Separator* | | | |
| | &Whisper Lists | `Ctrl+Shift+W` | `1onWhisperListsRequest()` | `WHISPERLISTS` |
| | &Whisper History | `Ctrl+Shift+H` | `1onWhisperHistory()` | `WHISPERHISTORY` |
| | &Invite Buddy | — | `1onInviteBuddy()` | `INVITE_BUDDY` |
| | &URLs | `Ctrl+U` | `1onUrlCatcherRequest()` | `URL_CATCHER` |
| | *Separator* | | | |
| | &Options | `Alt+P` | `1onOptionsRequest()` | `OPTIONS` |
| **P&lugins** | *Dynamic Plugin Menus* | — | `1onUpdatePluginsMenu()` | `PLUGINS` |
| **&Help** | &View Changelog | — | `1onViewChangelog()` | `CHANGELOG` |
| | View &Licenses | — | `1onViewLicenses()` | `LICENSE` |
| | Check for &Update | — | `1onCheckUpdate()` | `CHECK_UPDATE` |
| | *Separator* | | | |
| | &myTeamSpeak Account | — | `1onMyTeamSpeakAccountRequest()` | `MYTS_ACCOUNT` |
| | &About TeamSpeak 3 | — | `1onAbout()` | `ABOUT` |

---

## 3. Main Toolbar (`ImprovedToolBar`)

The main toolbar is implemented via `ImprovedToolBar` (`improvedtoolbar.cpp:781`), governed by configuration token `MAINWINDOW_MENU_AND_TOOLBAR` (`mainwindow.cpp:376`). Users can modify the toolbar via `CustomizeToolbar` (`customizetoolbar.cpp:455`, `Ui::CustomizeToolbar`).

### 3.1 Default Toolbar Button Order

1. **Connect Button (`actionConnect`):** Displays connect dialog or toggles connection menu.
2. **Bookmarks Button (`actionBookmarks`):** Quick drop-down menu of favorite servers.
3. **Separator / Stretcher**
4. **Activate / Mute Microphone (`actionMuteInput`):** Toggles recording capture device state (`INPUT_MUTED`).
5. **Mute Speakers / Headphones (`actionMuteOutput`):** Toggles master playback mute state (`HEADPHONES_MUTED`).
6. **Away Button (`AwayButton`):** Split-button icon toggling away status, with arrow for custom away messages (`onAwayMenuTriggered()`).
7. **Separator**
8. **Subscribe to All Channels (`actionToggleSubscribeMode`):** Cycles channel subscription state between automatic tree-wide caching and click-to-subscribe (`SUBSCRIBE_MODE`).
9. **Whisper Lists (`actionWhisperLists`):** Direct shortcut to `WhisperDialog` configuration (`Ctrl+Shift+W`).
10. **Flexible Stretcher (`ToolbarStretcher`):** Dynamically expands to push right-hand audio sliders and dock items to the far edge.
11. **Voice Volume Slider (`voiceVolumeInMainToolbar` / `quickAudioDockWidget`):** Integrated volume slider for rapid output adjustment (`onMasterVolumeVisible(bool)`).

---

## 4. Server Tabs & Chat Tabs (`ImprovedTabWidget`)

### 4.1 Server Tabs (`MainWindowServerTabsWidget`)

- Implemented by `ImprovedTabWidget` + `ImprovedTabBar` + `ImprovedTabBarCloseButton` (`improvedtabbarclosebutton.cpp`).
- **Dynamic Titles:** Formatted as `%1 on %2` or `<Server Name>` (updated via `onServerNameChanged(uint64, QString)`).
- **Tab Icon States:**
  - `CONNECTED` (Green sphere / icon)
  - `DISCONNECTED` (Red sphere / icon)
  - `CONNECTING` (Rotating spinner frames)
  - `INPUT_MUTED` / `OUTPUT_MUTED` (Mini overlay badges)
- **Context Menu (`onServerTabContextMenu`):**
  - Connect to New Tab (`Ctrl+N` / middle click)
  - Disconnect from Server
  - Close Tab
  - Close Other Tabs

### 4.2 Chat Tabs (`ChatTabWidget`)

- Houses persistent and transient chat streams:
  - **Server Chat Tab (Index 0):** Always present; logs server notices, welcome messages, connect/disconnect logs.
  - **Channel Chat Tab (Index 1):** Follows the client's current channel; context clears or re-attaches as the user navigates channels.
  - **Private Query Tabs (Dynamic):** Opened on incoming/outgoing private messages; includes nickname title, avatar icon, and unread message counters.
- **Visual Notification:** Unread tab activity changes tab text to bold red with unread counters (`(2) Alice`).

---

## 5. Channel/Client Tree Node Rendering & Spacer Rules

The channel tree is driven by `ImprovedTreeView` (`QTreeView#ServerTreeView`), backed by `TreeModel` (`treemodel.cpp:128`), `TreeItem` (`treeitem.cpp:127`), and rendered via `TreeDelegate` (`QItemDelegate` / `QStyledItemDelegate`).

### 5.1 Node Column Layout & Icon Positioning

```
+-----------------------------------------------------------------------------------------+
| [Status / Talk Icon]  Channel / Client Name          [Badges: Flags, Groups, MyTS, Avatar]
+-----------------------------------------------------------------------------------------+
```

1. **Left-to-Right Status Icons (Channel):**
   - Channel Type / Subscription Icon: Subscribed (green icon), Unsubscribed (grey icon).
   - Channel Flags: Default channel badge, Password protected (key/lock), Moderated (talk power requirement icon), Opus Music icon.
2. **Left-to-Right Status Icons (Client):**
   - Talk Status: Talking (bright blue), Idle (dim blue), Talk power requested (orange question), Microphone muted (mic with red slash), Headphones muted (speaker with red slash), Away (yellow envelope/idle).
   - Commander Badge: Channel Commander (orange lamp instead of blue).
3. **Right-Aligned Badges & Decoration Icons:**
   - Country Flag: Derived from GeoIP lookup (e.g. `de.png`, `us.png`).
   - Server Group Badges: Rendered strictly ordered by server group `sort_id` / `icon_id`.
   - Channel Group Badge: Channel-level role (e.g., Channel Admin `C`, Operator `O`, Voice `V`).
   - Overwolf & myTeamSpeak Badges: Overwolf active icon, MyTS verified badges.
   - Client Avatar: Optional thumbnail rendering when enabled in user options.

### 5.2 Channel Spacer Syntax & Parsing Rules

TeamSpeak channels matching specific prefix patterns are treated as visual tree separators. The client strips the tag prefix before rendering.

| Spacer Tag Pattern | Rendering Mode | Description & Visual Effect |
|---|---|---|
| `[spacer]` / `[spacer#]` | **Left-Aligned Spacer** | Text is left-aligned in the channel row without standard channel icons. |
| `[cspacer]` / `[cspacer#]` | **Center-Aligned Spacer** | Text is horizontally centered across the full width of the channel tree pane. |
| `[rspacer]` / `[rspacer#]` | **Right-Aligned Spacer** | Text is right-aligned to the right boundary of the channel tree pane. |
| `[*spacer]` / `[*spacer#]` | **Repeating Character Spacer** | Repeats the trailing string across the entire width of the line to form dividers. |

#### Exact Repeating Divider Patterns (`[*spacer]`)
- `[*spacer]---` or `[*spacer]___` : Renders a continuous solid horizontal dividing rule.
- `[*spacer]===` : Renders a continuous double-line dividing rule.
- `[*spacer]-.-` : Renders a decorative dash-dot divider line (`-.-.-.-.-.-`).
- `[*spacer]...` : Renders a dotted separator line across the channel tree.

*Note on Uniqueness:* The optional suffix `#` (e.g., `[cspacer0]`, `[cspacer1]`, `[spacer_rules]`) is used because TeamSpeak 3 virtual servers enforce unique channel names; the client regex `^\[(\*|c|r)?spacer[^\]]*\](.*)$` discards the tag and index, displaying only the target payload string.

---

## 6. Status Bar Layout (`QStatusBar`)

The main window status bar (`mainwindow.cpp:478`, `onShowHideStatusBar(bool)`) coordinates connection diagnostics, transfer activity, and cloud account synchronization.

```
+--------------------------------------------------------------------------------------------------------------------+
| [Ping: 24 ms | 0.00%] [StatusError] [Chars: 0/1024] [Whisper: None] [Sync: OK] [MyTS: Alice] [HosterBannerButton] |
+--------------------------------------------------------------------------------------------------------------------+
```

| Component Class | Object Name | File Reference | Display Content & Functionality |
|---|---|---|---|
| `PingWidget` | `pingWidget` | `exe_qt_dump.txt:1055` | Displays current round-trip time (`RTT`) in milliseconds and packet loss percentage. |
| `ServerStatusWidget` | `serverStatusWidget` | `src/UI_qt/serverstatuswidget.cpp:1718` | Detailed server connection summary (bandwidth, connected client count, uptime). |
| `StatusErrorDisplay` | `statusErrorDisplay` | `exe_qt_dump.txt:1359` | Shows icon and text for recent client/server error responses (tooltip shows error details). |
| `StatusCharCountDisplay`| `statusCharCount` | `exe_qt_dump.txt:1358` | Character counter for `ChatLineEdit` displaying `<current>/<max>` (e.g. `45/1024`). |
| `StatusOfflineMessages` | `statusOfflineDisplay`| `exe_qt_dump.txt:1361` | Envelope indicator showing unread offline messages (`onOpenOfflineMessageDialog()`). |
| `StatusTransferDisplay` | `statusTransferDisplay`| `exe_qt_dump.txt:1365` | Active file transfer progress bar and speed display (`showFileTransferView()`). |
| `StatusWhisperDisplay` | `statusWhisperDisplay` | `exe_qt_dump.txt:1366` | Last received whisper sender indicator (`onWhisperReceived(uint64)`). |
| `SyncStatusWidget` | `syncStatusWidget` | `com::teamspeak::ui::SyncStatusWidget` | Cloud bookmark/identity synchronization indicator (green synced, spinning sync, red conflict). |
| `MyTSIDStatusWidget` | `myTSIDStatusWidget` | `com::teamspeak::ui::MyTSIDStatusWidget` | myTeamSpeak login identity badge and state indicator (`MyTSIDState`). |
| `HosterButton` | `hosterButton` | `mainwindow.cpp:1onHosterButtonTriggered()`| Clickable host provider logo/button loading the provider's website. |
