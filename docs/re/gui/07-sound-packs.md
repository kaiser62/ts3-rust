# TeamSpeak 3 Client: Sound Packs & Audio Subsystem Specification

**Document Version:** 1.0.0  
**Scope:** Reverse-engineering specification of the TeamSpeak 3 Client sound notification system, INI sound pack configuration format, event mapping grammar, TTS speech syntax, audio backend plugin architecture (`soundbackends`), DirectSound 8 C ABI implementation, Windows Audio Session (WASAPI) host engine, and native Rust implementation architecture for `ts3-rust`.

**Primary System References:**
- Sound Packs Root: `C:\Program Files\TeamSpeak 3 Client\sound\`
  - Default Female Pack: [`default/settings.ini`](file:///C:/Program%20Files/TeamSpeak%203%20Client/sound/default/settings.ini) (217 lines, 79 WAV files)
  - Default Male Pack: [`default_male/settings.ini`](file:///C:/Program%20Files/TeamSpeak%203%20Client/sound/default_male/settings.ini) (217 lines, 72 WAV files + 7 inherited)
  - Default Speech (TTS): [`default_speech/settings.ini`](file:///C:/Program%20Files/TeamSpeak%203%20Client/sound/default_speech/settings.ini) (206 lines, `say(...)` scripts)
  - Muted Pack: [`nosounds/settings.ini`](file:///C:/Program%20Files/TeamSpeak%203%20Client/sound/nosounds/settings.ini) (5 lines, zero sounds)
- Audio Backends Root: `C:\Program Files\TeamSpeak 3 Client\soundbackends\`
  - DirectSound Backend DLL: [`soundbackends/directsound_win64.dll`](file:///C:/Program%20Files/TeamSpeak%203%20Client/soundbackends/directsound_win64.dll) (PE64, 132,608 bytes, 15 exports)
- Host Binary Core: `C:\Program Files\TeamSpeak 3 Client\ts3client_win64.exe` (PE64)
  - Built-in WASAPI Engine: `WAS@audio@teamspeak@com`, `CMMNotificationClient@audio@teamspeak@com`
  - Dynamic Backend Loader: `SoundBackendInterface_Dynamic@audio@teamspeak@com`
  - Audio Processing Pipeline: `team_audio` (`sound_device_manager.cpp`, `playback_manager.cpp`, `capture_manager.cpp`, `playback_wave_file_data.cpp`, `wave_reader.cpp`)
  - UI Notification Engine: `notificationssetup.cpp`, `notificationssetupcombobox.cpp`, `Ui_NotificationsSetup`

---

## 1. Architectural Overview

TeamSpeak 3's audio and notification system is structured into two distinct layers:
1. **Notification & Sound Pack Presentation Layer (Client UI)**: An event-driven notification dispatch system reading declarative INI sound packs that map internal client events (channel join, poke, kick, mute, whisper) to wave playback commands (`play(...)`) or Text-to-Speech synthesizers (`say(...)`).
2. **Audio I/O & DSP Pipeline Layer (`team_audio` Subsystem)**: A modular C++ audio engine decoupling device management, format conversion, mixing, echo cancellation (AEC), and hardware backends. Audio hardware backends are divided into **static backends** compiled directly into the executable (Windows Audio Session / WASAPI on Windows) and **dynamic backends** loaded as shared libraries from the `soundbackends/` directory (e.g. `directsound_win64.dll`).

```
+------------------------------------------------------------------------------------+
|                             TeamSpeak 3 Client Core                                |
|                                                                                    |
|  +---------------------------+             +------------------------------------+  |
|  |     Event Dispatcher      |             |        Qt UI Notification          |  |
|  |  (Channel, Client, Chat,  |             |  (notificationssetup.cpp:842)      |  |
|  |   TalkPower, Error, Poke) |             +-----------------+------------------+  |
|  +-------------+-------------+                               |                     |
|                |                                             |                     |
|                v                                             v                     |
|  +-------------------------------------------------------------------------------+ |
|  |                     Sound Pack Engine (sound/settings.ini)                    | |
|  |                                                                               | |
|  |  * Config: [info] name, version, author, readonly                             | |
|  |  * Grammar: play("<rel_path>.wav") | say("<tts template>") | <empty>          | |
|  |  * Variable Expander: ${clientType}, ${clientname}, ${channelname}, ${server}  | |
|  |  * Contact Resolution: neutral | friend | blocked_user                        | |
|  +-----------------------------+-------------------------------------------------+ |
|                                |                                                   |
|                +---------------+---------------+                                   |
|                |                               |                                   |
|                v (WAV Path)                    v (TTS Text)                        |
|  +---------------------------+   +-------------------------------+                 |
|  |    playback_wave_file     |   |   OS Text-To-Speech Engine    |                 |
|  |   (wave_reader.cpp)       |   |   (SAPI 5.x / Windows.Media)  |                 |
|  +-------------+-------------+   +---------------+---------------+                 |
|                |                                 |                                 |
|                +----------------+----------------+                                 |
|                                 | (Linear PCM 44.1k/48k Mono)                      |
|                                 v                                                  |
|  +-------------------------------------------------------------------------------+ |
|  |                       team_audio Device & Mixer Subsystem                     | |
|  |                                                                               | |
|  |  * Sound_Device_Manager (sound_device_manager.cpp)                            | |
|  |  * GenericSoundChannelMixer & GenericSoundChannel (generic_sound_channel.cpp) | |
|  |  * SoundBufferConverter & ChannelConverter_* (generic_channel_converter.cpp)  | |
|  |  * Team_Apm_Impl (WebRTC APM wrapper: AEC, AGC, Noise Suppression)            | |
|  |  * Comfort Noise Generator (Cng) & AGC (Agc)                                  | |
|  +-----------------------------+-------------------------------------------------+ |
|                                |                                                   |
|                +---------------+---------------+                                   |
|                |                               |                                   |
|                v (Built-in Static)             v (Dynamic Plugin C ABI)            |
|  +---------------------------+   +-------------------------------+                 |
|  | SoundBackendInterface_    |   | SoundBackendInterface_        |                 |
|  | Static (WASAPI Engine)    |   | Dynamic (soundbackends/)      |                 |
|  | * WAS@audio@teamspeak@com |   | * directsound_win64.dll       |                 |
|  | * CMMNotificationClient   |   |   ts3soundbackend_* C ABI     |                 |
|  +-------------+-------------+   +---------------+---------------+                 |
|                |                                 |                                 |
|                v                                 v                                 |
|         Windows WASAPI                    DirectSound 8                            |
|       (Exclusive / Shared)                (DSOUND.dll)                             |
+------------------------------------------------------------------------------------+
```

---

## 2. Sound Pack Architecture & Configuration Format

### 2.1 File System Organization

Sound packs reside under `C:\Program Files\TeamSpeak 3 Client\sound\`:
```
sound/
├── default/                  # Default Female voice pack (79 WAV files + settings.ini)
│   ├── settings.ini          # Core mapping configuration (217 lines)
│   ├── 3d_test.wav           # 44.1 kHz, 16-bit mono PCM
│   ├── stop_talking.wav      # 48.0 kHz, 16-bit mono PCM
│   ├── whisper_notify.wav    # 48.0 kHz, 16-bit mono PCM
│   └── ... (76 other wavs)
├── default_male/             # Default Male voice pack (72 WAV files + settings.ini)
│   ├── settings.ini          # Male mapping configuration (217 lines)
│   └── ... (re-uses 7 non-vocal/special WAV files from ../default/)
├── default_speech/           # Default Text-to-Speech pack
│   └── settings.ini          # Speech synthesis template mappings (206 lines)
└── nosounds/                 # Muted / silent profile
    └── settings.ini          # Header only; zero event mappings (5 lines)
```

### 2.2 INI Configuration Grammar

Sound pack configurations are standard INI files divided into two sections:
1. `[info]`: Pack metadata read by the client UI.
2. `[soundfiles]`: Key-value event assignment table.

#### `[info]` Section Schema
| Key | Type | Description | Observed Example |
| :--- | :--- | :--- | :--- |
| `name` | String | Display name shown in Client Settings -> Notifications | `Default Sound Pack (Female)` |
| `version` | String | Version number string | `1.0` |
| `author` | String | Creator or organization | `TeamSpeak Systems GmbH` |
| `readonly` | Integer (`0` or `1`) | If `1`, user edits cannot mutate this file in-place | `1` |

When `readonly = 1`, any user customizations performed in the UI are persisted to the user profile's `ts3client.ini` under `[Notifications]` rather than overwriting the system file (`Attempting to save read-only sounds ini`, [`ts3client_win64.exe:0x100b2b0`](file:///C:/Program%20Files/TeamSpeak%203%20Client/ts3client_win64.exe)).

#### `[soundfiles]` Value Syntax
Each line in `[soundfiles]` assigns an event trigger to a script expression:
- **WAV Playback**: `play("<filename>.wav")`  
  Plays the target wave file relative to the sound pack's directory. Relative directory traversals (`play("../default/<file>.wav")`) are explicitly permitted and used by `default_male` and `default_speech`.
- **Text-to-Speech**: `say("<speech template string>")`  
  Invokes the host operating system's TTS engine (e.g. SAPI / Speech Synthesis API) with template variable interpolation.
- **Silent / Unbound**: `<empty>`  
  The key has no right-hand expression (e.g. `CHANNEL_CREATED_BY_OTHER = `). The client suppresses audio playback for this event.

### 2.3 Template Variable Interpolation

The TeamSpeak 3 Sound Engine evaluates four runtime variables before dispatching sound playback or TTS:

| Variable | Target Type | Evaluated Value | Context Available |
| :--- | :--- | :--- | :--- |
| `${clientType}` | WAV Filename Prefix | `"neutral"`, `"friend"`, or `"blocked_user"` | Client connection, join, switch, kick, ban, and move events |
| `${clientname}` | TTS String | Nickname / phonetic nickname of the subject client | Client actions, poke, kick, ban, rename, group assignments |
| `${servername}` | TTS String | Virtual server name (`virtualserver_name`) | Connection connect/disconnect, timed out, server-wide actions |
| `${channelname}`| TTS String | Channel name (`channel_name`) | Channel creation, edit, move, switch, join, kick |

#### Client Type Resolution Logic
In `default` and `default_male`, filenames use template substitution:
```ini
CLIENT_CONNECTION_CONNECTED_CURRENT_CHANNEL = play("${clientType}_connection_connected_currentchannel.wav")
```
When an event occurs:
1. The client looks up the target client's UUID in `FriendFoeManager` ([`friendfoemanager.cpp`](file:///C:/Program%20Files/TeamSpeak%203%20Client/ts3client_win64.exe)).
2. If marked as Friend -> `${clientType}` evaluates to `"friend"`.
3. If marked as Blocked -> `${clientType}` evaluates to `"blocked_user"`.
4. Otherwise -> `${clientType}` evaluates to `"neutral"` ([`notificationssetup.cpp:842`](file:///C:/Program%20Files/TeamSpeak%203%20Client/ts3client_win64.exe)).

---

## 3. Exhaustive Client Event to Sound Pack Mapping

TeamSpeak 3 client defines exactly **101 event keys** in `[soundfiles]`. Below is the complete, exhaustive cross-reference across all 4 shipped sound packs (`default`, `default_male`, `default_speech`, and `nosounds`).


### 3.1 General / Connection

| Event Key | Default (Female) | Default Male | Default Speech (TTS) | No Sounds |
| :--- | :--- | :--- | :--- | :--- |
| `CONNECTION_CONNECTED` | `play("connected.wav")` (L23) | `play("connected.wav")` (L23) | `say("Connected to ${servername}")` (L11) | *silent* |
| `CONNECTION_DISCONNECTED` | `play("disconnected.wav")` (L24) | `play("disconnected.wav")` (L24) | `say("Disconnected from ${servername}")` (L12) | *silent* |
| `CONNECTION_LOST_CONNECTION` | `play("connection_lost.wav")` (L25) | `play("connection_lost.wav")` (L25) | `say("Connection lost")` (L13) | *silent* |

### 3.2 Other Notifications

| Event Key | Default (Female) | Default Male | Default Speech (TTS) | No Sounds |
| :--- | :--- | :--- | :--- | :--- |
| `OTHER_RECEIVED_POKE` | `play("you_were_poked.wav")` (L28) | `play("you_were_poked.wav")` (L28) | `say("You were poked by ${clientname}")` (L189) | *silent* |
| `OTHER_FILETRANSFER_COMPLETE` | `play("filetransfer_complete.wav")` (L29) | `play("filetransfer_complete.wav")` (L29) | `say("File transfer complete")` (L190) | *silent* |
| `OTHER_FILETRANSFER_FAILED` | `play("filetransfer_failed.wav")` (L30) | `play("filetransfer_failed.wav")` (L30) | `say("File transfer failed")` (L191) | *silent* |
| `OTHER_WHISPERLIST_EMPTY` | `play("whisperlist_is_empty.wav")` (L31) | `play("whisperlist_is_empty.wav")` (L31) | `say("Whisper list is empty")` (L192) | *silent* |
| `OTHER_WHISPERTARGET_NOT_FOUND` | `play("no_whisper_target_found.wav")` (L32) | `play("no_whisper_target_found.wav")` (L32) | `say("No whisper target found")` (L193) | *silent* |
| `OTHER_WHISPERTARGET_TOO_MANY` | `play("too_many_whisper_targets.wav")` (L33) | `play("too_many_whisper_targets.wav")` (L33) | `say("Too many whisper targets")` (L194) | *silent* |

### 3.3 Special / Audio Testing & Feedback

| Event Key | Default (Female) | Default Male | Default Speech (TTS) | No Sounds |
| :--- | :--- | :--- | :--- | :--- |
| `SPECIAL_WHISPER_NOTIFY` | `play("whisper_notify.wav")` (L34) | `play("../default/whisper_notify.wav")` (L34) | `play("../default/whisper_notify.wav")` (L201) | *silent* |
| `SPECIAL_MIC_CLICK_OTHER_OFF` | `play("mic_click_off.wav")` (L37) | `play("../default/mic_click_off.wav")` (L37) | `play("../default/mic_click_off.wav")` (L202) | *silent* |
| `SPECIAL_MIC_CLICK_OTHER_ON` | `play("mic_click_on.wav")` (L38) | `play("../default/mic_click_on.wav")` (L38) | `play("../default/mic_click_on.wav")` (L203) | *silent* |
| `SPECIAL_MIC_CLICK_SELF_OFF` | `play("mic_click_off.wav")` (L39) | `play("../default/mic_click_off.wav")` (L39) | `play("../default/mic_click_off.wav")` (L204) | *silent* |
| `SPECIAL_MIC_CLICK_SELF_ON` | `play("mic_click_on.wav")` (L40) | `play("../default/mic_click_on.wav")` (L40) | `play("../default/mic_click_on.wav")` (L205) | *silent* |
| `SPECIAL_3D_TEST` | `play("3d_test.wav")` (L41) | `play("../default/3d_test.wav")` (L41) | `play("../default/3d_test.wav")` (L206) | *silent* |
| `SPECIAL_SOUND_TEST` | `play("testsound.wav")` (L42) | `play("testsound.wav")` (L42) | `say("Testing your playback sound system")` (L200) | *silent* |
| `SPECIAL_TALKING_WHILE_MUTED` | `play("stop_talking.wav")` (L43) | `play("../default/stop_talking.wav")` (L43) | `say("Talking while muted")` (L199) | *silent* |

### 3.4 Channel Sounds

| Event Key | Default (Female) | Default Male | Default Speech (TTS) | No Sounds |
| :--- | :--- | :--- | :--- | :--- |
| `CHANNEL_CREATED_BY_YOU` | `play("channel_created.wav")` (L50) | `play("channel_created.wav")` (L50) | `say("You created channel ${channelname}")` (L19) | *silent* |
| `CHANNEL_CREATED_BY_OTHER` | *silent* | *silent* | `say("Channel ${channelname} was created by ${clientname}")` (L20) | *silent* |
| `CHANNEL_DELETED_BY_YOU` | `play("channel_deleted.wav")` (L54) | `play("channel_deleted.wav")` (L54) | `say("Channel ${channelname} deleted")` (L23) | *silent* |
| `CHANNEL_DELETED_BY_OTHER` | *silent* | *silent* | `say("Channel ${channelname} was deleted by ${clientname}")` (L24) | *silent* |
| `CHANNEL_DELETED_BY_SERVER` | *silent* | *silent* | `say("Channel ${channelname} was deleted by the server")` (L25) | *silent* |
| `CHANNEL_EDITED_CURRENT_BY_YOU` | `play("channel_edited.wav")` (L59) | `play("channel_edited.wav")` (L59) | `say("You edited your channel ${channelname}")` (L28) | *silent* |
| `CHANNEL_EDITED_CURRENT_BY_OTHER` | `play("your_channel_was_edited.wav")` (L60) | `play("your_channel_was_edited.wav")` (L60) | `say("Your channel ${channelname} was edited by ${clientname}")` (L29) | *silent* |
| `CHANNEL_EDITED_OTHER_BY_YOU` | `play("channel_edited.wav")` (L63) | `play("channel_edited.wav")` (L63) | `say("You edited channel ${channelname}")` (L32) | *silent* |
| `CHANNEL_EDITED_OTHER_BY_OTHER` | *silent* | *silent* | `say("Channel ${channelname} was edited by ${clientname}")` (L33) | *silent* |
| `CHANNEL_EDITED_OTHER_BY_SERVER` | *silent* | *silent* | `say("Channel ${channelname} was edited by the server")` (L34) | *silent* |
| `CHANNEL_MOVED_BY_YOU` | `play("channel_moved.wav")` (L68) | `play("channel_moved.wav")` (L68) | `say("You moved channel ${channelname}")` (L37) | *silent* |
| `CHANNEL_MOVED_BY_OTHER` | *silent* | *silent* | `say("Channel ${channelname} was moved by ${clientname}")` (L38) | *silent* |

### 3.5 Server Sounds

| Event Key | Default (Female) | Default Male | Default Speech (TTS) | No Sounds |
| :--- | :--- | :--- | :--- | :--- |
| `SERVER_EDITED_BY_YOU` | `play("you_edited_server.wav")` (L76) | `play("you_edited_server.wav")` (L76) | `say("You edited the server")` (L44) | *silent* |
| `SERVER_EDITED_BY_OTHER` | *silent* | *silent* | `say("Server edited by ${clientname}")` (L45) | *silent* |
| `SERVER_INSUFFICIENT_PERMISSIONS` | `play("insufficient_permissions.wav")` (L78) | `play("insufficient_permissions.wav")` (L78) | `say("Insufficient permissions")` (L48) | *silent* |
| `SERVER_ERROR` | `play("error.wav")` (L79) | `play("error.wav")` (L79) | `say("Error")` (L47) | *silent* |

### 3.6 Client Connection & Movement

| Event Key | Default (Female) | Default Male | Default Speech (TTS) | No Sounds |
| :--- | :--- | :--- | :--- | :--- |
| `CLIENT_CONNECTION_CONNECTED_SERVER` | *silent* | *silent* | `say("${clientname} joined ${servername}")` (L54) | *silent* |
| `CLIENT_CONNECTION_CONNECTED_CURRENT_CHANNEL` | `play("${clientType}_connection_connected_currentchannel.wav")` (L87) | `play("${clientType}_connection_connected_currentchannel.wav")` (L87) | `say("${clientname} entered your channel ${channelname} on ${servername}")` (L55) | *silent* |
| `CLIENT_CONNECTION_DISCONNECTED_SERVER` | *silent* | *silent* | `say("${clientname} disconnected from ${servername}")` (L58) | *silent* |
| `CLIENT_CONNECTION_DISCONNECTED_CURRENT_CHANNEL` | `play("${clientType}_connection_disconnected_currentchannel.wav")` (L91) | `play("${clientType}_connection_disconnected_currentchannel.wav")` (L91) | `say("${clientname} disconnected from your channel ${channelname} from ${servername}")` (L59) | *silent* |
| `CLIENT_CONNECTION_LOST_CONNECTION_SERVER` | *silent* | *silent* | `say("${clientname} timed out from ${servername}")` (L62) | *silent* |
| `CLIENT_CONNECTION_LOST_CONNECTION_CURRENT_CHANNEL` | `play("${clientType}_connection_connectionlost_currentchannel.wav")` (L95) | `play("${clientType}_connection_connectionlost_currentchannel.wav")` (L95) | `say("${clientname} in your channel ${channelname} timed out from ${servername}")` (L63) | *silent* |
| `CLIENT_SWITCHED_TO_CURRENT_CHANNEL_APPEARS` | `play("${clientType}_switched_tocurrentchannel.wav")` (L98) | `play("${clientType}_switched_tocurrentchannel.wav")` (L98) | `say("${clientname} joined your channel ${channelname} on ${servername} and appears")` (L66) | *silent* |
| `CLIENT_SWITCHED_TO_CURRENT_CHANNEL_STAYS` | `play("${clientType}_switched_tocurrentchannel.wav")` (L99) | `play("${clientType}_switched_tocurrentchannel.wav")` (L99) | `say("${clientname} joined your channel ${channelname} on ${servername}")` (L67) | *silent* |
| `CLIENT_SWITCHED_FROM_CURRENT_CHANNEL_DISAPPEARS` | `play("${clientType}_switched_awayfromcurrentchannel.wav")` (L102) | `play("${clientType}_switched_awayfromcurrentchannel.wav")` (L102) | `say("${clientname} left your channel and disappears to ${channelname} on ${servername}")` (L70) | *silent* |
| `CLIENT_SWITCHED_FROM_CURRENT_CHANNEL_STAYS` | `play("${clientType}_switched_awayfromcurrentchannel.wav")` (L103) | `play("${clientType}_switched_awayfromcurrentchannel.wav")` (L103) | `say("${clientname} left your channel and switched to ${channelname} on ${servername}")` (L71) | *silent* |
| `CLIENT_SWITCHED_TO_OTHER_CHANNEL_APPEARS` | *silent* | *silent* | `say("${clientname} switched channel to ${channelname} and appears on ${servername}")` (L74) | *silent* |
| `CLIENT_SWITCHED_TO_OTHER_CHANNEL_DISAPPEARS` | *silent* | *silent* | `say("${clientname} switched channel to ${channelname} and disappears on ${servername}")` (L75) | *silent* |
| `CLIENT_SWITCHED_TO_OTHER_CHANNEL_STAYS` | *silent* | *silent* | `say("${clientname} switched channel to ${channelname} on ${servername}")` (L76) | *silent* |
| `CLIENT_MOVED_TO_CURRENT_CHANNEL_APPEARS` | `play("${clientType}_moved_tocurrentchannel.wav")` (L111) | `play("${clientType}_moved_tocurrentchannel.wav")` (L111) | `say("${clientname} was moved to your channel ${channelname} and appears on ${servername}")` (L79) | *silent* |
| `CLIENT_MOVED_TO_CURRENT_CHANNEL_STAYS` | `play("${clientType}_moved_tocurrentchannel.wav")` (L112) | `play("${clientType}_moved_tocurrentchannel.wav")` (L112) | `say("${clientname} was moved to your channel ${channelname} on ${servername}")` (L80) | *silent* |
| `CLIENT_MOVED_FROM_CURRENT_CHANNEL_DISAPPEARS` | `play("${clientType}_moved_awayfromcurrentchannel.wav")` (L115) | `play("${clientType}_moved_awayfromcurrentchannel.wav")` (L115) | `say("${clientname} was moved out of your channel ${channelname} on ${servername} and disappears")` (L83) | *silent* |
| `CLIENT_MOVED_FROM_CURRENT_CHANNEL_STAYS` | `play("${clientType}_moved_awayfromcurrentchannel.wav")` (L116) | `play("${clientType}_moved_awayfromcurrentchannel.wav")` (L116) | `say("${clientname} was moved out of your channel ${channelname} on ${servername}")` (L84) | *silent* |
| `CLIENT_MOVED_TO_OTHER_CHANNEL_APPEARS` | *silent* | *silent* | `say("${clientname} was moved to channel ${channelname} on ${servername} and appears")` (L87) | *silent* |
| `CLIENT_MOVED_TO_OTHER_CHANNEL_DISAPPEARS` | *silent* | *silent* | `say("${clientname} was moved to channel ${channelname} on ${servername} and disappears")` (L88) | *silent* |
| `CLIENT_MOVED_TO_OTHER_CHANNEL_STAYS` | *silent* | *silent* | `say("${clientname} was moved to channel ${channelname}")` (L89) | *silent* |
| `CLIENT_RENAMED_BY_YOU` | *silent* | *silent* | `say("you are now known as ${clientname}")` (L92) | *silent* |
| `CLIENT_RENAMED_BY_OTHER` | *silent* | *silent* | `say("${clientname} has been renamed")` (L93) | *silent* |

### 3.7 Client Moderation & Actions

| Event Key | Default (Female) | Default Male | Default Speech (TTS) | No Sounds |
| :--- | :--- | :--- | :--- | :--- |
| `CLIENT_WAS_KICKED_FROM_CHANNEL_TO_CURRENT_CHANNEL_APPEARS` | `play("${clientType}_kicked_channel_tocurrentchannel.wav")` (L128) | `play("${clientType}_kicked_channel_tocurrentchannel.wav")` (L128) | `say("${clientname} was kicked to your ${channelname} on ${servername} and appears")` (L96) | *silent* |
| `CLIENT_WAS_KICKED_FROM_CHANNEL_TO_CURRENT_CHANNEL_STAYS` | `play("${clientType}_kicked_channel_tocurrentchannel.wav")` (L129) | `play("${clientType}_kicked_channel_tocurrentchannel.wav")` (L129) | `say("${clientname} was kicked to your channel ${channelname} on ${servername}")` (L97) | *silent* |
| `CLIENT_WAS_KICKED_FROM_CHANNEL_FROM_CURRENT_CHANNEL_DISAPPEARS` | `play("${clientType}_kicked_channel_awayfromcurrentchannel.wav")` (L132) | `play("${clientType}_kicked_channel_awayfromcurrentchannel.wav")` (L132) | `say("${clientname} was kicked out of your channel ${channelname} on ${servername} and disappears")` (L100) | *silent* |
| `CLIENT_WAS_KICKED_FROM_CHANNEL_FROM_CURRENT_CHANNEL_STAYS` | `play("${clientType}_kicked_channel_awayfromcurrentchannel.wav")` (L133) | `play("${clientType}_kicked_channel_awayfromcurrentchannel.wav")` (L133) | `say("${clientname} was kicked out of your channel ${channelname} on ${servername}")` (L101) | *silent* |
| `CLIENT_WAS_KICKED_FROM_CHANNEL_TO_OTHER_CHANNEL_APPEARS` | *silent* | *silent* | `say("${clientname} was kicked from channel ${channelname} on ${servername} and appears")` (L104) | *silent* |
| `CLIENT_WAS_KICKED_FROM_CHANNEL_TO_OTHER_CHANNEL_DISAPPEARS` | *silent* | *silent* | `say("${clientname} was kicked from channel ${channelname} on ${servername} and disappears")` (L105) | *silent* |
| `CLIENT_WAS_KICKED_FROM_CHANNEL_TO_OTHER_CHANNEL_STAYS` | *silent* | *silent* | `say("${clientname} was kicked from channel ${channelname} on ${servername}")` (L106) | *silent* |
| `CLIENT_WAS_KICKED_FROM_SERVER_SERVER` | *silent* | *silent* | `say("${clientname} was kicked from ${servername}")` (L109) | *silent* |
| `CLIENT_WAS_KICKED_FROM_SERVER_CURRENT_CHANNEL` | `play("${clientType}_kicked_server_currentchannel.wav")` (L142) | `play("${clientType}_kicked_server_currentchannel.wav")` (L142) | `say("${clientname} in your channel was kicked from ${servername}")` (L110) | *silent* |
| `CLIENT_WAS_BANNED_SERVER` | *silent* | *silent* | `say("${clientname} was banned from ${servername}")` (L113) | *silent* |
| `CLIENT_WAS_BANNED_CURRENT_CHANNEL` | `play("${clientType}_banned_server_currentchannel.wav")` (L146) | `play("${clientType}_banned_server_currentchannel.wav")` (L146) | `say("${clientname} in your channel was banned from ${servername}")` (L114) | *silent* |
| `CLIENT_SERVERGROUP_ADDED_BY_SERVER` | *silent* | *silent* | *silent* | *silent* |
| `CLIENT_SERVERGROUP_ADDED_BY_USER` | *silent* | *silent* | *silent* | *silent* |
| `CLIENT_SERVERGROUP_REMOVED_BY_SERVER` | *silent* | *silent* | *silent* | *silent* |
| `CLIENT_SERVERGROUP_REMOVED_BY_USER` | *silent* | *silent* | *silent* | *silent* |
| `CLIENT_CHANNELGROUP_CHANGED_BY_SERVER` | *silent* | *silent* | *silent* | *silent* |
| `CLIENT_CHANNELGROUP_CHANGED_BY_USER` | *silent* | *silent* | *silent* | *silent* |
| `CLIENT_RECORDING_START` | `play("neutral_recording_started_currentchannel.wav")` (L159) | `play("neutral_recording_started_currentchannel.wav")` (L159) | `say("${clientname} starts recording on ${servername}")` (L127) | *silent* |
| `CLIENT_RECORDING_STOP` | `play("neutral_recording_stopped_currentchannel.wav")` (L160) | `play("neutral_recording_stopped_currentchannel.wav")` (L160) | `say("${clientname} stops recording on ${servername}")` (L128) | *silent* |
| `CLIENT_RECORDING_IN_CHANNEL` | `play("neutral_recording_active_currentchannel.wav")` (L161) | `play("neutral_recording_active_currentchannel.wav")` (L161) | `say("User in your channel is recording on ${servername}")` (L129) | *silent* |
| `CLIENT_REQUESTED_TALK_POWER` | `play("talkpower_requested.wav")` (L164) | `play("talkpower_requested.wav")` (L164) | `say("${clientname} requested talk power on ${servername}")` (L132) | *silent* |

### 3.8 Self Actions & Permissions

| Event Key | Default (Female) | Default Male | Default Speech (TTS) | No Sounds |
| :--- | :--- | :--- | :--- | :--- |
| `YOU_SWITCHED_CHANNEL` | `play("channel_switched.wav")` (L171) | `play("channel_switched.wav")` (L171) | `say("You switched to channel ${channelname} on ${servername}")` (L137) | *silent* |
| `YOU_WERE_MOVED_TO_DIFFERENT_CHANNEL` | `play("you_were_moved.wav")` (L172) | `play("you_were_moved.wav")` (L172) | `say("You were moved to channel ${channelname} by ${clientname} on ${servername}")` (L138) | *silent* |
| `YOU_WERE_KICKED_FROM_SERVER` | `play("you_kicked_server.wav")` (L175) | `play("you_kicked_server.wav")` (L175) | `say("You were kicked from the server by ${clientname}")` (L141) | *silent* |
| `YOU_WERE_KICKED_FROM_CHANNEL` | `play("you_kicked_channel.wav")` (L176) | `play("you_kicked_channel.wav")` (L176) | `say("You were kicked from channel ${channelname} by ${clientname}")` (L142) | *silent* |
| `YOU_WERE_BANNED` | `play("you_were_banned.wav")` (L177) | `play("you_were_banned.wav")` (L177) | `say("You were banned by ${clientname}")` (L144) | *silent* |
| `YOU_WERE_GRANTED_TALK_POWER` | `play("talkpower_granted.wav")` (L180) | `play("talkpower_granted.wav")` (L180) | `say("Talk power granted")` (L145) | *silent* |
| `YOU_WERE_REVOKED_TALK_POWER` | `play("talkpower_revoked.wav")` (L181) | `play("talkpower_revoked.wav")` (L181) | `say("Talk power revoked")` (L146) | *silent* |
| `YOU_SERVERGROUP_ADDED_BY_SERVER` | *silent* | *silent* | *silent* | *silent* |
| `YOU_SERVERGROUP_ADDED_BY_USER` | `play("servergroup_assigned.wav")` (L185) | `play("servergroup_assigned.wav")` (L185) | `say("Server group assigned")` (L150) | *silent* |
| `YOU_SERVERGROUP_REMOVED_BY_SERVER` | *silent* | *silent* | *silent* | *silent* |
| `YOU_SERVERGROUP_REMOVED_BY_USER` | `play("servergroup_revoked.wav")` (L187) | `play("servergroup_revoked.wav")` (L187) | `say("Server group revoked")` (L152) | *silent* |
| `YOU_CHANNELGROUP_CHANGED_BY_SERVER` | *silent* | *silent* | *silent* | *silent* |
| `YOU_CHANNELGROUP_CHANGED_BY_USER` | `play("channelgroup_changed.wav")` (L191) | `play("channelgroup_changed.wav")` (L191) | `say("Channel group changed")` (L156) | *silent* |

### 3.9 Chat Messaging

| Event Key | Default (Female) | Default Male | Default Speech (TTS) | No Sounds |
| :--- | :--- | :--- | :--- | :--- |
| `CHAT_SENT_MESSAGE_CLIENT` | `play("chat_message_outbound.wav")` (L198) | `play("../default/chat_message_outbound.wav")` (L198) | `say("Message sent")` (L161) | *silent* |
| `CHAT_RECEIVED_MESSAGE_CLIENT` | `play("chat_message_inbound.wav")` (L199) | `play("../default/chat_message_inbound.wav")` (L199) | `say("Message received")` (L162) | *silent* |
| `CHAT_SENT_MESSAGE_CHANNEL` | `play("chat_message_outbound.wav")` (L200) | `play("../default/chat_message_outbound.wav")` (L200) | `say("Message sent")` (L163) | *silent* |
| `CHAT_RECEIVED_MESSAGE_CHANNEL` | `play("chat_message_inbound.wav")` (L201) | `play("../default/chat_message_inbound.wav")` (L201) | `say("Message received")` (L164) | *silent* |
| `CHAT_SENT_MESSAGE_SERVER` | `play("chat_message_outbound.wav")` (L202) | `play("../default/chat_message_outbound.wav")` (L202) | `say("Message sent")` (L165) | *silent* |
| `CHAT_RECEIVED_MESSAGE_SERVER` | `play("chat_message_inbound.wav")` (L203) | `play("../default/chat_message_inbound.wav")` (L203) | `say("Message received")` (L166) | *silent* |

### 3.10 Status & Audio State

| Event Key | Default (Female) | Default Male | Default Speech (TTS) | No Sounds |
| :--- | :--- | :--- | :--- | :--- |
| `SOUND_CAPTURE_MUTED` | `play("mic_muted.wav")` (L210) | `play("mic_muted.wav")` (L210) | `say("Microphone muted")` (L171) | *silent* |
| `SOUND_CAPTURE_UNMUTED` | `play("mic_activated.wav")` (L211) | `play("mic_activated.wav")` (L211) | `say("Microphone activated")` (L172) | *silent* |
| `SOUND_PLAYBACK_MUTED` | `play("sound_muted.wav")` (L212) | `play("sound_muted.wav")` (L212) | `say("Sound muted")` (L177) | *silent* |
| `SOUND_PLAYBACK_UNMUTED` | `play("sound_resumed.wav")` (L213) | `play("sound_resumed.wav")` (L213) | `say("Sound resumed")` (L178) | *silent* |
| `STATUS_SET_AWAY` | `play("away_activated.wav")` (L216) | `play("away_activated.wav")` (L216) | `say("See you soon")` (L183) | *silent* |
| `STATUS_SET_PRESENT` | `play("away_deactivated.wav")` (L217) | `play("away_deactivated.wav")` (L217) | `say("Welcome back")` (L184) | *silent* |

---

## 4. Complete Audio Sample Inventory (`.wav` Files)

All TeamSpeak 3 audio samples are single-channel (Mono), 16-bit signed PCM audio files. Most recordings are digitized at 44,100 Hz. Two non-vocal notifications (`stop_talking.wav` and `whisper_notify.wav`) are formatted at 48,000 Hz.

### 4.1 Default Female Sound Pack (`default/*.wav` - 79 Files)

Voice talent and production credits: Jon St. John Productions ([`default/settings.ini:8`](file:///C:/Program%20Files/TeamSpeak%203%20Client/sound/default/settings.ini#L8)).

| # | Filename | Sample Rate | Ch | Bit Depth | Duration | File Size | Primary Event Trigger |
| :-: | :--- | :-: | :-: | :-: | :-: | :-: | :--- |
| 1 | `3d_test.wav` | 44100 Hz | 1 | 16-bit | 1.74s | 153942 B | Dynamic / Event |
| 2 | `away_activated.wav` | 44100 Hz | 1 | 16-bit | 0.76s | 67190 B | Dynamic / Event |
| 3 | `away_deactivated.wav` | 44100 Hz | 1 | 16-bit | 0.76s | 67034 B | Dynamic / Event |
| 4 | `blocked_user_banned_server_currentchannel.wav` | 44100 Hz | 1 | 16-bit | 2.64s | 233098 B | Dynamic / Event |
| 5 | `blocked_user_connection_connected_currentchannel.wav` | 44100 Hz | 1 | 16-bit | 1.89s | 166562 B | Dynamic / Event |
| 6 | `blocked_user_connection_connectionlost_currentchannel.wav` | 44100 Hz | 1 | 16-bit | 2.26s | 199874 B | Dynamic / Event |
| 7 | `blocked_user_connection_disconnected_currentchannel.wav` | 44100 Hz | 1 | 16-bit | 2.31s | 203530 B | Dynamic / Event |
| 8 | `blocked_user_kicked_channel_awayfromcurrentchannel.wav` | 44100 Hz | 1 | 16-bit | 2.05s | 180794 B | Dynamic / Event |
| 9 | `blocked_user_kicked_channel_tocurrentchannel.wav` | 44100 Hz | 1 | 16-bit | 1.97s | 173818 B | Dynamic / Event |
| 10 | `blocked_user_kicked_server_currentchannel.wav` | 44100 Hz | 1 | 16-bit | 2.60s | 229304 B | Dynamic / Event |
| 11 | `blocked_user_moved_awayfromcurrentchannel.wav` | 44100 Hz | 1 | 16-bit | 2.15s | 190004 B | Dynamic / Event |
| 12 | `blocked_user_moved_tocurrentchannel.wav` | 44100 Hz | 1 | 16-bit | 1.94s | 171108 B | Dynamic / Event |
| 13 | `blocked_user_switched_awayfromcurrentchannel.wav` | 44100 Hz | 1 | 16-bit | 1.65s | 145952 B | Dynamic / Event |
| 14 | `blocked_user_switched_tocurrentchannel.wav` | 44100 Hz | 1 | 16-bit | 1.66s | 147008 B | Dynamic / Event |
| 15 | `channel_created.wav` | 44100 Hz | 1 | 16-bit | 1.07s | 94194 B | Dynamic / Event |
| 16 | `channel_deleted.wav` | 44100 Hz | 1 | 16-bit | 1.03s | 90882 B | Dynamic / Event |
| 17 | `channel_edited.wav` | 44100 Hz | 1 | 16-bit | 1.01s | 88898 B | Dynamic / Event |
| 18 | `channel_moved.wav` | 44100 Hz | 1 | 16-bit | 0.92s | 81144 B | Dynamic / Event |
| 19 | `channel_switched.wav` | 44100 Hz | 1 | 16-bit | 1.00s | 87998 B | Dynamic / Event |
| 20 | `channelgroup_changed.wav` | 44100 Hz | 1 | 16-bit | 1.30s | 114698 B | Dynamic / Event |
| 21 | `chat_message_inbound.wav` | 44100 Hz | 1 | 16-bit | 1.00s | 88244 B | Dynamic / Event |
| 22 | `chat_message_outbound.wav` | 44100 Hz | 1 | 16-bit | 0.05s | 4148 B | Dynamic / Event |
| 23 | `connected.wav` | 44100 Hz | 1 | 16-bit | 0.62s | 55262 B | Dynamic / Event |
| 24 | `connection_lost.wav` | 44100 Hz | 1 | 16-bit | 0.99s | 87394 B | Dynamic / Event |
| 25 | `disconnected.wav` | 44100 Hz | 1 | 16-bit | 0.95s | 84018 B | Dynamic / Event |
| 26 | `error.wav` | 44100 Hz | 1 | 16-bit | 0.47s | 41870 B | Dynamic / Event |
| 27 | `filetransfer_complete.wav` | 44100 Hz | 1 | 16-bit | 1.38s | 122026 B | Dynamic / Event |
| 28 | `filetransfer_failed.wav` | 44100 Hz | 1 | 16-bit | 1.21s | 106982 B | Dynamic / Event |
| 29 | `friend_banned_server_currentchannel.wav` | 44100 Hz | 1 | 16-bit | 2.39s | 211382 B | Dynamic / Event |
| 30 | `friend_connection_connected_currentchannel.wav` | 44100 Hz | 1 | 16-bit | 1.39s | 122982 B | Dynamic / Event |
| 31 | `friend_connection_connectionlost_currentchannel.wav` | 44100 Hz | 1 | 16-bit | 1.82s | 161014 B | Dynamic / Event |
| 32 | `friend_connection_disconnected_currentchannel.wav` | 44100 Hz | 1 | 16-bit | 1.82s | 160938 B | Dynamic / Event |
| 33 | `friend_kicked_channel_awayfromcurrentchannel.wav` | 44100 Hz | 1 | 16-bit | 1.76s | 155838 B | Dynamic / Event |
| 34 | `friend_kicked_channel_tocurrentchannel.wav` | 44100 Hz | 1 | 16-bit | 1.74s | 153482 B | Dynamic / Event |
| 35 | `friend_kicked_server_currentchannel.wav` | 44100 Hz | 1 | 16-bit | 2.36s | 208762 B | Dynamic / Event |
| 36 | `friend_moved_awayfromcurrentchannel.wav` | 44100 Hz | 1 | 16-bit | 1.78s | 156818 B | Dynamic / Event |
| 37 | `friend_moved_tocurrentchannel.wav` | 44100 Hz | 1 | 16-bit | 1.67s | 147848 B | Dynamic / Event |
| 38 | `friend_switched_awayfromcurrentchannel.wav` | 44100 Hz | 1 | 16-bit | 1.28s | 112878 B | Dynamic / Event |
| 39 | `friend_switched_tocurrentchannel.wav` | 44100 Hz | 1 | 16-bit | 1.32s | 116498 B | Dynamic / Event |
| 40 | `greeting.wav` | 44100 Hz | 1 | 16-bit | 1.37s | 120666 B | Dynamic / Event |
| 41 | `insufficient_permissions.wav` | 44100 Hz | 1 | 16-bit | 1.37s | 121066 B | Dynamic / Event |
| 42 | `mic_activated.wav` | 44100 Hz | 1 | 16-bit | 1.21s | 106982 B | Dynamic / Event |
| 43 | `mic_click_off.wav` | 44100 Hz | 1 | 16-bit | 0.07s | 6220 B | Dynamic / Event |
| 44 | `mic_click_on.wav` | 44100 Hz | 1 | 16-bit | 0.03s | 2968 B | Dynamic / Event |
| 45 | `mic_muted.wav` | 44100 Hz | 1 | 16-bit | 1.02s | 89936 B | Dynamic / Event |
| 46 | `neutral_banned_server_currentchannel.wav` | 44100 Hz | 1 | 16-bit | 2.48s | 218768 B | Dynamic / Event |
| 47 | `neutral_connection_connected_currentchannel.wav` | 44100 Hz | 1 | 16-bit | 1.45s | 127760 B | Dynamic / Event |
| 48 | `neutral_connection_connectionlost_currentchannel.wav` | 44100 Hz | 1 | 16-bit | 1.83s | 161498 B | Dynamic / Event |
| 49 | `neutral_connection_disconnected_currentchannel.wav` | 44100 Hz | 1 | 16-bit | 1.90s | 167834 B | Dynamic / Event |
| 50 | `neutral_kicked_channel_awayfromcurrentchannel.wav` | 44100 Hz | 1 | 16-bit | 2.02s | 178416 B | Dynamic / Event |
| 51 | `neutral_kicked_channel_tocurrentchannel.wav` | 44100 Hz | 1 | 16-bit | 1.86s | 163914 B | Dynamic / Event |
| 52 | `neutral_kicked_server_currentchannel.wav` | 44100 Hz | 1 | 16-bit | 2.53s | 223686 B | Dynamic / Event |
| 53 | `neutral_moved_awayfromcurrentchannel.wav` | 44100 Hz | 1 | 16-bit | 1.76s | 155802 B | Dynamic / Event |
| 54 | `neutral_moved_tocurrentchannel.wav` | 44100 Hz | 1 | 16-bit | 1.74s | 153818 B | Dynamic / Event |
| 55 | `neutral_recording_active_currentchannel.wav` | 44100 Hz | 1 | 16-bit | 1.81s | 160290 B | Dynamic / Event |
| 56 | `neutral_recording_started_currentchannel.wav` | 44100 Hz | 1 | 16-bit | 2.11s | 185978 B | Dynamic / Event |
| 57 | `neutral_recording_stopped_currentchannel.wav` | 44100 Hz | 1 | 16-bit | 2.10s | 185126 B | Dynamic / Event |
| 58 | `neutral_switched_awayfromcurrentchannel.wav` | 44100 Hz | 1 | 16-bit | 1.33s | 117182 B | Dynamic / Event |
| 59 | `neutral_switched_tocurrentchannel.wav` | 44100 Hz | 1 | 16-bit | 1.34s | 118682 B | Dynamic / Event |
| 60 | `no_whisper_target_found.wav` | 44100 Hz | 1 | 16-bit | 1.51s | 133786 B | Dynamic / Event |
| 61 | `servergroup_assigned.wav` | 44100 Hz | 1 | 16-bit | 1.33s | 117248 B | Dynamic / Event |
| 62 | `servergroup_revoked.wav` | 44100 Hz | 1 | 16-bit | 1.48s | 130438 B | Dynamic / Event |
| 63 | `sound_muted.wav` | 44100 Hz | 1 | 16-bit | 1.01s | 89424 B | Dynamic / Event |
| 64 | `sound_resumed.wav` | 44100 Hz | 1 | 16-bit | 1.02s | 89978 B | Dynamic / Event |
| 65 | `stop_talking.wav` | 48000 Hz | 1 | 16-bit | 0.47s | 45742 B | Dynamic / Event |
| 66 | `talkpower_granted.wav` | 44100 Hz | 1 | 16-bit | 1.27s | 111962 B | Dynamic / Event |
| 67 | `talkpower_requested.wav` | 44100 Hz | 1 | 16-bit | 1.72s | 151946 B | Dynamic / Event |
| 68 | `talkpower_revoked.wav` | 44100 Hz | 1 | 16-bit | 1.50s | 132770 B | Dynamic / Event |
| 69 | `testsound.wav` | 44100 Hz | 1 | 16-bit | 1.61s | 141950 B | Dynamic / Event |
| 70 | `too_many_whisper_targets.wav` | 44100 Hz | 1 | 16-bit | 1.35s | 118994 B | Dynamic / Event |
| 71 | `whisper_notify.wav` | 48000 Hz | 1 | 16-bit | 0.17s | 16648 B | Dynamic / Event |
| 72 | `whisperlist_is_empty.wav` | 44100 Hz | 1 | 16-bit | 1.40s | 124054 B | Dynamic / Event |
| 73 | `you_edited_server.wav` | 44100 Hz | 1 | 16-bit | 1.23s | 108638 B | Dynamic / Event |
| 74 | `you_kicked_channel.wav` | 44100 Hz | 1 | 16-bit | 1.43s | 126418 B | Dynamic / Event |
| 75 | `you_kicked_server.wav` | 44100 Hz | 1 | 16-bit | 1.46s | 128804 B | Dynamic / Event |
| 76 | `you_were_banned.wav` | 44100 Hz | 1 | 16-bit | 1.50s | 132570 B | Dynamic / Event |
| 77 | `you_were_moved.wav` | 44100 Hz | 1 | 16-bit | 0.87s | 77390 B | Dynamic / Event |
| 78 | `you_were_poked.wav` | 44100 Hz | 1 | 16-bit | 0.94s | 82766 B | Dynamic / Event |
| 79 | `your_channel_was_edited.wav` | 44100 Hz | 1 | 16-bit | 1.38s | 121976 B | Dynamic / Event |

### 4.2 Default Male Sound Pack (`default_male/*.wav` - 72 Files)

Voice talent and production credits: Jon St. John Productions ([`default_male/settings.ini:8`](file:///C:/Program%20Files/TeamSpeak%203%20Client/sound/default_male/settings.ini#L8)).
`default_male` contains 72 unique vocal WAV files and delegates all 7 non-vocal sound effects directly to `../default/` via relative paths in `settings.ini`:
1. `../default/3d_test.wav` ([`default_male/settings.ini:41`](file:///C:/Program%20Files/TeamSpeak%203%20Client/sound/default_male/settings.ini#L41))
2. `../default/chat_message_inbound.wav` ([`default_male/settings.ini:199, 201, 203`](file:///C:/Program%20Files/TeamSpeak%203%20Client/sound/default_male/settings.ini#L199))
3. `../default/chat_message_outbound.wav` ([`default_male/settings.ini:198, 200, 202`](file:///C:/Program%20Files/TeamSpeak%203%20Client/sound/default_male/settings.ini#L198))
4. `../default/mic_click_off.wav` ([`default_male/settings.ini:37, 39`](file:///C:/Program%20Files/TeamSpeak%203%20Client/sound/default_male/settings.ini#L37))
5. `../default/mic_click_on.wav` ([`default_male/settings.ini:38, 40`](file:///C:/Program%20Files/TeamSpeak%203%20Client/sound/default_male/settings.ini#L38))
6. `../default/stop_talking.wav` ([`default_male/settings.ini:43`](file:///C:/Program%20Files/TeamSpeak%203%20Client/sound/default_male/settings.ini#L43))
7. `../default/whisper_notify.wav` ([`default_male/settings.ini:34`](file:///C:/Program%20Files/TeamSpeak%203%20Client/sound/default_male/settings.ini#L34))

| # | Filename | Sample Rate | Ch | Bit Depth | Duration | File Size | Primary Event Trigger |
| :-: | :--- | :-: | :-: | :-: | :-: | :-: | :--- |
| 1 | `away_activated.wav` | 44100 Hz | 1 | 16-bit | 1.02s | 89818 B | Dynamic / Event |
| 2 | `away_deactivated.wav` | 44100 Hz | 1 | 16-bit | 1.07s | 94938 B | Dynamic / Event |
| 3 | `blocked_user_banned_server_currentchannel.wav` | 44100 Hz | 1 | 16-bit | 2.86s | 252198 B | Dynamic / Event |
| 4 | `blocked_user_connection_connected_currentchannel.wav` | 44100 Hz | 1 | 16-bit | 2.11s | 186074 B | Dynamic / Event |
| 5 | `blocked_user_connection_connectionlost_currentchannel.wav` | 44100 Hz | 1 | 16-bit | 2.11s | 186246 B | Dynamic / Event |
| 6 | `blocked_user_connection_disconnected_currentchannel.wav` | 44100 Hz | 1 | 16-bit | 2.36s | 208346 B | Dynamic / Event |
| 7 | `blocked_user_kicked_channel_awayfromcurrentchannel.wav` | 44100 Hz | 1 | 16-bit | 2.42s | 213722 B | Dynamic / Event |
| 8 | `blocked_user_kicked_channel_tocurrentchannel.wav` | 44100 Hz | 1 | 16-bit | 2.30s | 202874 B | Dynamic / Event |
| 9 | `blocked_user_kicked_server_currentchannel.wav` | 44100 Hz | 1 | 16-bit | 2.82s | 248758 B | Dynamic / Event |
| 10 | `blocked_user_moved_awayfromcurrentchannel.wav` | 44100 Hz | 1 | 16-bit | 2.31s | 203738 B | Dynamic / Event |
| 11 | `blocked_user_moved_tocurrentchannel.wav` | 44100 Hz | 1 | 16-bit | 2.25s | 198362 B | Dynamic / Event |
| 12 | `blocked_user_switched_awayfromcurrentchannel.wav` | 44100 Hz | 1 | 16-bit | 1.97s | 173786 B | Dynamic / Event |
| 13 | `blocked_user_switched_tocurrentchannel.wav` | 44100 Hz | 1 | 16-bit | 2.00s | 176858 B | Dynamic / Event |
| 14 | `channel_created.wav` | 44100 Hz | 1 | 16-bit | 1.25s | 110042 B | Dynamic / Event |
| 15 | `channel_deleted.wav` | 44100 Hz | 1 | 16-bit | 1.25s | 110042 B | Dynamic / Event |
| 16 | `channel_edited.wav` | 44100 Hz | 1 | 16-bit | 1.25s | 110810 B | Dynamic / Event |
| 17 | `channel_moved.wav` | 44100 Hz | 1 | 16-bit | 1.21s | 106970 B | Dynamic / Event |
| 18 | `channel_switched.wav` | 44100 Hz | 1 | 16-bit | 1.18s | 104666 B | Dynamic / Event |
| 19 | `channelgroup_changed.wav` | 44100 Hz | 1 | 16-bit | 1.63s | 144090 B | Dynamic / Event |
| 20 | `connected.wav` | 44100 Hz | 1 | 16-bit | 0.86s | 75994 B | Dynamic / Event |
| 21 | `connection_lost.wav` | 44100 Hz | 1 | 16-bit | 1.26s | 111322 B | Dynamic / Event |
| 22 | `disconnected.wav` | 44100 Hz | 1 | 16-bit | 1.11s | 98010 B | Dynamic / Event |
| 23 | `error.wav` | 44100 Hz | 1 | 16-bit | 0.67s | 59098 B | Dynamic / Event |
| 24 | `filetransfer_complete.wav` | 44100 Hz | 1 | 16-bit | 1.51s | 133338 B | Dynamic / Event |
| 25 | `filetransfer_failed.wav` | 44100 Hz | 1 | 16-bit | 1.52s | 133850 B | Dynamic / Event |
| 26 | `friend_banned_server_currentchannel.wav` | 44100 Hz | 1 | 16-bit | 2.50s | 220634 B | Dynamic / Event |
| 27 | `friend_connection_connected_currentchannel.wav` | 44100 Hz | 1 | 16-bit | 1.61s | 142298 B | Dynamic / Event |
| 28 | `friend_connection_connectionlost_currentchannel.wav` | 44100 Hz | 1 | 16-bit | 2.07s | 183002 B | Dynamic / Event |
| 29 | `friend_connection_disconnected_currentchannel.wav` | 44100 Hz | 1 | 16-bit | 1.98s | 174818 B | Dynamic / Event |
| 30 | `friend_kicked_channel_awayfromcurrentchannel.wav` | 44100 Hz | 1 | 16-bit | 2.08s | 183770 B | Dynamic / Event |
| 31 | `friend_kicked_channel_tocurrentchannel.wav` | 44100 Hz | 1 | 16-bit | 1.98s | 174554 B | Dynamic / Event |
| 32 | `friend_kicked_server_currentchannel.wav` | 44100 Hz | 1 | 16-bit | 2.52s | 222206 B | Dynamic / Event |
| 33 | `friend_moved_awayfromcurrentchannel.wav` | 44100 Hz | 1 | 16-bit | 1.99s | 175358 B | Dynamic / Event |
| 34 | `friend_moved_tocurrentchannel.wav` | 44100 Hz | 1 | 16-bit | 2.01s | 177626 B | Dynamic / Event |
| 35 | `friend_switched_awayfromcurrentchannel.wav` | 44100 Hz | 1 | 16-bit | 1.60s | 141482 B | Dynamic / Event |
| 36 | `friend_switched_tocurrentchannel.wav` | 44100 Hz | 1 | 16-bit | 1.91s | 168410 B | Dynamic / Event |
| 37 | `greeting.wav` | 44100 Hz | 1 | 16-bit | 1.75s | 154970 B | Dynamic / Event |
| 38 | `insufficient_permissions.wav` | 44100 Hz | 1 | 16-bit | 1.45s | 127706 B | Dynamic / Event |
| 39 | `mic_activated.wav` | 44100 Hz | 1 | 16-bit | 1.45s | 127706 B | Dynamic / Event |
| 40 | `mic_muted.wav` | 44100 Hz | 1 | 16-bit | 1.31s | 115930 B | Dynamic / Event |
| 41 | `neutral_banned_server_currentchannel.wav` | 44100 Hz | 1 | 16-bit | 2.36s | 208346 B | Dynamic / Event |
| 42 | `neutral_connection_connected_currentchannel.wav` | 44100 Hz | 1 | 16-bit | 1.59s | 140762 B | Dynamic / Event |
| 43 | `neutral_connection_connectionlost_currentchannel.wav` | 44100 Hz | 1 | 16-bit | 1.85s | 163802 B | Dynamic / Event |
| 44 | `neutral_connection_disconnected_currentchannel.wav` | 44100 Hz | 1 | 16-bit | 1.92s | 169178 B | Dynamic / Event |
| 45 | `neutral_kicked_channel_awayfromcurrentchannel.wav` | 44100 Hz | 1 | 16-bit | 2.02s | 178394 B | Dynamic / Event |
| 46 | `neutral_kicked_channel_tocurrentchannel.wav` | 44100 Hz | 1 | 16-bit | 2.05s | 181466 B | Dynamic / Event |
| 47 | `neutral_kicked_server_currentchannel.wav` | 44100 Hz | 1 | 16-bit | 2.40s | 212186 B | Dynamic / Event |
| 48 | `neutral_moved_awayfromcurrentchannel.wav` | 44100 Hz | 1 | 16-bit | 2.23s | 196826 B | Dynamic / Event |
| 49 | `neutral_moved_tocurrentchannel.wav` | 44100 Hz | 1 | 16-bit | 2.21s | 195290 B | Dynamic / Event |
| 50 | `neutral_recording_active_currentchannel.wav` | 44100 Hz | 1 | 16-bit | 2.05s | 180954 B | Dynamic / Event |
| 51 | `neutral_recording_started_currentchannel.wav` | 44100 Hz | 1 | 16-bit | 2.32s | 204506 B | Dynamic / Event |
| 52 | `neutral_recording_stopped_currentchannel.wav` | 44100 Hz | 1 | 16-bit | 2.31s | 203994 B | Dynamic / Event |
| 53 | `neutral_switched_awayfromcurrentchannel.wav` | 44100 Hz | 1 | 16-bit | 1.68s | 148442 B | Dynamic / Event |
| 54 | `neutral_switched_tocurrentchannel.wav` | 44100 Hz | 1 | 16-bit | 1.63s | 143834 B | Dynamic / Event |
| 55 | `no_whisper_target_found.wav` | 44100 Hz | 1 | 16-bit | 1.54s | 135898 B | Dynamic / Event |
| 56 | `servergroup_assigned.wav` | 44100 Hz | 1 | 16-bit | 1.53s | 134874 B | Dynamic / Event |
| 57 | `servergroup_revoked.wav` | 44100 Hz | 1 | 16-bit | 1.41s | 124634 B | Dynamic / Event |
| 58 | `sound_muted.wav` | 44100 Hz | 1 | 16-bit | 1.16s | 102618 B | Dynamic / Event |
| 59 | `sound_resumed.wav` | 44100 Hz | 1 | 16-bit | 1.29s | 114084 B | Dynamic / Event |
| 60 | `talkpower_granted.wav` | 44100 Hz | 1 | 16-bit | 1.29s | 114394 B | Dynamic / Event |
| 61 | `talkpower_requested.wav` | 44100 Hz | 1 | 16-bit | 1.88s | 165594 B | Dynamic / Event |
| 62 | `talkpower_revoked.wav` | 44100 Hz | 1 | 16-bit | 1.46s | 129334 B | Dynamic / Event |
| 63 | `testsound.wav` | 44100 Hz | 1 | 16-bit | 1.91s | 168410 B | Dynamic / Event |
| 64 | `too_many_whisper_targets.wav` | 44100 Hz | 1 | 16-bit | 1.46s | 129242 B | Dynamic / Event |
| 65 | `whisperlist_is_empty.wav` | 44100 Hz | 1 | 16-bit | 1.47s | 129754 B | Dynamic / Event |
| 66 | `you_edited_server.wav` | 44100 Hz | 1 | 16-bit | 1.63s | 143858 B | Dynamic / Event |
| 67 | `you_kicked_channel.wav` | 44100 Hz | 1 | 16-bit | 1.72s | 151514 B | Dynamic / Event |
| 68 | `you_kicked_server.wav` | 44100 Hz | 1 | 16-bit | 1.63s | 143834 B | Dynamic / Event |
| 69 | `you_were_banned.wav` | 44100 Hz | 1 | 16-bit | 1.85s | 163802 B | Dynamic / Event |
| 70 | `you_were_moved.wav` | 44100 Hz | 1 | 16-bit | 1.18s | 104558 B | Dynamic / Event |
| 71 | `you_were_poked.wav` | 44100 Hz | 1 | 16-bit | 1.29s | 113882 B | Dynamic / Event |
| 72 | `your_channel_was_edited.wav` | 44100 Hz | 1 | 16-bit | 1.71s | 150746 B | Dynamic / Event |

---

## 5. Audio Backend Architecture & Reverse-Engineering Findings

### 5.1 Dynamic Sound Backend Plugin Subsystem (`soundbackends/`)

TeamSpeak 3 client discovers audio output and capture drivers dynamically at startup:
- **Scan Directory**: `<TeamSpeak 3 Client>/soundbackends/` ([`sound_device_manager.cpp`](file:///C:/Program%20Files/TeamSpeak%203%20Client/ts3client_win64.exe))
- **File Filter**: `*_win64.dll` (on 64-bit Windows) or `*_x64.dll` (on 64-bit Linux). Files containing `_old_` are ignored.
- **Protected Mode Identifiers**: Built-in backend modes (such as WASAPI `"windowsaudiosession"`) are protected; dynamic libraries cannot override them (`" is an protected mode_id name. Library not loaded."`, [`ts3client_win64.exe:0x114f790`](file:///C:/Program%20Files/TeamSpeak%203%20Client/ts3client_win64.exe)).

#### Discovered Backend Libraries
In standard TeamSpeak 3 Client 3.6.x (win64):
- `soundbackends/directsound_win64.dll` (132,608 bytes, SHA256: `bd41056a7df171dd3e2cc879660a8f804b27818d2087bd7cce4de7dcd3e150bc`).
- DirectSound is the only shipped external DLL backend; the primary modern audio backend (`windowsaudiosession` / WASAPI) is statically compiled directly into `ts3client_win64.exe`.

### 5.2 DirectSound Backend C ABI (`directsound_win64.dll`)

`directsound_win64.dll` exports exactly 15 C-callable functions implementing the TeamSpeak sound backend plugin interface:

```c
// TeamSpeak 3 Sound Backend C ABI
#define TS3_SOUND_OK                   0
#define TS3_SOUND_ERROR                1
#define TS3_SOUND_DEVICE_PLAYBACK      1
#define TS3_SOUND_DEVICE_CAPTURE       2

typedef struct TS3SoundBackendInfo {
    uint64_t backend_id;        // Offset 0x00: Backend identifier (1 = DirectSound)
    const char* backend_name;   // Offset 0x08: "Direct Sound"
    uint64_t default_latency_ms;// Offset 0x10: Default buffer latency (200 ms)
    char     inline_buf[16];    // Offset 0x18: Inline string buffer ("Direct Sound\0")
} TS3SoundBackendInfo;

// Function Exports:
TS3SoundBackendInfo* ts3soundbackend_getInfo(void);
uint32_t ts3soundbackend_initialize(void** backend_ctx, void* callbacks);
uint32_t ts3soundbackend_finalize(void* backend_ctx);
uint32_t ts3soundbackend_isSupported(void);
uint32_t ts3soundbackend_enumDevices(void* backend_ctx, uint32_t mode, void*** device_list, uint32_t* count);
uint32_t ts3soundbackend_getDefaultDevice(void* backend_ctx, uint32_t mode, char** default_device_guid);
uint32_t ts3soundbackend_openDevice(void* backend_ctx, uint32_t mode, const char* device_guid, void** device_handle);
uint32_t ts3soundbackend_closeDevice(void* device_handle);
uint32_t ts3soundbackend_startDevice(void* device_handle);
uint32_t ts3soundbackend_stopDevice(void* device_handle);
const char* ts3soundbackend_getErrorString(uint32_t error_code, char* error_buffer);
uint32_t ts3soundbackend_associateAECDevice(void* render_handle, void* capture_handle);
uint32_t ts3soundbackend_lockDSP(void* device_handle);
uint32_t ts3soundbackend_unlockDSP(void* device_handle);
uint32_t ts3soundbackend_waitForClosedDevices(void* backend_ctx);
```

#### Detailed Export Disassembly & Findings
1. **`ts3soundbackend_getInfo`** (`RVA 0x4130`, [`directsound_win64.dll:0x180004130`](file:///C:/Program%20Files/TeamSpeak%203%20Client/soundbackends/directsound_win64.dll)):
   Returns pointer to constant structure at `0x18001ab98` containing backend ID `1`, name `"Direct Sound"`, and default latency `200` ms:
   ```x86asm
   180004130: 48 8d 05 61 6a 01 00    lea    rax, [rip + 0x16a61]  # 0x18001ab98
   180004137: c3                      ret
   ```
2. **`ts3soundbackend_isSupported`** (`RVA 0x41d0`, [`directsound_win64.dll:0x1800041d0`](file:///C:/Program%20Files/TeamSpeak%203%20Client/soundbackends/directsound_win64.dll)):
   Calls `DirectSoundCreate8` probe and checks OS version using `VerifyVersionInfoW`. Returns 1 if DirectSound 8.0+ is operational.
3. **`ts3soundbackend_initialize`** (`RVA 0x4140`, [`directsound_win64.dll:0x180004140`](file:///C:/Program%20Files/TeamSpeak%203%20Client/soundbackends/directsound_win64.dll)):
   Creates a hidden Win32 window with window class `"TeampSpeak3_DirectSound"` (retaining the vendor typo) via `CreateWindowExW` to anchor the DirectSound focus cooperative level (`DSSCL_PRIORITY`). Initializes device tracking structures and callback tables (`m_DS->m_Callbacks.getPlaybackData`).
4. **Internal DirectSound Classes (RTTI)**:
   - `DeviceContext`: Base context class managing COM lifecycle (`CoInitializeEx` / `CoUninitialize`).
   - `CaptureDeviceContext`: Manages `IDirectSoundCapture8`, notification event handles (`CreateEventW`), and circular capture buffer pointers (`CREATECAPTUREBUFFER`).
   - `RenderDeviceContext`: Manages `IDirectSound8`, primary cooperative buffer, secondary streaming buffer (`CREATEBUFFER`), and playback loop timing.
5. **Emulation Warning**:
   If hardware acceleration is absent, the DLL logs:
   `"Opening emulated direct sound device. This is bad for latency. Please enable hardware accelleration."`

### 5.3 Built-in WASAPI Subsystem (`ts3client_win64.exe`)

The default and recommended Windows audio driver in TeamSpeak 3 is Windows Audio Session API (WASAPI):
- **Implementation Class**: `WAS@audio@teamspeak@com` inheriting from `I_Sound_Backend@audio@teamspeak@com`.
- **Static Wrapper**: `SoundBackendInterface_Static@audio@teamspeak@com`.
- **Device Hotplug Monitoring**: `CMMNotificationClient@audio@teamspeak@com` implementing COM `IMMNotificationClient`. Detects device disconnects (`AUDCLNT_E_DEVICE_INVALIDATED`), default endpoint changes, and automatically transitions playback streams.

### 5.4 Audio DSP & Voice Processing Pipeline (`team_audio`)

TeamSpeak 3 Client's `team_audio` module comprises:
1. **Device Management**:
   - `Sound_Device_Manager` ([`sound_device_manager.cpp`](file:///C:/Program%20Files/TeamSpeak%203%20Client/ts3client_win64.exe)): Orchestrates active backend instances, maps friendly device names to GUIDs/endpoint IDs, and routes capture/playback modes.
2. **Channel Mixing & Format Conversion**:
   - `GenericSoundChannelMixer` & `GenericSoundChannel`: Manages individual client audio streams and local notification playback channels.
   - `SoundBufferConverter`: Handles sample rate conversions (e.g. 44.1 kHz sound pack samples to 48.0 kHz voice pipeline).
   - Channel Converters:
     - `ChannelConverter_MonoToStereo`
     - `ChannelConverter_MonoToSurround`
     - `ChannelConverter_StereoToMono`
     - `ChannelConverter_SurroundToStereo`
     - `ChannelConverter_SurroundWithLFEToMono`
3. **Acoustic Echo Cancellation (AEC) & DSP**:
   - `Team_Apm_Impl` ([`AVI_Team_Apm`](file:///C:/Program%20Files/TeamSpeak%203%20Client/ts3client_win64.exe)): TeamSpeak Audio Processing Module wrapping WebRTC APM algorithms for Acoustic Echo Cancellation, Noise Suppression, and Voice Activity Detection (VAD).
   - `PreProcessSpeex` & `PreProcessBase`: Legacy Speex preprocessing filter.
   - `Cng` ([`playback@audio@teamspeak@com`](file:///C:/Program%20Files/TeamSpeak%203%20Client/ts3client_win64.exe)): Comfort Noise Generator injecting synthetic low-level room acoustics during voice silence.
   - `Agc` ([`playback@audio@teamspeak@com`](file:///C:/Program%20Files/TeamSpeak%203%20Client/ts3client_win64.exe)): Automatic Gain Control normalizing incoming remote talkers.
4. **WAV File Playback Engine**:
   - `wave_reader.cpp`: RIFF/WAVE header parser extracting format chunks (`fmt `, `data`), verifying PCM encoding.
   - `playback_wave_file_data.cpp`: Instantiates a transient playback channel on `Playback_Manager`, decodes wave frames into memory, and feeds samples into `GenericSoundChannelMixer`.

---

## 6. Rust Implementation Architecture for `ts3-rust`

To achieve 100% interoperability with TeamSpeak 3 sound packs and provide modern low-latency audio I/O in `ts3-rust`, the audio architecture is divided into two modules: `ts3_soundpack` and `ts3_audio`.

### 6.1 Sound Pack Parser & Dispatcher (`crates/ts3_soundpack`)

```rust
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SoundEventType {
    // Connection
    ConnectionConnected,
    ConnectionDisconnected,
    ConnectionLostConnection,
    // Other
    OtherReceivedPoke,
    OtherFiletransferComplete,
    OtherFiletransferFailed,
    OtherWhisperlistEmpty,
    OtherWhispertargetNotFound,
    OtherWhispertargetTooMany,
    // Special
    SpecialWhisperNotify,
    SpecialMicClickOtherOff,
    SpecialMicClickOtherOn,
    SpecialMicClickSelfOff,
    SpecialMicClickSelfOn,
    Special3dTest,
    SpecialSoundTest,
    SpecialTalkingWhileMuted,
    // Channel
    ChannelCreatedByYou,
    ChannelCreatedByOther,
    ChannelDeletedByYou,
    ChannelDeletedByOther,
    ChannelDeletedByServer,
    ChannelEditedCurrentByYou,
    ChannelEditedCurrentByOther,
    ChannelEditedOtherByYou,
    ChannelEditedOtherByOther,
    ChannelEditedOtherByServer,
    ChannelMovedByYou,
    ChannelMovedByOther,
    // Server
    ServerEditedByYou,
    ServerEditedByOther,
    ServerInsufficientPermissions,
    ServerError,
    // Client Connection & Movement
    ClientConnectionConnectedServer,
    ClientConnectionConnectedCurrentChannel,
    ClientConnectionDisconnectedServer,
    ClientConnectionDisconnectedCurrentChannel,
    ClientConnectionLostConnectionServer,
    ClientConnectionLostConnectionCurrentChannel,
    ClientSwitchedToCurrentChannelAppears,
    ClientSwitchedToCurrentChannelStays,
    ClientSwitchedFromCurrentChannelDisappears,
    ClientSwitchedFromCurrentChannelStays,
    ClientSwitchedToOtherChannelAppears,
    ClientSwitchedToOtherChannelDisappears,
    ClientSwitchedToOtherChannelStays,
    ClientMovedToCurrentChannelAppears,
    ClientMovedToCurrentChannelStays,
    ClientMovedFromCurrentChannelDisappears,
    ClientMovedFromCurrentChannelStays,
    ClientMovedToOtherChannelAppears,
    ClientMovedToOtherChannelDisappears,
    ClientMovedToOtherChannelStays,
    ClientRenamedByYou,
    ClientRenamedByOther,
    // Client Moderation
    ClientWasKickedFromChannelToCurrentChannelAppears,
    ClientWasKickedFromChannelToCurrentChannelStays,
    ClientWasKickedFromChannelFromCurrentChannelDisappears,
    ClientWasKickedFromChannelFromCurrentChannelStays,
    ClientWasKickedFromChannelToOtherChannelAppears,
    ClientWasKickedFromChannelToOtherChannelDisappears,
    ClientWasKickedFromChannelToOtherChannelStays,
    ClientWasKickedFromServerServer,
    ClientWasKickedFromServerCurrentChannel,
    ClientWasBannedServer,
    ClientWasBannedCurrentChannel,
    ClientServergroupAddedByServer,
    ClientServergroupAddedByUser,
    ClientServergroupRemovedByServer,
    ClientServergroupRemovedByUser,
    ClientChannelgroupChangedByServer,
    ClientChannelgroupChangedByUser,
    ClientRecordingStart,
    ClientRecordingStop,
    ClientRecordingInChannel,
    ClientRequestedTalkPower,
    // Self Sounds
    YouSwitchedChannel,
    YouWereMovedToDifferentChannel,
    YouWereKickedFromServer,
    YouWereKickedFromChannel,
    YouWereBanned,
    YouWereGrantedTalkPower,
    YouWereRevokedTalkPower,
    YouServergroupAddedByServer,
    YouServergroupAddedByUser,
    YouServergroupRemovedByServer,
    YouServergroupRemovedByUser,
    YouChannelgroupChangedByServer,
    YouChannelgroupChangedByUser,
    // Chat
    ChatSentMessageClient,
    ChatReceivedMessageClient,
    ChatSentMessageChannel,
    ChatReceivedMessageChannel,
    ChatSentMessageServer,
    ChatReceivedMessageServer,
    // Status
    SoundCaptureMuted,
    SoundCaptureUnmuted,
    SoundPlaybackMuted,
    SoundPlaybackUnmuted,
    StatusSetAway,
    StatusSetPresent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SoundAction {
    Play(PathBuf),
    Say(String),
    Silent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientRelation {
    Neutral,
    Friend,
    BlockedUser,
}

impl ClientRelation {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Neutral => "neutral",
            Self::Friend => "friend",
            Self::BlockedUser => "blocked_user",
        }
    }
}

pub struct SoundPack {
    pub base_path: PathBuf,
    pub name: String,
    pub version: String,
    pub author: String,
    pub readonly: bool,
    pub events: HashMap<SoundEventType, SoundActionTemplate>,
}

pub enum SoundActionTemplate {
    Play(String),
    Say(String),
    Silent,
}

impl SoundPack {
    pub fn load_from_dir(pack_dir: &Path) -> Result<Self, SoundPackError> {
        let ini_path = pack_dir.join("settings.ini");
        let content = std::fs::read_to_string(&ini_path)?;
        // Parse INI sections [info] and [soundfiles]
        // ...
        todo!()
    }

    pub fn resolve_action(
        &self,
        event: SoundEventType,
        client_rel: ClientRelation,
        client_name: Option<&str>,
        channel_name: Option<&str>,
        server_name: Option<&str>,
    ) -> SoundAction {
        match self.events.get(&event) {
            Some(SoundActionTemplate::Play(template)) => {
                let resolved_path = template.replace("${clientType}", client_rel.as_str());
                SoundAction::Play(self.base_path.join(resolved_path))
            }
            Some(SoundActionTemplate::Say(template)) => {
                let mut text = template.clone();
                if let Some(c) = client_name {
                    text = text.replace("${clientname}", c);
                }
                if let Some(ch) = channel_name {
                    text = text.replace("${channelname}", ch);
                }
                if let Some(s) = server_name {
                    text = text.replace("${servername}", s);
                }
                SoundAction::Say(text)
            }
            Some(SoundActionTemplate::Silent) | None => SoundAction::Silent,
        }
    }
}
```

### 6.2 Low-Latency Audio Subsystem (`crates/ts3_audio`)

`ts3-rust` implements cross-platform audio using `cpal` with rodio/symphonia for wave playback:
1. **Device Enumeration & Backend Selection**:
   - Windows: WASAPI (Default Shared/Exclusive) via `cpal::host_from_id(cpal::HostId::Wasapi)`.
   - DirectSound fallback support via Windows Multimedia / `cpal`.
   - Linux: PulseAudio / ALSA / PipeWire via `cpal`.
   - macOS: CoreAudio via `cpal`.
2. **Audio Mixing Pipeline**:
   - `AudioMixer`: Manages independent audio sink channels:
     - Voice Stream Sink (jitter-buffered OPUS/CELT frames).
     - UI Notification Sink (pre-decoded PCM wave samples).
     - Text-to-Speech Sink (platform TTS stream).
3. **Notification Volume & Output Mute Rules**:
   - Master volume slider and sound pack volume slider.
   - When output is muted: Sound pack notifications can be configured to either mute completely or play only critical alerts (`ImportantSounds`).
