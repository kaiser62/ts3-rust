# TeamSpeak 3 Client Icon Packs & Graphics (`gfx/`) Reverse Engineering

This document details the structure, configuration, and complete logical-to-physical icon mappings of the archives and assets in `C:\Program Files\TeamSpeak 3 Client\gfx\`.

---

## 1. Overview of `gfx/` Assets

The `C:\Program Files\TeamSpeak 3 Client\gfx\` directory contains 4 ZIP archives and 2 standalone PNG images:

| Filename | Size (Bytes) | Entry Count | Format | Purpose |
| :--- | ---: | ---: | :--- | :--- |
| `default_colored_2014.zip` | 293,648 | 209 | SVG + INI + TXT | Default modern colored vector icon pack (2014+), including `settings.ini` and 10 SVG emoticons. |
| `default_mono_2014.zip` | 309,323 | 208 | SVG + INI + TXT | Modern monochrome vector icon pack (2014+), identical `settings.ini` to `default_colored_2014.zip` (omits `warning_error.svg`). |
| `default.zip` | 331,752 | 231 | PNG + TXT | Legacy raster fallback icon pack (`16x16_`, `20x20_`, `24x24_`, `32x32_`, `100x100_`, `200x200_`, `7x5_`, `8x7_` prefixed PNGs). Has no `settings.ini`. |
| `countries.zip` | 144,825 | 247 | PNG (`16x11`, 24bpp RGB) | Country flag icons named by lowercase ISO 3166-1 alpha-2 code (e.g., `de.png`, `us.png`, `gb.png`). |
| `tile_150.png` | 14,489 | 1 | PNG (`150x150`, 32bpp ARGB) | Windows Start Menu medium tile icon (TeamSpeak 3 logo). |
| `tile_70.png` | 7,392 | 1 | PNG (`70x70`, 32bpp ARGB) | Windows Start Menu small tile icon (TeamSpeak 3 logo). |

---

## 2. Icon Pack Architecture & `settings.ini` Mechanism

Modern TS3 icon packs (`default_colored_2014.zip` and `default_mono_2014.zip`) use a root `settings.ini` file with three sections:

1. **`[info]`**: Metadata comments (notes that every icon should be at least `32x32` for down-scalability).
2. **`[options]`**:
   - `FALLBACK=true`: Instructs the client to fall back to `default.zip` if a requested icon key or file is missing from the active icon pack.
   - `PLACEHOLDER=2`: Defines missing-icon behavior when fallback also fails (`0` = no placeholder, `1` = empty/transparent placeholder, `2` = blue-bordered placeholder).
3. **`[gfxfiles]`**: Maps 213 internal TS3 logical icon identifiers (uppercase keys) to relative file paths inside the archive (e.g., `PLAYER_ON = player_on.svg`).
   - Also supports optional Qt style icon overrides (commented out by default): `MSGBOX_QUESTION`, `MSGBOX_INFORMATION`, `MSGBOX_WARNING`, `MSGBOX_CRITICAL`, `SAVE`, `ARROW_RIGHT`, `ARROW_LEFT`, `ARROW_UP`, `ARROW_DOWN`, `VERTICAL_EXTENSION_BUTTON`.

### Legacy Fallback Resolution (`default.zip`)
`default.zip` does not contain a `settings.ini`. Instead, when TS3 resolves an icon from `default.zip` (either directly when "Default" legacy pack is selected or via `FALLBACK=true`), it prepends the requested pixel size (`16x16_`, `20x20_`, `24x24_`, `32x32_`) to the lowercase logical name (or uses unprefixed filenames for `group_100.png`..`group_600.png` and `warning_external_link.png`).

---

## 3. Core Logical Icon Mappings (Quick Reference)

### 3.1 Player / Client States (Tree View & Status)

| Logical State / Concept | `settings.ini` Key | `default_colored_2014.zip` | `default.zip` (Legacy Raster) |
| :--- | :--- | :--- | :--- |
| **Player Talking (Normal)** | `PLAYER_ON` | `player_on.svg` | `16x16_player_on.png`, `24x24_player_on.png`, `32x32_player_on.png` |
| **Player Silent / Not Talking** | `PLAYER_OFF` | `player_off.svg` | `16x16_player_off.png`, `24x24_player_off.png`, `32x32_player_off.png` |
| **Player Whispering** | `PLAYER_WHISPER` | `player_whisper.svg` | `16x16_player_whisper.png`, `24x24_player_whisper.png`, `32x32_player_whisper.png` |
| **Channel Commander Talking** | `PLAYER_COMMANDER_ON` | `player_commander_on.svg` | `16x16_player_commander_on.png` |
| **Channel Commander Silent** | `PLAYER_COMMANDER_OFF` | `player_commander_off.svg` | `16x16_player_commander_off.png` |
| **Channel Commander Badge/Toggle** | `CHANNEL_COMMANDER` | `channel_commander.svg` | `16x16_channel_commander.png` |
| **Away Status** | `AWAY` | `away.svg` | `16x16_away.png`, `24x24_away.png`, `32x32_away.png` |
| **Input (Microphone) Muted** | `INPUT_MUTED` | `input_muted.svg` | `16x16_input_muted.png`, `24x24_input_muted.png`, `32x32_input_muted.png` |
| **Input Muted Locally (By User)** | `INPUT_MUTED_LOCAL` | `input_muted_local.svg` | `16x16_input_muted_local.png`, `24x24_input_muted_local.png`, `32x32_input_muted_local.png` |
| **Output (Sound/Headset) Muted** | `OUTPUT_MUTED` | `output_muted.svg` | `16x16_output_muted.png`, `24x24_output_muted.png`, `32x32_output_muted.png` |
| **Hardware Input Disabled/Muted** | `HARDWARE_INPUT_MUTED` | `hardware_input_muted.svg` | `16x16_hardware_input_muted.png`, `24x24_hardware_input_muted.png`, `32x32_hardware_input_muted.png` |
| **Hardware Output Disabled/Muted** | `HARDWARE_OUTPUT_MUTED` | `hardware_output_muted.svg` | `16x16_hardware_output_muted.png`, `24x24_hardware_output_muted.png`, `32x32_hardware_output_muted.png` |
| **Priority Speaker / Is Talker** | `IS_TALKER` | `is_talker.svg` | `16x16_is_talker.png` |
| **Requesting Talk Power** | `TALK_POWER_REQUEST` | `talk_power_request.svg` | `16x16_request_talk_power.png` |
| **On Whisper List Indicator** | `ON_WHISPERLIST` | `on_whisperlist.svg` | `16x16_on_whisperlist.png` |
| **Player Text Chat Indicator** | `PLAYER_CHAT` | `player_chat.svg` | `16x16_player_chat.png`, `32x32_player_chat.png` |

### 3.2 Channel States & Overlay Badges

| Logical State / Concept | `settings.ini` Key | `default_colored_2014.zip` | `default.zip` (Legacy Raster) |
| :--- | :--- | :--- | :--- |
| **Channel Open (Unsubscribed)** | `CHANNEL_GREEN` | `channel_green.svg` | `16x16_channel_green.png` |
| **Channel Open (Subscribed)** | `CHANNEL_GREEN_SUBSCRIBED` | `channel_green_subscribed.svg` | `16x16_channel_green_subscribed.png` |
| **Channel Password-Protected (Unsubscribed)** | `CHANNEL_YELLOW` | `channel_yellow.svg` | `16x16_channel_yellow.png` |
| **Channel Password-Protected (Subscribed)** | `CHANNEL_YELLOW_SUBSCRIBED` | `channel_yellow_subscribed.svg` | `16x16_channel_yellow_subscribed.png` |
| **Channel Full / No Access (Unsubscribed)** | `CHANNEL_RED` | `channel_red.svg` | `16x16_channel_red.png` |
| **Channel Full / No Access (Subscribed)** | `CHANNEL_RED_SUBSCRIBED` | `channel_red_subscribed.svg` | `16x16_channel_red_subscribed.png` |
| **Private Channel** | `CHANNEL_PRIVATE` | `channel_private.svg` | `16x16_channel_private.png` |
| **Default Channel Badge (Home)** | `DEFAULT` / `MAKE_CURRENT_CHANNEL_DEFAULT` | `default.svg` / `channel_default.svg` | `16x16_default.png` |
| **Moderated Channel Badge** | `MODERATED` | `moderated.svg` | `16x16_moderated.png` |
| **Music Codec (Opus Music) Badge** | `MUSIC` | `music.svg` | `16x16_music.png` |
| **Password Protected Badge** | `REGISTER` / `SECURITY` | `register.svg` / `security.svg` | `16x16_register.png` / `16x16_security.png` |
| **Virtual Server Root (Green)** | `SERVER_GREEN` | `server_green.svg` | `16x16_server_green.png` |

### 3.3 Main Toolbar Actions

| Toolbar Action | `settings.ini` Key | `default_colored_2014.zip` | `default.zip` (Legacy Raster) |
| :--- | :--- | :--- | :--- |
| **Connect** | `CONNECT` / `CONNECT_NEW_TAB` | `connect.svg` | `16x16_connect.png`, `24x24_connect.png` |
| **Disconnect** | `DISCONNECT` / `DISCONNECT_ALL` | `disconnect.svg` | `16x16_disconnect.png`, `24x24_disconnect.png` |
| **Bookmarks** | `BOOKMARK_MANAGER` | `bookmark_manager.svg` | `16x16_bookmark_manager.png`, `24x24_bookmark_manager.png` |
| **Activate Microphone** | `ACTIVATE_MICROPHONE` | `activate_microphone.svg` | `24x24_activate_microphone.png` |
| **Mute Microphone** | `INPUT_MUTED` | `input_muted.svg` | `16x16_input_muted.png`, `24x24_input_muted.png` |
| **Mute Speakers/Headphones** | `OUTPUT_MUTED` | `output_muted.svg` | `16x16_output_muted.png`, `24x24_output_muted.png` |
| **Toggle Away** | `AWAY` | `away.svg` | `16x16_away.png`, `24x24_away.png` |
| **Subscribe to All Channels** | `SUBSCRIBE_TO_ALL_CHANNELS` | `subscribe_to_all_channels.svg` | `16x16_subscribe_to_all_channels.png`, `24x24_subscribe_to_all_channels.png` |
| **Unsubscribe from All Channels** | `UNSUBSCRIBE_FROM_ALL_CHANNELS` | `unsubscribe_from_all_channels.svg` | `16x16_unsubscribe_from_all_channels.png`, `24x24_unsubscribe_from_all_channels.png` |
| **Settings / Options** | `SETTINGS` | `settings.svg` | `16x16_settings.png` |
| **File Transfers** | `FILETRANSFER` | `filetransfer.svg` | `16x16_filetransfer.png`, `24x24_filetransfer.png` |

### 3.4 Built-In Server & Channel Group Icons (`icon_id` 100–600)

In TeamSpeak 3, built-in group icons use numeric IDs `100`, `200`, `300`, `500`, and `600` (configured via permission `i_icon_id`):

| `i_icon_id` | `settings.ini` Key | `default_colored_2014.zip` | `default.zip` | Standard Group Assignment |
| ---: | :--- | :--- | :--- | :--- |
| `100` | `GROUP_100` | `group_100.svg` | `group_100.png` | Channel Admin (C) |
| `200` | `GROUP_200` | `group_200.svg` | `group_200.png` | Operator (O) |
| `300` | `GROUP_300` | `group_300.svg` | `group_300.png` | Server Admin (S) |
| `500` | `GROUP_500` | `group_500.svg` | `group_500.png` | Guest / Server Group 500 |
| `600` | `GROUP_600` | `group_600.svg` | `group_600.png` | Voice / Channel Group 600 |

---

## 4. Complete `settings.ini` Mapping Table (All 213 Keys)

Below is the exact, complete `[gfxfiles]` mapping table from `default_colored_2014.zip` (and `default_mono_2014.zip`):

| Logical Key (`[gfxfiles]`) | Target Filename (`default_colored_2014.zip`) | Size (Colored SVG) | Size (Mono SVG) |
| :--- | :--- | ---: | ---: |
| `3D_SOUND` | `3d_sound.svg` | 1,295 B | 5,267 B |
| `3D_SOUND_ME` | `3d_sound_me.svg` | 2,166 B | 7,422 B |
| `3D_SOUND_OTHER` | `3d_sound_user.svg` | 2,228 B | 7,922 B |
| `ABOUT` | `about.svg` | 1,869 B | 2,817 B |
| `ACTIVATE_MICROPHONE` | `activate_microphone.svg` | 1,882 B | 2,932 B |
| `ADD` | `add.svg` | 1,274 B | 7,783 B |
| `ADD_FOE` | `add_foe.svg` | 2,481 B | 2,644 B |
| `ADD_FOLDER` | `add_folder.svg` | 1,571 B | 5,746 B |
| `ADD_FRIEND` | `add_friend.svg` | 2,484 B | 2,682 B |
| `ADDON` | `addon.svg` | 2,444 B | 1,923 B |
| `ADDON_BROWSER` | `browse-addon-online.svg` | 12,790 B | 9,224 B |
| `ADDON_COLLECTION` | `addon-collection.svg` | 2,571 B | 1,657 B |
| `ADDON_ICONPACK` | `icon-pack.svg` | 126,510 B | 58,663 B |
| `ADDON_SOUNDPACK` | `sound-pack.svg` | 5,036 B | 3,409 B |
| `ADDON_THEME` | `theme.svg` | 5,300 B | 3,432 B |
| `ADDON_TRANSLATION` | `translation.svg` | 4,524 B | 3,233 B |
| `APPLY` | `apply.svg` | 1,085 B | 1,608 B |
| `AWAY` | `away.svg` | 2,262 B | 5,981 B |
| `BAN_CLIENT` | `ban_client.svg` | 2,457 B | 2,137 B |
| `BAN_LIST` | `ban_list.svg` | 3,431 B | 7,893 B |
| `BOOKMARK_ADD` | `bookmark_add.svg` | 2,501 B | 2,577 B |
| `BOOKMARK_ADD_FOLDER` | `bookmark_add_folder.svg` | 2,326 B | 7,684 B |
| `BOOKMARK_DUPLICATE` | `bookmark_duplicate.svg` | 2,731 B | 4,785 B |
| `BOOKMARK_MANAGER` | `bookmark_manager.svg` | 866 B | 1,426 B |
| `BOOKMARK_REMOVE` | `bookmark_remove.svg` | 1,319 B | 2,079 B |
| `BOOKMARK_SET_FOR_ALL` | `default_for_all_bookmarks.svg` | 1,962 B | 2,243 B |
| `BROKEN_IMAGE` | `broken_image.svg` | 930 B | 943 B |
| `CAPTURE` | `capture.svg` | 1,114 B | 2,843 B |
| `CHANGE_NICKNAME` | `change_nickname.svg` | 1,097 B | 1,334 B |
| `CHANGELOG` | `changelog.svg` | 1,754 B | 6,742 B |
| `CHANNEL_CHAT` | `channel_chat.svg` | 1,379 B | 4,987 B |
| `CHANNEL_COLLAPSE_ALL` | `channel_collapse_all.svg` | 2,240 B | 1,809 B |
| `CHANNEL_COLLAPSE_SUB` | `channel_collapse_all.svg` | 2,240 B | 1,809 B |
| `CHANNEL_COMMANDER` | `channel_commander.svg` | 1,853 B | 1,491 B |
| `CHANNEL_CREATE` | `channel_create.svg` | 2,091 B | 1,755 B |
| `CHANNEL_CREATE_SPACER` | `channel_create.svg` | 2,091 B | 1,755 B |
| `CHANNEL_CREATE_SUB` | `channel_create_sub.svg` | 2,616 B | 6,112 B |
| `CHANNEL_DELETE` | `channel_delete.svg` | 2,101 B | 5,893 B |
| `CHANNEL_EDIT` | `channel_edit.svg` | 3,938 B | 7,800 B |
| `CHANNEL_EXPAND_ALL` | `channel_expand_all.svg` | 1,707 B | 5,847 B |
| `CHANNEL_EXPAND_SUB` | `channel_expand_all.svg` | 1,707 B | 5,847 B |
| `CHANNEL_GREEN` | `channel_green.svg` | 867 B | 5,049 B |
| `CHANNEL_GREEN_SUBSCRIBED` | `channel_green_subscribed.svg` | 1,379 B | 5,558 B |
| `CHANNEL_PRIVATE` | `channel_private.svg` | 1,945 B | 6,042 B |
| `CHANNEL_RED` | `channel_red.svg` | 1,307 B | 5,390 B |
| `CHANNEL_RED_SUBSCRIBED` | `channel_red_subscribed.svg` | 1,446 B | 5,899 B |
| `CHANNEL_SWITCH` | `channel_switch.svg` | 1,496 B | 6,614 B |
| `CHANNEL_YELLOW` | `channel_yellow.svg` | 1,272 B | 5,361 B |
| `CHANNEL_YELLOW_SUBSCRIBED` | `channel_yellow_subscribed.svg` | 1,864 B | 5,870 B |
| `CHECK_UPDATE` | `check_update.svg` | 1,427 B | 2,633 B |
| `CLIENT_HIDE` | `client_hide.svg` | 1,116 B | 1,510 B |
| `CLIENT_SHOW` | `client_show.svg` | 1,318 B | 1,444 B |
| `CLOSE_BUTTON` | `close_button.svg` | 1,361 B | 2,139 B |
| `COMPLAINT_LIST` | `complaint_list.svg` | 2,605 B | 7,865 B |
| `CONNECT` | `connect.svg` | 1,961 B | 2,772 B |
| `CONNECT_NEW_TAB` | `connect.svg` | 1,961 B | 2,772 B |
| `CONTACT` | `edit_friend_foe_status.svg` | 2,228 B | 2,331 B |
| `COPY` | `copy.svg` | 2,383 B | 3,438 B |
| `COPY_URL` | `copy_url.svg` | 9,009 B | 7,662 B |
| `DEFAULT` | `default.svg` | 1,443 B | 8,460 B |
| `DELETE` | `delete.svg` | 1,685 B | 2,142 B |
| `DELETE_AVATAR` | `delete_avatar.svg` | 2,047 B | 8,887 B |
| `DELETE_AVATAR_OTHER` | `delete_avatar.svg` | 2,047 B | 8,887 B |
| `DISCONNECT` | `disconnect.svg` | 2,297 B | 3,145 B |
| `DISCONNECT_ALL` | `disconnect.svg` | 2,297 B | 3,145 B |
| `DOWN` | `down.svg` | 584 B | 869 B |
| `DOWN_ARROW` | `down.svg` | 584 B | 869 B |
| `DOWNLOAD` | `download.svg` | 4,327 B | 2,009 B |
| `EDIT` | `edit.svg` | 2,345 B | 2,552 B |
| `EDIT_FRIEND_FOE_STATUS` | `edit_friend_foe_status.svg` | 2,228 B | 2,331 B |
| `EMOTICON` | `emoticon.svg` | 1,892 B | 903 B |
| `ERROR` | `error.svg` | 1,629 B | 1,498 B |
| `FILE_HOME` | `file_home.svg` | 1,476 B | 1,247 B |
| `FILE_REFRESH` | `file_refresh.svg` | 1,665 B | 5,435 B |
| `FILE_UP` | `folder_up.svg` | 1,775 B | 1,493 B |
| `FILETRANSFER` | `filetransfer.svg` | 1,896 B | 7,405 B |
| `FILTER_CLEAR` | `delete.svg` | 1,685 B | 2,142 B |
| `FIND` | `find.svg` | 1,558 B | 5,823 B |
| `FOLDER` | `folder.svg` | 1,141 B | 5,027 B |
| `GROUP_100` | `group_100.svg` | 2,541 B | 2,725 B |
| `GROUP_200` | `group_200.svg` | 2,469 B | 2,723 B |
| `GROUP_300` | `group_300.svg` | 2,756 B | 2,959 B |
| `GROUP_500` | `group_500.svg` | 2,785 B | 3,009 B |
| `GROUP_600` | `group_600.svg` | 1,707 B | 1,905 B |
| `GUISETUP` | `guisetup.svg` | 1,347 B | 1,549 B |
| `HARDWARE_INPUT_MUTED` | `hardware_input_muted.svg` | 1,879 B | 1,746 B |
| `HARDWARE_OUTPUT_MUTED` | `hardware_output_muted.svg` | 2,117 B | 2,096 B |
| `HOSTER_BUTTON` | `hoster_button.svg` | 2,457 B | 1,502 B |
| `HOTKEYS` | `hotkeys.svg` | 1,648 B | 1,544 B |
| `ICONSVIEW` | `iconsview.svg` | 1,945 B | 1,926 B |
| `ICONVIEWER` | `iconviewer.svg` | 1,719 B | 1,444 B |
| `IDENTITY_ADD` | `add_friend.svg` | 2,484 B | 2,682 B |
| `IDENTITY_DEFAULT` | `identity_default.svg` | 2,809 B | 2,440 B |
| `IDENTITY_EXPORT` | `identity_export.svg` | 1,608 B | 1,320 B |
| `IDENTITY_IMPORT` | `identity_import.svg` | 1,956 B | 2,234 B |
| `IDENTITY_MANAGER` | `identity_manager.svg` | 2,980 B | 3,123 B |
| `IDENTITY_REMOVE` | `remove_friend.svg` | 2,681 B | 2,364 B |
| `INFO` | `info.svg` | 2,276 B | 2,594 B |
| `INPUT_MUTED` | `input_muted.svg` | 2,093 B | 2,766 B |
| `INPUT_MUTED_LOCAL` | `input_muted_local.svg` | 2,601 B | 2,141 B |
| `INVITE_BUDDY` | `invite_buddy.svg` | 3,098 B | 3,418 B |
| `IS_TALKER` | `is_talker.svg` | 1,863 B | 2,524 B |
| `KICK_FROM_CHANNEL` | `kick_channel.svg` | 1,945 B | 1,340 B |
| `KICK_FROM_SERVER` | `kick_server.svg` | 2,235 B | 1,527 B |
| `LISTVIEW` | `listview.svg` | 3,119 B | 3,391 B |
| `LOADING_IMAGE` | `loading_image.svg` | 10,140 B | 7,789 B |
| `MAKE_CURRENT_CHANNEL_DEFAULT` | `channel_default.svg` | 1,917 B | 2,826 B |
| `MESSAGE_INCOMING` | `message_incoming.svg` | 1,051 B | 1,013 B |
| `MESSAGE_INFO` | `message_info.svg` | 2,017 B | 1,700 B |
| `MESSAGE_OUTGOING` | `message_outgoing.svg` | 783 B | 722 B |
| `MESSAGES` | `messages.svg` | 3,605 B | 3,681 B |
| `MODERATED` | `moderated.svg` | 997 B | 1,167 B |
| `MOVE_CLIENT_TO_OWN_CHANNEL` | `move_client_to_own_channel.svg` | 1,840 B | 1,983 B |
| `MUSIC` | `music.svg` | 953 B | 1,162 B |
| `MYTS_ACCOUNT` | `user-account.svg` | 4,904 B | 6,089 B |
| `MYTS_ACCOUNT_ERROR` | `conflict-icon.svg` | 2,301 B | 2,144 B |
| `MYTS_ACCOUNT_OFFLINE` | `sync-disable.svg` | 2,555 B | 1,949 B |
| `MYTS_ACCOUNT_ONLINE` | `sync-enable.svg` | 2,785 B | 2,136 B |
| `NEW_CHAT` | `new_chat.svg` | 1,254 B | 1,441 B |
| `NOTIFICATIONS` | `notifications.svg` | 1,619 B | 1,780 B |
| `OFFLINE_MESSAGES` | `offline_messages.svg` | 1,367 B | 1,519 B |
| `ON_WHISPERLIST` | `on_whisperlist.svg` | 2,816 B | 2,911 B |
| `OUTPUT_MUTED` | `output_muted.svg` | 4,429 B | 9,332 B |
| `PERMISSION_OVERVIEW` | `permission_overview.svg` | 1,992 B | 1,770 B |
| `PERMISSIONS_CHANNEL_CLIENT` | `permission_client.svg` | 3,159 B | 2,840 B |
| `PERMISSIONS_CHANNEL_GROUPS` | `permission_channel.svg` | 2,154 B | 1,942 B |
| `PERMISSIONS_CHANNELS` | `permission_channel.svg` | 2,154 B | 1,942 B |
| `PERMISSIONS_CLIENTS` | `permission_client.svg` | 3,159 B | 2,840 B |
| `PERMISSIONS_SERVER_GROUPS` | `permission_server_groups.svg` | 3,246 B | 2,587 B |
| `PHONETICSNICKNAME` | `phoneticsnickname.svg` | 1,229 B | 1,353 B |
| `PING_1` | `ping_1.svg` | 711 B | 714 B |
| `PING_2` | `ping_2.svg` | 880 B | 882 B |
| `PING_3` | `ping_3.svg` | 1,052 B | 1,044 B |
| `PING_4` | `ping_4.svg` | 1,298 B | 1,271 B |
| `PING_CALCULATING` | `ping_calculating.svg` | 721 B | 946 B |
| `PING_DISCONNECTED` | `ping_disconnected.svg` | 722 B | 714 B |
| `PLAY` | `play.svg` | 808 B | 992 B |
| `PLAYBACK` | `volume.svg` | 2,011 B | 2,561 B |
| `PLAYER_CHAT` | `player_chat.svg` | 1,753 B | 1,582 B |
| `PLAYER_COMMANDER_OFF` | `player_commander_off.svg` | 2,754 B | 2,113 B |
| `PLAYER_COMMANDER_ON` | `player_commander_on.svg` | 2,753 B | 2,113 B |
| `PLAYER_OFF` | `player_off.svg` | 3,064 B | 2,074 B |
| `PLAYER_ON` | `player_on.svg` | 3,000 B | 2,058 B |
| `PLAYER_WHISPER` | `player_whisper.svg` | 2,669 B | 1,440 B |
| `PLUGINS` | `plugins.svg` | 1,604 B | 1,792 B |
| `POKE` | `poke.svg` | 1,573 B | 1,488 B |
| `PRESENT` | `present.svg` | 1,406 B | 1,582 B |
| `QUIT` | `delete.svg` | 1,685 B | 2,142 B |
| `RECORDING_START` | `recording_start.svg` | 978 B | 1,113 B |
| `RECORDING_STOP` | `recording_stop.svg` | 762 B | 976 B |
| `REFRESH` | `refresh.svg` | 951 B | 1,156 B |
| `REGISTER` | `register.svg` | 1,592 B | 1,708 B |
| `RELOAD` | `reload.svg` | 2,032 B | 1,651 B |
| `REMOVE_FOE` | `remove_foe.svg` | 2,654 B | 2,369 B |
| `REMOVE_FRIEND` | `remove_friend.svg` | 2,681 B | 2,364 B |
| `RENAME` | `change_nickname.svg` | 1,097 B | 1,334 B |
| `SEARCH` | `find.svg` | 1,558 B | 5,823 B |
| `SEARCH_CLEAR` | `delete.svg` | 1,685 B | 2,142 B |
| `SEARCH_HIDE` | `delete.svg` | 1,685 B | 2,142 B |
| `SECURITY` | `security.svg` | 944 B | 1,040 B |
| `SELECTFOLDER` | `selectfolder.svg` *(missing in SVG packs; falls back to `24x24_selectfolder.png`)* | — | — |
| `SEND_COMPLAINT` | `send_complaint.svg` | 3,896 B | 3,055 B |
| `SERVER_GREEN` | `server_green.svg` | 1,622 B | 1,973 B |
| `SERVER_LOG` | `server_log.svg` | 2,434 B | 3,290 B |
| `SERVER_QUERY` | `server_query.svg` | 3,270 B | 3,637 B |
| `SETTINGS` | `settings.svg` | 2,537 B | 2,699 B |
| `SORT_BY_NAME` | `sort_by_name.svg` | 1,347 B | 1,513 B |
| `SOUNDPACK` | `soundpack.svg` | 1,095 B | 1,320 B |
| `STOP` | `stop.svg` *(missing in SVG packs; falls back to `16x16_stop.png`)* | — | — |
| `SUBSCRIBE_MODE` | `subscribe_mode.svg` | 2,475 B | 1,818 B |
| `SUBSCRIBE_TO_ALL_CHANNELS` | `subscribe_to_all_channels.svg` | 2,803 B | 1,671 B |
| `SUBSCRIBE_TO_CHANNEL` | `channel_green_subscribed.svg` | 1,379 B | 5,558 B |
| `SUBSCRIBE_TO_CHANNEL_FAMILY` | `subscribe_to_channel_family.svg` | 2,117 B | 1,782 B |
| `SWITCH_ADVANCED` | `switch_advanced.svg` | 1,470 B | 1,288 B |
| `SWITCH_STANDARD` | `switch_standard.svg` | 1,448 B | 1,282 B |
| `SYNC` | `sync-icon.svg` | 3,245 B | 2,376 B |
| `SYNC_DISABLE` | `sync-disable.svg` | 2,555 B | 1,949 B |
| `SYNC_ENABLE` | `sync-enable.svg` | 2,785 B | 2,136 B |
| `TAB_CLOSE_BUTTON` | `tab_close_button.svg` | 1,353 B | 1,316 B |
| `TALK_POWER_CANCEL_REQUEST` | `talk_power_request_cancel.svg` | 2,151 B | 2,579 B |
| `TALK_POWER_GRANT` | `talk_power_grant.svg` | 2,014 B | 2,939 B |
| `TALK_POWER_GRANT_NEXT` | `talk_power_grant_next.svg` | 2,347 B | 1,925 B |
| `TALK_POWER_REQUEST` | `talk_power_request.svg` | 2,181 B | 1,915 B |
| `TALK_POWER_REVOKE` | `talk_power_revoke.svg` | 2,242 B | 1,543 B |
| `TALK_POWER_REVOKE_ALL_GRANT_NEXT` | `talk_power_revoke_all_grant_next.svg` | 2,383 B | 1,851 B |
| `TEMP_SERVER_PASSWORD` | `temp_server_password.svg` | 31,358 B | 1,279 B |
| `TEMP_SERVER_PASSWORD_ADD` | `temp_server_password_add.svg` | 3,922 B | 3,530 B |
| `TEXTFORMAT` | `textformat.svg` | 1,339 B | 1,242 B |
| `TEXTFORMAT_BOLD` | `textformat_bold.svg` | 1,972 B | 1,521 B |
| `TEXTFORMAT_FOREGROUND` | `textformat_foreground.svg` | 1,304 B | 1,227 B |
| `TEXTFORMAT_ITALIC` | `textformat_italic.svg` | 608 B | 587 B |
| `TEXTFORMAT_UNDERLINE` | `textformat_underline.svg` | 1,010 B | 998 B |
| `TOGGLE_BLOCK_INCOMING_WHISPERS` | `toggle_whisper.svg` | 1,811 B | 2,631 B |
| `TOGGLE_SERVER_QUERY_CLIENTS` | `toggle_server_query_clients.svg` | 1,633 B | 1,945 B |
| `TOKEN` | `token.svg` | 1,166 B | 1,424 B |
| `TOKEN_USE` | `token_use.svg` | 1,388 B | 1,574 B |
| `UNSUBSCRIBE_FROM_ALL_CHANNELS` | `unsubscribe_from_all_channels.svg` | 1,700 B | 1,046 B |
| `UNSUBSCRIBE_FROM_CHANNEL` | `channel_unsubscribed.svg` | 1,106 B | 5,049 B |
| `UNSUBSCRIBE_FROM_CHANNEL_FAMILY` | `unsubscribe_from_channel_family.svg` | 1,871 B | 1,345 B |
| `UNSUBSCRIBE_MODE` | `unsubscribe_mode.svg` | 2,147 B | 1,358 B |
| `UP` | `up.svg` | 829 B | 888 B |
| `UP_ARROW` | `up.svg` | 829 B | 888 B |
| `UPLOAD` | `upload.svg` | 4,323 B | 1,195 B |
| `UPLOAD_AVATAR` | `upload_avatar.svg` | 2,221 B | 1,947 B |
| `URLCATCHER` | `urlcatcher.svg` | 6,587 B | 6,562 B |
| `VIRTUALSERVER_EDIT` | `virtualserver_edit.svg` | 2,122 B | 2,453 B |
| `VOLUME` | `volume.svg` | 2,011 B | 2,561 B |
| `WARNING` | `warning.svg` | 1,631 B | 907 B |
| `WARNING_EXTERNAL_LINK` | `warning_external_link.svg` | 3,411 B | 2,335 B |
| `WEBLIST` | `weblist.svg` | 2,998 B | 3,065 B |
| `WHISPER` | `whisper.svg` | 2,049 B | 1,773 B |
| `WHISPERHISTORY` | `whisperlists.svg` | 3,488 B | 3,396 B |
| `WHISPERLISTS` | `whisperlists.svg` | 3,488 B | 3,396 B |

### Unmapped / Extra SVG Files in `default_colored_2014.zip`
The following SVG files exist inside `default_colored_2014.zip` (and `default_mono_2014.zip` except `warning_error.svg`) without an explicit entry in `settings.ini` (or because their key was redirected in `settings.ini`):
- `channel_groups.svg`
- `channel_subscribed.svg`
- `clear.svg`
- `contact.svg` (`CONTACT` maps to `edit_friend_foe_status.svg`)
- `favorite.svg`
- `select_all.svg`
- `send.svg`
- `subscribe_to_channel.svg` (`SUBSCRIBE_TO_CHANNEL` maps to `channel_green_subscribed.svg`)
- `warning_error.svg` (only in `default_colored_2014.zip`)

Conversely, two keys in `settings.ini` point to `.svg` files that are **absent** from both 2014 SVG zips and therefore rely on `FALLBACK=true` to load from `default.zip`:
- `SELECTFOLDER = selectfolder.svg` -> falls back to `24x24_selectfolder.png` in `default.zip`.
- `STOP = stop.svg` -> falls back to `16x16_stop.png` in `default.zip`.

---

## 5. Emoticons (`emoticons/emoticons.txt`)

Each icon pack includes an `emoticons/` directory with `emoticons.txt` mapping emoticon files to chat text shortcuts:

### `default_colored_2014.zip` & `default_mono_2014.zip` (10 SVG emoticons)
```ini
smile.svg = ":)"
smile.svg = ":-)"
laugh.svg = ":D"
laugh.svg = ":-D"
cool.svg = "8)"
cool.svg = "8-)"
twinkle.svg = ";)"
twinkle.svg = ";-)"
sad.svg = ":("
sad.svg = ":-("
angry.svg = ":C"
angry.svg = ":-C"
scream.svg = ":0"
scream.svg = ":-0"
skeptical.svg = ":/"
skeptical.svg = ":-/"
stunned.svg = ":x"
stunned.svg = ":-x"
tongue.svg = ":P"
tongue.svg = ":-P"
```

### `default.zip` (4 PNG emoticons)
```ini
cool.png = "8)"
sad.png = ":-("
smile.png = ":-)"
twinkle.png = ";-)"
```

---

## 6. Country Flags (`countries.zip`)

`countries.zip` contains 247 raster flag icons (`16x11` pixels, 24-bit RGB PNG) with no `settings.ini`. The client maps a client's `client_country` property (2-letter ISO country code, lowercased) directly to `<cc>.png`:

`ad`, `ae`, `af`, `ag`, `ai`, `al`, `am`, `an`, `ao`, `ar`, `as`, `at`, `au`, `aw`, `ax`, `az`, `ba`, `bb`, `bd`, `be`, `bf`, `bg`, `bh`, `bi`, `bj`, `bl`, `bm`, `bn`, `bo`, `br`, `bs`, `bt`, `bv`, `bw`, `by`, `bz`, `ca`, `cc`, `cd`, `cf`, `cg`, `ch`, `ci`, `ck`, `cl`, `cm`, `cn`, `co`, `cr`, `cs`, `cu`, `cv`, `cw`, `cx`, `cy`, `cz`, `de`, `dj`, `dk`, `dm`, `do`, `dz`, `ec`, `ee`, `eg`, `eh`, `er`, `es`, `et`, `fi`, `fj`, `fk`, `fm`, `fo`, `fr`, `ga`, `gb`, `gd`, `ge`, `gf`, `gg`, `gh`, `gi`, `gl`, `gm`, `gn`, `gp`, `gq`, `gr`, `gs`, `gt`, `gu`, `gw`, `gy`, `hk`, `hm`, `hn`, `hr`, `ht`, `hu`, `id`, `ie`, `il`, `im`, `in`, `io`, `iq`, `ir`, `is`, `it`, `je`, `jm`, `jo`, `jp`, `ke`, `kg`, `kh`, `ki`, `km`, `kn`, `kp`, `kr`, `kw`, `ky`, `kz`, `la`, `lb`, `lc`, `li`, `lk`, `lr`, `ls`, `lt`, `lu`, `lv`, `ly`, `ma`, `mc`, `md`, `me`, `mg`, `mh`, `mk`, `ml`, `mm`, `mn`, `mo`, `mp`, `mq`, `mr`, `ms`, `mt`, `mu`, `mv`, `mw`, `mx`, `my`, `mz`, `na`, `nc`, `ne`, `nf`, `ng`, `ni`, `nl`, `no`, `np`, `nr`, `nu`, `nz`, `om`, `pa`, `pe`, `pf`, `pg`, `ph`, `pk`, `pl`, `pm`, `pn`, `pr`, `ps`, `pt`, `pw`, `py`, `qa`, `re`, `ro`, `rs`, `ru`, `rw`, `sa`, `sb`, `sc`, `sd`, `se`, `sg`, `sh`, `si`, `sj`, `sk`, `sl`, `sm`, `sn`, `so`, `sr`, `st`, `sv`, `sy`, `sz`, `tc`, `td`, `tf`, `tg`, `th`, `tj`, `tk`, `tl`, `tm`, `tn`, `to`, `tr`, `tt`, `tv`, `tw`, `tz`, `ua`, `ug`, `uk`, `um`, `us`, `uy`, `uz`, `va`, `vc`, `ve`, `vg`, `vi`, `vn`, `vu`, `wf`, `ws`, `ye`, `yt`, `za`, `zm`, `zw`.
