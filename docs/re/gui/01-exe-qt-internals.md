# TeamSpeak 3 Client (`ts3client_win64.exe`) — Embedded Qt Resources, `.ui` Forms, RTTI Class Hierarchy & Menu/Action Internals

**Target Binary:** `C:\Program Files\TeamSpeak 3 Client\ts3client_win64.exe`  
**Build Origin (from embedded PDB/assert strings):** `C:\builds\hxz19fiJ\0\teamspeak\teamspeak_client_ui\src\UI_qt\`  
**Compiler / Qt Toolchain:** MSVC 2019 64-bit (`msvc2019_64`), Qt 5.15.2 (`Qt5Core`, `Qt5Gui`, `Qt5Widgets`, `Qt5Network`, `Qt5Svg`, `Qt5Sql`, `Qt5WebEngineWidgets`, `Qt5WebChannel`, `Qt5PrintSupport`)

---

## 1. Embedded Qt Resources (`:/`) & External Asset Paths

### 1.1 Compiled-In Qt Resource Paths (`qrc` / `:/`)

Mining `ts3client_win64.exe` for `:/` prefixes reveals that TeamSpeak 3 embeds only core branding, icon fonts, legal text, and `QtWebChannel` / Addon bridge scripts inside the `.exe` `qrc` tree, while delegating themes, icon packs, sound packs, and translations to external directories:

| Embedded Resource Path (`:/...`) | Category | Purpose in Client UI |
|---|---|---|
| `:/about/copyright.html` | HTML Template | Copyright and licensing text rendered in `AboutDialog` |
| `:/application/teamspeak_logo` | Image / Branding | Primary TeamSpeak logo graphic |
| `:/application/teamspeak_logo_minimal_dimmed` | Image / Branding | Minimal dimmed logo variant (status/splash/empty view) |
| `:/application/teamspeak_logo_minimal_glowing` | Image / Branding | Minimal glowing logo variant (active/hover state) |
| `:/fonts/teamspeak.ttf` | TrueType Font | Custom TeamSpeak glyph/icon font |
| `:/fonts/teamspeak2020.ttf` | TrueType Font | Updated 2020 TeamSpeak UI icon/symbol font |
| `:/qtwebchannel/qwebchannel.js` | QtWebChannel JS | IPC bridge between C++ `QWebChannel` and `QWebEnginePage` |
| `:/setup/addons_setup.js` | Addons WebEngine JS | Injected script for `com::teamspeak::ui::addons::Webengine_Page` (`Addons_Setup`) |

### 1.2 Custom URL Schemes & Internal Resource Paths

- `:///ts3file/` — Internal URL scheme used by `TextBrowser`, `InfoFrame`, and `ImageHandler` to reference remote channel/avatar images and file-transfer previews.
- `emoticons/emoticons.txt` — Relative path inside active icon/emoticon pack defining smiley mappings for `EmoticonsDisplay` (`Emoticon`).
- `translations/../styles` — Relative lookup path used by `StyleSheetHelper` (`teamspeak_client_ui\src\UI_qt\tools\stylesheethelper.cpp`) to locate `.qss` stylesheets and `.tpl` HTML templates (`MainWindow.qss`, `ChatTab.qss`, `InfoFrame.tpl`, etc.).

### 1.3 Logical Icon Keys (`IconManager` / `settings.ini` `[gfxfiles]`)

`IconManager` (`ICONMANGER` logger channel) resolves symbolic uppercase icon identifiers from `gfx/<pack>.zip` (`default_colored_2014.zip`, `default_mono_2014.zip`, `countries.zip`):

- **Connection & Server:** `CONNECT`, `CONNECT_NEW_TAB`, `DISCONNECT`, `DISCONNECT_ALL`, `SERVER_LOG`, `SERVER_QUERY`, `WEBLIST`, `HOSTER_BUTTON`, `BOOKMARK_ADD`, `BOOKMARK_ADD_FOLDER`, `BOOKMARK_DUPLICATE`, `BOOKMARK_MANAGER`, `BOOKMARK_REMOVE`, `BOOKMARK_SET_FOR_ALL`
- **Audio & Status:** `ACTIVATE_MICROPHONE`, `INPUT_MUTE`, `INPUT_MUTED`, `OUTPUT_MUTED`, `HARDWARE_INPUT_MUTED`, `HARDWARE_OUTPUT_MUTED`, `AWAY`, `PRESENT`, `PLAYER_ON`, `PLAYER_OFF`, `PLAYER_COMMANDER_ON`, `PLAYER_COMMANDER_OFF`, `PLAYER_WHISPER`, `3D_SOUND`, `3D_SOUND_ME`, `3D_SOUND_OTHER`, `CAPTURE`, `RECORDING_START`, `RECORDING_STOP`, ` WHISPER`, `WHISPERLISTS`, `WHISPERHISTORY`
- **Channel & Tree:** `CHANNEL_CHAT`, `CHANNEL_COLLAPSE_ALL`, `CHANNEL_COLLAPSE_SUB`, `CHANNEL_EXPAND_ALL`, `CHANNEL_EXPAND_SUB`, `CHANNEL_COMMANDER`, `CHANNEL_CREATE`, `CHANNEL_CREATE_SPACER`, `CHANNEL_CREATE_SUB`, `CHANNEL_DELETE`, `CHANNEL_EDIT`, `CHANNEL_GREEN`, `CHANNEL_GREEN_SUBSCRIBED`, `CHANNEL_YELLOW`, `CHANNEL_YELLOW_SUBSCRIBED`, `CHANNEL_RED`, `CHANNEL_RED_SUBSCRIBED`, `CHANNEL_PRIVATE`, `CHANNEL_SWITCH`
- **Permissions & Moderation:** `PERMISSIONS_SERVER_GROUPS`, `PERMISSIONS_CHANNEL_GROUPS`, `PERMISSIONS_CHANNELS`, `PERMISSIONS_CLIENTS`, `PERMISSIONS_CHANNEL_CLIENT`, `TOKEN`, `TOKEN_USE`, `BAN_CLIENT`, `BAN_LIST`, `COMPLAINT_LIST`, `GROUP_100`, `GROUP_200`, `GROUP_300`, `GROUP_500`, `GROUP_600`
- **Tools, Identity & Addons:** `IDENTITY_MANAGER`, `IDENTITY_ADD`, `IDENTITY_DEFAULT`, `CONTACTS`, `EDIT_FRIEND_FOE_STATUS`, `OFFLINE_MESSAGES`, `FILETRANSFER`, `FILE_HOME`, `FILE_REFRESH`, `FILE_UP`, `URLCATCHER`, `INVITE_BUDDY`, `PHONETICSNICKNAME`, `CHANGE_NICKNAME`, `DELETE_AVATAR`, `DELETE_AVATAR_OTHER`, `ICONVIEWER`, `ICONSVIEW`, `SETTINGS`, `GUISETUP`, `HOTKEYS`, `PLUGINS`, `ADDON`, `ADDON_COLLECTION`, `ADDON_BROWSER`, `ADDON_ICONPACK`, `ADDON_SOUNDPACK`, `ADDON_THEME`, `ADDON_TRANSLATION`, `MYTS_ACCOUNT`, `OVERWOLF`, `CHECK_UPDATE`, `CHANGELOG`, `ABOUT`, `QUIT`

---

## 2. Compiled Qt `.ui` Form Classes (`Ui_*` & `Ui::*`)

Every Qt Designer `.ui` form compiled via `uic` produces a base `Ui_<FormName>` class and a `Ui::<FormName>` subclass (`.?AVUi_<FormName>@@` and `.?AV<FormName>@Ui@@` in MSVC RTTI). Exactly **71 `Ui_*` form classes** are present in `ts3client_win64.exe`:

| # | `Ui_*` RTTI Class (`Ui::<Name>`) | Concrete Widget / Dialog Subclass | Source File (`teamspeak_client_ui\src\UI_qt\...`) | Functional Area |
|---|---|---|---|---|
| 1 | `Ui_AccountRecoveryDialog` | `com::teamspeak::ui::Account_Recovery_Key_Dialog` | *(myTeamSpeak Account UI)* | myTeamSpeak Account Recovery Key display/entry |
| 2 | `Ui_AccountVerifyDialog` | `com::teamspeak::ui::Account_Verify_Dialog` | *(myTeamSpeak Account UI)* | myTeamSpeak email/account verification |
| 3 | `Ui_applicationSetup` | `ApplicationSetup` | `setup\applicationsetup.cpp` | Options -> Application settings page |
| 4 | `Ui_awayDialog` | `AwayDialog` | `awaydialog.cpp` | Set Away message dialog |
| 5 | `Ui_BanAddDialog` | `BanAddDialog` (base `BanDialogBase`) | `tools\bandialog.cpp` | Add/Edit Ban rule dialog |
| 6 | `Ui_BanClientDialog` | `BanClientDialog` (base `BanDialogBase`) | `tools\bandialog.cpp` | Interactive Ban Client dialog |
| 7 | `Ui_BanList` | `BanList` | `tools\banlist.cpp` | Server Ban List manager (`Ctrl+Shift+B`) |
| 8 | `Ui_captureSoundSetupDialog` | `CaptureSoundSetupDialog` | `setup\capturesoundsetup.cpp` | Options -> Capture audio/VAD/PTT settings page |
| 9 | `Ui_ChangeMasterPasswordDialog` | `ChangeMasterPasswordDialog` | `clientencryption.cpp` / `passwordconfig.cpp` | Master password setup/change |
| 10 | `Ui_ChannelDescriptionEditTearOffWidget` | `ChannelDescriptionEditTearOffWidget` | `channeledithelpers.cpp` | Detachable BBCode channel description editor |
| 11 | `Ui_chatSetupDialog` | `ChatSetupDialog` | `setup\chatsetup.cpp` | Options -> Chat settings page |
| 12 | `Ui_ChooseNicknameDialog` | `ChooseNickname` | `customdialogs.cpp` | First-run / quick nickname prompt |
| 13 | `Ui_ClientDatabaseListViewer` | `ClientDatabaseListViewer` | `tools\clientdatabaselistviewer.cpp` | Tools -> List All Clients (server DB viewer) |
| 14 | `Ui_ClientLogView` | `ClientLogView` | `tools\clientlogview.cpp` | Tools -> Client Log (`Ctrl+L`) |
| 15 | `Ui_ComplainView` | `ComplainView` | `tools\complainview.cpp` | Tools -> Complaints List |
| 16 | `Ui_ConnectionInfoDialog` | `ConnectionInfoDialog` | `connectioninfodialog.cpp` | Client Connection Info (ping, packet loss, bandwidth) |
| 17 | `Ui_CreateChannelDialog` | `CreateChannelDialog` | `createchanneldialog.cpp` | Create / Edit Channel dialog |
| 18 | `Ui_createProfileDialog` | `CreateProfileDialog` | `setup\profilemanager.cpp` | Create Capture/Playback/Hotkey profile |
| 19 | `Ui_CustomizeToolbar` | `CustomizeToolbar` | `customizetoolbar.cpp` | Customize Main Toolbar dialog |
| 20 | `Ui_downloadSetupDialog` | `DownloadSetupDialog` | `setup\downloadsetupdialog.cpp` | Options -> Downloads / File Transfer bandwidth |
| 21 | `Ui_EasyConfirmDialog` | `EasyConfirmDialog` | `customdialogs.cpp` | "Don't show again" confirmation dialog |
| 22 | `Ui_EasyPermissionsWidget` | `EasyPermissionsWidget` | `tools\easypermissionswidget.cpp` | Simplified permissions editor view |
| 23 | `Ui_FileBrowser` | `FileBrowser` | `tools\filebrowser.cpp` | Channel File Browser window |
| 24 | `Ui_FileTransferView` | `FileTransferView` | `tools\filetransferview.cpp` | Tools -> File Transfers (`Ctrl+Shift+T`) |
| 25 | `Ui_FriendFoeDefaultsDialog` | `FriendFoeDefaultsDialog` | `tools\friendfoemanager.cpp` | Contacts default friend/foe behavior dialog |
| 26 | `Ui_FriendFoeManager` | `FriendFoeManager` | `tools\friendfoemanager.cpp` | Tools -> Contacts (`FriendFoeManager`) |
| 27 | `Ui_GatherUploadInformationClass` | `GatherUploadInformation` | `gatheruploadinformation.cpp` | Pre-upload folder/file size scanner dialog |
| 28 | `Ui_guiSetupDialog` | `GUISetupDialog` | `setup\optionsdialog.cpp` | Options -> Design (Theme, Style, Icon Pack, Transparency) |
| 29 | `Ui_HotkeyDialog` | `HotkeyDialog` | `setup\hotkeydialog.cpp` | Single Hotkey assignment & action tree picker |
| 30 | `Ui_HotkeyMsgDialog` | `HotkeyMsgDialog` | `setup\hotkeydialog.cpp` | "Press key combination..." capture overlay |
| 31 | `Ui_HotkeysSetup` | `HotkeysSetup` | `setup\hotkeyssetup.cpp` | Options -> Hotkeys settings page |
| 32 | `Ui_IconViewer` | `IconViewer` | `tools\iconviewer.cpp` | Server/local Icon Viewer & uploader |
| 33 | `Ui_IdentityQualityWidget` | `IdentityQualityWidget` | `tools\identityqualitywidget.cpp` | Security level progress bar / improver widget |
| 34 | `Ui_InvitationDialog` | `InvitationDialog` | `tools\invitationdialog.cpp` | Tools -> Invite Buddy (`ts3server://` link/token generator) |
| 35 | `Ui_LicenseViewer` | `LicenseViewer` | `tools\licenseviewer.cpp` | Help -> View License |
| 36 | `Ui_MessagesSetup` | `MessagesSetup` | `setup\messagessetup.cpp` | Options -> Messages (away/disconnect/poke presets) |
| 37 | `Ui_MsgDialog` | `MsgDialog` | `msgdialog.cpp` | Generic text/poke/kick reason prompt dialog |
| 38 | `Ui_NotificationsSetup` | `NotificationsSetup` | `setup\notificationssetup.cpp` | Options -> Notifications (Sound pack & event TTS/wav matrix) |
| 39 | `Ui_OfflineMessagesDialog` | `OfflineMessagesDialog` | `tools\offlinemessagesdialog.cpp` | Tools -> Offline Messages inbox (`Ctrl+O`) |
| 40 | `Ui_OfflineMessagesNew` | `OfflineMessagesNew` | `tools\offlinemessagesnew.cpp` | Compose / Reply Offline Message dialog |
| 41 | `Ui_passwordDialog` | `PasswordDialog` | `customdialogs.cpp` | Server / Channel password entry dialog |
| 42 | `Ui_PermChunkChannels` | `PermChunkChannels` | `tools\permchunks.cpp` | Permissions -> Channels chunk panel |
| 43 | `Ui_PermChunkClient` | `PermChunkClient` | `tools\permchunks.cpp` | Permissions -> Client / Channel Client chunk panel |
| 44 | `Ui_PermChunkGroups` | `PermChunkGroups` | `tools\permchunks.cpp` | Permissions -> Server / Channel Groups chunk panel |
| 45 | `Ui_PermGroupCopyDialog` | `PermGroupCopyDialog` | `tools\permgroupcopydialog.cpp` | Copy Server/Channel Group dialog |
| 46 | `Ui_PermissionsHelpWidget` | `PermissionsHelpWidget` | `tools\permissionshelpwidget.cpp` | Permission description & help pane |
| 47 | `Ui_PermissionsTree` | `PermissionsTree` | `tools\permissionstree.cpp` | Filterable permission tree + grant columns |
| 48 | `Ui_PermissionsWidget` | `PermissionsWidget` | `tools\permissionswidget.cpp` | Top-level Permissions tabbed container (`Ctrl+F1..F5`) |
| 49 | `Ui_PermissionValuesDialogUI` | `PermissionValuesDialog` | `tools\permissionvaluesdialog.cpp` | Multi-value / bitmask permission editor |
| 50 | `Ui_PermOverviewDialog` | `PermOverviewDialog` | `tools\permoverviewdialog.cpp` | Client Permission Overview diagnostic table |
| 51 | `Ui_PermServerGroupWidget` | `PermServerGroupWidget` | `tools\permgroupwidgets.cpp` | Server Groups assignment & client list panel |
| 52 | `Ui_PhoneticNicknameDialog` | `PhoneticNicknameDialog` | `tools\phoneticnicknamedialog.cpp` | Self -> Set Phonetic Nickname |
| 53 | `Ui_playbackSoundSetupDialog` | `PlaybackSoundSetupDialog` | `setup\optionsdialog.cpp` | Options -> Playback audio & volume settings page |
| 54 | `Ui_securitySetupDialog` | `SecuritySetupDialog` | `setup\securitysetupdialog.cpp` | Options -> Security (cache, master password, clearances) |
| 55 | `Ui_ServerConnectionInfoDialog` | `ServerConnectionInfoDialog` | `serverconnectioninfodialog.cpp` | Server Connection Info (traffic, uptime, packet statistics) |
| 56 | `Ui_ServerLogView` | `ServerLogView` | `tools\serverlogview.cpp` | Tools -> Server Log (`Ctrl+Shift+L`) |
| 57 | `Ui_ServerLogViewFilterDialog` | `ServerLogViewFilterDialog` | `tools\serverlogview.cpp` | Server Log filter configuration dialog |
| 58 | `Ui_ServerLogViewHighlightDialog` | `ServerLogViewHighlightDialog` | `tools\serverlogviewhighlightdialog.cpp` | Server Log keyword color highlighting dialog |
| 59 | `Ui_ServerQueryLoginDialog` | `ServerQueryLoginDialog` | `tools\serverquerylogindialog.cpp` | Tools -> ServerQuery Login |
| 60 | `Ui_ServerQueryPasswordDialog` | `ServerQueryPasswordDialog` | `tools\serverquerylogindialog.cpp` | Display generated ServerQuery login password |
| 61 | `Ui_sound3Dwidget` | `Sound3DWidget` | `tools\sound3Dwidget.cpp` | 3D Sound spatial positioning canvas dialog |
| 62 | `Ui_TempPasswordAddDialog` | `TempPasswordAddDialog` | `tools\temppasswordwidget.cpp` | Create temporary server password dialog |
| 63 | `Ui_TempPasswordWidget` | `TempPasswordWidget` | `tools\temppasswordwidget.cpp` | Temporary server passwords list tab |
| 64 | `Ui_TokenAdd` | `TokenAdd` | `tools\tokenview.cpp` | Create Privilege Key dialog |
| 65 | `Ui_TokenView` | `TokenView` | `tools\tokenview.cpp` | Permissions -> Privilege Keys manager (`Ctrl+Shift+K`) |
| 66 | `Ui_TransferResumeDialog` | `TransferResumeDialog` | `tools\transferresumedialog.cpp` | File collision (Overwrite / Resume / Cancel) dialog |
| 67 | `Ui_UpdateDialog` | `UpdateDialog` | `tools\updatedialog.cpp` | Client Updater progress & channel selection dialog |
| 68 | `Ui_UrlCatcherViewer` | `UrlCatcherViewer` | `tools\urlcatcherviewer.cpp` | Tools -> Collected URLs (`Ctrl+U`) |
| 69 | `Ui_UserVolumeWidget` | `UserVolumeWidget` | `tools\uservolumewidget.cpp` | Per-client volume modifier & local mute popup |
| 70 | `Ui_VirtualServerEditDialog` | `VirtualServerEditDialog` | `virtualservereditdialog.cpp` | Edit Virtual Server (name, welcome msg, host banner, transfers, flood, misc) |
| 71 | `Ui_WhisperHistoryWidget` | `WhisperHistoryWidget` | `tools\whisperhistorywidget.cpp` | Tools -> Whisper History |

*(Note: Several newer dialogs such as `ConnectDialog`, `BookmarksManager`, `IdentityDialog`, `WhisperDialog`, `OptionsDialog`, and `AboutDialog` construct their layouts programmatically or without exporting an RTTI `Ui_*` base class).*

---

## 3. RTTI Class Hierarchy: Qt Base Classes & Custom TeamSpeak Classes

### 3.1 Qt Framework Base Classes Referenced in RTTI (`112` classes)

- **Application & Core:** `QCoreApplication`, `QGuiApplication`, `QApplication`, `QObject`, `QThread`, `QTimer`, `QProcess`, `QSettings`, `QTranslator`, `QMimeData`, `QSharedData`, `QLocalServer`, `QNetworkAccessManager`, `QIODevice`, `QFileDevice`, `QFile`, `QAbstractNativeEventFilter`
- **Top-Level Windows & Containers:** `QMainWindow`, `QDialog`, `QDockWidget`, `QWidget`, `QFrame`, `QGroupBox`, `QScrollArea`, `QAbstractScrollArea`, `QSplitter`, `QStackedWidget`, `QTabWidget`, `QTabBar`, `QToolBox`, `QToolBar`, `QMenuBar`, `QMenu`, `QDialogButtonBox`, `QMessageBox`, `QInputDialog`
- **Layouts:** `QLayout`, `QLayoutItem`, `QBoxLayout`, `QHBoxLayout`, `QVBoxLayout`, `QGridLayout`, `QSpacerItem`
- **Item Views & Models:** `QAbstractItemView`, `QTreeView`, `QTreeWidget`, `QTreeWidgetItem`, `QTableView`, `QTableWidget`, `QTableWidgetItem`, `QListView`, `QListWidget`, `QListWidgetItem`, `QAbstractItemModel`, `QAbstractListModel`, `QAbstractTableModel`, `QAbstractProxyModel`, `QSortFilterProxyModel`, `QStandardItemModel`, `QStandardItem`, `QItemSelectionModel`, `QAbstractItemDelegate`, `QItemDelegate`, `QStyledItemDelegate`
- **Controls & Editors:** `QAbstractButton`, `QPushButton`, `QToolButton`, `QRadioButton`, `QCheckBox`, `QButtonGroup`, `QComboBox`, `QCompleter`, `QLineEdit`, `QTextEdit`, `QTextBrowser`, `QTextDocument`, `QTextObjectInterface`, `QLabel`, `QProgressBar`, `QAbstractSlider`, `QSlider`, `QAbstractSpinBox`, `QSpinBox`, `QDoubleSpinBox`, `QValidator`, `QRegExpValidator`
- **Actions, Shortcuts & Tray:** `QAction`, `QActionGroup`, `QWidgetAction`, `QShortcut`, `QSystemTrayIcon`, `QDrag`
- **Graphics View & Painting:** `QGraphicsView`, `QGraphicsScene`, `QGraphicsItem`, `QGraphicsObject`, `QAbstractGraphicsShapeItem`, `QGraphicsEllipseItem`, `QGraphicsTextItem`, `QPaintDevice`, `QPixmap`, `QMovie`, `QFileIconProvider`, `QStyle`, `QCommonStyle`, `QProxyStyle`
- **Embedded Chromium (`QtWebEngine`):** `QWebEngineView`, `QWebEnginePage`, `QWebEngineProfile`, `QWebChannel`

### 3.2 Custom TeamSpeak GUI & Widget Classes (Global + `com::teamspeak::ui`)

#### Application & Main Window Shell
- `TS3Application` (`uimain.cpp`) — Subclasses `QApplication`; handles single-instance IPC (`SingleInstanceChecker` via `QLocalServer`) and `WinHelpers::Ts3NativeEventFilter`.
- `MainWindow` (`mainwindow.cpp`) — Subclasses `QMainWindow`; owns `MainWindowServerTabsWidget`, `MainWindowChatWidget`, `InfoFrame`, `NewsBanner`, `TrayIcon`, `SearchFrame`, `MasterVolumeWidget`, and `quickAudioDockWidget`.
- `TrayIcon` (`trayicon.cpp`) — Subclasses `QSystemTrayIcon`; manages system tray context menu, server/status icon overlays, and minimize-to-tray (`HideOnClose`, `hidden_on_minimize_tray`).
- `Proxy_Style` / `StyleSheetHelper` (`tools\stylesheethelper.cpp`) — Custom `QProxyStyle` and dynamic QSS engine (`QtStyleSheet` property).

#### Reusable "Improved*" Custom Qt Controls
TeamSpeak wraps standard Qt widgets in a custom `Improved*` layer for middle-click tab closing, drag-and-drop, Escape handling, and custom styling:
- `ImprovedWidget` (`improvedwidget.cpp`)
- `ImprovedMenuBar`
- `ImprovedToolBar` (`improvedtoolbar.cpp`)
- `ImprovedTabWidget` / `ImprovedTabBar` / `ImprovedTabBarCloseButton` (`improvedtabs.cpp`, `improvedtabbarclosebutton.cpp`)
- `ImprovedTreeView` / `ImprovedTableView`
- `ImprovedTextEdit` / `ImprovedTextEditDrop` (`improvedtextedit.cpp`)
- `ImprovedLineEdit` / `ImprovedLineEditDrop` / `EscLineEdit` / `DragDropEscLineEdit` (`esclineedit.cpp`)
- `ImprovedInputDialog` (`improvedinputdialog.cpp`)
- `ImprovedSlider` / `DBSlider` (`improvedslider.cpp`) — Decibel-scaled audio slider
- `ImprovedShortcut` / `FocusShortcut` / `WindowCloseShortcut` (`improvedshortcut.cpp`)
- `DropToolButton` / `DropRootToolButton` / `MoreLessButton` (`morelessbutton.cpp`)
- `FlowLayout` — Responsive flow layout for badges/addons
- `ActivatedLabel`, `HeadlineLabel`, `UrlClickLabel`, `UrlClickLabelExternalWarning`, `com::teamspeak::ui::HelpLabel`, `com::teamspeak::ui::ClickableWidget`, `com::teamspeak::ui::StyleableWidget`

#### Server Connection Tabs & Channel/Client Tree (`ServerView`)
- `ServerViewManager` (`serverviewmanager.cpp`) — Manages multi-server tabs (`MainWindowServerTabsWidget`), active `ServerView` routing, and tab context menus.
- `ServerView` (`serverview.cpp`) — Per-server view hosting the channel/client tree, `Banner` (`banner.cpp`, `BannerMovie`), and `ServerConnectionHandler`.
- `TreeView` (`treeview.cpp`) — Main channel/client tree view (`QTreeView` subclass) with drag-and-drop client/channel moving.
- `TreeModel` / `AbstractTreeModel` (`treemodel.cpp`) — Hierarchical model backing `TreeView`.
- `TreeItem` / `AbstractTreeItem` (`treeitem.cpp`) — Polymorphic tree nodes:
  - `ChannelContainer`, `Channel`, `ChanClients`, `Client`
- `TreeDelegate` — Custom item delegate rendering channel status icons, client talk indicators, country flags, badges, server group icons, and phonetic/away tags.
- `TreeViewSelectionModel` — Multi-client selection model for batch kicks/moves.

#### Right-Pane Info Frame & Bottom Chat Area
- `InfoFrame` (`infoframe.cpp`) — HTML/template-driven (`TextBrowser` subclass) right pane displaying selected Server, Channel, or Client details, avatars, badges, descriptions, and volume sliders.
- `ChatTabWidget` (`chattabwidget.cpp`) — Bottom tabbed chat pane (`MainWindowChatWidget`) with Server log tab, Channel chat tab, and private Client chat tabs.
- `ChatTab` (`chattab.cpp`) — Individual chat tab hosting a `TextBrowser` (`textbrowser.cpp` / `TextDocument`), `ChatDragDropper`, and typing indicator.
- `ChatLineEdit` (`chatlineedit.cpp`) — Multi-command (`/close`, `/clear`, `/exit`, `/<plugin-command>`) and BBCode/emoticon input widget.
- `EmoticonsDisplay` / `Emoticon` (`emoticonsdisplay.cpp`) — Popup smiley selector (`EmoticonButton`).
- `SearchFrame` (`searchframe.cpp`) — Find bar (`Ctrl+F`) for `TreeView` and `ChatTabWidget` (`SearchFrameLineEdit`).

#### Status Bar Indicators (`MainWindow` bottom bar)
- `ServerStatusWidget` (`serverstatuswidget.cpp`)
- `StatusCharCountDisplay` — Chat input character counter
- `StatusErrorDisplay` / `com::teamspeak::ui::ErrorIndicator`
- `StatusOfflineMessagesDisplay` — Unread offline message badge
- `StatusTransferDisplay` — Active file transfer throughput indicator
- `StatusWhisperDisplay` — Active incoming/outgoing whisper indicator
- `PingWidget` — Live ping/packet-loss status bar widget
- `RecorderLabel` (`tools\recorder.cpp`) — Active call recording timer/indicator
- `com::teamspeak::ui::MyTSIDStatusWidget` / `com::teamspeak::ui::SyncStatusWidget` — Cloud sync & account status indicator

#### Audio, 3D Sound & Quick Audio Dock
- `MasterVolumeWidget` — Master playback volume popup (`Volume Toolbox`)
- `com::teamspeak::ui::audio::Quick_Audio` / `com::teamspeak::ui::audio::Client_Volume_Mixer` / `com::teamspeak::ui::audio::Client_Volume_Slider` (`quickAudioDockWidget`)
- `UserVolumeWidget` (`tools\uservolumewidget.cpp`)
- `VADControl` / `VADDisplay` (`setup\vad_control.cpp`) — Live voice activity detection meter with `-minvadlevel` / `-maxvadlevel` sliders
- `Sound3DWidget` / `Sound3DGraphicsView` / `Sound3DManager` / `CustomGraphicsItem` / `CustomGraphicsTextItem` (`tools\sound3Dwidget.cpp`, `tools\customgraphicsitem.cpp`)

#### Bookmarks, Identities, Contacts & Whisper System
- **Bookmarks (`com::teamspeak::ui::*`):** `Bookmarks`, `BookmarksManager`, `BookmarksMenu`, `BookmarksTreeModel`, `BookmarksTreeView` (`bookmarks.cpp`, `bookmarksmanager.cpp`, `bookmarkstreeview.cpp`)
- **Identities:** `com::teamspeak::ui::IdentityDialog`, `com::teamspeak::ui::IdentityList`, `IdentityQualityWidget`, `ImproverThread` (`tools\identitydialog.cpp`, `tools\identitymanager.cpp`, `tools\identityqualitywidget.cpp`)
- **Contacts (Friend/Foe):** `FriendFoeManager`, `FriendFoeList`, `FriendFoeListData`, `FriendFoeListItem`, `FriendFoeListSeenItem`, `FriendFoeTreeView`, `FriendFoeTreeItem`, `FriendFoeTreeItemDelegate`, `FriendFoeDefaultsDialog` (`tools\friendfoemanager.cpp`, `tools\friendfoelist.cpp`)
- **Whispers:** `com::teamspeak::ui::WhisperManager`, `com::teamspeak::ui::WhisperDialog`, `com::teamspeak::ui::WhisperSetup`, `com::teamspeak::ui::BaseWhisperListView`, `com::teamspeak::ui::WhisperListView`, `com::teamspeak::ui::WhisperListClientChannelView`, `com::teamspeak::ui::WhisperListModel`, `com::teamspeak::ui::ActiveWhispers`, `WhisperReceiveManager`, `WhisperHistoryWidget` (`tools\whisper*.cpp`)

#### Permissions System (`PermissionsManager` & `PermissionsWidget`)
- `PermissionsManager` (`tools\permissionsmanager.cpp`)
- `PermissionsWidget` (`tools\permissionswidget.cpp`) + `com::teamspeak::ui::BasicAdvancedSwitcher`
- **5 Tab Panels:** `PermServerGroupWidget`, `PermClientWidget`, `PermChannelsWidget`, `PermChannelGroupWidget`, `PermChannelClientWidget`
- **Reusable Chunks:** `PermChunkGroups`, `PermChunkGroupsDelegate`, `PermChunkChannels`, `PermChunkClient`, `PermWidget`
- **Tree & Models:** `PermissionsTree`, `PermissionsTree2`, `PermissionsTreeView`, `PermissionsTreeProxyModel`, `PermissionsTreeDelegate`, `PermGroupsListModel`, `PermGroupsListView`, `PermClientsDragDropListModel`, `PermClientsDragDropListView`, `Template*` (`TemplateServerAdmin`, `TemplateNormal`, `TemplateGuest`, `TemplateChannelAdmin`, `TemplateOperator`, `TemplateVoice`, `TemplateChannelGuest`)
- **Helper Dialogs:** `PermAddClientDialog`, `PermConfirmDeleteGroupDialog`, `PermGroupCopyDialog`, `PermissionValuesDialog`, `PermOverviewDialog`, `PermOverviewDelegate`, `PermissionsHelpWidget`, `EasyPermissionsWidget`, `ChannelGroupsOfClientDialog`, `ChannelGroupsOfClientDelegate`

#### Options Dialog (`OptionsDialog` & `AbstractOptionsPage` subclasses)
- `OptionsDialog` (`setup\optionsdialog.cpp`) + `PagesListWidget` (left sidebar category list)
- `AbstractOptionsPage` subclasses:
  1. `ApplicationSetup` — Application, server, misc, update channel (`setup\applicationsetup.cpp`)
  2. `com::teamspeak::ui::AccountSetup` / `Account_Setup_Controller` / `SyncOverviewPage` — myTeamSpeak Account & Cloud Sync
  3. `PlaybackSoundSetupDialog` — Playback profiles, device, volume, voice packs (`setup\optionsdialog.cpp`)
  4. `CaptureSoundSetupDialog` — Capture profiles, PTT, VAD (`VADControl`), DSP echo cancellation/AGC (`setup\capturesoundsetup.cpp`)
  5. `ChatSetupDialog` — Chat display, timestamps, logs, URLs (`setup\chatsetup.cpp`)
  6. `com::teamspeak::ui::WhisperSetup` — Whisper permissions, alerts, history
  7. `DownloadSetupDialog` — File transfer folder & speed limits (`setup\downloadsetupdialog.cpp`)
  8. `GUISetupDialog` — Theme, style, icon pack, animated avatars, transparency (`setup\optionsdialog.cpp`)
  9. `SecuritySetupDialog` — Master password, cache clearing (`CacheClearer`), certificates (`setup\securitysetupdialog.cpp`)
  10. `MessagesSetup` — Default away, disconnect, kick, poke messages (`setup\messagessetup.cpp`)
  11. `HotkeysSetup` / `HotkeyTableWidget` / `HotkeyTableWidgetDelegate` / `HotkeysTreeProxyModel` / `HotkeysTreeDelegate` (`setup\hotkeyssetup.cpp`)
  12. ` com::teamspeak::ui::addons::Addons_Setup` / `Addon_Frame` / `Download_Manager_Widget` / `Webengine_View` — Addons & Plugins browser (`addons.cpp`)
  13. `NotificationsSetup` / `NotificationsSetupComboBox` / `NotificationsSetupDelegate` / `StatusModel` (`setup\notificationssetup.cpp`)

---

## 4. Main Window Menus, Actions, Shortcuts & Slot Mapping

From `MAINWINDOW_ACTIONS` and `MAINWINDOW_MENU_AND_TOOLBAR` in `teamspeak_client_ui\src\UI_qt\mainwindow.cpp`, the Main Window menu bar (`ImprovedMenuBar`) and customizable toolbar (`ImprovedToolBar`) are structured into **7 top-level menus**:

### 4.1 `&Connections` Menu (`onUpdateConnectionsMenu()`)

| Action Label | Internal Action Key / Icon | Default Shortcut | Connected `MainWindow` Slot |
|---|---|---|---|
| `&Connect` | `CONNECT` | `Ctrl+S` | `onConnectRequest()` |
| `&Disconnect from Current` | `DISCONNECT` | `Ctrl+D` | `onDisconnectCurrentRequest()` |
| `&Disconnect from All` | `DISCONNECT_ALL` | `Ctrl+Shift+D` *(or menu)* | *(disconnects all `ServerView`s)* |
| `&Server Connection Info` | `INFO` | *(context/menu)* | `onServerConnectionInfoRequest()` |
| `&Quit` | `QUIT` | `Ctrl+Q` | `onQuit()` |

### 4.2 `&Bookmarks` Menu (`com::teamspeak::ui::BookmarksMenu`)

| Action Label | Internal Action Key / Icon | Default Shortcut | Connected Slot / Signal |
|---|---|---|---|
| `&Add to Bookmarks` | `BOOKMARK_ADD` | *(menu)* | `onAddBookmark()` |
| `&Manage Bookmarks` | `BOOKMARK_MANAGER` | `Ctrl+B` | `onBookmarksManagerRequest(QString)` |
| *(Dynamic Bookmark Tree)* | `FOLDER` / Server Icons | *(per-bookmark)* | `connectToBookmarkRequest(ConnectInfo, bool, ServerView*)` |

### 4.3 `S&elf` Menu (`onUpdateSelfMenu()`)

| Action Label | Internal Action Key / Icon | Default Shortcut | Connected `MainWindow` Slot |
|---|---|---|---|
| `&Capture Profile` *(submenu)* | `CAPTURE` | — | `onCaptureProfileAction(QAction*)` |
| `&Playback Profile` *(submenu)* | `PLAY` / `Playback` | — | `onPlaybackProfileAction(QAction*)` |
| `&Hotkey Profile` *(submenu)* | `HOTKEYS` | — | `onHotkeyProfileAction(QAction*)` |
| `&Sound Pack` *(submenu)* | `ADDON_SOUNDPACK` | — | `onSoundPackAction(QAction*)` |
| `&Set Away` / `&Online` (`Away Menu`) | `AWAY` / `PRESENT` | `Ctrl+Shift+Z` *(toggle)* | `onAwayMenuTriggered(QAction*)` / `onToggleAway()` / `onTakeAway()` / `onTakeAwayGlobally()` |
| `&Activate Microphone` | `ACTIVATE_MICROPHONE` | — | `onActivateCurrentInputRequest()` |
| `Mute Microphone` / `Unmute Microphone` | `INPUT_MUTE` / `INPUT_MUTED` | — | `onMuteCurrentInputRequest()` |
| `Mute Speakers/Headphones` | `OUTPUT_MUTED` | — | `onMuteCurrentOutputRequest()` |
| `Mute Speakers/Headphones on all servers` | `OUTPUT_MUTED` | — | `onMuteGlobalOutputRequest()` |
| `T&oggle Block Incoming Whispers` | `TOGGLE_BLOCK_INCOMING_WHISPERS` | — | `onToggleBlockWhisper()` |
| `&Channel Commander` | `CHANNEL_COMMANDER` | — | `onChannelCommander(bool)` |
| `&Priority Speaker` | `PLAYER_COMMANDER_ON` | — | `onSetPrioritySpeakerRequest(bool)` |
| `Set &Phonetic Nickname` | `PHONETICSNICKNAME` | — | `onSetPhoneticsNickname()` |
| `&Change Nickname` | `CHANGE_NICKNAME` | — | `onClientRenameRequest()` |
| `&Request Talk Power` | — | — | `onRequestClientTalkPower()` |
| `&Connection Info` | `INFO` | `Ctrl+I` | `onSelfClientConnectionInfoAction()` |

### 4.4 `Per&missions` Menu (`onUpdatePermissionMenu()`)

| Action Label | Internal Action Key / Icon | Default Shortcut | Connected `MainWindow` Slot |
|---|---|---|---|
| `&Server Groups` | `PERMISSIONS_SERVER_GROUPS` | `Ctrl+F1` | `onPermServerGroups()` |
| `&Client Permissions` | `PERMISSIONS_CLIENTS` | `Ctrl+F2` | `onPermClient()` |
| `&Channel Permissions` | `PERMISSIONS_CHANNELS` | `Ctrl+F3` | `onPermChannel()` |
| `&Channel Groups` | `PERMISSIONS_CHANNEL_GROUPS` | `Ctrl+F4` | `onPermChannelGroups()` |
| `&Channel Client Permissions` | `PERMISSIONS_CHANNEL_CLIENT` | `Ctrl+F5` | `onPermChannelClient()` |
| `Channel &Groups of Client` | `PERMISSIONS_CHANNEL_GROUPS` | — | `onShowChannelGroupsOfClient()` |
| `&List All Clients` | `LIST` | — | `onShowAllKnownClients()` |
| `&Privilege Keys` | `TOKEN` | — | `onCreateToken()` / `TokenView` |
| `Use &Privilege Key` | `TOKEN_USE` | — | `onUseToken()` |

### 4.5 `&Tools` Menu (`onUpdateToolsMenu()`)

| Action Label | Internal Action Key / Icon | Default Shortcut | Connected `MainWindow` Slot |
|---|---|---|---|
| `&Identities` | `IDENTITY_MANAGER` | `Ctrl+I` | `onIdentityManager()` |
| `&Contacts` | `CONTACTS` | `Ctrl+Shift+C` | `onFriendFoeManagerRequest()` |
| `Offline Messages` | `OFFLINE_MESSAGES` | `Ctrl+O` | `onOpenOfflineMessageDialog()` |
| `&Whisper Lists` | `WHISPERLISTS` | `Ctrl+Shift+W` | `WhisperDialog` |
| `&Whisper History` | `WHISPERHISTORY` | `Ctrl+Shift+H` | `onWhisperHistory()` |
| `Collected &URLs` | `URLCATCHER` | `Ctrl+U` | `onUrlCatcherRequest()` |
| `File &Transfers` | `FILETRANSFER` | `Ctrl+Shift+T` | `onFileTransferRequest()` |
| `Start &Recording` / `Stop &Recording` | `RECORDING_START` / `RECORDING_STOP` | `Ctrl+Shift+R` | `Recorder` (`tools\recorder.cpp`) |
| `&Invite Buddy` | `INVITE_BUDDY` | — | `InvitationDialog` |
| `Ban List` | `BAN_LIST` | `Ctrl+Shift+B` | `BanList` |
| `Complaints List` | `COMPLAINT_LIST` | — | `ComplainView` |
| `&ServerQuery Login` | `SERVER_QUERY` | — | `ServerQueryLoginDialog` |
| `&Server Log` | `SERVER_LOG` | `Ctrl+Shift+L` | `onServerLogRequest()` |
| `Client &Log` | `INFO` | `Ctrl+L` | `onClientLogRequest()` |
| `&Options` | `SETTINGS` | `Alt+P` (`Ctrl+Shift+O`) | `onOptionsRequest()` |

### 4.6 `P&lugins` & `&Overwolf` Menu (`onUpdatePluginsMenu()`, `onUpdateOverwolfMenu()`)

- Dynamically populated from loaded DLL plugins (`pluginsupport.cpp`, `onPluginMenuItemTriggered(QAction*)`).
- `Start &Overwolf` (`OVERWOLF`, `Overwolf::OverwolfMonitor`).

### 4.7 `&Help` Menu

| Action Label | Internal Action Key / Icon | Connected `MainWindow` Slot / Handler |
|---|---|---|
| `Visit TeamSpeak &Webpage` | — | Opens `https://www.teamspeak.com` |
| `Visit TeamSpeak Community Forum` | — | Opens `https://community.teamspeak.com` |
| `&Rent own TeamSpeak Server` | `HOSTER_BUTTON` | `onHosterButtonTriggered()` (`https://hosting.teamspeakusa.com`) |
| `&myTeamSpeak Account` | `MYTS_ACCOUNT` | Opens `https://www.myteamspeak.com/` |
| `Check for &Update` | `CHECK_UPDATE` | `onCheckUpdate()` |
| `View &Changelog` | `CHANGELOG` | `ChangeLogViewer` (`tools\changelogviewer.cpp`) |
| `View &License` | — | `LicenseViewer` (`tools\licenseviewer.cpp`) |
| `Show News &Ticker` | — | Toggles `NewsBanner` / `NewsWidget` (`newswidget.cpp`) |
| `&About TeamSpeak 3` | `ABOUT` | `AboutDialog` |
| `About &Qt` | — | `QApplication::aboutQt()` |

---

## 5. Channel & Client Tree Context Menu Actions (`ServerView` / `TreeView`)

Right-clicking items in `TreeView` (`onTreeContextMenuRequest(QPoint)`) constructs context-sensitive menus depending on the target `TreeItem` type (`Server`, `Channel`, or `Client`):

### 5.1 Virtual Server Root Node Context Menu
- `&Edit Virtual Server` -> `VirtualServerEditDialog` (`virtualservereditdialog.cpp`)
- `&Server Connection Info` -> `onServerConnectionInfoRequest()`
- `&View Icons` -> `IconViewer` (`tools\iconviewer.cpp`)
- `&Change Nickname` -> `onClientRenameRequest()`
- `Set &Phonetic Nickname` -> `onSetPhoneticsNickname()`
- `&Server Groups` / `&Channel Groups`
- `&Create Channel` (`onCreateChannelRequest()`) / `&Create Spacer` (`onCreateSpacer()`)
- `&Expand All` (`onExpandAll()`) / `&Collapse All` (`onCollapseAll()`)

### 5.2 Channel Node Context Menu
- `&Switch to Channel` (`onSwitchChannelRequest()`)
- `&Open Text Chat` (`onOpenChatRequest()`)
- `&Open File Browser` (`onFileBrowserRequest(TreeItem*)`)
- `&Subscribe to Channel` / `&Unsubscribe from Channel` (`onSubscribeChannelRequest()`, `onUnsubscribeChannelRequest()`)
- `&Subscribe to Channel Family` / `&Unsubscribe from Channel Family` (`onSubscribeChannelFamilyRequest()`, `onUnsubscribeChannelFamilyRequest()`)
- `&Create Channel` (`onCreateChannelRequest()`) / `&Create Sub-Channel` (`onCreateSubChannelRequest()`)
- `&Edit Channel` (`onEditChannelRequest(TreeItem*)`)
- `&Delete Channel` (`onDeleteChannelRequest(TreeItem*, bool)`)
- `&Channel Permissions` (`onChannelPermissions()`)
- `&Expand Subchannels` / `&Collapse Subchannels` (`onExpandSubchannels()`, `onCollapseSubchannels()`)

### 5.3 Client Node Context Menu
- `&Open Text Chat` (`onOpenTextChat()`)
- `&Poke Client` (`onClientPokeRequest()`)
- `&Change Volume` (`onChangeVolume()` -> `UserVolumeWidget`)
- `&Mute Client` / `&Unmute Client` (`onMuteClient()`, `onUnmuteClient()`)
- `&Add as Friend` / `&Add as Blocked` / `&Edit in Contacts` (`FriendFoeManager`)
- `&Server Groups` *(submenu)* (`onClientSetServerGroup(QAction*)`)
- `&Channel Group` *(submenu)* (`onClientSetChannelGroup(QAction*)`)
- `&Permissions` *(submenu: Overview, Client Permissions, Channel Client Permissions, Channel Groups of Client)*
- `&Grant Talk Power` / `&Revoke Talk Power` / `&Grant Next Talk Power` (`onGrantTalkPower()`, `onRevokeTalkPower()`, `onGrantNextTalkPower()`)
- `&Priority Speaker` (`onSetPrioritySpeakerRequest(bool)`)
- `&Allow Whisper` / `&Ignore Whisper`
- `&Move Client to own Channel` (`onMoveClientToOwnChannelRequest()`)
- `&Kick Client from Channel` (`onKickClientFromChannelRequest(TreeItem*, bool)`)
- `&Kick Client from Server` (`onKickClientFromServerRequest(TreeItem*, bool)`)
- `&Ban Client` (`onClientBanRequest()` -> `BanClientDialog`)
- `&Make Complaint` (`onMakeComplaint()`)
- `&Edit Description` (`onClientDescriptionRequest()`)
- `&Send Offline Message` (`onSendOfflineMessage()`)
- `&Client Connection Info` (`onClientConnectionInfoRequest(TreeItem*)`)
- `&Find Client in Channel Tree` (`Ctrl+F`)

---

## 6. CLI Launch Arguments & Persistent UI State Keys

### 6.1 Supported CLI Flags (`mainwindow.cpp`)
- `-connect=<address>`
- `-pw=<password>`
- `-nickname=<nickname>`
- `-channel=<channel_path>`
- `-channelid=<id>`
- `-channelpw=<password>`
- `-token=<privilege_key>` / `-privilegekey=<key>`
- `-capture=<profile>`
- `-playback=<profile>`
- `-hotkeys=<profile>`
- `-label=<tab_label>`
- `-newtab`
- `-mytsid`
- `-showqueryclients`
- `-minvadlevel=<db>` / `-maxvadlevel=<db>`

### 6.2 Persistent Window & Layout State Keys (`QSettings` / Protobuf `Config_data`)
- `MainWindowGeometry`, `MainWindowMaximized`, `MainWindowMaximizedScreen`, `WindowOpacity`, `AlwaysStayOnTop`, `HideOnClose`, `StatusBarVisible`
- `ToolBarLocation`, `BarLocation`, `voiceVolumeInMainToolbar`, `quickAudioDockWidget`
- `ChatAreaHeight`, `InfoAreaWidth`
- `BanListHeaderState`, `FileTransferTreeViewHeader`, `UrlCatcherHeaderState`, `TempPasswordHeader`
- `ShowFileTransfersOnStart`, `ReconnectAfterStandby`, `WarnTalkMuted`, `PreviousAllowedWhisperMode`, `AllowWhisperMode`, `FlashTimeout`, `FlashesCount`, `LastUpdateCheck`
