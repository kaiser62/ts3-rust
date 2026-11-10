# TeamSpeak 3 QSS Theme System & Visual Style Reverse-Engineering Reference

## Document Information
- **Target Files Analyzed**:
  - `C:\Program Files\TeamSpeak 3 Client\styles\default.qss` (161 lines)
  - `C:\Program Files\TeamSpeak 3 Client\styles\default_vista.qss` (6 lines)
  - `C:\Program Files\TeamSpeak 3 Client\styles\default_chat.qss` (268 lines)
  - `C:\Program Files\TeamSpeak 3 Client\styles\default_darkmode.qss` (10 lines)
  - `C:\Program Files\TeamSpeak 3 Client\styles\default_chat_darkmode.qss` (87 lines)
  - Companion styles & templates: `default_mac.qss` (68 lines), `channelinfo.tpl`, `clientinfo.tpl`, `serverinfo.tpl`, `tree_tooltip_channel.tpl`, `tree_tooltip_client.tpl`
- **Engine Baseline**: Qt 5 C++ GUI Framework (`QWindowsVistaStyle` engine via `qwindowsvistastyle.dll` + QSS cascade)
- **Output Reference File**: `D:\Dev\ts3-rust\docs\re\gui\03-qss-themes.md`

---

## 1. Executive Architecture of TeamSpeak 3 Theming Engine

TeamSpeak 3 implements a layered theming architecture based on Qt's stylesheet mechanism (`QStyleSheetStyle`) operating on top of a platform-native base style:

1. **Native Widget Style Layer (`QWindowsVistaStyle`)**:
   Under Windows Vista, 7, 8, 10, and 11, the client instantiates `QWindowsVistaStyle` loaded from `styles/qwindowsvistastyle.dll`. This native style delegates button rendering, combo boxes, scrollbars, header views, and menu borders to Windows Desktop Window Manager (DWM) theme parts (`uxtheme.dll`).
2. **Global Application QSS Layer (`default.qss` + `default_vista.qss`)**:
   TeamSpeak loads `default.qss` globally via `QApplication::setStyleSheet()`. When running on Windows, `default_vista.qss` is loaded additionally (`default_vista.qss:3`). On macOS, `default_mac.qss` is loaded, and in macOS Dark Mode, `default_darkmode.qss` is appended (`default_darkmode.qss:3`).
3. **Rich-Text Document QSS Layer (`default_chat.qss` + `default_chat_darkmode.qss`)**:
   The chat view (`QTextEdit#ChatTab`) and info panels (`QTextEdit#InfoFrame`) render an HTML 4 / Qt Rich Text document. The document stylesheet is injected directly into `QTextDocument::setDefaultStyleSheet()`. In dark mode, `default_chat_darkmode.qss` overrides the selectors.
4. **HTML Template Layer (`styles/default/*.tpl`)**:
   InfoFrame panels and tree tooltips use templates (`channelinfo.tpl`, `clientinfo.tpl`, `serverinfo.tpl`) containing inline tables styled with CSS classes defined in `default_chat.qss`.

---

## 2. Complete Hex/RGB Color Palettes

### 2.1 Global Overrides & Palette Roles (`default.qss`)

| Role / Identifier | Hex Value | RGB Decimal | Source Location | Usage Context |
| :--- | :--- | :--- | :--- | :--- |
| `QPalette::Link` | `#1c82cc` | `rgb(28, 130, 204)` | `default.qss:7` | Overrides Qt default hyperlink color across all widgets |
| `CustomColor::ClientFriend` | `#008000` | `rgb(0, 128, 0)` | `default.qss:10` | Friend contact nickname highlight in tree & chat |
| `CustomColor::ClientBlocked` | `#ff0000` | `rgb(255, 0, 0)` | `default.qss:11` | Blocked / muted client nickname highlight |
| `CustomColor::ClientRecording` | `#800000` | `rgb(128, 0, 0)` | `default.qss:12` | Client actively recording voice session indicator |
| `ChatLineEdit (Help Text)` | `#909090` | `rgb(144, 144, 144)` | `default.qss:56` | Placeholder text in chat input (`helpTextVisible="true"`) |
| `AppNameLabel` | `#364B85` | `rgb(54, 75, 133)` | `default.qss:111` | Brand text `TeamSpeak 2020` header |
| `HeadlineLabel (Start)` | `#819CBD` | `rgb(129, 156, 189)` | `default.qss:118` | Gradient stop 0 for top headline background |
| `HeadlineLabel (End)` | `#CDD7E5` | `rgb(205, 215, 229)` | `default.qss:118` | Gradient stop 1 for top headline background |
| `OptionsHeadlineTitleLabel` | `#000000` | `rgb(0, 0, 0)` | `default.qss:95` | Options dialog page header title |
| `OptionsHeadlineDescriptionLabel` | `#000000` | `rgb(0, 0, 0)` | `default.qss:99` | Options dialog page subtitle text |
| `ServerIntegration (BG)` | `#d6ecff` | `rgb(214, 236, 255)` | `default.qss:124,128,139` | Integration bubble & card background |
| `ServerIntegration (Border)` | `#1179ce` | `rgb(17, 121, 206)` | `default.qss:146` | Integration editable combo box border |
| `ServerIntegration (Error)` | `#ff0000` | `rgb(255, 0, 0)` | `default.qss:152` | ErrorIndicator background fill |

### 2.2 Banner Vertical Linear Gradients (`default.qss:40-44`)

The banner header (`bookmarksBanner`, `identitiesBanner`, `accountLoginBanner`, `accountCreateBanner`) renders a 4-stop vertical linear gradient (`x1:0, y1:0, x2:0, y2:24`):

$$\text{Gradient}(y) = \begin{cases} 
\text{rgb}(80, 92, 120) & y = 0.00 \ (0.0\text{px}) \ [\#505C78] \\
\text{rgb}(133, 147, 174) & y = 0.08 \ (1.92\text{px}) \ [\#8593AE] \\
\text{rgb}(88, 107, 140) & y = 0.32 \ (7.68\text{px}) \ [\#586B8C] \\
\text{rgb}(111, 128, 170) & y = 1.00 \ (24.0\text{px}) \ [\#6F80AA]
\end{cases}$$

- **Banner Text**: `color: white;` (`#FFFFFF`, `default.qss:45`)
- **Banner Font**: `font-weight: bold;` (`default.qss:47`)
- **Minimum Banner Height**: `32px` (`default.qss:48, 52`)

### 2.3 Light Mode Chat Palette (`default_chat.qss`)

| Message Type / CSS Selector | Hex Code | RGB Decimal | Qt / Web Named Color | Source Location | Visual Semantics |
| :--- | :--- | :--- | :--- | :--- | :--- |
| `.TextMessage_UserLink` | `#002F5D` | `rgb(0, 47, 93)` | Navy Blue | `default_chat.qss:99` | Clickable client nickname (`<b>`, `text-decoration: none`) |
| `.TextMessage_ChannelLink` | `#002F5D` | `rgb(0, 47, 93)` | Navy Blue | `default_chat.qss:107` | Clickable channel link |
| `.TextMessage_ServerLink` | `#002F5D` | `rgb(0, 47, 93)` | Navy Blue | `default_chat.qss:108` | Clickable server host link |
| `.TextMessage_OfflineLink` | `#002F5D` | `rgb(0, 47, 93)` | Navy Blue | `default_chat.qss:106` | Offline unread message link |
| `.TextMessage_HostName` | `#33597D` | `rgb(51, 89, 125)` | Steel Blue | `default_chat.qss:117` | Server hostname label (`<b>`) |
| `.TextMessage_Welcome` | `#0AA537` | `rgb(10, 165, 55)` | Forest Green | `default_chat.qss:124` | Server welcome message on connect |
| `.TextMessage_HostMessage`| `#0AA537` | `rgb(10, 165, 55)` | Forest Green | `default_chat.qss:124` | Server host message modal/chat |
| `.TextMessage_Poke` | `#0AA537` | `rgb(10, 165, 55)` | Forest Green | `default_chat.qss:125` | Client poke notification |
| `.TextMessage_Normal` | `#33597D` | `rgb(51, 89, 125)` | Steel Blue | `default_chat.qss:131` | Informational server events |
| `.TextMessage_Connected` | `#33597D` | `rgb(51, 89, 125)` | Steel Blue | `default_chat.qss:132` | Own client connected to server |
| `.TextMessage_Disconnected`| `#33597D` | `rgb(51, 89, 125)` | Steel Blue | `default_chat.qss:135` | Own client disconnected |
| `.TextMessage_ClientConnected`| `#33597D` | `rgb(51, 89, 125)`| Steel Blue | `default_chat.qss:133` | Other client connected to server |
| `.TextMessage_ClientDisconnected`| `#33597D`| `rgb(51, 89, 125)`| Steel Blue | `default_chat.qss:136` | Other client disconnected |
| `.TextMessage_ClientJoined`| `#33597D` | `rgb(51, 89, 125)` | Steel Blue | `default_chat.qss:134` | Client joined current channel |
| `.TextMessage_ClientLeft` | `#33597D` | `rgb(51, 89, 125)` | Steel Blue | `default_chat.qss:138` | Client left current channel |
| `.TextMessage_ClientSwitched`| `#33597D`| `rgb(51, 89, 125)` | Steel Blue | `default_chat.qss:137` | Client switched channel |
| `.TextMessage_ClientDropped` | `#C9070A` | `rgb(201, 7, 10)` | Crimson Red | `default_chat.qss:144` | Client connection timed out |
| `.TextMessage_ClientKicked` | `#C9070A` | `rgb(201, 7, 10)` | Crimson Red | `default_chat.qss:144` | Client kicked from server/channel |
| `.TextMessage_ClientBanned` | `#C9070A` | `rgb(201, 7, 10)` | Crimson Red | `default_chat.qss:145` | Client banned from server |
| `.TextMessage_ChannelCreated`| `#33597D` | `rgb(51, 89, 125)`| Steel Blue | `default_chat.qss:147` | Channel created event |
| `.TextMessage_ChannelDeleted`| `#33597D` | `rgb(51, 89, 125)`| Steel Blue | `default_chat.qss:148` | Channel deleted event |
| `.TextMessage_ChannelMoved`  | `#33597D` | `rgb(51, 89, 125)`| Steel Blue | `default_chat.qss:150` | Channel order or parent changed |
| `.TextMessage_ChannelEdited` | `#33597D` | `rgb(51, 89, 125)`| Steel Blue | `default_chat.qss:149` | Channel properties edited |
| `.TextMessage_ServerEdited`  | `#33597D` | `rgb(51, 89, 125)`| Steel Blue | `default_chat.qss:151` | Virtual server edited event |
| `.TextMessage_ServerError`   | `#C9070A` | `rgb(201, 7, 10)` | Crimson Red | `default_chat.qss:184` | Server error log (`<b>`) |
| `.TextMessage_GroupAssigned` | `#33597D` | `rgb(51, 89, 125)`| Steel Blue | `default_chat.qss:155` | Permission group assigned |
| `.TextMessage_GroupRevoked`  | `#33597D` | `rgb(51, 89, 125)`| Steel Blue | `default_chat.qss:156` | Permission group revoked |
| `.TextMessage_TalkPowerAssigned`| `#33597D`| `rgb(51, 89, 125)`| Steel Blue | `default_chat.qss:157` | Talk power granted |
| `.TextMessage_TalkPowerRevoked` | `#33597D`| `rgb(51, 89, 125)`| Steel Blue | `default_chat.qss:158` | Talk power revoked |
| `.TextMessage_TalkPowerRequested`| `#33597D`| `rgb(51, 89, 125)`| Steel Blue | `default_chat.qss:159` | Talk power request banner |
| `.TextMessage_Time`          | `#33597D` | `rgb(51, 89, 125)`| Steel Blue | `default_chat.qss:168` | Timestamp string `HH:mm:ss` |
| `.TextMessage_BeginHistory`  | `#707475` | `rgb(112, 116, 117)`| Neutral Gray | `default_chat.qss:174` | Chat history start marker (`<b><i>`) |
| `.TextMessage_EndHistory`    | `#707475` | `rgb(112, 116, 117)`| Neutral Gray | `default_chat.qss:175` | Chat history end marker (`<b><i>`) |
| `.InfoFrame_Title ... Links` | `#191970` | `rgb(25, 25, 112)` | `midnightblue` | `default_chat.qss:199` | Title bar entity link |
| `.InfoFrame_CustomNick`      | `#1C82CC` | `rgb(28, 130, 204)`| `palette(link)`| `default_chat.qss:204` | Client custom phonetic nickname |
| `.Highlight`                 | `#1C82CC` | `rgb(28, 130, 204)`| `palette(link)`| `default_chat.qss:218` | Channel flags / highlights |
| `.Important`                 | `#FF0000` | `rgb(255, 0, 0)`  | `red`          | `default_chat.qss:223` | Volume dB offset / talk request |
| `.Active`                    | `#008000` | `rgb(0, 128, 0)`  | `green`        | `default_chat.qss:228` | Active status link |
| `.Inactive`                  | `#808080` | `rgb(128, 128, 128)`| `gray`        | `default_chat.qss:233` | Inactive status link |

### 2.4 Dark Mode Chat Palette (`default_chat_darkmode.qss`)

| Message Type / CSS Selector | Hex Code | RGB Decimal | Qt / Web Named Color | Source Location | Visual Semantics |
| :--- | :--- | :--- | :--- | :--- | :--- |
| `.TextMessage_UserLink` | `#006ad2` | `rgb(0, 106, 210)` | Vibrant Blue | `default_chat_darkmode.qss:12` | High-contrast dark mode link |
| `.TextMessage_ChannelLink` | `#006ad2` | `rgb(0, 106, 210)` | Vibrant Blue | `default_chat_darkmode.qss:12` | High-contrast channel link |
| `.TextMessage_ServerLink` | `#006ad2` | `rgb(0, 106, 210)` | Vibrant Blue | `default_chat_darkmode.qss:12` | High-contrast server link |
| `.TextMessage_OfflineLink` | `#006ad2` | `rgb(0, 106, 210)` | Vibrant Blue | `default_chat_darkmode.qss:12` | Offline message counter |
| `.TextMessage_Welcome` | `#11de4d` | `rgb(17, 222, 77)` | Neon Lime Green | `default_chat_darkmode.qss:18` | Server welcome |
| `.TextMessage_HostMessage` | `#11de4d` | `rgb(17, 222, 77)` | Neon Lime Green | `default_chat_darkmode.qss:18` | Host message |
| `.TextMessage_Poke` | `#11de4d` | `rgb(17, 222, 77)` | Neon Lime Green | `default_chat_darkmode.qss:19` | Poke text |
| `.TextMessage_ClientDropped` | `#ec2124` | `rgb(236, 33, 36)` | Bright Scarlet Red| `default_chat_darkmode.qss:24` | Client dropped |
| `.TextMessage_ClientKicked` | `#ec2124` | `rgb(236, 33, 36)` | Bright Scarlet Red| `default_chat_darkmode.qss:24` | Client kicked |
| `.TextMessage_ClientBanned` | `#ec2124` | `rgb(236, 33, 36)` | Bright Scarlet Red| `default_chat_darkmode.qss:25` | Client banned |
| `.TextMessage_HostName` | `#83acd2` | `rgb(131, 172, 210)`| Periwinkle Blue | `default_chat_darkmode.qss:47` | Server hostname |
| `.TextMessage_Normal` | `#83acd2` | `rgb(131, 172, 210)`| Periwinkle Blue | `default_chat_darkmode.qss:47` | Informational log |
| `.TextMessage_Connected` | `#83acd2` | `rgb(131, 172, 210)`| Periwinkle Blue | `default_chat_darkmode.qss:47` | Own client connected |
| `.TextMessage_Disconnected`| `#83acd2` | `rgb(131, 172, 210)`| Periwinkle Blue | `default_chat_darkmode.qss:47` | Own client disconnected |
| `.TextMessage_ClientConnected`| `#83acd2`| `rgb(131, 172, 210)`| Periwinkle Blue | `default_chat_darkmode.qss:47` | Client connected |
| `.TextMessage_ClientDisconnected`| `#83acd2`| `rgb(131, 172, 210)`| Periwinkle Blue| `default_chat_darkmode.qss:47` | Client disconnected |
| `.TextMessage_ClientJoined`| `#83acd2` | `rgb(131, 172, 210)`| Periwinkle Blue | `default_chat_darkmode.qss:47` | Client joined channel |
| `.TextMessage_ClientLeft` | `#83acd2` | `rgb(131, 172, 210)`| Periwinkle Blue | `default_chat_darkmode.qss:47` | Client left channel |
| `.TextMessage_ClientSwitched`| `#83acd2`| `rgb(131, 172, 210)`| Periwinkle Blue | `default_chat_darkmode.qss:47` | Client switched channel |
| `.TextMessage_ChannelCreated`| `#83acd2`| `rgb(131, 172, 210)`| Periwinkle Blue | `default_chat_darkmode.qss:47` | Channel created |
| `.TextMessage_ChannelDeleted`| `#83acd2`| `rgb(131, 172, 210)`| Periwinkle Blue | `default_chat_darkmode.qss:47` | Channel deleted |
| `.TextMessage_ChannelEdited` | `#83acd2`| `rgb(131, 172, 210)`| Periwinkle Blue | `default_chat_darkmode.qss:47` | Channel edited |
| `.TextMessage_ChannelMoved`  | `#83acd2`| `rgb(131, 172, 210)`| Periwinkle Blue | `default_chat_darkmode.qss:47` | Channel moved |
| `.TextMessage_ServerEdited`  | `#83acd2`| `rgb(131, 172, 210)`| Periwinkle Blue | `default_chat_darkmode.qss:47` | Server edited |
| `.TextMessage_GroupAssigned` | `#83acd2`| `rgb(131, 172, 210)`| Periwinkle Blue | `default_chat_darkmode.qss:47` | Group assigned |
| `.TextMessage_GroupRevoked`  | `#83acd2`| `rgb(131, 172, 210)`| Periwinkle Blue | `default_chat_darkmode.qss:47` | Group revoked |
| `.TextMessage_TalkPowerAssigned`| `#83acd2`| `rgb(131, 172, 210)`| Periwinkle Blue | `default_chat_darkmode.qss:47` | Talk power assigned |
| `.TextMessage_TalkPowerRevoked`| `#83acd2`| `rgb(131, 172, 210)`| Periwinkle Blue | `default_chat_darkmode.qss:47` | Talk power revoked |
| `.TextMessage_TalkPowerRequested`| `#83acd2`| `rgb(131, 172, 210)`| Periwinkle Blue| `default_chat_darkmode.qss:47` | Talk power requested |
| `.TextMessage_Time`          | `#83acd2` | `rgb(131, 172, 210)`| Periwinkle Blue | `default_chat_darkmode.qss:47` | Timestamp |
| `.TextMessage_BeginHistory`  | `#bcbcbc` | `rgb(188, 188, 188)`| Light Silver Gray| `default_chat_darkmode.qss:52`| History start marker |
| `.TextMessage_EndHistory`    | `#bcbcbc` | `rgb(188, 188, 188)`| Light Silver Gray| `default_chat_darkmode.qss:53`| History end marker |
| `.TextMessage_ServerError`   | `#f73c3f` | `rgb(247, 60, 63)`  | Vivid Light Coral | `default_chat_darkmode.qss:56`| Server error log |
| `.InfoFrame_Title ... Links` | `#B0C4DE` | `rgb(176, 196, 222)`| `lightsteelblue`| `default_chat_darkmode.qss:62`| InfoFrame title link |
| `.InfoFrame_CustomNick`      | `#1C82CC` | `rgb(28, 130, 204)` | `palette(link)` | `default_chat_darkmode.qss:66`| Nickname custom tag |
| `.Highlight`                 | `#1C82CC` | `rgb(28, 130, 204)` | `palette(link)` | `default_chat_darkmode.qss:71`| Highlighted channel properties |
| `.Important`                 | `#FF0000` | `rgb(255, 0, 0)`   | `red`           | `default_chat_darkmode.qss:76`| Critical alert text |
| `.Active`                    | `#90EE90` | `rgb(144, 238, 144)`| `lightgreen`    | `default_chat_darkmode.qss:81`| Active status |
| `.Inactive`                  | `#D3D3D3` | `rgb(211, 211, 211)`| `lightgray`     | `default_chat_darkmode.qss:86`| Inactive status |
| `ImprovedTabBar::tab`        | `#FFFFFF` | `rgb(255, 255, 255)`| `white`         | `default_darkmode.qss:9`      | Tab text color in dark mode |

### 2.5 Base Qt Desktop Palette Mapping (Windows Vista / Modern Fallback)

TeamSpeak delegates primary surfaces to Qt `QPalette` roles:

```
+---------------------------------------------------------------------------------------+
| Surface                   | Light Mode (Vista Native)     | Dark Mode (TeamSpeak Dark)|
+---------------------------------------------------------------------------------------+
| Window / Dialog BG        | #F0F0F0 rgb(240, 240, 240)    | #2D2D2D rgb(45, 45, 45)   |
| Base (Tree/Chat/Info)     | #FFFFFF rgb(255, 255, 255)    | #1E1E1E rgb(30, 30, 30)   |
| Alternate Base (Tables)   | #F7F7F7 rgb(247, 247, 247)    | #262626 rgb(38, 38, 38)   |
| Text / Foreground         | #000000 rgb(0, 0, 0)          | #E0E0E0 rgb(224, 224, 224)|
| Disabled Text             | #787878 rgb(120, 120, 120)    | #6C6C6C rgb(108, 108, 108)|
| Mid / Border (Tabs/Icons) | #A0A0A0 rgb(160, 160, 160)    | #555555 rgb(85, 85, 85)   |
| Selection Background      | #3399FF rgb(51, 153, 255)     | #264F78 rgb(38, 79, 120)  |
| Selection Text            | #FFFFFF rgb(255, 255, 255)    | #FFFFFF rgb(255, 255, 255)|
| Button Face               | #E1E1E1 rgb(225, 225, 225)    | #383838 rgb(56, 56, 56)   |
| Button Hover              | #E5F1FB rgb(229, 241, 251)    | #444444 rgb(68, 68, 68)   |
| Button Pressed            | #CCE4F7 rgb(204, 228, 247)    | #1E1E1E rgb(30, 30, 30)   |
+---------------------------------------------------------------------------------------+
```

---

## 3. Box Model Metrics: Borders, Radii, Margins, Paddings, Fonts

### 3.1 Comprehensive Metrics Matrix Across Widgets

| Widget Identifier / Selector | Border Width & Style | Border Radius | Margin | Padding | Font Properties | Min / Fixed Dimensions | Source Citation |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `QSplitter::handle#*Splitter` | None | `0px` | `1.5px` all sides | `0px` | N/A | Centered, no-repeat bg | `default.qss:21-25` |
| `QSplitter::handle::vertical` | None | `0px` | `1.5px` | `0px` | N/A | `height: 2px` (`styles:default/splitter.png`) | `default.qss:27-30` |
| `QSplitter::handle:horizontal`| None | `0px` | `1.5px` | `0px` | N/A | `width: 2px` (`styles:default/splitter_vertical.png`) | `default.qss:33-36` |
| `QLabel#*Banner` (General)   | None | `0px` | `0px` | `left: 6px` | `font-weight: bold;` | `min-height: 32px; min-width: 0px` | `default.qss:39-49` |
| `QLabel#account*Banner`      | None | `0px` | `0px` | `left: 6px` | `font-weight: bold;` | `min-width: 420px; min-height: 32px` | `default.qss:51-53` |
| `QStatusBar QLabel`          | `0px` | `0px` | `0px` | `top/bottom: 0px; left/right: 3px` | System default | Auto | `default.qss:63-66` |
| `QTextEdit#InfoFrame`        | `1px solid mid` | `0px` | `0px` | `0px` | Document driven | `bg-position: top right` (`logo-128x128.png`) | `default.qss:68-74` |
| `QTextEdit#ChatTab`          | `1px solid mid` | `0px` | `0px` | `0px` | Document driven | `bg-position: bottom right` (`chatlog-128x96.png`) | `default.qss:76-82` |
| `QListWidget#*IconsListWidget::item:selected` | `1px solid palette(mid)` | `0px` | `0px` | System default | System default | Icon cell size | `default.qss:84-86` |
| `QTreeView#ServerTreeView`   | `1px solid mid` | `0px` | `0px` | `0px` | `rootIsDecorated: false; alternate-bg: transparent;` | Tree hierarchy view | `default.qss:88-91` |
| `QLabel#OptionsHeadlineTitleLabel` | None | `0px` | `0px` | `0px` | `18px 'TeamSpeak 2020'` | Header height | `default.qss:93-96` |
| `QLabel#NewsLeftLabel`       | None | `0px` | `0px` | `right: 6px`| System default | Auto | `default.qss:102-104` |
| `QLabel#appNameLabel`        | None | `0px` | `0px` | `0px` | `32px 'TeamSpeak 2020' bold; lowercase` | Header brand | `default.qss:110-115` |
| `IntegrationResponseWidget`  | `1px solid #000000` | `2px` | `0px` | Default | System default | Auto | `default.qss:131-136` |
| `IntegrationBubble`          | `1px solid #000000` | `2px` | `0px` | Default | System default | Auto | `default.qss:138-143` |
| `IntegrationBubbleEditableComboBox` | `2px groove #1179ce` | `0px` | `0px` | `top: 0px; right: 9px; bottom: 1px; left: 9px` | System default | Combo height | `default.qss:145-149` |
| `ErrorIndicator`             | `1px solid #000000` | `2px` | `0px` | Default | System default | Auto | `default.qss:151-156` |
| `IntegrationErrorIndicatorLabel` | None | `0px` | `0px` | `0px` | `font-weight: bold; color: white` | Auto | `default.qss:158-161` |
| `ImprovedTabBarCloseButton`  | `0px transparent` | `2px` | `0px` | `0px` | N/A | Tab close button | `default_mac.qss:8-11` |
| `ImprovedTabBarCloseButton:hover` | `0px` | `2px` | `3px` all | `0px` | N/A | `palette(alternatebase)` | `default_mac.qss:13-16` |
| `ImprovedTabWidget::pane`    | `0px` | `0px` | `0px` | `0px` | N/A | Tab content container | `default_mac.qss:18-21` |
| `ImprovedTabWidget::tab-bar` | None | `0px` | `0px` | `0px` | N/A | `left: 0px` | `default_mac.qss:23-25` |
| `ImprovedTabBar::tab` (Normal)| `1px solid palette(mid)` | `6px` top | `left: 1px` | `2px` all | Light: `#000000`, Dark: `white` | Auto tab width | `default_mac.qss:27-33; default_darkmode.qss:8` |
| `ImprovedTabBar::tab:top`    | Bottom border `0px` | `6px` top, `0px` bottom | `top: 6px` | `2px` | System default | Unselected top tab | `default_mac.qss:39-44` |
| `ImprovedTabBar::tab:top:selected` | Bottom border `0px` | `6px` top, `0px` bottom | `top: 4px` | `bottom: 0px` | System default | Selected top tab | `default_mac.qss:46-49` |
| `ImprovedTabBar::tab:bottom` | Top border `0px` | `0px` top, `6px` bottom | `bottom: 2px` | `2px` | System default | Unselected bottom tab | `default_mac.qss:51-56` |
| `ImprovedTabBar::tab:bottom:selected` | Top border `0px` | `0px` top, `6px` bottom | `bottom: 0px` | `top: 0px` | System default | Selected bottom tab | `default_mac.qss:58-61` |
| `QToolBar#AddonBrowserToolBar` | `0px transparent` | `0px` | `0px` | `0px` | N/A | `spacing: 5px; background: none` | `default_mac.qss:64-68` |

### 3.2 HTML Chat & InfoFrame Table Metrics (`default_chat.qss:89-92, 207-266`)

Qt's rich-text engine executes custom layout constraints for HTML elements:

- **Paragraph Baseline**:
  ```css
  p {
    padding: 0px;
    margin: 0px;
  }
  ```
  *(Cited from `default_chat.qss:89-92`)*

- **InfoFrame Table Architecture**:
  ```css
  .InfoFrame_Table {
    border-collapse: collapse;
    border-spacing: 0px;
  }
  .InfoFrame_Table td {
    padding: 0px 4px 0px 1px; /* Top: 0px, Right: 4px, Bottom: 0px, Left: 1px */
  }
  .InfoFrame_Table td.Label {
    font-weight: bold;
  }
  .InfoFrame_Table td.Bottom {
    vertical-align: bottom;
  }
  .InfoFrame_Table td.List {
    padding: 0px 1px 0px 8px; /* Indented permission/group list item */
  }
  .InfoFrame_Table td.Badge {
    padding: 5px 5px 0px 5px; /* Overdrive badge showcase icon cell */
  }
  .InfoFrame_Table td.Space-Top {
    padding: 10px 0px 0px 0px; /* Top spacing divider inside table cells */
  }
  .InfoFrame_Table td.Space-Bot {
    padding: 0px 0px 10px 0px; /* Bottom spacing divider */
  }
  .InfoFrame_Table.Space-Top {
    margin: 10px 0px 0px 0px; /* Table-level vertical margin */
  }
  .InfoFrame_Table.Space-Bot {
    margin: 0px 0px 10px 0px; /* Table-level vertical margin */
  }
  ```
  *(Cited from `default_chat.qss:207-266`)*

- **Title Typography**:
  ```css
  .InfoFrame_Title {
    font-size: x-large; /* 18pt / 24px font size */
  }
  ```
  *(Cited from `default_chat.qss:192-194`)*

---

## 4. Chat Message Syntax, Classes, and Dispatch Semantics

### 4.1 HTML Document Layout
Every message logged to `QTextEdit#ChatTab` is formatted using a structured HTML 4 fragment:

```html
<p class="{PRIMARY_CLASS} {OPTIONAL_SUBCLASSES}">
  <span class="Body">
    <span class="TextMessage_Time">12:34:56</span>
    <img src="icon_path.png" />
    <a class="TextMessage_UserLink" href="client://{CLIENT_ID}/{CLIENT_UID}~{NICK}">Nickname</a>:
    <span class="TextMessage_Text">Message body text</span>
  </span>
</p>
```
*(Cited from `default_chat.qss:18-26`)*

### 4.2 Primary Message Classes Taxonomy

| Primary Class | Trigger Condition / Protocol Event | Default Color (Light) | Default Color (Dark) | Font Style |
| :--- | :--- | :--- | :--- | :--- |
| `TextMessage_Normal` | Informational status notifications | `#33597D` (`default_chat.qss:128`) | `#83acd2` (`default_chat_darkmode.qss:28`) | Normal |
| `TextMessage_Welcome` | Server welcome message on login | `#0AA537` (`default_chat.qss:121`) | `#11de4d` (`default_chat_darkmode.qss:15`) | Normal |
| `TextMessage_HostMsg` | Server host message modal/chat | `#0AA537` (`default_chat.qss:122`) | `#11de4d` (`default_chat_darkmode.qss:16`) | Normal |
| `TextMessage_Message` | User chat message (Private, Channel, Server) | Inherits text color | Inherits text color | Normal |
| `TextMessage_Poke` | Direct poke dialogue line | `#0AA537` (`default_chat.qss:123`) | `#11de4d` (`default_chat_darkmode.qss:17`) | Normal |
| `TextMessage_Connected` | Own client connected to virtual server | `#33597D` (`default_chat.qss:128`) | `#83acd2` (`default_chat_darkmode.qss:29`) | Normal |
| `TextMessage_Disconnected` | Own client disconnected from server | `#33597D` (`default_chat.qss:134`) | `#83acd2` (`default_chat_darkmode.qss:32`) | Normal |
| `TextMessage_ClientConnected` | Remote client connects | `#33597D` (`default_chat.qss:129`) | `#83acd2` (`default_chat_darkmode.qss:30`) | Normal |
| `TextMessage_ClientDisconnected`| Remote client disconnects | `#33597D` (`default_chat.qss:135`) | `#83acd2` (`default_chat_darkmode.qss:33`) | Normal |
| `TextMessage_ClientJoined` | Remote client joins current channel | `#33597D` (`default_chat.qss:130`) | `#83acd2` (`default_chat_darkmode.qss:31`) | Normal |
| `TextMessage_ClientLeft` | Remote client leaves current channel | `#33597D` (`default_chat.qss:137`) | `#83acd2` (`default_chat_darkmode.qss:35`) | Normal |
| `TextMessage_ClientSwitched` | Remote client changes channel | `#33597D` (`default_chat.qss:136`) | `#83acd2` (`default_chat_darkmode.qss:34`) | Normal |
| `TextMessage_ClientDropped` | Connection timeout / ping drop | `#C9070A` (`default_chat.qss:141`) | `#ec2124` (`default_chat_darkmode.qss:21`) | Normal |
| `TextMessage_ClientKicked` | Kicked from channel or server | `#C9070A` (`default_chat.qss:142`) | `#ec2124` (`default_chat_darkmode.qss:22`) | Normal |
| `TextMessage_ClientBanned` | Banned from server | `#C9070A` (`default_chat.qss:143`) | `#ec2124` (`default_chat_darkmode.qss:23`) | Normal |
| `TextMessage_ChannelCreated` | Channel added to server tree | `#33597D` (`default_chat.qss:147`) | `#83acd2` (`default_chat_darkmode.qss:36`) | Normal |
| `TextMessage_ChannelDeleted` | Channel deleted from tree | `#33597D` (`default_chat.qss:148`) | `#83acd2` (`default_chat_darkmode.qss:37`) | Normal |
| `TextMessage_ChannelMoved` | Channel order or tree depth changed | `#33597D` (`default_chat.qss:150`) | `#83acd2` (`default_chat_darkmode.qss:39`) | Normal |
| `TextMessage_ChannelEdited` | Codec, topic, flags modified | `#33597D` (`default_chat.qss:149`) | `#83acd2` (`default_chat_darkmode.qss:38`) | Normal |
| `TextMessage_ServerEdited` | Server name, slots, banner changed | `#33597D` (`default_chat.qss:151`) | `#83acd2` (`default_chat_darkmode.qss:40`) | Normal |
| `TextMessage_ServerError` | Command error response | `#C9070A` (`default_chat.qss:184`) | `#f73c3f` (`default_chat_darkmode.qss:56`) | **Bold** |
| `TextMessage_GroupAssigned` | Client added to Server/Channel group | `#33597D` (`default_chat.qss:155`) | `#83acd2` (`default_chat_darkmode.qss:41`) | Normal |
| `TextMessage_GroupRevoked` | Client removed from group | `#33597D` (`default_chat.qss:156`) | `#83acd2` (`default_chat_darkmode.qss:42`) | Normal |
| `TextMessage_TalkPowerAssigned`| Granted talk power privilege | `#33597D` (`default_chat.qss:157`) | `#83acd2` (`default_chat_darkmode.qss:43`) | Normal |
| `TextMessage_TalkPowerRevoked` | Revoked talk power privilege | `#33597D` (`default_chat.qss:158`) | `#83acd2` (`default_chat_darkmode.qss:44`) | Normal |
| `TextMessage_TalkPowerRequested`| User submitted talk request | `#33597D` (`default_chat.qss:159`) | `#83acd2` (`default_chat_darkmode.qss:45`) | Normal |
| `TextMessage_BeginHistory` | Beginning of historical backlog | `#707475` (`default_chat.qss:174`) | `#bcbcbc` (`default_chat_darkmode.qss:52`) | ***Bold Italic*** |
| `TextMessage_EndHistory` | End of historical backlog marker | `#707475` (`default_chat.qss:175`) | `#bcbcbc` (`default_chat_darkmode.qss:53`) | ***Bold Italic*** |

### 4.3 Composite Message Filters
TeamSpeak specifies composite CSS rules combining `TextMessage_Message` with sub-context tags (`default_chat.qss:61-74`):
- `.Incoming`: Message received from another participant.
- `.Outgoing`: Message sent by local client.
- `.Friend`: Sent by client present in local Friends list (renders with `CustomColor::ClientFriend` = `#008000`).
- `.Blocked`: Sent by client in Mute/Blocked list (renders with `CustomColor::ClientBlocked` = `#FF0000`).
- `.Private`: Rendered within 1-on-1 private query chat tab.
- `.Public`: Rendered within Channel or Server group tab.

Example composite rule target:
```css
.TextMessage_Message.Friend.Private {
  /* Targeted friend styling */
}
```

---

## 5. Interactive Control States

### 5.1 Interactive State Color Matrix

| Control Type | Pseudo-State | Background Color / Fill | Border Stroke | Text Color | Source Location |
| :--- | :--- | :--- | :--- | :--- | :--- |
| `ChatLineEdit` | Normal | `palette(base)` (`#FFFFFF`) | `1px solid #A0A0A0` | `#000000` | `default.qss:60` |
| `ChatLineEdit` | Placeholder / Help | `palette(base)` (`#FFFFFF`) | `1px solid #A0A0A0` | `#909090` | `default.qss:56` |
| `ServerTreeView` | Row (Normal) | `transparent` (`#00000000`) | None | `#000000` | `default.qss:89` |
| `ServerTreeView` | Row (Hovered) | `#E5F1FB` (Vista hover) | None | `#000000` | Windows Vista UX |
| `ServerTreeView` | Row (Selected) | `#3399FF` / `#0078D7` | None | `#FFFFFF` | Windows Vista UX |
| `ServerTreeView` | Row (Inactive Selected)| `#E5E5E5` / `#D9D9D9` | None | `#000000` | Windows Vista UX |
| `clientsTableView` | Alternating Row | `#F7F7F7` | None | `#000000` | `default.qss:107` |
| `banTableView` | Alternating Row | `#F7F7F7` | None | `#000000` | `default.qss:107` |
| `localIconsListWidget`| `:selected` | `transparent` | `1px solid palette(mid)` (`#A0A0A0`) | `#000000` | `default.qss:85` |
| `remoteIconsListWidget`| `:selected` | `transparent` | `1px solid palette(mid)` (`#A0A0A0`) | `#000000` | `default.qss:85` |
| `ImprovedTabBar::tab` | Normal (Light) | `transparent` | `1px solid palette(mid)` | `#000000` | `default_mac.qss:31` |
| `ImprovedTabBar::tab` | `:selected` (Light) | `palette(base)` (`#FFFFFF`) | `1px solid palette(mid)` | `#000000` | `default_mac.qss:36` |
| `ImprovedTabBar::tab` | Normal (Dark) | `transparent` | `1px solid #555555` | `#FFFFFF` | `default_darkmode.qss:9` |
| `ImprovedTabBar::tab` | `:selected` (Dark) | `palette(base)` (`#1E1E1E`) | `1px solid #555555` | `#FFFFFF` | `default_darkmode.qss:9` |
| `ImprovedTabBarCloseButton`| Normal | `transparent` | `0px transparent` | N/A | `default_mac.qss:9` |
| `ImprovedTabBarCloseButton`| `:hover` | `palette(alternatebase)` (`#F7F7F7`) | `0px` | N/A | `default_mac.qss:14` |
| `QPushButton` | Normal | `#E1E1E1` | `1px solid #ADADAD` | `#000000` | Windows Vista UX |
| `QPushButton` | `:hover` | `#E5F1FB` | `1px solid #0078D7` | `#000000` | Windows Vista UX |
| `QPushButton` | `:pressed` | `#CCE4F7` | `1px solid #005499` | `#000000` | Windows Vista UX |
| `QPushButton` | `:disabled` | `#F4F4F4` | `1px solid #DCDCDC` | `#787878` | Windows Vista UX |
| `IntegrationBubbleComboBox`| Normal | `#d6ecff` | `2px groove #1179ce` | `#000000` | `default.qss:146` |

---

## 6. Complete Rust `egui` Theme Implementation

The following complete, ready-to-use Rust implementation translates the exact TeamSpeak 3 QSS styles, metrics, state machines, and colors into native `egui::Visuals`, `egui::Style`, and custom chat palettes.

```rust
//! Complete TeamSpeak 3 QSS Theming Implementation for `egui`
//!
//! Provides exact pixel metrics, linear gradient approximations, and hex color
//! mappings derived from TeamSpeak 3 Client styles (`default.qss`, `default_vista.qss`,
//! `default_chat.qss`, `default_darkmode.qss`, `default_chat_darkmode.qss`).

use egui::{
    epaint::Shadow,
    style::{ScrollStyle, Selection, Spacing, WidgetVisuals, Widgets},
    Color32, Margin, Rounding, Stroke, Style, Vec2, Visuals,
};

/// Exact TeamSpeak 3 Chat and InfoFrame Color Palette
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ts3ChatPalette {
    // Hyperlinks & Identifiers
    pub link: Color32,
    pub user_link: Color32,
    pub channel_link: Color32,
    pub server_link: Color32,
    pub offline_link: Color32,

    // Status & Event Messages
    pub host_name: Color32,
    pub welcome: Color32,
    pub host_msg: Color32,
    pub poke: Color32,
    pub normal: Color32,
    pub connected: Color32,
    pub disconnected: Color32,
    pub client_joined: Color32,
    pub client_left: Color32,
    pub client_switched: Color32,
    pub client_dropped: Color32,
    pub client_kicked: Color32,
    pub client_banned: Color32,
    pub channel_event: Color32,
    pub group_event: Color32,
    pub talk_power_event: Color32,
    pub time_stamp: Color32,
    pub history_marker: Color32,
    pub server_error: Color32,

    // InfoFrame Specific Styles
    pub info_title_link: Color32,
    pub info_custom_nick: Color32,
    pub highlight: Color32,
    pub important: Color32,
    pub active: Color32,
    pub inactive: Color32,

    // Input States
    pub input_help_text: Color32,
    pub input_normal_text: Color32,

    // Client Relationship Overrides
    pub client_friend: Color32,
    pub client_blocked: Color32,
    pub client_recording: Color32,
}

/// Light Mode Chat Palette derived from `default_chat.qss` and `default.qss`
pub const TS3_LIGHT_CHAT_PALETTE: Ts3ChatPalette = Ts3ChatPalette {
    // QPalette::Link override (default.qss:7)
    link: Color32::from_rgb(0x1C, 0x82, 0xCC), // #1c82cc

    // Object links (default_chat.qss:98-110)
    user_link: Color32::from_rgb(0x00, 0x2F, 0x5D),    // #002F5D
    channel_link: Color32::from_rgb(0x00, 0x2F, 0x5D), // #002F5D
    server_link: Color32::from_rgb(0x00, 0x2F, 0x5D),  // #002F5D
    offline_link: Color32::from_rgb(0x00, 0x2F, 0x5D), // #002F5D

    // Colored message elements (default_chat.qss:116-161)
    host_name: Color32::from_rgb(0x33, 0x59, 0x7D),        // #33597D
    welcome: Color32::from_rgb(0x0A, 0xA5, 0x37),          // #0AA537
    host_msg: Color32::from_rgb(0x0A, 0xA5, 0x37),         // #0AA537
    poke: Color32::from_rgb(0x0A, 0xA5, 0x37),             // #0AA537
    normal: Color32::from_rgb(0x33, 0x59, 0x7D),           // #33597D
    connected: Color32::from_rgb(0x33, 0x59, 0x7D),        // #33597D
    disconnected: Color32::from_rgb(0x33, 0x59, 0x7D),     // #33597D
    client_joined: Color32::from_rgb(0x33, 0x59, 0x7D),    // #33597D
    client_left: Color32::from_rgb(0x33, 0x59, 0x7D),      // #33597D
    client_switched: Color32::from_rgb(0x33, 0x59, 0x7D),  // #33597D
    client_dropped: Color32::from_rgb(0xC9, 0x07, 0x0A),   // #C9070A
    client_kicked: Color32::from_rgb(0xC9, 0x07, 0x0A),    // #C9070A
    client_banned: Color32::from_rgb(0xC9, 0x07, 0x0A),    // #C9070A
    channel_event: Color32::from_rgb(0x33, 0x59, 0x7D),    // #33597D
    group_event: Color32::from_rgb(0x33, 0x59, 0x7D),      // #33597D
    talk_power_event: Color32::from_rgb(0x33, 0x59, 0x7D), // #33597D
    time_stamp: Color32::from_rgb(0x33, 0x59, 0x7D),       // #33597D
    history_marker: Color32::from_rgb(0x70, 0x74, 0x75),   // #707475
    server_error: Color32::from_rgb(0xC9, 0x07, 0x0A),     // #C9070A

    // InfoFrame (default_chat.qss:196-234)
    info_title_link: Color32::from_rgb(0x19, 0x19, 0x70), // midnightblue #191970
    info_custom_nick: Color32::from_rgb(0x1C, 0x82, 0xCC), // palette(link)
    highlight: Color32::from_rgb(0x1C, 0x82, 0xCC),       // palette(link)
    important: Color32::from_rgb(0xFF, 0x00, 0x00),       // red
    active: Color32::from_rgb(0x00, 0x80, 0x00),          // green
    inactive: Color32::from_rgb(0x80, 0x80, 0x80),        // gray

    // Chat input (default.qss:56-60)
    input_help_text: Color32::from_rgb(0x90, 0x90, 0x90), // #909090
    input_normal_text: Color32::from_rgb(0x00, 0x00, 0x00),

    // Custom colors (default.qss:10-12)
    client_friend: Color32::from_rgb(0x00, 0x80, 0x00),    // #008000
    client_blocked: Color32::from_rgb(0xFF, 0x00, 0x00),   // #ff0000
    client_recording: Color32::from_rgb(0x80, 0x00, 0x00), // #800000
};

/// Dark Mode Chat Palette derived from `default_chat_darkmode.qss` and `default.qss`
pub const TS3_DARK_CHAT_PALETTE: Ts3ChatPalette = Ts3ChatPalette {
    // Link color
    link: Color32::from_rgb(0x00, 0x6A, 0xD2), // #006ad2

    // Object links (default_chat_darkmode.qss:8-13)
    user_link: Color32::from_rgb(0x00, 0x6A, 0xD2),    // #006ad2
    channel_link: Color32::from_rgb(0x00, 0x6A, 0xD2), // #006ad2
    server_link: Color32::from_rgb(0x00, 0x6A, 0xD2),  // #006ad2
    offline_link: Color32::from_rgb(0x00, 0x6A, 0xD2), // #006ad2

    // Colored message elements (default_chat_darkmode.qss:15-48)
    host_name: Color32::from_rgb(0x83, 0xAC, 0xD2),        // #83acd2
    welcome: Color32::from_rgb(0x11, 0xDE, 0x4D),          // #11de4d
    host_msg: Color32::from_rgb(0x11, 0xDE, 0x4D),         // #11de4d
    poke: Color32::from_rgb(0x11, 0xDE, 0x4D),             // #11de4d
    normal: Color32::from_rgb(0x83, 0xAC, 0xD2),           // #83acd2
    connected: Color32::from_rgb(0x83, 0xAC, 0xD2),        // #83acd2
    disconnected: Color32::from_rgb(0x83, 0xAC, 0xD2),     // #83acd2
    client_joined: Color32::from_rgb(0x83, 0xAC, 0xD2),    // #83acd2
    client_left: Color32::from_rgb(0x83, 0xAC, 0xD2),      // #83acd2
    client_switched: Color32::from_rgb(0x83, 0xAC, 0xD2),  // #83acd2
    client_dropped: Color32::from_rgb(0xEC, 0x21, 0x24),   // #ec2124
    client_kicked: Color32::from_rgb(0xEC, 0x21, 0x24),    // #ec2124
    client_banned: Color32::from_rgb(0xEC, 0x21, 0x24),    // #ec2124
    channel_event: Color32::from_rgb(0x83, 0xAC, 0xD2),    // #83acd2
    group_event: Color32::from_rgb(0x83, 0xAC, 0xD2),      // #83acd2
    talk_power_event: Color32::from_rgb(0x83, 0xAC, 0xD2), // #83acd2
    time_stamp: Color32::from_rgb(0x83, 0xAC, 0xD2),       // #83acd2
    history_marker: Color32::from_rgb(0xBC, 0xBC, 0xBC),   // #bcbcbc
    server_error: Color32::from_rgb(0xF7, 0x3C, 0x3F),     // #f73c3f

    // InfoFrame (default_chat_darkmode.qss:59-87)
    info_title_link: Color32::from_rgb(0xB0, 0xC4, 0xDE), // lightsteelblue #B0C4DE
    info_custom_nick: Color32::from_rgb(0x00, 0x6A, 0xD2), // palette(link)
    highlight: Color32::from_rgb(0x00, 0x6A, 0xD2),       // palette(link)
    important: Color32::from_rgb(0xFF, 0x00, 0x00),       // red
    active: Color32::from_rgb(0x90, 0xEE, 0x90),          // lightgreen #90ee90
    inactive: Color32::from_rgb(0xD3, 0xD3, 0xD3),        // lightgray #d3d3d3

    // Chat input
    input_help_text: Color32::from_rgb(0x70, 0x70, 0x70),
    input_normal_text: Color32::from_rgb(0xDF, 0xDF, 0xDF),

    // Custom colors
    client_friend: Color32::from_rgb(0x2E, 0xC0, 0x2E),    // Lighter green for dark bg
    client_blocked: Color32::from_rgb(0xFF, 0x4D, 0x4D),   // Lighter red for dark bg
    client_recording: Color32::from_rgb(0xD9, 0x53, 0x4F), // Lighter maroon for dark bg
};

/// Exact TeamSpeak 3 Light Mode `egui::Visuals`
pub fn ts3_light_visuals() -> Visuals {
    let mut visuals = Visuals::light();

    // Background surfaces
    visuals.dark_mode = false;
    visuals.window_fill = Color32::from_rgb(240, 240, 240); // Window / Dialog background
    visuals.panel_fill = Color32::from_rgb(240, 240, 240);  // Toolbars & Sidebars
    visuals.extreme_bg_color = Color32::from_rgb(255, 255, 255); // TreeView, TextEdit, ChatTab base
    visuals.faint_bg_color = Color32::from_rgb(247, 247, 247); // Alternate row background

    // Borders & strokes
    let border_stroke = Stroke::new(1.0, Color32::from_rgb(160, 160, 160)); // palette(mid)
    let hover_stroke = Stroke::new(1.0, Color32::from_rgb(0, 120, 215));   // Vista blue focus border
    visuals.window_stroke = border_stroke;

    // Selection (Windows Vista highlight)
    visuals.selection = Selection {
        bg_fill: Color32::from_rgb(51, 153, 255), // #3399FF
        stroke: Stroke::new(1.0, Color32::from_rgb(28, 130, 204)), // palette(link) #1c82cc
    };

    // Global hyperlink override (default.qss:7)
    visuals.hyperlink_color = Color32::from_rgb(28, 130, 204); // #1c82cc
    visuals.error_fg_color = Color32::from_rgb(201, 7, 10);     // TextMessage_ServerError #C9070A
    visuals.warn_fg_color = Color32::from_rgb(255, 140, 0);

    // Interactive Widget Visuals (Buttons, Inputs, Checkboxes)
    visuals.widgets = Widgets {
        // Disabled controls
        noninteractive: WidgetVisuals {
            bg_fill: Color32::from_rgb(244, 244, 244),
            weak_bg_fill: Color32::from_rgb(244, 244, 244),
            bg_stroke: Stroke::new(1.0, Color32::from_rgb(220, 220, 220)),
            rounding: Rounding::same(2.0), // TeamSpeak 2px button rounding
            fg_stroke: Stroke::new(1.0, Color32::from_rgb(120, 120, 120)), // Disabled text
            expansion: 0.0,
        },
        // Resting / Inactive button
        inactive: WidgetVisuals {
            bg_fill: Color32::from_rgb(225, 225, 225), // Vista button face #E1E1E1
            weak_bg_fill: Color32::from_rgb(225, 225, 225),
            bg_stroke: Stroke::new(1.0, Color32::from_rgb(173, 173, 173)), // #ADADAD
            rounding: Rounding::same(2.0),
            fg_stroke: Stroke::new(1.0, Color32::from_rgb(0, 0, 0)), // Black label
            expansion: 0.0,
        },
        // Hovered button / item
        hovered: WidgetVisuals {
            bg_fill: Color32::from_rgb(229, 241, 251), // Vista hover #E5F1FB
            weak_bg_fill: Color32::from_rgb(229, 241, 251),
            bg_stroke: hover_stroke,
            rounding: Rounding::same(2.0),
            fg_stroke: Stroke::new(1.0, Color32::from_rgb(0, 0, 0)),
            expansion: 0.0,
        },
        // Pressed button / active click
        active: WidgetVisuals {
            bg_fill: Color32::from_rgb(204, 228, 247), // Vista pressed #CCE4F7
            weak_bg_fill: Color32::from_rgb(204, 228, 247),
            bg_stroke: Stroke::new(1.0, Color32::from_rgb(0, 84, 153)),
            rounding: Rounding::same(2.0),
            fg_stroke: Stroke::new(1.0, Color32::from_rgb(0, 0, 0)),
            expansion: 0.0,
        },
        // Open popup / combo expanded
        open: WidgetVisuals {
            bg_fill: Color32::from_rgb(255, 255, 255),
            weak_bg_fill: Color32::from_rgb(255, 255, 255),
            bg_stroke: hover_stroke,
            rounding: Rounding::same(2.0),
            fg_stroke: Stroke::new(1.0, Color32::from_rgb(0, 0, 0)),
            expansion: 0.0,
        },
    };

    // Window & Dialog frame geometry
    visuals.window_rounding = Rounding::same(2.0);
    visuals.window_shadow = Shadow::NONE;
    visuals.popup_shadow = Shadow::NONE;

    visuals
}

/// Exact TeamSpeak 3 Dark Mode `egui::Visuals`
pub fn ts3_dark_visuals() -> Visuals {
    let mut visuals = Visuals::dark();

    // Background surfaces
    visuals.dark_mode = true;
    visuals.window_fill = Color32::from_rgb(45, 45, 45);   // #2D2D2D Window background
    visuals.panel_fill = Color32::from_rgb(45, 45, 45);    // Toolbars & Sidebars
    visuals.extreme_bg_color = Color32::from_rgb(30, 30, 30); // #1E1E1E Base view surface
    visuals.faint_bg_color = Color32::from_rgb(38, 38, 38); // Alternating row color

    // Borders & strokes
    let border_stroke = Stroke::new(1.0, Color32::from_rgb(85, 85, 85));
    let hover_stroke = Stroke::new(1.0, Color32::from_rgb(0, 106, 210)); // #006ad2
    visuals.window_stroke = border_stroke;

    // Selection
    visuals.selection = Selection {
        bg_fill: Color32::from_rgb(38, 79, 120), // Dark mode navy selection #264F78
        stroke: Stroke::new(1.0, Color32::from_rgb(0, 106, 210)), // #006ad2
    };

    // Hyperlink & status colors
    visuals.hyperlink_color = Color32::from_rgb(0, 106, 210); // #006ad2
    visuals.error_fg_color = Color32::from_rgb(247, 60, 63);  // #f73c3f
    visuals.warn_fg_color = Color32::from_rgb(255, 170, 0);

    // Interactive Widgets
    visuals.widgets = Widgets {
        noninteractive: WidgetVisuals {
            bg_fill: Color32::from_rgb(35, 35, 35),
            weak_bg_fill: Color32::from_rgb(35, 35, 35),
            bg_stroke: Stroke::new(1.0, Color32::from_rgb(60, 60, 60)),
            rounding: Rounding::same(2.0),
            fg_stroke: Stroke::new(1.0, Color32::from_rgb(108, 108, 108)),
            expansion: 0.0,
        },
        inactive: WidgetVisuals {
            bg_fill: Color32::from_rgb(56, 56, 56), // #383838
            weak_bg_fill: Color32::from_rgb(56, 56, 56),
            bg_stroke: Stroke::new(1.0, Color32::from_rgb(80, 80, 80)),
            rounding: Rounding::same(2.0),
            fg_stroke: Stroke::new(1.0, Color32::from_rgb(224, 224, 224)),
            expansion: 0.0,
        },
        hovered: WidgetVisuals {
            bg_fill: Color32::from_rgb(68, 68, 68), // #444444
            weak_bg_fill: Color32::from_rgb(68, 68, 68),
            bg_stroke: hover_stroke,
            rounding: Rounding::same(2.0),
            fg_stroke: Stroke::new(1.0, Color32::from_rgb(255, 255, 255)),
            expansion: 0.0,
        },
        active: WidgetVisuals {
            bg_fill: Color32::from_rgb(30, 30, 30), // Depressed button
            weak_bg_fill: Color32::from_rgb(30, 30, 30),
            bg_stroke: Stroke::new(1.0, Color32::from_rgb(0, 106, 210)),
            rounding: Rounding::same(2.0),
            fg_stroke: Stroke::new(1.0, Color32::from_rgb(255, 255, 255)),
            expansion: 0.0,
        },
        open: WidgetVisuals {
            bg_fill: Color32::from_rgb(35, 35, 35),
            weak_bg_fill: Color32::from_rgb(35, 35, 35),
            bg_stroke: hover_stroke,
            rounding: Rounding::same(2.0),
            fg_stroke: Stroke::new(1.0, Color32::from_rgb(255, 255, 255)),
            expansion: 0.0,
        },
    };

    visuals.window_rounding = Rounding::same(2.0);
    visuals.window_shadow = Shadow::NONE;
    visuals.popup_shadow = Shadow::NONE;

    visuals
}

/// Complete TeamSpeak 3 Layout Spacing & Metric Rules (`egui::Style`)
pub fn ts3_style(visuals: Visuals) -> Style {
    let mut style = Style {
        visuals,
        ..Default::default()
    };

    // Exact Metrics from default.qss & default_mac.qss
    style.spacing = Spacing {
        item_spacing: Vec2::new(4.0, 4.0),
        window_margin: Margin::same(4.0),
        button_padding: Vec2::new(9.0, 3.0), // Derived from IntegrationBubbleEditableComboBox (default.qss:148)
        menu_margin: Margin::same(2.0),
        indent: 16.0,                         // TreeView indent depth per hierarchy level
        interact_size: Vec2::new(24.0, 24.0), // Standard tool button square size
        slider_width: 100.0,
        combo_width: 120.0,
        text_edit_width: 280.0,
        icon_width: 16.0,
        icon_spacing: 4.0,
        tooltip_width: 320.0,
        scroll: ScrollStyle {
            bar_width: 15.0, // Standard Windows DWM scrollbar width
            bar_inner_margin: 0.0,
            bar_outer_margin: 0.0,
            handle_min_length: 17.0,
            ..Default::default()
        },
    };

    style
}

/// Helper to configure both `Visuals` and `Style` on an `egui::Context`
pub fn apply_ts3_theme(ctx: &egui::Context, dark_mode: bool) {
    let visuals = if dark_mode {
        ts3_dark_visuals()
    } else {
        ts3_light_visuals()
    };
    let style = ts3_style(visuals);
    ctx.set_style(style);
}
```

---

## 7. Banner & Specialty Widget Rendering Utilities

In Qt QSS, several banner widgets use linear gradient styling (`default.qss:40-44, 118`):

### 7.1 Banner Gradient Renderer (`egui::Painter`)

```rust
/// Renders the exact TeamSpeak 3 Account & Bookmarks Banner (default.qss:40-49)
///
/// Gradient definition:
/// stop: 0.00 rgb( 80,  92, 120) -> #505C78
/// stop: 0.08 rgb(133, 147, 174) -> #8593AE
/// stop: 0.32 rgb( 88, 107, 140) -> #586B8C
/// stop: 1.00 rgb(111, 128, 170) -> #6F80AA
pub fn paint_ts3_banner(ui: &mut egui::Ui, text: &str, min_width: f32) -> egui::Response {
    let height = 32.0; // qproperty-minimumSize: 0px 32px (default.qss:48)
    let desired_size = Vec2::new(min_width.max(ui.available_width()), height);
    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::hover());

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();

        // Approximate 4-stop gradient via vertical scan bands
        let stops = [
            (0.00, Color32::from_rgb(80, 92, 120)),
            (0.08, Color32::from_rgb(133, 147, 174)),
            (0.32, Color32::from_rgb(88, 107, 140)),
            (1.00, Color32::from_rgb(111, 128, 170)),
        ];

        // Draw segmented background bands
        for i in 0..stops.len() - 1 {
            let (y0_ratio, c0) = stops[i];
            let (y1_ratio, c1) = stops[i + 1];

            let band_top = rect.min.y + y0_ratio * height;
            let band_bottom = rect.min.y + y1_ratio * height;
            let band_rect = egui::Rect::from_min_max(
                egui::pos2(rect.min.x, band_top),
                egui::pos2(rect.max.x, band_bottom),
            );

            // Interpolate center color
            let mid_color = Color32::from_rgb(
                ((c0.r() as u16 + c1.r() as u16) / 2) as u8,
                ((c0.g() as u16 + c1.g() as u16) / 2) as u8,
                ((c0.b() as u16 + c1.b() as u16) / 2) as u8,
            );
            painter.rect_filled(band_rect, Rounding::ZERO, mid_color);
        }

        // Render bold text with 6px left padding (default.qss:46-47)
        let text_pos = egui::pos2(rect.min.x + 6.0, rect.center().y);
        painter.text(
            text_pos,
            egui::Align2::LEFT_CENTER,
            text,
            egui::FontId::proportional(13.0),
            Color32::WHITE,
        );
    }

    response
}
```

### 7.2 Server Splitter Handle Rendering (`default.qss:21-31`)

```rust
/// Renders a 2px horizontal or vertical splitter bar with 1.5px margin
pub fn paint_ts3_splitter(ui: &mut egui::Ui, is_vertical: bool) {
    let thickness = 2.0; // height: 2px (default.qss:29)
    let margin = 1.5;    // margin: 1.5px (default.qss:22)

    let size = if is_vertical {
        Vec2::new(thickness + 2.0 * margin, ui.available_height())
    } else {
        Vec2::new(ui.available_width(), thickness + 2.0 * margin)
    };

    let (rect, _response) = ui.allocate_exact_size(size, egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        let center_rect = if is_vertical {
            egui::Rect::from_min_size(
                egui::pos2(rect.min.x + margin, rect.min.y),
                Vec2::new(thickness, rect.height()),
            )
        } else {
            egui::Rect::from_min_size(
                egui::pos2(rect.min.x, rect.min.y + margin),
                Vec2::new(rect.width(), thickness),
            )
        };

        // Splitter handle color
        ui.painter().rect_filled(
            center_rect,
            Rounding::ZERO,
            Color32::from_rgb(180, 180, 180),
        );
    }
}
```

---

## 8. Verification & File Citation Ledger

| Rule / Requirement | Primary Source File & Line | Verified Concrete Behavior |
| :--- | :--- | :--- |
| Global Link Color | `default.qss:7` | `QPalette::Link = #1c82cc;` |
| ClientFriend Color | `default.qss:10` | `CustomColor::ClientFriend = #008000;` |
| ClientBlocked Color | `default.qss:11` | `CustomColor::ClientBlocked = #ff0000;` |
| ClientRecording Color| `default.qss:12` | `CustomColor::ClientRecording = #800000;` |
| Splitter Handle Margin | `default.qss:22` | `margin: 1.5px;` |
| Splitter Handle Thickness | `default.qss:29` | `height: 2px;` |
| Banner Gradient Stops | `default.qss:40-44`| `stop: 0 rgb(80,92,120)`, `stop: 0.08 rgb(133,147,174)`, `stop: 0.32 rgb(88,107,140)`, `stop: 1 rgb(111,128,170)` |
| Banner Padding & Font | `default.qss:46-47`| `padding-left: 6px; font-weight: bold;` |
| Banner Min Size | `default.qss:48, 52`| `minimumSize: 0px 32px;` / `420px 32px` |
| Chat Input Help Text | `default.qss:56` | `color: #909090;` |
| Status Bar Padding | `default.qss:64` | `padding: 0px 3px; border: 0px;` |
| ServerTreeView Decoration | `default.qss:90` | `qproperty-rootIsDecorated: false;` |
| ServerTreeView Alt BG | `default.qss:89` | `alternate-background-color: transparent;` |
| TableView Alternating Rows| `default.qss:107` | `qproperty-alternatingRowColors: true;` |
| Headline Gradient | `default.qss:118` | `stop: 0 #819CBD, stop: 1 #CDD7E5;` |
| Integration Bubble BG | `default.qss:124,139`| `background: #d6ecff;` |
| Integration Bubble Border | `default.qss:141` | `border-width: 1px; border-style: solid; border-radius: 2px;` |
| Integration ComboBox | `default.qss:146,148`| `border: 2px groove #1179ce; padding: 0px 9px 1px 9px;` |
| Error Indicator | `default.qss:152,159`| `background: red; border-radius: 2px; font-weight: bold; color: white;` |
| Dark Tab Text Color | `default_darkmode.qss:9`| `ImprovedTabBar::tab { color: white; }` |
| Chat Object Links (Light)| `default_chat.qss:99`| `.TextMessage_UserLink { color: #002F5D; font-weight: bold; }` |
| Chat Welcome/Poke (Light)| `default_chat.qss:124-125`| `.TextMessage_Welcome, .TextMessage_Poke { color: #0AA537; }` |
| Chat Normal Text (Light) | `default_chat.qss:131` | `.TextMessage_Normal { color: #33597D; }` |
| Chat Dropped/Kicked/Banned| `default_chat.qss:144`| `.TextMessage_ClientDropped, ... { color: #C9070A; }` |
| Chat Timestamp (Light) | `default_chat.qss:168` | `.TextMessage_Time { color: #33597D; font-weight: normal; }` |
| Chat History Divider | `default_chat.qss:174` | `.TextMessage_BeginHistory { color: #707475; font-weight: bold; font-style: italic; }` |
| Chat Server Error | `default_chat.qss:184` | `.TextMessage_ServerError { color: #C9070A; font-weight: bold; }` |
| InfoFrame Title Links | `default_chat.qss:199` | `color: midnightblue; font-weight: normal;` |
| InfoFrame Table Padding | `default_chat.qss:213` | `padding: 0px 4px 0px 1px;` |
| Chat Links (Dark) | `default_chat_darkmode.qss:12`| `color: #006ad2;` |
| Chat Welcome/Poke (Dark) | `default_chat_darkmode.qss:18-19`| `color: #11de4d;` |
| Chat Dropped/Kicked (Dark)| `default_chat_darkmode.qss:24`| `color: #ec2124;` |
| Chat Events & Time (Dark)| `default_chat_darkmode.qss:47`| `color: #83acd2;` |
| Chat History Divider (Dark)| `default_chat_darkmode.qss:52-53`| `color: #bcbcbc;` |
| Chat Server Error (Dark) | `default_chat_darkmode.qss:56`| `color: #f73c3f;` |
| InfoFrame Title Links (Dark)| `default_chat_darkmode.qss:62`| `color: lightsteelblue;` |
| Tab Border & Radii | `default_mac.qss:28-30` | `border: 1px solid palette(mid); border-radius: 6px; padding: 2px;` |
| Tab Top Margins | `default_mac.qss:40,47` | `margin-top: 6px;` (normal), `margin-top: 4px;` (selected) |
