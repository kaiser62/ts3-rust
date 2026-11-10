// Owned by Pool Agent 3 (arthoroy27@gmail.com)
// Sound pack player loading `C:\Program Files\TeamSpeak 3 Client\sound\default\settings.ini` + `.wav` cues
// and voice capture/playback state from `docs/re/gui/07-sound-packs.md`.

use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use cpal::traits::{DeviceTrait, HostTrait};
use rodio::{Decoder, OutputStream, Sink};

pub const DEFAULT_SOUND_PACK_DIR: &str = r"C:\Program Files\TeamSpeak 3 Client\sound\default";
pub const DEFAULT_SOUND_PACK_INI: &str =
    r"C:\Program Files\TeamSpeak 3 Client\sound\default\settings.ini";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SoundEvent {
    Connected,
    Disconnected,
    ConnectionLost,
    ServerError,
    ChannelSwitched,
    ChannelCreated,
    ChannelEdited,
    ChannelDeleted,
    UserJoinedServer,
    UserJoinedCurrentChannel,
    UserLeftCurrentChannel,
    UserSwitchedToCurrentChannel,
    UserPokedYou,
    ChatReceivedServer,
    ChatReceivedChannel,
    ChatReceivedPrivate,
    ChatSent,
    MicMuted,
    MicUnmuted,
    OutputMuted,
    OutputUnmuted,
    AwayActivated,
    AwayDeactivated,
    MicClickOn,
    MicClickOff,
}

impl SoundEvent {
    pub const ALL: [SoundEvent; 25] = [
        SoundEvent::Connected,
        SoundEvent::Disconnected,
        SoundEvent::ConnectionLost,
        SoundEvent::ServerError,
        SoundEvent::ChannelSwitched,
        SoundEvent::ChannelCreated,
        SoundEvent::ChannelEdited,
        SoundEvent::ChannelDeleted,
        SoundEvent::UserJoinedServer,
        SoundEvent::UserJoinedCurrentChannel,
        SoundEvent::UserLeftCurrentChannel,
        SoundEvent::UserSwitchedToCurrentChannel,
        SoundEvent::UserPokedYou,
        SoundEvent::ChatReceivedServer,
        SoundEvent::ChatReceivedChannel,
        SoundEvent::ChatReceivedPrivate,
        SoundEvent::ChatSent,
        SoundEvent::MicMuted,
        SoundEvent::MicUnmuted,
        SoundEvent::OutputMuted,
        SoundEvent::OutputUnmuted,
        SoundEvent::AwayActivated,
        SoundEvent::AwayDeactivated,
        SoundEvent::MicClickOn,
        SoundEvent::MicClickOff,
    ];

    /// Primary INI key in `settings.ini` `[soundfiles]` section corresponding to this event.
    pub fn ini_key(self) -> &'static str {
        match self {
            SoundEvent::Connected => "CONNECTION_CONNECTED",
            SoundEvent::Disconnected => "CONNECTION_DISCONNECTED",
            SoundEvent::ConnectionLost => "CONNECTION_LOST_CONNECTION",
            SoundEvent::ServerError => "SERVER_ERROR",
            SoundEvent::ChannelSwitched => "YOU_SWITCHED_CHANNEL",
            SoundEvent::ChannelCreated => "CHANNEL_CREATED_BY_YOU",
            SoundEvent::ChannelEdited => "CHANNEL_EDITED_CURRENT_BY_YOU",
            SoundEvent::ChannelDeleted => "CHANNEL_DELETED_BY_YOU",
            SoundEvent::UserJoinedServer => "CLIENT_CONNECTION_CONNECTED_SERVER",
            SoundEvent::UserJoinedCurrentChannel => "CLIENT_CONNECTION_CONNECTED_CURRENT_CHANNEL",
            SoundEvent::UserLeftCurrentChannel => "CLIENT_CONNECTION_DISCONNECTED_CURRENT_CHANNEL",
            SoundEvent::UserSwitchedToCurrentChannel => "CLIENT_SWITCHED_TO_CURRENT_CHANNEL_STAYS",
            SoundEvent::UserPokedYou => "OTHER_RECEIVED_POKE",
            SoundEvent::ChatReceivedServer => "CHAT_RECEIVED_MESSAGE_SERVER",
            SoundEvent::ChatReceivedChannel => "CHAT_RECEIVED_MESSAGE_CHANNEL",
            SoundEvent::ChatReceivedPrivate => "CHAT_RECEIVED_MESSAGE_CLIENT",
            SoundEvent::ChatSent => "CHAT_SENT_MESSAGE_SERVER",
            SoundEvent::MicMuted => "SOUND_CAPTURE_MUTED",
            SoundEvent::MicUnmuted => "SOUND_CAPTURE_UNMUTED",
            SoundEvent::OutputMuted => "SOUND_PLAYBACK_MUTED",
            SoundEvent::OutputUnmuted => "SOUND_PLAYBACK_UNMUTED",
            SoundEvent::AwayActivated => "STATUS_SET_AWAY",
            SoundEvent::AwayDeactivated => "STATUS_SET_PRESENT",
            SoundEvent::MicClickOn => "SPECIAL_MIC_CLICK_SELF_ON",
            SoundEvent::MicClickOff => "SPECIAL_MIC_CLICK_SELF_OFF",
        }
    }

    /// Canonical filename expected for this event.
    pub fn canonical_filename(self) -> &'static str {
        match self {
            SoundEvent::Connected => "connected.wav",
            SoundEvent::Disconnected => "disconnected.wav",
            SoundEvent::ConnectionLost => "connection_lost.wav",
            SoundEvent::ServerError => "error.wav",
            SoundEvent::ChannelSwitched => "channel_switched.wav",
            SoundEvent::ChannelCreated => "channel_created.wav",
            SoundEvent::ChannelEdited => "channel_edited.wav",
            SoundEvent::ChannelDeleted => "channel_deleted.wav",
            SoundEvent::UserJoinedServer => "user_joined.wav",
            SoundEvent::UserJoinedCurrentChannel => "user_joined_current_channel.wav",
            SoundEvent::UserLeftCurrentChannel => "user_left_current_channel.wav",
            SoundEvent::UserSwitchedToCurrentChannel => "user_switched_to_current_channel.wav",
            SoundEvent::UserPokedYou => "you_were_poked.wav",
            SoundEvent::ChatReceivedServer
            | SoundEvent::ChatReceivedChannel
            | SoundEvent::ChatReceivedPrivate => "chat_message_inbound.wav",
            SoundEvent::ChatSent => "chat_message_outbound.wav",
            SoundEvent::MicMuted => "mic_muted.wav",
            SoundEvent::MicUnmuted => "mic_activated.wav",
            SoundEvent::OutputMuted => "sound_muted.wav",
            SoundEvent::OutputUnmuted => "sound_resumed.wav",
            SoundEvent::AwayActivated => "away_activated.wav",
            SoundEvent::AwayDeactivated => "away_deactivated.wav",
            SoundEvent::MicClickOn => "mic_click_on.wav",
            SoundEvent::MicClickOff => "mic_click_off.wav",
        }
    }

    /// Fallback `.wav` filename on disk in `sound/default` when `settings.ini` uses `${clientType}`
    /// (resolved to `neutral`) or leaves the server-wide event unbound.
    pub fn fallback_filename(self) -> &'static str {
        match self {
            SoundEvent::UserJoinedServer => "neutral_connection_connected_currentchannel.wav",
            SoundEvent::UserJoinedCurrentChannel => {
                "neutral_connection_connected_currentchannel.wav"
            }
            SoundEvent::UserLeftCurrentChannel => {
                "neutral_connection_disconnected_currentchannel.wav"
            }
            SoundEvent::UserSwitchedToCurrentChannel => "neutral_switched_tocurrentchannel.wav",
            other => other.canonical_filename(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParsedSoundPackIni {
    pub name: String,
    pub version: String,
    pub author: String,
    pub readonly: bool,
    /// Raw key -> resolved `play("...")` filename (with `${clientType}` expanded to `"neutral"`).
    pub play_directives: HashMap<String, String>,
}

/// Extract the filename argument from a `play("...")` directive and expand `${clientType}` to `neutral`.
pub fn parse_play_directive(expr: &str) -> Option<String> {
    let trimmed = expr.trim();
    let rest = trimmed.strip_prefix("play(")?.strip_suffix(')')?.trim();
    let inner = rest
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(rest);
    if inner.is_empty() {
        return None;
    }
    Some(inner.replace("${clientType}", "neutral"))
}

/// Parse a TeamSpeak 3 sound pack `settings.ini` string into metadata and `play("...")` directives.
pub fn parse_settings_ini_str(content: &str) -> ParsedSoundPackIni {
    let mut name = "Default Sound Pack (Female)".to_string();
    let mut version = "1.0".to_string();
    let mut author = "TeamSpeak Systems GmbH".to_string();
    let mut readonly = true;
    let mut play_directives = HashMap::new();
    let mut section = String::new();

    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if let Some(sec) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            section = sec.trim().to_ascii_lowercase();
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let key = k.trim();
        let val = v.trim();
        match section.as_str() {
            "info" => match key {
                "name" => name = val.to_string(),
                "version" => version = val.to_string(),
                "author" => author = val.to_string(),
                "readonly" => readonly = val == "1" || val.eq_ignore_ascii_case("true"),
                _ => {}
            },
            "soundfiles" => {
                if let Some(wav_file) = parse_play_directive(val) {
                    play_directives.insert(key.to_string(), wav_file);
                }
            }
            _ => {}
        }
    }

    ParsedSoundPackIni {
        name,
        version,
        author,
        readonly,
        play_directives,
    }
}

pub struct SoundManager {
    pub enabled: bool,
    pub pack_name: String,
    pub wav_paths: HashMap<SoundEvent, PathBuf>,
}

impl Default for SoundManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SoundManager {
    pub fn new() -> Self {
        Self::from_pack_dir(Path::new(DEFAULT_SOUND_PACK_DIR))
    }

    /// Load a sound pack directory containing `settings.ini` and `.wav` files.
    pub fn from_pack_dir(pack_dir: &Path) -> Self {
        let ini_path = pack_dir.join("settings.ini");
        let parsed = std::fs::read_to_string(&ini_path)
            .map(|c| parse_settings_ini_str(&c))
            .unwrap_or_else(|_| ParsedSoundPackIni {
                name: "Default Sound Pack (Female)".to_string(),
                version: "1.0".to_string(),
                author: "TeamSpeak Systems GmbH".to_string(),
                readonly: true,
                play_directives: HashMap::new(),
            });

        let wav_paths = Self::resolve_wav_paths(pack_dir, &parsed.play_directives);

        Self {
            enabled: true,
            pack_name: parsed.name,
            wav_paths,
        }
    }

    /// Resolve every `SoundEvent` variant to its `.wav` path in `pack_dir`, consulting
    /// parsed `settings.ini` `play("...")` directives as well as canonical and `neutral_*` fallbacks.
    pub fn resolve_wav_paths(
        pack_dir: &Path,
        play_directives: &HashMap<String, String>,
    ) -> HashMap<SoundEvent, PathBuf> {
        let mut map = HashMap::with_capacity(SoundEvent::ALL.len());

        for event in SoundEvent::ALL {
            let canonical_path = pack_dir.join(event.canonical_filename());
            let ini_resolved_path = play_directives
                .get(event.ini_key())
                .map(|rel| pack_dir.join(rel));
            let fallback_path = pack_dir.join(event.fallback_filename());

            // 1. Prefer an existing file from `settings.ini` if present on disk.
            // 2. Otherwise, if canonical filename exists on disk (e.g. `user_joined.wav`), use it.
            // 3. Otherwise, if `fallback_filename()` exists on disk (e.g. `neutral_*.wav`), use it.
            // 4. Otherwise, fall back to the `settings.ini` path or canonical path.
            let chosen = if let Some(ref ini_p) = ini_resolved_path {
                if ini_p.exists() {
                    ini_p.clone()
                } else if canonical_path.exists() {
                    canonical_path
                } else if fallback_path.exists() {
                    fallback_path
                } else {
                    ini_p.clone()
                }
            } else if canonical_path.exists() {
                canonical_path
            } else if fallback_path.exists() {
                fallback_path
            } else {
                canonical_path
            };

            map.insert(event, chosen);
        }

        map
    }

    /// Return the resolved `.wav` path for `event`, if mapped.
    pub fn wav_path(&self, event: SoundEvent) -> Option<&Path> {
        self.wav_paths.get(&event).map(PathBuf::as_path)
    }

    /// Non-blocking `.wav` playback on a background thread using `rodio`.
    /// Gracefully returns if disabled, file is missing, or the host has no audio output device.
    pub fn play(&self, event: SoundEvent) {
        if !self.enabled {
            return;
        }
        let Some(path) = self.wav_paths.get(&event).cloned() else {
            return;
        };
        if !path.is_file() {
            return;
        }

        let _ = std::thread::Builder::new()
            .name("ts3-sound-play".to_string())
            .spawn(move || {
                let Ok((_stream, stream_handle)) = OutputStream::try_default() else {
                    return;
                };
                let Ok(file) = File::open(&path) else {
                    return;
                };
                let Ok(source) = Decoder::new(BufReader::new(file)) else {
                    return;
                };
                let Ok(sink) = Sink::try_new(&stream_handle) else {
                    return;
                };
                sink.append(source);
                sink.sleep_until_end();
            });
    }
}

/// Audio device enumeration and VAD / PTT level tracker using `cpal`.
#[derive(Debug, Clone)]
pub struct AudioDeviceManager {
    pub input_devices: Vec<String>,
    pub output_devices: Vec<String>,
    pub selected_input: Option<String>,
    pub selected_output: Option<String>,
    pub current_voice_level_db: f32,
    pub vad_threshold_db: f32,
    pub push_to_talk_active: bool,
}

impl Default for AudioDeviceManager {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioDeviceManager {
    pub fn new() -> Self {
        let mut mgr = Self {
            input_devices: Vec::new(),
            output_devices: Vec::new(),
            selected_input: None,
            selected_output: None,
            current_voice_level_db: -60.0,
            vad_threshold_db: -40.0,
            push_to_talk_active: false,
        };
        mgr.refresh_devices();
        mgr
    }

    /// Enumerate available input (capture) and output (playback) devices via `cpal`.
    pub fn refresh_devices(&mut self) {
        self.input_devices.clear();
        self.output_devices.clear();

        let host = cpal::default_host();

        if let Some(default_in) = host.default_input_device().and_then(|d| d.name().ok()) {
            self.selected_input.get_or_insert(default_in);
        }
        if let Some(default_out) = host.default_output_device().and_then(|d| d.name().ok()) {
            self.selected_output.get_or_insert(default_out);
        }

        if let Ok(inputs) = host.input_devices() {
            for dev in inputs {
                if let Ok(name) = dev.name() {
                    if !self.input_devices.contains(&name) {
                        self.input_devices.push(name);
                    }
                }
            }
        }

        if let Ok(outputs) = host.output_devices() {
            for dev in outputs {
                if let Ok(name) = dev.name() {
                    if !self.output_devices.contains(&name) {
                        self.output_devices.push(name);
                    }
                }
            }
        }
    }

    /// Update current microphone voice level (in dB, clamped to `[-60.0, 0.0]`).
    pub fn update_voice_level_db(&mut self, level_db: f32) {
        self.current_voice_level_db = level_db.clamp(-60.0, 0.0);
    }

    /// Returns `true` if either Push-To-Talk is held or the current voice level exceeds the VAD threshold.
    pub fn is_voice_detected(&self) -> bool {
        self.push_to_talk_active || self.current_voice_level_db >= self.vad_threshold_db
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_play_directive() {
        assert_eq!(
            parse_play_directive(r#"play("connected.wav")"#),
            Some("connected.wav".to_string())
        );
        assert_eq!(
            parse_play_directive(r#"play("${clientType}_connection_connected_currentchannel.wav")"#),
            Some("neutral_connection_connected_currentchannel.wav".to_string())
        );
        assert_eq!(parse_play_directive(""), None);
        assert_eq!(parse_play_directive(r#"say("Connected")"#), None);
    }

    #[test]
    fn test_parse_default_settings_ini_and_resolve_wavs() {
        let pack_dir = Path::new(DEFAULT_SOUND_PACK_DIR);
        let ini_path = Path::new(DEFAULT_SOUND_PACK_INI);

        if ini_path.exists() {
            let content = std::fs::read_to_string(ini_path).expect("read settings.ini");
            let parsed = parse_settings_ini_str(&content);
            assert_eq!(parsed.name, "Default Sound Pack (Female)");
            assert_eq!(parsed.version, "1.0");
            assert!(parsed.readonly);
            assert_eq!(
                parsed.play_directives.get("CONNECTION_CONNECTED").map(String::as_str),
                Some("connected.wav")
            );
            assert_eq!(
                parsed
                    .play_directives
                    .get("CLIENT_CONNECTION_CONNECTED_CURRENT_CHANNEL")
                    .map(String::as_str),
                Some("neutral_connection_connected_currentchannel.wav")
            );
        }

        let mgr = SoundManager::new();
        assert!(mgr.enabled);
        assert_eq!(mgr.pack_name, "Default Sound Pack (Female)");
        assert_eq!(mgr.wav_paths.len(), SoundEvent::ALL.len());

        let expected_files: &[(SoundEvent, &[&str])] = &[
            (SoundEvent::Connected, &["connected.wav"]),
            (SoundEvent::Disconnected, &["disconnected.wav"]),
            (SoundEvent::ConnectionLost, &["connection_lost.wav"]),
            (SoundEvent::ServerError, &["error.wav"]),
            (SoundEvent::ChannelSwitched, &["channel_switched.wav"]),
            (SoundEvent::ChannelCreated, &["channel_created.wav"]),
            (SoundEvent::ChannelEdited, &["channel_edited.wav"]),
            (SoundEvent::ChannelDeleted, &["channel_deleted.wav"]),
            (
                SoundEvent::UserJoinedServer,
                &[
                    "user_joined.wav",
                    "neutral_connection_connected_currentchannel.wav",
                ],
            ),
            (
                SoundEvent::UserJoinedCurrentChannel,
                &[
                    "user_joined_current_channel.wav",
                    "neutral_connection_connected_currentchannel.wav",
                ],
            ),
            (
                SoundEvent::UserLeftCurrentChannel,
                &[
                    "user_left_current_channel.wav",
                    "neutral_connection_disconnected_currentchannel.wav",
                ],
            ),
            (
                SoundEvent::UserSwitchedToCurrentChannel,
                &[
                    "user_switched_to_current_channel.wav",
                    "neutral_switched_tocurrentchannel.wav",
                ],
            ),
            (SoundEvent::UserPokedYou, &["you_were_poked.wav"]),
            (SoundEvent::ChatReceivedServer, &["chat_message_inbound.wav"]),
            (SoundEvent::ChatReceivedChannel, &["chat_message_inbound.wav"]),
            (SoundEvent::ChatReceivedPrivate, &["chat_message_inbound.wav"]),
            (SoundEvent::ChatSent, &["chat_message_outbound.wav"]),
            (SoundEvent::MicMuted, &["mic_muted.wav"]),
            (SoundEvent::MicUnmuted, &["mic_activated.wav"]),
            (SoundEvent::OutputMuted, &["sound_muted.wav"]),
            (SoundEvent::OutputUnmuted, &["sound_resumed.wav"]),
            (SoundEvent::AwayActivated, &["away_activated.wav"]),
            (SoundEvent::AwayDeactivated, &["away_deactivated.wav"]),
            (SoundEvent::MicClickOn, &["mic_click_on.wav"]),
            (SoundEvent::MicClickOff, &["mic_click_off.wav"]),
        ];

        for (event, valid_names) in expected_files {
            let path = mgr.wav_path(*event).expect("event mapped");
            let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            assert!(
                valid_names.contains(&file_name),
                "unexpected file_name {file_name:?} for {event:?}, expected one of {valid_names:?}"
            );
            if pack_dir.exists() {
                assert!(
                    path.exists(),
                    "expected resolved wav path to exist on disk for {event:?}: {}",
                    path.display()
                );
            }
        }
    }

    #[test]
    fn test_audio_device_manager_enumeration_and_vad() {
        let mut dev_mgr = AudioDeviceManager::new();
        assert_eq!(dev_mgr.vad_threshold_db, -40.0);
        dev_mgr.update_voice_level_db(-50.0);
        assert!(!dev_mgr.is_voice_detected());
        dev_mgr.update_voice_level_db(-30.0);
        assert!(dev_mgr.is_voice_detected());
        dev_mgr.update_voice_level_db(-55.0);
        dev_mgr.push_to_talk_active = true;
        assert!(dev_mgr.is_voice_detected());
    }
}

