// Owned by Pool Agent 7 (arthoroy29@gmail.com)

use std::collections::HashMap;

pub use ts3_proto::identity::IdentityData;

/// Configuration required to establish a TeamSpeak 3 client connection.
#[derive(Debug, Clone)]
pub struct ConnectConfig {
    pub server_addr: String,
    pub nickname: String,
    pub server_password: Option<String>,
    pub default_channel: Option<String>,
    pub default_channel_password: Option<String>,
    pub identity: IdentityData,
}

impl ConnectConfig {
    /// Creates a new [`ConnectConfig`] with the given server address, nickname, and identity.
    pub fn new(
        server_addr: impl Into<String>,
        nickname: impl Into<String>,
        identity: IdentityData,
    ) -> Self {
        Self {
            server_addr: server_addr.into(),
            nickname: nickname.into(),
            server_password: None,
            default_channel: None,
            default_channel_password: None,
            identity,
        }
    }
}

impl Default for ConnectConfig {
    fn default() -> Self {
        Self {
            server_addr: "127.0.0.1:9987".to_string(),
            nickname: "TeamSpeakUser".to_string(),
            server_password: None,
            default_channel: None,
            default_channel_password: None,
            identity: IdentityData::generate(),
        }
    }
}

/// Virtual server properties received in `initserver` and `notifyserveredited` / `notifyserverupdated`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VirtualServerInfo {
    pub name: String,
    pub welcome_message: String,
    pub platform: String,
    pub version: String,
    pub max_clients: u32,
    pub clients_online: u32,
    pub channels_online: u32,
    pub uid: String,
    pub hostbanner_url: String,
    pub hostbanner_gfx_url: String,
    pub uptime: u64,
}

impl VirtualServerInfo {
    /// Parses a [`VirtualServerInfo`] from a parameter map (such as `initserver`).
    pub fn from_map(map: &HashMap<String, String>) -> Self {
        let mut info = Self::default();
        info.apply_map(map);
        info
    }

    /// Updates fields present in `map`.
    pub fn apply_map(&mut self, map: &HashMap<String, String>) {
        if let Some(v) = map.get("virtualserver_name") {
            self.name = v.clone();
        }
        if let Some(v) = map.get("virtualserver_welcomemessage") {
            self.welcome_message = v.clone();
        }
        if let Some(v) = map.get("virtualserver_platform") {
            self.platform = v.clone();
        }
        if let Some(v) = map.get("virtualserver_version") {
            self.version = v.clone();
        }
        if let Some(v) = map.get("virtualserver_maxclients").and_then(|s| s.parse().ok()) {
            self.max_clients = v;
        }
        if let Some(v) = map.get("virtualserver_clientsonline").and_then(|s| s.parse().ok()) {
            self.clients_online = v;
        }
        if let Some(v) = map.get("virtualserver_channelsonline").and_then(|s| s.parse().ok()) {
            self.channels_online = v;
        }
        if let Some(v) = map.get("virtualserver_unique_identifier") {
            self.uid = v.clone();
        }
        if let Some(v) = map.get("virtualserver_hostbanner_url") {
            self.hostbanner_url = v.clone();
        }
        if let Some(v) = map.get("virtualserver_hostbanner_gfx_url") {
            self.hostbanner_gfx_url = v.clone();
        }
        if let Some(v) = map.get("virtualserver_uptime").and_then(|s| s.parse().ok()) {
            self.uptime = v;
        }
    }
}

/// Channel properties received in `channellist`, `notifychannelcreated`, `notifychanneledited`, etc.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChannelInfo {
    pub cid: u64,
    pub pid: u64,
    pub order: u64,
    pub name: String,
    pub topic: String,
    pub codec: u8,
    pub codec_quality: u8,
    pub max_clients: i32,
    pub total_clients: u32,
    pub is_default: bool,
    pub is_password: bool,
    pub is_permanent: bool,
    pub is_semi_permanent: bool,
    pub needed_talk_power: i32,
    pub icon_id: u32,
    pub subscribed: bool,
}

impl ChannelInfo {
    /// Parses a [`ChannelInfo`] from a parameter map.
    pub fn from_map(map: &HashMap<String, String>) -> Self {
        let mut info = Self::default();
        info.apply_map(map);
        info
    }

    /// Updates fields present in `map`.
    pub fn apply_map(&mut self, map: &HashMap<String, String>) {
        if let Some(v) = map.get("cid").and_then(|s| s.parse().ok()) {
            self.cid = v;
        }
        if let Some(v) = map
            .get("cpid")
            .or_else(|| map.get("pid"))
            .and_then(|s| s.parse().ok())
        {
            self.pid = v;
        }
        if let Some(v) = map
            .get("channel_order")
            .or_else(|| map.get("order"))
            .and_then(|s| s.parse().ok())
        {
            self.order = v;
        }
        if let Some(v) = map.get("channel_name") {
            self.name = v.clone();
        }
        if let Some(v) = map.get("channel_topic") {
            self.topic = v.clone();
        }
        if let Some(v) = map.get("channel_codec").and_then(|s| s.parse().ok()) {
            self.codec = v;
        }
        if let Some(v) = map.get("channel_codec_quality").and_then(|s| s.parse().ok()) {
            self.codec_quality = v;
        }
        if let Some(v) = map.get("channel_maxclients").and_then(|s| s.parse().ok()) {
            self.max_clients = v;
        }
        if let Some(v) = map.get("total_clients").and_then(|s| s.parse::<i64>().ok()) {
            self.total_clients = v.max(0) as u32;
        }
        if let Some(v) = map.get("channel_flag_default").and_then(|s| parse_bool(s)) {
            self.is_default = v;
        }
        if let Some(v) = map.get("channel_flag_password").and_then(|s| parse_bool(s)) {
            self.is_password = v;
        }
        if let Some(v) = map.get("channel_flag_permanent").and_then(|s| parse_bool(s)) {
            self.is_permanent = v;
        }
        if let Some(v) = map
            .get("channel_flag_semi_permanent")
            .and_then(|s| parse_bool(s))
        {
            self.is_semi_permanent = v;
        }
        if let Some(v) = map
            .get("channel_needed_talk_power")
            .and_then(|s| s.parse().ok())
        {
            self.needed_talk_power = v;
        }
        if let Some(v) = map.get("channel_icon_id").and_then(|s| parse_icon_id(s)) {
            self.icon_id = v;
        }
    }
}

/// Client properties received in `clientlist`, `notifycliententerview`, `notifyclientupdated`, etc.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClientInfo {
    pub clid: u16,
    pub cid: u64,
    pub dbid: u64,
    pub uid: String,
    pub nickname: String,
    pub client_type: u8,
    pub away: bool,
    pub away_message: String,
    pub input_muted: bool,
    pub output_muted: bool,
    pub input_hardware: bool,
    pub output_hardware: bool,
    pub talk_power: i32,
    pub is_talker: bool,
    pub is_channel_commander: bool,
    pub server_groups: Vec<u64>,
    pub channel_group_id: u64,
    pub country: String,
    pub version: String,
    pub platform: String,
    pub talking: bool,
}

impl ClientInfo {
    /// Parses a [`ClientInfo`] from a parameter map.
    pub fn from_map(map: &HashMap<String, String>) -> Self {
        let mut info = Self::default();
        info.apply_map(map);
        info
    }

    /// Updates fields present in `map`.
    pub fn apply_map(&mut self, map: &HashMap<String, String>) {
        if let Some(v) = map.get("clid").and_then(|s| s.parse().ok()) {
            self.clid = v;
        }
        if let Some(v) = map
            .get("ctid")
            .or_else(|| map.get("cid"))
            .and_then(|s| s.parse().ok())
        {
            self.cid = v;
        }
        if let Some(v) = map.get("client_database_id").and_then(|s| s.parse().ok()) {
            self.dbid = v;
        }
        if let Some(v) = map.get("client_unique_identifier") {
            self.uid = v.clone();
        }
        if let Some(v) = map.get("client_nickname") {
            self.nickname = v.clone();
        }
        if let Some(v) = map.get("client_type").and_then(|s| s.parse().ok()) {
            self.client_type = v;
        }
        if let Some(v) = map.get("client_away").and_then(|s| parse_bool(s)) {
            self.away = v;
        }
        if let Some(v) = map.get("client_away_message") {
            self.away_message = v.clone();
        }
        if let Some(v) = map.get("client_input_muted").and_then(|s| parse_bool(s)) {
            self.input_muted = v;
        }
        if let Some(v) = map.get("client_output_muted").and_then(|s| parse_bool(s)) {
            self.output_muted = v;
        }
        if let Some(v) = map.get("client_input_hardware").and_then(|s| parse_bool(s)) {
            self.input_hardware = v;
        }
        if let Some(v) = map.get("client_output_hardware").and_then(|s| parse_bool(s)) {
            self.output_hardware = v;
        }
        if let Some(v) = map.get("client_talk_power").and_then(|s| s.parse().ok()) {
            self.talk_power = v;
        }
        if let Some(v) = map.get("client_is_talker").and_then(|s| parse_bool(s)) {
            self.is_talker = v;
        }
        if let Some(v) = map
            .get("client_is_channel_commander")
            .and_then(|s| parse_bool(s))
        {
            self.is_channel_commander = v;
        }
        if let Some(v) = map.get("client_servergroups") {
            self.server_groups = v
                .split(',')
                .filter_map(|part| part.trim().parse::<u64>().ok())
                .collect();
        }
        if let Some(v) = map
            .get("client_channel_group_id")
            .and_then(|s| s.parse().ok())
        {
            self.channel_group_id = v;
        }
        if let Some(v) = map.get("client_country") {
            self.country = v.clone();
        }
        if let Some(v) = map.get("client_version") {
            self.version = v.clone();
        }
        if let Some(v) = map.get("client_platform") {
            self.platform = v.clone();
        }
        if let Some(v) = map.get("client_flag_talking").and_then(|s| parse_bool(s)) {
            self.talking = v;
        }
    }
}

/// Target scope for a chat message (`sendtextmessage` / `notifytextmessage`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatTarget {
    /// Targetmode `3`: Virtual server-wide chat.
    Server,
    /// Targetmode `2`: Current channel chat.
    Channel,
    /// Targetmode `1`: Private message to/from a specific client (`clid`).
    Client(u16),
}

impl ChatTarget {
    /// Returns the wire `targetmode` integer (`1` = Client, `2` = Channel, `3` = Server).
    pub fn target_mode(&self) -> u8 {
        match self {
            Self::Client(_) => 1,
            Self::Channel => 2,
            Self::Server => 3,
        }
    }
}

/// A received or sent text message in TeamSpeak 3.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatMessage {
    pub timestamp: String,
    pub target: ChatTarget,
    pub invoker_id: u16,
    pub invoker_name: String,
    pub invoker_uid: String,
    pub message: String,
}

/// Events emitted by the TeamSpeak 3 client to the application/GUI layer.
#[derive(Debug, Clone)]
pub enum TsEvent {
    Connected {
        my_clid: u16,
        server: VirtualServerInfo,
    },
    Disconnected {
        reason: String,
    },
    ChannelsUpdated(Vec<ChannelInfo>),
    ClientsUpdated(Vec<ClientInfo>),
    ClientEntered(ClientInfo),
    ClientLeft {
        clid: u16,
        reason_msg: String,
    },
    ClientMoved {
        clid: u16,
        new_cid: u64,
    },
    ChatReceived(ChatMessage),
    PokeReceived {
        invoker_id: u16,
        invoker_name: String,
        message: String,
    },
    VoicePacket {
        talker_id: u16,
        codec: u8,
        audio_data: Vec<u8>,
    },
    ServerError {
        id: u32,
        msg: String,
    },
    PingUpdated {
        rtt_ms: u32,
        packet_loss: f32,
    },
}

/// Point-in-time snapshot of the client's world state returned by `Ts3ClientHandle::snapshot_state()`.
#[derive(Debug, Clone, Default)]
pub struct ClientStateSnapshot {
    pub connected: bool,
    pub my_clid: u16,
    pub my_cid: u64,
    pub server: VirtualServerInfo,
    pub channels: Vec<ChannelInfo>,
    pub clients: Vec<ClientInfo>,
    pub rtt_ms: u32,
    pub packet_loss: f32,
}

#[inline]
pub(crate) fn parse_bool(s: &str) -> Option<bool> {
    if s.is_empty() {
        return None;
    }
    if s.eq_ignore_ascii_case("false") || s.as_bytes()[0] == b'0' {
        Some(false)
    } else {
        Some(true)
    }
}

#[inline]
pub(crate) fn parse_icon_id(s: &str) -> Option<u32> {
    if let Ok(u) = s.parse::<u64>() {
        Some(u as u32)
    } else if let Ok(i) = s.parse::<i64>() {
        Some(i as i32 as u32)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ts3_proto::command::ParsedCommand;

    #[test]
    fn test_virtual_server_info_from_initserver() {
        let cmd = ParsedCommand::parse(
            "initserver virtualserver_name=My\\sTS3\\sServer virtualserver_welcomemessage=Welcome! \
             virtualserver_platform=Linux virtualserver_version=3.13.9\\s[Build:\\s1674480572] \
             virtualserver_maxclients=32 virtualserver_clientsonline=5 virtualserver_channelsonline=10 \
             virtualserver_unique_identifier=serveruid123= virtualserver_hostbanner_url=https:\\/\\/example.com \
             virtualserver_hostbanner_gfx_url=https:\\/\\/example.com\\/banner.png virtualserver_uptime=86400 aclid=7",
        )
        .unwrap();

        let info = VirtualServerInfo::from_map(&cmd.items[0]);
        assert_eq!(info.name, "My TS3 Server");
        assert_eq!(info.welcome_message, "Welcome!");
        assert_eq!(info.platform, "Linux");
        assert_eq!(info.version, "3.13.9 [Build: 1674480572]");
        assert_eq!(info.max_clients, 32);
        assert_eq!(info.clients_online, 5);
        assert_eq!(info.channels_online, 10);
        assert_eq!(info.uid, "serveruid123=");
        assert_eq!(info.hostbanner_url, "https://example.com");
        assert_eq!(info.hostbanner_gfx_url, "https://example.com/banner.png");
        assert_eq!(info.uptime, 86400);
    }

    #[test]
    fn test_channel_info_from_channellist() {
        let cmd = ParsedCommand::parse(
            "channellist cid=1 cpid=0 channel_order=0 channel_name=Default\\sChannel channel_topic=Main\\sTopic \
             channel_codec=4 channel_codec_quality=6 channel_maxclients=-1 total_clients=3 \
             channel_flag_default=1 channel_flag_password=0 channel_flag_permanent=1 channel_flag_semi_permanent=0 \
             channel_needed_talk_power=25 channel_icon_id=-1240768336|\
             cid=2 cpid=1 channel_order=0 channel_name=Sub\\sChannel channel_topic= \
             channel_codec=5 channel_codec_quality=10 channel_maxclients=10 total_clients=0 \
             channel_flag_default=0 channel_flag_password=1 channel_flag_permanent=0 channel_flag_semi_permanent=1 \
             channel_needed_talk_power=0 channel_icon_id=3054198960",
        )
        .unwrap();

        let items = cmd.expanded_items();
        assert_eq!(items.len(), 2);

        let ch1 = ChannelInfo::from_map(&items[0]);
        assert_eq!(ch1.cid, 1);
        assert_eq!(ch1.pid, 0);
        assert_eq!(ch1.order, 0);
        assert_eq!(ch1.name, "Default Channel");
        assert_eq!(ch1.topic, "Main Topic");
        assert_eq!(ch1.codec, 4);
        assert_eq!(ch1.codec_quality, 6);
        assert_eq!(ch1.max_clients, -1);
        assert_eq!(ch1.total_clients, 3);
        assert!(ch1.is_default);
        assert!(!ch1.is_password);
        assert!(ch1.is_permanent);
        assert!(!ch1.is_semi_permanent);
        assert_eq!(ch1.needed_talk_power, 25);
        assert_eq!(ch1.icon_id, 3054198960);

        let ch2 = ChannelInfo::from_map(&items[1]);
        assert_eq!(ch2.cid, 2);
        assert_eq!(ch2.pid, 1);
        assert_eq!(ch2.name, "Sub Channel");
        assert_eq!(ch2.codec, 5);
        assert_eq!(ch2.codec_quality, 10);
        assert_eq!(ch2.max_clients, 10);
        assert_eq!(ch2.total_clients, 0);
        assert!(!ch2.is_default);
        assert!(ch2.is_password);
        assert!(!ch2.is_permanent);
        assert!(ch2.is_semi_permanent);
        assert_eq!(ch2.icon_id, 3054198960);
    }

    #[test]
    fn test_client_info_from_enter_view_and_update() {
        let cmd = ParsedCommand::parse(
            "notifycliententerview cfid=0 ctid=1 reasonid=0 clid=12 client_unique_identifier=uid_abc= \
             client_nickname=Alice client_input_muted=0 client_output_muted=1 client_input_hardware=1 \
             client_output_hardware=1 client_database_id=99 client_channel_group_id=8 client_servergroups=6,7,12 \
             client_away=1 client_away_message=AFK\\slunch client_type=0 client_talk_power=75 \
             client_is_talker=1 client_is_channel_commander=1 client_country=US",
        )
        .unwrap();

        let mut cl = ClientInfo::from_map(&cmd.items[0]);
        assert_eq!(cl.clid, 12);
        assert_eq!(cl.cid, 1);
        assert_eq!(cl.dbid, 99);
        assert_eq!(cl.uid, "uid_abc=");
        assert_eq!(cl.nickname, "Alice");
        assert_eq!(cl.client_type, 0);
        assert!(cl.away);
        assert_eq!(cl.away_message, "AFK lunch");
        assert!(!cl.input_muted);
        assert!(cl.output_muted);
        assert!(cl.input_hardware);
        assert!(cl.output_hardware);
        assert_eq!(cl.talk_power, 75);
        assert!(cl.is_talker);
        assert!(cl.is_channel_commander);
        assert_eq!(cl.server_groups, vec![6, 7, 12]);
        assert_eq!(cl.channel_group_id, 8);
        assert_eq!(cl.country, "US");

        // Apply notifyclientupdated
        let upd = ParsedCommand::parse(
            "notifyclientupdated clid=12 client_nickname=Alice\\sUpdated client_away=0 client_away_message= \
             client_flag_talking=1 client_version=3.6.2 client_platform=Windows",
        )
        .unwrap();
        cl.apply_map(&upd.items[0]);
        assert_eq!(cl.nickname, "Alice Updated");
        assert!(!cl.away);
        assert_eq!(cl.away_message, "");
        assert!(cl.talking);
        assert_eq!(cl.version, "3.6.2");
        assert_eq!(cl.platform, "Windows");
    }
}
