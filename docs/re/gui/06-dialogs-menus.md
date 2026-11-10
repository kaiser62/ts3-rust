# 06 — TeamSpeak 3 Client GUI Reverse-Engineering: Dialogs & Context Menus

**Target Executable:** `C:\Program Files\TeamSpeak 3 Client\ts3client_win64.exe` (23,464,048 bytes, Qt 5.15.2, MSVC 2019 x64)  
**Translation Resources:** `C:\Program Files\TeamSpeak 3 Client\translations\lagos_de.qm`, `lagos.qm`, `permissions.qm`  
**Reference Build Tree in Binary:** `C:\builds\hxz19fiJ\0\teamspeak\teamspeak_client_ui\`  
**Verification Method:** Exact widget object names, Qt class names, context strings, and UI strings verified via static binary analysis of embedded `ui_*.h` generated code, `.rdata` string tables, Qt signal/slot metadata (`QMetaObject`), and `.qm` translation catalogs.

---

## 1. Overview & Architecture of TS3 Client GUI

The TeamSpeak 3 desktop client interface is constructed using **Qt 5 (Widgets)**. Dialog forms are built using Qt Designer (`.ui` files), compiled via `uic` into `ui_<name>.h` headers located under `out\win_x64\Release_x64\obj\ts_client_ui.gen\qtgen\`, and integrated into C++ controller classes under `src\UI_qt\`.

| Component / Dialog | Qt Controller Class | Generated UI Class / Header | Qt Context Name |
|---|---|---|---|
| **Connect Dialog** | `ConnectDialog` | `Ui::connectDialog` (`ui_connectdialog.h`) | `connectDialog` / `ConnectDialog` |
| **Bookmarks Manager** | `BookmarksManager` | Handcrafted / Composite tree view | `BookmarksManager` / `com::teamspeak::ui::BookmarksManager` |
| **Identities Dialog** | `IdentityDialog` | `Ui::IdentityDialog` (`ui_identitydialog.h`) | `IdentityDialog` / `com::teamspeak::ui::IdentityDialog` |
| **Security Level** | `IdentityQualityWidget` | `Ui::IdentityQualityWidget` (`ui_identityqualitywidget.h`) | `IdentityQualityWidget` |
| **Main Window / Menus**| `MainWindow` | Custom QMenuBar & QToolBar | `MainWindow` |
| **Options / Settings** | `OptionsDialog` | `QListWidget` + `QStackedWidget` | `OptionsDialog` |
| **Create / Edit Channel**| `CreateChannelDialog` | `Ui::CreateChannelDialog` (`ui_createchanneldialog.h`)| `CreateChannelDialog` |
| **Virtual Server Settings**| `VirtualServerEditDialog`| `Ui::VirtualServerEditDialog` | `VirtualServerEditDialog` |
| **Ban Add / Edit** | `BanAddDialog` | `Ui::BanAddDialog` (`ui_banadddialog.h`) | `BanAddDialog` |
| **Ban Client (Quick)**| `BanClientDialog` | `Ui::BanClientDialog` (`ui_banclientdialog.h`) | `BanClientDialog` |
| **Ban List** | `BanList` | `Ui::BanList` (`ui_banlist.h`) | `BanList` |
| **Poke / Notice Dialog**| `MsgDialog` / `PokeDialog`| `Ui::MsgDialog` (`ui_msgdialog.h`) | `PokeDialog` / `MsgDialog` |
| **Complaints List** | `ComplainView` | `Ui::ComplainView` (`ui_complainview.h`) | `ComplainView` |
| **Privilege Keys (Tokens)**| `TokenView` / `TokenAdd` | Custom dialogs (`tokenview.cpp`, `tokenadd.cpp`) | `TokenView` / `TokenAdd` |
| **Whisper Lists** | `WhisperDialog` | Custom layout (`whisperdialog.cpp`) | `WhisperDialog` / `com::teamspeak::ui::WhisperDialog` |
| **Tree Context Menus** | `ServerView` | `QMenu` popups (`serverview.cpp`) | `ServerView` |

---

## 2. Connect Dialog (`ConnectDialog`)

- **Controller Implementation:** `src\UI_qt\connectdialog.cpp`
- **UI Header:** `ui_connectdialog.h:332-333`
- **Context Names:** `connectDialog` (UI form) / `ConnectDialog` (C++ class)

### 2.1 Widget Object Hierarchy & Properties

```
QDialog: connectDialog ("Connect")
├── QLabel: label_17 ("Server Nickname or &Address:")
├── QLineEdit: ipLineEdit [Buddy: label_17]
├── QLabel: label_20 ("Server &Password:")
├── QLineEdit: serverPasswordLineEdit [EchoMode: Password, Buddy: label_20]
├── QLabel: label_18 ("&Nickname:")
├── QLineEdit: nickLineEdit [Buddy: label_18]
├── QLabel: nickToShortWarning ("[Too Short]", Visible: false)
├── QPushButton: moreButton ("&More" / "&Less")
├── QWidget: advancedWidget (Toggled via moreButton)
│   ├── QFrame: line (Horizontal separator)
│   ├── QLabel: identitiesLabel / label_23 ("&Identity:")
│   ├── QComboBox: identitiesComboBox
│   ├── QLabel: label_19 ("&Default Channel:")
│   ├── QLineEdit: defaultChannelLineEdit
│   ├── QLabel: label_21 ("Channel &Password:")
│   ├── QLineEdit: defaultChannelPasswordLineEdit [EchoMode: Password]
│   ├── QLabel: label_22 ("One-Time &Privilege Key:")
│   ├── QLineEdit: tokenLineEdit (Tooltip: "One-time privilege key used after connecting. Once used, the privilege key will become invalid.")
│   ├── QLabel: label_playback ("&Playback Profile:")
│   ├── QComboBox: playbackProfileComboBox
│   ├── QLabel: label_capture ("&Capture Profile:")
│   ├── QComboBox: captureProfileComboBox
│   ├── QLabel: label_hotkey ("Hotkey Profile:")
│   └── QComboBox: hotkeyProfileComboBox
└── QDialogButtonBox / QHBoxLayout:
    ├── QPushButton: connectButton ("&Connect", Default: true)
    ├── QPushButton: connectNewTabButton ("In New &Tab")
    └── QPushButton: cancelButton ("Cancel")
```

### 2.2 Signal / Slot Connections (`[ui_connectdialog.h:332-333]`)

- `moreButton::clicked(bool)` $\rightarrow$ `advancedWidget::setVisible(bool)`
- `connectButton::clicked()` $\rightarrow$ `ConnectDialog::accept()`
- `connectNewTabButton::clicked()` $\rightarrow$ `ConnectDialog::onConnectInNewTab()`
- `cancelButton::clicked()` $\rightarrow$ `ConnectDialog::reject()`

---

## 3. Bookmarks Manager (`BookmarksManager`)

- **Controller Implementation:** `src\UI_qt\bookmarksmanager.cpp`, `src\UI_qt\bookmarkstreeview.cpp`
- **Context Names:** `BookmarksManager`, `com::teamspeak::ui::BookmarksManager`
- **Shortcut:** `Ctrl+B`

### 3.1 Layout & Widget Object Tree

```
QDialog: BookmarksManager ("Bookmarks")
├── QWidget: bookmarksBanner (myTeamSpeak synchronization banner)
├── QSplitter: bookmarksSplitter
│   ├── QSplitter: bookmarksLocalRemoteSplitter (Left side: Tree views)
│   │   ├── QWidget: layoutWidget_1
│   │   │   ├── QLabel: remoteBookmarksLabel ("Synchronized Bookmarks" / "myTeamSpeak Bookmarks")
│   │   │   └── QTreeView: remoteTreeView (Model: remote bookmarks tree)
│   │   └── QWidget: layoutWidget_2
│   │       ├── QLabel: localBookmarksLabel ("Local Bookmarks")
│   │       └── QTreeView: localTreeView (Model: local bookmarks tree)
│   └── QWidget: layoutWidget_3 (Right side: Edit panel)
│       ├── QWidget: labelWidget
│       │   ├── QLabel: labelEditLabel ("Bookmark Name:")
│       │   └── QLineEdit: labelEdit (Custom server label)
│       ├── QWidget: addressWidget
│       │   ├── QLabel: label_address ("Server Nickname or &Address:")
│       │   └── QLineEdit: addressEdit (Hostname / IP:Port)
│       ├── QWidget: serverPasswordWidget
│       │   ├── QLabel: label_5 ("Server &Password:")
│       │   └── QLineEdit: serverPasswordEdit [EchoMode: Password]
│       ├── QWidget: nicknameWidget
│       │   ├── QLabel: label_nick ("&Nickname:")
│       │   └── QLineEdit: nicknameEdit
│       ├── QWidget: phoneticNicknameWidget
│       │   ├── QLabel: label_13 ("&Phonetic Nickname:")
│       │   └── QLineEdit: phoneticNicknameLineEdit
│       ├── QWidget: identityWidget
│       │   ├── QLabel: label_9 ("&Identity:")
│       │   └── QComboBox: identityComboBox
│       ├── QWidget: defaultChannelWidget
│       │   ├── QLabel: label_4 ("&Default Channel:")
│       │   ├── QLineEdit: defaultChannelEdit
│       │   └── QToolButton: makeCurrentChannelDefaultToolButton ("Make current channel the default channel")
│       ├── QWidget: defaultChannelPasswordWidget
│       │   ├── QLabel: label_8 ("Default Channel &Password:")
│       │   └── QLineEdit: defaultChannelPasswordEdit [EchoMode: Password]
│       ├── QWidget: playbackProfileWidget
│       │   ├── QLabel: label_7 ("&Playback Profile:")
│       │   └── QComboBox: playbackComboBox
│       ├── QWidget: captureProfileWidget
│       │   ├── QLabel: label_6 ("&Capture Profile:")
│       │   └── QComboBox: captureComboBox
│       ├── QWidget: hotkeyProfileWidget
│       │   ├── QLabel: label_11 ("Hotkey Profile:")
│       │   └── QComboBox: hotkeyComboBox
│       ├── QWidget: soundPackWidget
│       │   ├── QLabel: label_3 ("Sound Pack:")
│       │   └── QComboBox: soundPackComboBox
│       └── QWidget: extraOptionsWidget (Expanded via basicAdvancedLabel "More")
│           ├── QCheckBox: autoconnectCheckBox ("Connect on Startup")
│           ├── QCheckBox: showServerQueryClientsCheckBox ("Show ServerQuery Clients")
│           └── QCheckBox: sendMyTSIDCheckBox ("Enable myTeamSpeak features")
├── QHBoxLayout: horizontalLayout_2 (Bottom action bar)
│   ├── QPushButton: newBookmarkButton ("Add Bookmark")
│   ├── QPushButton: newBookmarkFolderButton ("Add Folder")
│   ├── QPushButton: removeBookmarkButton ("Remove")
│   ├── QLabel: basicAdvancedLabel ("More" / "Less" expandable toggle)
│   └── QDialogButtonBox: OK, Cancel, Apply
```

### 3.2 Context Menu Actions on Tree View (`bookmarkstreeview.cpp`)
- `&Connect`
- `Connect in New &Tab`
- `&Edit Bookmark`
- `Add Bookmark`
- `Add Folder`
- `Duplicate`
- `Remove`

---

## 4. Identities Dialog (`IdentityDialog`) & Security Level

- **Controller Implementation:** `src\UI_qt\identitydialog.cpp`, `src\UI_qt\tools\identityqualitywidget.cpp`
- **UI Headers:** `ui_identitydialog.h:342-343`, `ui_identityqualitywidget.h:193`
- **Context Names:** `IdentityDialog`, `IdentityQualityWidget`, `com::teamspeak::ui::IdentityDialog`
- **Shortcut:** `Ctrl+I` (in Tools / Identities)

### 4.1 Identities Dialog Hierarchy (`ui_identitydialog.h`)

```
QDialog: IdentityDialog ("Identities")
├── QWidget: identitiesBanner (myTeamSpeak account status banner)
├── QSplitter: identitiesLocalRemoteSplitter
│   ├── QListWidget: remoteIdentitiesListWidget ("Synchronized Identities")
│   └── QListWidget: localIdentitiesListWidget ("Local Identities")
├── QWidget: layoutWidget (Right side editor)
│   ├── QLabel: label_identityName ("&Identity Name:")
│   ├── QLineEdit: identityLineEdit
│   ├── QLabel: identityNameWarningLabel (Validation warning)
│   ├── QLabel: label_nickname ("&Nickname:")
│   ├── QLineEdit: nicknameLineEdit
│   ├── QLabel: nicknameWarningLabel
│   ├── QLabel: label_phonetic ("&Phonetic Nickname:")
│   ├── QLineEdit: phoneticNicknameLineEdit
│   ├── QWidget: uniqueIDWidget
│   │   ├── QLabel: label_uid ("Unique ID:")
│   │   └── QLineEdit: uidLineEdit [ReadOnly: true]
│   └── QWidget: securityLevelWidget
│       ├── QLabel: label_secLevel ("Security Level:")
│       ├── QLabel: idQualityLevelLabel ("Level %1")
│       └── QPushButton: improvedIdentityQualityButton ("Improve")
├── QHBoxLayout: horizontalLayout_4 (Bottom button bar)
│   ├── QPushButton: idAddButton ("Add Identity")
│   ├── QPushButton: idRemoveButton ("Remove Identity")
│   ├── QPushButton: idSelectDefaultButton ("Set Default Identity")
│   └── QDialogButtonBox: OK, Cancel
```

### 4.2 Improve Security Level Dialog (`IdentityQualityWidget`)

- **UI Header:** `ui_identityqualitywidget.h:193`
- **Context:** `IdentityQualityWidget`
- **Algorithm:** Hashcash-like partial collision search on cryptographic identity key.

```
QWidget / QDialog: IdentityQualityWidget ("Identity Security Level")
├── QLabel: topInfoLabel ("Identity: %1" / "Insufficient identity security level for this server...")
├── QLabel: currentQualityLabel ("Current Security Level: %1")
├── QLabel: label_requested ("&Requested Security Level:")
├── QSpinBox: requestedQualitySpinBox [Range: 0-128]
├── QLabel: elapsedLabel ("Elapsed Time: 00:00")
├── QLabel: estimatedLabel ("Total Time Ballpark Figure: %1")
├── QPushButton: startButton ("Start" / "Stop")
└── QDialogButtonBox: Close
```

---

## 5. Main Window Menus & Options/Settings Dialog

### 5.1 Top-Level Menu Bar & Self Menu (`src\UI_qt\mainwindow.cpp:5156-5809`)

#### 1. Connections (`&Connections`)
- `&Connect` (`Ctrl+S`, slot: `1onConnectRequest()`, line 5156)
- `&Disconnect from Current` (`Ctrl+D`, slot: `1onDisconnectCurrentRequest()`, line 5163)
- `&Disconnect from All` (`Ctrl+Shift+D`, slot: `1onDisconnectAllRequest()`)
- *Separator*
- `&Quit` (`Ctrl+Q`, slot: `1onQuit()`, line 5189)

#### 2. Bookmarks (`&Bookmarks`)
- `&Manage Bookmarks` (`Ctrl+B`, slot: `1onBookmarksManagerRequest()`, line 842)
- `&Add to Bookmarks` (`Ctrl+Shift+B`, slot: `1onAddBookmark()`)
- *Separator*
- Dynamic bookmark entries & folder submenus

#### 3. Self Menu (`S&elf`, slot: `1onUpdateSelfMenu()`, line 5714)
- `&Set Away Status` / `&Toggle Away Status` (slot: `1onToggleAway()`, line 5199)
- Away Presets Submenu (`Away Menu`, line 5807):
  - `&Set Away` (Sets local away message)
  - `&Set Globally Away` (Sets away message across all server tabs)
  - Predefined away reason list
- `&Online` (Clear away status on current server, line 5229)
- `&Online on all servers` (Clear away globally, line 5235)
- *Separator*
- `&Activate Microphone` / Mute Microphone (`Ctrl+M`, slot: `1onMuteCurrentInputRequest()`, line 5242)
- `(Un)Mute the Speakers/Headphones` / Mute Sound (`Ctrl+Shift+M`, slot: `1onMuteCurrentOutputRequest()`, line 5249)
- Set &Phonetic Nickname (slot: `1onSetPhoneticNickname()`, line 5532)
- *Separator*
- `&Privilege Keys` / `Use &Privilege Key`
- Channel &Groups of Client (slot: `1onShowChannelGroupsOfClient()`, line 5434)
- `&Connection Info` (`Ctrl+I`, slot: `1onSelfClientConnectionInfoAction()`, line 5545)

#### 4. Permissions (`Per&missions`, slot: `1onUpdatePermissionMenu()`, line 5732)
- `&Server Groups` (`Ctrl+F1`, line 5368)
- `&Client Permissions` (`Ctrl+F2`, line 5360)
- `&Channel Permissions` (`Ctrl+F3`, line 5376)
- `&Channel Groups` (`Ctrl+F4`, line 5384)
- `&Channel Client Permissions` (`Ctrl+F5`, line 5392)
- `&Privilege Keys` (line 5532)

#### 5. Tools (`&Tools`, slot: `1onUpdateToolsMenu()`, line 5769)
- `Collected &URLs` (line 5286)
- `File &Transfers` (`Ctrl+T`, line 5294)
- `Start &Recording` / `Stop &Recording` (`Ctrl+1`, line 5478, 5485)
- `&Whisper Lists` (`Ctrl+Shift+W`, line 5571)
- `&Whisper History` (line 5586)
- `T&oggle Block Incoming Whispers` (line 5605)
- `&List All Clients` (`Ctrl+Shift+L`, line 5545)
- `&Server Log` (`Ctrl+L`, line 5344)
- `Client &Log` (`Ctrl+Shift+C`)
- `&ServerQuery Login` (line 5417)
- `&Invite Buddy` (line 5605)
- `&Toggle ServerQueryClients` (line 5620)
- *Separator*
- `&Options` (`Alt+P`, line 5272)

#### 6. Plugins / Addons (`P&lugins`, slot: `1onUpdatePluginsMenu()`, line 5783)
- List of installed external plugins / Lua scripts / Overwolf

#### 7. Help (`&Help`)
- `&About TeamSpeak 3`
- `About &Qt`
- `View &License`
- `View &Changelog`
- `Check for &Update`
- `Show News &Ticker`
- `Visit TeamSpeak &Webpage`
- `&Rent own TeamSpeak Server`
- `&myTeamSpeak Account`

---

### 5.2 Options / Settings Dialog (`OptionsDialog`)

- **Controller Implementation:** `src\UI_qt\setup\optionsdialog.cpp:141-382`
- **Context:** `OptionsDialog`
- **Structure:** `QListWidget: ContentsWidget` (left category icons) controlling `QStackedWidget: m_pagesList` (right panels), with `OptionsHeadlineTitleLabel` and `OptionsHeadlineDescriptionLabel`.

```
OptionsDialog ("Options")
├── QListWidget: ContentsWidget
│   ├── Application
│   ├── Design
│   ├── Downloads
│   ├── Security
│   ├── Playback
│   ├── Capture
│   ├── Chat
│   ├── Whisper
│   ├── Hotkeys
│   ├── Messages
│   ├── Notifications
│   ├── Addons
│   └── myTeamSpeak
└── QStackedWidget: m_pagesList
```

#### Settings Categories & Fields Breakdown

1. **Application** (`applicationsetup.cpp`, `ApplicationSetup`):
   - Language selection (`languageComboBox`)
   - Confirmations: "Warn when closing TeamSpeak with active connections", "Warn when disconnecting from server with active tabs"
   - Behavior: "Never show Poke Dialog" ("Completely disable the Poke Dialog to avoid problems when running fullscreen games")
   - News ticker toggle

2. **Design** (`ui_guisetup.h:321-322`, `guisetup.cpp`, `guiSetupDialog`):
   - Theme/Style: `styleComboBox`, `styleSheetComboBox`
   - Icons: `iconPackComboBox`, `moreStylesAndIconsLabel`
   - Window Opacity: `opacitySlider`
   - Server Tree:
     - Expand behavior: `expandLevelAllRadioButton`, `expandLevelOwnRadioButton`, `expandLevelSpinBox`
     - Display options: `sortClientsAfterChannelsCheckBox`, `enableCountryFlagCheckBox`, `enableOverwolfIconsCheckBox`, `enableBadgeIconsCheckBox`, `enableGroupIconsCheckBox`, `hideInaccessibleGroupsCheckBox`, `channelCommanderAsLampCheckBox`, `awayMessageBesideNickNameCheckBox`, `treeTooltipInfoCheckBox`
   - System Tray: `trayHideIconCheckBox`, `trayMinimizeCheckBox`, `trayCloseCheckBox`
   - Animations: `animatedAvatarsCheckBox`, `animatedImagesCheckBox`

3. **Downloads** (`ui_downloadsetup.h:321-324`, `downloadsetupdialog.cpp`, `downloadSetupDialog`):
   - Destination: `alwaySaveHereRadioButton`, `askEveryDownloadRadioButton`, `downloadFolderLabel`, `changeDownloadFolderPushButton`
   - Transfer UI: `showFileTransfersOnStartCheckBox`, `closeFileTransfersOnFinishedCheckBox`, `finishedMessageCheckBox`
   - Bandwidth limits: `unlimitUploadBWRadioButton`, `limitUploadBWRadioButton`, `uploadBWSpinBox`, `unlimitDownloadBWRadioButton`, `limitDownloadBWRadioButton`, `downloadBWSpinBox`
   - Simultaneous transfers: `uploadSimSpinBox`, `downloadSimSpinBox`

4. **Security** (`securitysetupdialog.cpp`, `ui_securitysetupdialog.h`):
   - Password saving: "Save channel passwords", "Save server passwords"
   - Master Password: `ChangeMasterPasswordDialog` (`ui_changemasterpassworddialog.h`)
   - Cache cleaner: `CacheClearer` (`ui_cacheclearer.h`)
   - Server certificate trust manager

5. **Playback** (`ui_playbacksoundsetupdialog.h:355-356`, `playbacksoundsetup.cpp`, `playbackSoundSetupDialog`):
   - Profile manager (`profileManager`)
   - Device: `playbackModeComboBox`, `playbackDeviceComboBox`, `customPlaybackDeviceLineEdit`
   - Volumes: `playbackVolumeFactorSlider`, `playbackVolumeFactorWaveSlider` (Master), `playbackVolumeFactorDbWaveSlider` (Sound pack)
   - Test button: `playTestSoundButton`
   - Options: `agcCheckBox` (Automatic Voice Gain Control), `playbackMicOwnCheckBox`, `sound3DCheckBox`, `playbackMicOthersCheckBox`
   - Comfort noise: `checkBoxComfortNoise`, `horizontalSliderComfortNoise` ("Quiet" to "Loud")

6. **Capture** (`ui_capturesoundsetupdialog.h:443-447`, `capturesoundsetup.cpp`, `captureSoundSetupDialog`):
   - Device: `captureModeComboBox`, `captureDeviceComboBox`, `customCaptureDeviceLineEdit`
   - Voice Transmission Modes:
     - Continuous: `radioButtonContinous`
     - Voice Activation Detection: `radioButtonVAD`, `vadControl` (VAD slider & level meter), `comboBoxVadMode`, `checkBoxVadOverPtt`
     - Push-To-Talk: `radioButtonPTT`, `hotkeyPushButton`, `delayPTTCheckBox`, `delayPTTSpinBox`
   - DSP Options: `echoReductionCheckBox`, `echoReductionSpinBox`, `denoisecheckBox`, `echoCancellationCheckBox`, typing attenuation

7. **Chat** (`ui_chatsetupdialog.h:226-227`, `chatsetup.cpp`, `chatSetupDialog`):
   - Capacity: `maxLinesSpinBox`
   - Typography: `chatFontButton`, `chatFontLabel`, `chatFontResetButton`
   - Automatic logging: `logChannelChatsCheckBox`, `logClientChatsCheckBox`, `logServerChatsCheckBox`
   - Reload history: `reloadChannelChatsCheckBox`, `reloadChannelChatsSpinBox`, `reloadClientChatsCheckBox`, `reloadClientChatsSpinBox`
   - Visible tabs: `cbServerChat`, `cbChannelChat`, `cbPrivateChat`

8. **Whisper** (`whispersetup.cpp`, `WhisperSetup`):
   - Notification sound toggles and volume slider
   - Permissions: "Block incoming whispers"
   - Shortcut button to open Whisper Lists

9. **Hotkeys** (`ui_hotkeydialog.h:191-192`, `hotkeyssetup.cpp`, `HotkeysSetup`):
   - Profile list & toolbar (Add, Edit, Remove, Duplicate)
   - Hotkey assignment dialog (`HotkeyDialog`):
     - Trigger: `keyDownRadioButton` ("On key down"), `keyUpRadioButton` ("On key release")
     - Flags: `discardCheckBox` ("Discard"), `activeTabCheckBox` ("Active tab"), `keepOpenCheckBox` ("Keep dialog open")
     - Action tree: `actionTreeView`, `advancedActionsButton`

10. **Messages** (`messagessetup.cpp`, `MessagesSetup`):
    - Table mapping notification events to destination output targets: Info Frame, Server Chat tab, Channel Chat tab, or Modal Dialog.

11. **Notifications** (`notificationssetup.cpp`, `NotificationsSetup`):
    - Sound Pack selector: `SoundPackComboBox`
    - Tree view of events: Client (Connected, Disconnected, Moved, Renamed, Muted, Talk Power), Channel (Created, Deleted, Edited), Server.
    - Checkbox columns per event: Play Sound, Text-to-Speech (TTS), Show in Chat.

12. **Addons** (`addons_setup.cpp`, `com::teamspeak::ui::addons::Addons_Setup`):
    - Tabs: "My Addons" vs "Browse online"
    - Categories: Themes, Icon Packs, Sound Packs, Translations, Plugins
    - Controls: Install, Uninstall, Enable/Disable, Configure

13. **myTeamSpeak** (`accountsetup.cpp`, `com::teamspeak::ui::AccountSetup`):
    - Authentication: Login, Register, Logout, Reset Password, New Recovery Key
    - Synchronized components checklist: Bookmarks, Identities, Hotkeys, Whisper Lists, Contacts

---

## 6. Create / Edit Channel Dialog (`CreateChannelDialog`)

- **Controller Implementation:** `src\UI_qt\createchanneldialog.cpp`
- **UI Header:** `ui_createchanneldialog.h:801-807`
- **Context:** `CreateChannelDialog`

### 6.1 Tab Widget & Field Hierarchy

```
QDialog: CreateChannelDialog ("Create Channel" / "Edit Channel")
├── QTabWidget: tabWidget
│   ├── QWidget: standardTab ("Standard")
│   │   ├── QLabel: nameLabel ("&Name:")
│   │   ├── QLineEdit: nameLineEdit
│   │   ├── QToolButton: toolButtonIcon ("&Icon:", tooltip: "Set channel icon.")
│   │   ├── QLabel: pwLabel ("&Password:")
│   │   ├── QLineEdit: passwordLineEdit [EchoMode: Password]
│   │   ├── QLabel: topicLabel ("&Topic:")
│   │   ├── QLineEdit: topicLineEdit
│   │   ├── QLabel: descLabel ("&Description:")
│   │   ├── QToolButton: descEditButton ("Tear off description editor")
│   │   ├── QTextEdit: descriptionTextEdit
│   │   ├── QGroupBox: continuityGroupBox ("Channel &Type")
│   │   │   ├── QRadioButton: temporaryRadioButton ("Temporary", "Channel will be deleted when the last user left.")
│   │   │   ├── QRadioButton: semiPermanentRadioButton ("Semi-Permanent", "Channel exists until server is restarted.")
│   │   │   ├── QRadioButton: permanentRadioButton ("Permanent", "Channel will exist until manually deleted.")
│   │   │   └── QCheckBox: defaultCheckBox ("Default Channel")
│   │   ├── QGroupBox: groupBox_2 ("Sort Order")
│   │   │   └── QComboBox: channelOrderComboBox (Select parent or sibling channel)
│   │   └── QGroupBox: groupBox ("Needed Talk Power")
│   │       └── QSpinBox: talkPowerSpinBox
│   │
│   ├── QWidget: audioTab ("Audio")
│   │   ├── QGroupBox: audioDefaultSettingsGroupBox ("Predefined Configurations")
│   │   │   ├── QRadioButton: codecVoiceMobileRadioButton ("Voice Mobile")
│   │   │   ├── QRadioButton: codecVoiceDesktopRadioButton ("Voice Desktop")
│   │   │   ├── QRadioButton: codecMusicRadioButton ("Music")
│   │   │   └── QRadioButton: codecCustomRadioButton ("Custom")
│   │   └── QGroupBox: audioCustomSettingsGroupBox ("Codec Details")
│   │       ├── QComboBox: codecComboBox (Opus Voice, Opus Music, Speex, CELT)
│   │       ├── QSlider: codecQualitySlider [Range: 0-10]
│   │       ├── QLabel: codecQualityDisplayLabel (Shows current numeric value)
│   │       └── QLabel: bandwidthLabel (Calculated bitrate e.g., "5.71 KiB/s")
│   │
│   ├── QWidget: permissionsTab ("Permissions")
│   │   ├── QGroupBox: groupBox_4 ("Channel Permissions")
│   │   │   ├── QSpinBox: joinPowerSpinBox ("Needed Join Power")
│   │   │   ├── QSpinBox: subscribePowerSpinBox ("Needed Subscribe Power")
│   │   │   ├── QSpinBox: descViewPowerSpinBox ("Needed Description View Power")
│   │   │   ├── QSpinBox: modifyPowerSpinBox ("Needed Modify Power")
│   │   │   └── QSpinBox: deletePowerSpinBox ("Needed Delete Power")
│   │   ├── QGroupBox: groupBox_5 ("File Transfer Permissions")
│   │   │   ├── QSpinBox: browsePowerSpinBox ("Needed Browse Power")
│   │   │   ├── QSpinBox: uploadPowerSpinBox ("Needed Upload Power")
│   │   │   ├── QSpinBox: downloadPowerSpinBox ("Needed Download Power")
│   │   │   ├── QSpinBox: renamePowerSpinBox ("Needed Rename Power")
│   │   │   └── QSpinBox: dirCreatePowerSpinBox ("Needed Directory Create Power")
│   │   └── QLabel: channelPermsInfoLabel
│   │
│   └── QWidget: advancedTab ("Advanced")
│       ├── QGroupBox: groupBox_3 ("Options")
│       │   ├── QLabel: label_phonetic ("&Phonetic Name:")
│       │   ├── QLineEdit: phoneticNameLineEdit
│       │   ├── QLabel: label_deleteDelay ("Delete delay for temporary channels:")
│       │   ├── QSpinBox: channelDeleteDelaySpinBox (Seconds)
│       │   ├── QPushButton: deleteDelayMaxButton ("Max")
│       │   └── QCheckBox: encryptedVoiceDataCheckBox ("Channel voice data encryption")
│       ├── QGroupBox: maxUsersGroupBox ("Max Users")
│       │   ├── QRadioButton: maxUsersUnlimitedRadioButton ("Unlimited")
│       │   ├── QRadioButton: maxUsersLimitedRadioButton ("Limited")
│       │   └── QSpinBox: maxUsersSpinBox
│       └── QGroupBox: familiyMaxUsersGroupBox ("Family Max Users")
│           ├── QRadioButton: familyMaxUsersInheritedRadioButton ("Inherited")
│           ├── QRadioButton: familyMaxUsersUnlimitedRadioButton ("Unlimited")
│           ├── QRadioButton: familyMaxUsersLimitedRadioButton ("Limited")
│           └── QSpinBox: familyMaxUsersSpinBox
└── QDialogButtonBox: OK, Cancel, Apply
```

### 6.2 Signal / Slot Connections (`[ui_createchanneldialog.h:801-807]`)

- `buttonBox::accepted()` $\rightarrow$ `CreateChannelDialog::accept()` (line 801)
- `buttonBox::rejected()` $\rightarrow$ `CreateChannelDialog::reject()` (line 802)
- `codecCustomRadioButton::toggled(bool)` $\rightarrow$ `audioCustomSettingsGroupBox::setEnabled(bool)` (line 803)
- `temporaryRadioButton::toggled(bool)` $\rightarrow$ `channelDeleteDelaySpinBox::setDisabled(bool)` (line 804)
- `maxUsersLimitedRadioButton::toggled(bool)` $\rightarrow$ `maxUsersSpinBox::setEnabled(bool)` (line 806)
- `familyMaxUsersLimitedRadioButton::toggled(bool)` $\rightarrow$ `familyMaxUsersSpinBox::setEnabled(bool)` (line 807)

---

## 7. Moderation, Privacy & Administration Dialogs

### 7.1 Poke Dialog (`PokeDialog` / `MsgDialog`)

- **Controller Implementation:** `src\UI_qt\msgdialog.cpp:104`
- **UI Header:** `ui_msgdialog.h:104`
- **Contexts:** `PokeDialog`, `MsgDialog`, `ServerView`

- **Sending Poke:** Triggered via Client Context Menu (`1onClientPokeRequest()`, `serverview.cpp:9892`).
  - Input dialog prompt: Title `"Poke"`, Label `"Poke Reason:"`.
- **Receiving Poke Display (`MsgDialog`):**
  - Window Title: `"You Have Been Poked"`
  - Widgets: `QLabel: serverLabel` (Server name), `QLabel: imgLabel` (Poke icon), `QTextBrowser: msgTextBrowser` (HTML formatted timestamp, poker nickname, and poke reason text).
  - Button: `OK` (`1accept()`).
  - Context menu actions: `"Copy"`, `"Select All"`.

---

### 7.2 Kick Dialogs

Triggered via `ServerView` context menu actions:
1. **Kick Client from Channel:** `1onKickClientFromChannelRequest()` (`serverview.cpp:9877`)
   - Title: `"Kick from Channel"`
   - Prompt: `"Kick Message:"`
   - OK / Cancel buttons
2. **Kick Client from Server:** `1onKickClientFromServerRequest()` (`serverview.cpp:9886`)
   - Title: `"Kick from Server"`
   - Prompt: `"Kick Message:"`
   - OK / Cancel buttons

---

### 7.3 Ban Dialogs (`BanAddDialog`, `BanClientDialog`, `BanList`)

- **Controller Implementation:** `src\UI_qt\tools\bandialog.cpp:448`, `src\UI_qt\tools\banlist.cpp:683`
- **UI Headers:** `ui_banadddialog.h:267-268`, `ui_banclientdialog.h:181-182`, `ui_banlist.h:143`

#### Ban Add / Edit Dialog (`BanAddDialog`):
- `QLineEdit: banIpLineEdit` (Label `ipRegexpLabel`: `"&IP:"`)
- `QLineEdit: banNameLineEdit` (Label `nameRegexpLabel`: `"&Name:"`)
- `QComboBox: banTypeComboBox` (Label: `"&Interpret IP/Name as:"`)
  - Options: `Wildcard ipv4`, `Wildcard ipv6`, `Fixed String`, `Regular Expression`
- `QLineEdit: banUniqueIDLineEdit` (Label `uniqueIdentityLabel`: `"&Unique ID:"`)
- `QLineEdit: banMytsIDLineEdit` (Label `mytsIDLabel`: `"&myTeamSpeak ID:"`)
- `QTextEdit: banReasonTextEdit` (Label: `"Reason:"`)
- Duration:
  - `QSpinBox: banDurationSpinBox`
  - `QComboBox: banDurationComboBox` (`seconds`, `minutes`, `hours`, `days`, `permanent`)
  - `QPushButton: maxButton` ("Set duration to maximum value")
- Presets:
  - `QComboBox: banPresetComboBox`
  - `QPushButton: saveButton` ("Save Preset")

#### Ban List Window (`BanList`):
- Window Title: `"Ban List - %1"`
- `QTableView: banTableView`
  - Columns: `Name`, `IP`, `UID`, `myTSID`, `Reason`, `Created`, `Creator`, `Expires`, `Enforced`
- Action Buttons:
  - `QPushButton: banAddButton` ("Add", tooltip: `Add new ban<br>(Ctrl+N)`)
  - `QPushButton: banRemoveButton` ("Remove", tooltip: `Remove selected bans<br>(Delete)`)
  - `QPushButton: banEditButton` ("Edit", tooltip: `Edit selected ban<br>(Return)`)
  - `QPushButton: reloadButton` ("Reload", tooltip: `Search bans by text<br>(Ctrl+F)`)
- Filtering Options:
  - `QCheckBox: showOnlyOwnBansCheckBox` ("Show only own bans")
  - `QCheckBox: highlightOwnBansCheckBox` ("Highlight own bans")

---

### 7.4 Whisper List Manager (`WhisperDialog`) & History

- **Controller Implementation:** `src\UI_qt\tools\whisperdialog.cpp:1512`
- **Context Names:** `WhisperDialog`, `com::teamspeak::ui::WhisperDialog`
- **Shortcut:** `Ctrl+Shift+W`

- **Left Panel (Whisper List Tree):**
  - Categories: `Synchronized Whisperlists` vs `Local Whisperlists`
  - Buttons: `New`, `Rename`, `Remove`, `Import`, `Export all`
- **Center Panel (Configuration):**
  - Hotkey assignment: `"Hotkey:"`, Button `"No Hotkey Assigned"` (opens key grabber)
  - Reply Hotkey: `"Reply Hotkey:"`
  - Whisper Target Group Selection:
    - `"Whisper to:"` combobox: `Clients & Channels` vs `Groups`
    - Group Targets: `All Clients`, `Current Channel`, `Parent Channel`, `All Parent Channels`, `Subchannels`, `Channel Family`, `Complete Channel Family`, `All Channels`, `Channel Commander`, `Server Group`, `Channel Group`
- **Right Panel (Target Selector Picker):**
  - Tabs: `Quick Access`, `Contacts`, `Clients & Channels`, `Groups`
- **Whisper History Widget (`WhisperHistoryWidget`, `ui_whisperhistorywidget.h:83-84`):**
  - `clientServerInfoLabel`
  - `QListWidget: whisperHistoryListWidget`
  - `QPushButton: optionsButton` ("Open whisper options")
  - `QPushButton: clearButton` ("Clear")

---

### 7.5 Complaints Viewer (`ComplainView`)

- **Controller Implementation:** `src\UI_qt\complainview.cpp:278`
- **UI Header:** `ui_complainview.h:104`
- **Context:** `ComplainView`
- **Hierarchy:**
  - `QTreeWidget: complainTreeView` (Columns: Target User, Submitter, Reason, Date/Time)
  - `QPushButton: complainRemoveButton` ("Remove selected...")
  - `QPushButton: pushButtonReload` ("Reload")
  - Context action: `"Ban Client"` (slot: `1onBanClient()`)

---

### 7.6 Privilege Key Dialogs (`TokenView` & `TokenAdd`)

- **Controller Implementation:** `src\UI_qt\tools\tokenview.cpp:68-228`, `src\UI_qt\tools\tokenadd.cpp`
- **Context Names:** `TokenView`, `TokenAdd`

#### Privilege Keys List (`TokenView`):
- `QTableView`: Columns: `Type`, `Group`, `Channel`, `Description`, `Created`
- Buttons:
  - `Create` ("Create new privilege key")
  - `Remove` ("Remove selected privilege keys")
  - `Reload` ("Reload all privilege keys")
  - Context menu: "Copy Privilege Key to clipboard", "Use Privilege Key as invitation"

#### Add Privilege Key (`TokenAdd`):
- `QComboBox`: `&Type:` (`Server Group` or `Channel Group`)
- `QComboBox`: `&Group:` (List of available groups)
- `QComboBox`: `&Channel:` (Target channel, active only when Type = Channel Group)
- `QLineEdit`: `&Description:` ("Optional description for this privilege key")
- Output field: `&Generated privilege key:` with `Copy to clipboard` button

---

## 8. Server Tree Right-Click Context Menus (`ServerView`)

All tree context menus are generated in `src\UI_qt\serverview.cpp:156-10400` in response to `customContextMenuRequested(QPoint)` (connected to `1onTreeContextMenuRequest(QPoint)`, line 207).

### 8.1 VirtualServer (Server Root) Context Menu

```
[Server Root Right-Click]
├── "Edit Virtual Server" (Opens VirtualServerEditDialog)
├── "&Server Connection Info" (Slot: 1onServerConnectionInfoRequest(), line 9779)
├── *Separator*
├── "&Create Channel" (Slot: 1onCreateChannelRequest(), line 9842)
├── "Create &Spacer" (Slot: 1onCreateSpacer(), line 10233)
├── *Separator*
├── "S&ubscribe to All Channels" (Slot: 1onSubscribeAllChannelRequest(), line 9800)
├── "U&nsubscribe from All Channels" (Slot: 1onUnsubscribeAllChannelRequest(), line 9816)
├── *Separator*
├── "Expand All" (Slot: 1onExpandAll(), line 9899)
├── "Collapse All" (Slot: 1onCollapseAll(), line 9906)
├── *Separator*
├── "Server Groups Dialog" (Opens Server Groups manager)
├── "Temporary Passwords" (Submenu: Add Temp Password, List Temp Passwords)
└── "&Change Nickname" (Slot: 1onClientRenameRequest(), line 9922)
```

---

### 8.2 Channel Context Menu

```
[Channel Right-Click]
├── "&Switch to Channel" (Slot: 1onSwitchChannelRequest(), line 9786)
├── *Separator*
├── "Open File Browser" (Slot: 1onFileBrowserRequest(), line 9952)
├── *Separator*
├── "S&ubscribe to Channel" (Slot: 1onSubscribeChannelRequest(), line 9792)
├── "U&nsubscribe from Channel" (Slot: 1onUnsubscribeChannelRequest(), line 9807)
├── "S&ubscribe to Channel Family" (Slot: 1onSubscribeChannelFamilyRequest(), line 9825)
├── "U&nsubscribe from Channel Family" (Slot: 1onUnsubscribeChannelFamilyRequest(), line 9835)
├── *Separator*
├── "&Create Channel" (Slot: 1onCreateChannelRequest(), line 9842)
├── "C&reate Sub-Channel" (Slot: 1onCreateSubChannelRequest(), line 9849)
├── "Create &Spacer" (Slot: 1onCreateSpacer(), line 10233)
├── "&Edit Channel" (Slot: 1onEditChannelRequest(), line 9856)
├── "&Delete Channel" (Slot: 1onDeleteChannelRequest(), line 9863)
├── *Separator*
├── "Permissions" (Submenu)
│   ├── "Channel Permissions" (Slot: 1onChannelPermissions())
│   ├── "Channel Client Permissions" (Slot: 1onChannelClientPermissions())
│   └── "Permission Overview" (Slot: 1onPermissionOverviewRequest())
├── *Separator*
├── "Expand Subchannels" (Slot: 1onExpandSubchannels(), line 9914)
└── "Collapse Subchannels" (Slot: 1onCollapseSubchannels(), line 9922)
```

---

### 8.3 Client Context Menu

```
[Client Right-Click]
├── "&Open Text Chat" (Slot: 1onOpenChatRequest(), line 9871)
├── "&Poke Client" (Slot: 1onClientPokeRequest(), line 9892)
├── "Send Offline Message" (Slot: 1onContextSendOfflineMessage(), line 2374)
├── *Separator*
├── "&Join Channel of Client" (Slot: 1onJoinClientChannelRequest(), line 9934)
├── "&Move Client to own Channel" (Slot: 1onMoveClientToOwnChannelRequest(), line 9940)
├── "Move Client to Channel" (Submenu listing all server channels)
├── "&Find Client in Channel Tree" (Slot: 1onScrollToThisClient(), line 9946)
├── *Separator*
├── "Change &Volume" (Opens popup UserVolumeWidget slider)
├── "Mute Client" / "Unmute Client" (Slot: 1onMuteClient() / 1onUnmuteClient(), line 10002)
├── "&Priority Speaker" (Slot: 1onSetPrioritySpeakerRequest(bool))
├── "&Channel Commander" (Slot: 1onChannelCommander(bool), line 10225)
├── "Request Talk Power" / "Cancel Talk Power Request" (Slot: 1onRequestClientTalkPower())
├── "Grant Talk Power" / "Revoke Talk Power" (Slot: 1onGrantTalkPower() / 1onRevokeTalkPower())
├── *Separator*
├── "&Kick Client from Channel" (Slot: 1onKickClientFromChannelRequest(), line 9877)
├── "&Kick Client from Server" (Slot: 1onKickClientFromServerRequest(), line 9886)
├── "&Ban Client" (Slot: 1onClientBanRequest())
├── *Separator*
├── "Set Server Group" (Submenu: QActionGroup with checkable server groups, slot: 1onClientSetServerGroup(QAction*), line 10395)
├── "Set Channel Group" (Submenu: QActionGroup with radio channel groups, slot: 1onClientSetChannelGroup(QAction*), line 10331)
├── "Channel Groups of Client" (Slot: 1onShowChannelGroupsOfClient())
├── "Permissions" (Submenu)
│   ├── "Client Permissions" (Slot: 1onClientPermissions())
│   ├── "Channel Client Permissions" (Slot: 1onChannelClientPermissions())
│   └── "Permission Overview" (Slot: 1onPermissionOverviewRequest())
├── *Separator*
├── "Add to Contacts" / "Edit in Contacts" / "Remove from Contacts" (line 2133-2134)
├── "&Change Description" (Slot: 1onClientDescriptionRequest())
├── "Delete Avatar" (Action: "Delete users avatar")
└── "&Client Connection Info" (Slot: 1onClientConnectionInfoRequest(), line 9772)
```

---

## 9. Verification & Reference Matrix

| Feature Area | Source C++ / Header Reference | Binary VA / Offset | Verification Status |
|---|---|---|---|
| Connect Dialog | `ui_connectdialog.h:332-333` | Offset `0x00FD7638` | **Verified** |
| Bookmarks Manager | `bookmarksmanager.cpp`, `bookmarkstreeview.cpp` | Offset `0x00FC8B70` | **Verified** |
| Identities Dialog | `ui_identitydialog.h:342-343` | Offset `0x010936E0` | **Verified** |
| Security Level | `ui_identityqualitywidget.h:193` | Offset `0x010957CF` | **Verified** |
| Menu Bar Actions | `mainwindow.cpp:5156-5809` | Offset `0x00FFE380` | **Verified** |
| Options Dialog | `optionsdialog.cpp:141-382` | Offset `0x010078A0` | **Verified** |
| Channel Properties | `ui_createchanneldialog.h:801-807` | Offset `0x00FDC3F0` | **Verified** |
| Ban Dialogs | `ui_banadddialog.h:267`, `bandialog.cpp:448` | Offset `0x00FE2D60` | **Verified** |
| Server Context Menu | `serverview.cpp:9772-9842` | Offset `0x010A2B90` | **Verified** |
| Channel Context Menu | `serverview.cpp:9786-9922` | Offset `0x010A2E20` | **Verified** |
| Client Context Menu | `serverview.cpp:9871-10395` | Offset `0x010A3810` | **Verified** |
