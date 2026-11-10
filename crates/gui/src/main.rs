// Owned by Pool Agent 8 (arthoroy29a@gmail.com)
// Main TeamSpeak 3 1:1 Desktop Client entry point (`ts3-gui`).

pub mod assets;
pub mod chat_pane;
pub mod dialogs;
pub mod info_pane;
pub mod menus_toolbar;
pub mod sound;
pub mod theme;
pub mod tree_view;

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tokio::sync::mpsc;
use ts3_client::connection::{
    hash_password, ChannelInfo, ChatTarget, ClientInfo, ConnectConfig, Ts3Client,
    Ts3ClientHandle, TsEvent, VirtualServerInfo,
};
use ts3_proto::command::Command;
use ts3_proto::identity::IdentityData;

use crate::assets::Ts3Assets;
use crate::chat_pane::{
    render_chat_pane, ActiveChatTab, ChatLineKind, ChatOutgoingAction, ChatPaneState,
    FormattedChatLine, PrivateChatTab,
};
use crate::dialogs::{render_dialogs, BookmarkEntry, DialogAction, DialogsState};
use crate::info_pane::render_info_pane;
use crate::menus_toolbar::{
    render_main_toolbar, render_menu_bar, render_server_tab_bar, render_status_bar,
    ServerTabSummary, StatusBarInfo, ToolbarState, UiCommand,
};
use crate::sound::{SoundEvent, SoundManager};
use crate::theme::{apply_ts3_theme, Ts3Palette};
use crate::tree_view::{render_server_tree, SelectedTreeNode, TreeViewState};

#[derive(Debug, Clone)]
pub struct StartupCliArgs {
    pub server_addr: String,
    pub server_password: Option<String>,
    pub nickname: String,
    pub default_channel: Option<String>,
    pub auto_connect: bool,
}

impl StartupCliArgs {
    pub fn from_env() -> Self {
        let mut server_addr = std::env::var("TS3_SERVER")
            .unwrap_or_else(|_| "192.168.1.140:9987".to_string());
        let mut server_password = std::env::var("TS3_PASSWORD").ok().filter(|s| !s.is_empty());
        let mut nickname = std::env::var("TS3_NICKNAME")
            .or_else(|_| std::env::var("TS3_NICK"))
            .unwrap_or_else(|_| "TeamSpeakUser".to_string());
        let mut default_channel = std::env::var("TS3_CHANNEL").ok().filter(|s| !s.is_empty());
        let mut auto_connect = std::env::var("TS3_AUTO_CONNECT")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

        let args: Vec<String> = std::env::args().skip(1).collect();
        let mut idx = 0;
        while idx < args.len() {
            match args[idx].as_str() {
                "--server" | "-s" => {
                    if let Some(val) = args.get(idx + 1) {
                        server_addr = val.clone();
                        idx += 1;
                    }
                }
                "--password" | "-p" => {
                    if let Some(val) = args.get(idx + 1) {
                        if !val.is_empty() {
                            server_password = Some(val.clone());
                        }
                        idx += 1;
                    }
                }
                "--nick" | "--nickname" | "-n" => {
                    if let Some(val) = args.get(idx + 1) {
                        nickname = val.clone();
                        idx += 1;
                    }
                }
                "--channel" | "-c" => {
                    if let Some(val) = args.get(idx + 1) {
                        if !val.is_empty() {
                            default_channel = Some(val.clone());
                        }
                        idx += 1;
                    }
                }
                "--auto-connect" | "-a" => {
                    auto_connect = true;
                }
                _ => {}
            }
            idx += 1;
        }

        Self {
            server_addr,
            server_password,
            nickname,
            default_channel,
            auto_connect,
        }
    }
}

pub enum AsyncConnectResult {
    Success {
        tab_index: usize,
        server_addr: String,
        nickname: String,
        handle: Ts3ClientHandle,
        event_rx: mpsc::UnboundedReceiver<TsEvent>,
    },
    Failed {
        tab_index: usize,
        server_addr: String,
        error: String,
    },
}

pub struct ServerTabSession {
    pub title: String,
    pub server_addr: String,
    pub nickname: String,
    pub connected: bool,
    pub connecting: bool,
    pub status_text: String,
    pub handle: Option<Ts3ClientHandle>,
    pub event_rx: Option<mpsc::UnboundedReceiver<TsEvent>>,
    pub tree_state: TreeViewState,
    pub chat_state: ChatPaneState,
    pub server: VirtualServerInfo,
    pub channels: Vec<ChannelInfo>,
    pub clients: Vec<ClientInfo>,
    pub my_clid: u16,
    pub my_cid: u64,
    pub rtt_ms: u32,
    pub packet_loss: f32,
}

impl ServerTabSession {
    pub fn new_empty(nickname: &str) -> Self {
        let mut chat_state = ChatPaneState {
            auto_scroll: true,
            ..Default::default()
        };
        chat_state.server_lines.push(FormattedChatLine {
            timestamp: now_hms(),
            kind: ChatLineKind::ServerLog,
            sender_clid: None,
            sender_uid: None,
            sender_name: None,
            text: "Ready. Use Connections -> Connect (Ctrl+S) or Bookmarks (Ctrl+B) to join a TeamSpeak 3 server.".to_string(),
            is_own_message: false,
        });

        Self {
            title: "No Server".to_string(),
            server_addr: String::new(),
            nickname: nickname.to_string(),
            connected: false,
            connecting: false,
            status_text: "Disconnected".to_string(),
            handle: None,
            event_rx: None,
            tree_state: TreeViewState::default(),
            chat_state,
            server: VirtualServerInfo::default(),
            channels: Vec::new(),
            clients: Vec::new(),
            my_clid: 0,
            my_cid: 0,
            rtt_ms: 0,
            packet_loss: 0.0,
        }
    }

    pub fn push_server_log(&mut self, kind: ChatLineKind, text: impl Into<String>) {
        self.chat_state.server_lines.push(FormattedChatLine {
            timestamp: now_hms(),
            kind,
            sender_clid: None,
            sender_uid: None,
            sender_name: None,
            text: text.into(),
            is_own_message: false,
        });
        if self.chat_state.active_tab != ActiveChatTab::Server {
            self.chat_state.server_unread = self.chat_state.server_unread.saturating_add(1);
        }
    }

    pub fn push_channel_log(&mut self, kind: ChatLineKind, text: impl Into<String>) {
        self.chat_state.channel_lines.push(FormattedChatLine {
            timestamp: now_hms(),
            kind,
            sender_clid: None,
            sender_uid: None,
            sender_name: None,
            text: text.into(),
            is_own_message: false,
        });
        if self.chat_state.active_tab != ActiveChatTab::Channel {
            self.chat_state.channel_unread = self.chat_state.channel_unread.saturating_add(1);
        }
    }

    pub fn total_unread(&self) -> usize {
        let priv_unread: usize = self.chat_state.private_tabs.iter().map(|t| t.unread).sum();
        self.chat_state.server_unread + self.chat_state.channel_unread + priv_unread
    }

    pub fn current_channel_name(&self) -> String {
        self.channels
            .iter()
            .find(|c| c.cid == self.my_cid)
            .map(|c| c.name.clone())
            .unwrap_or_else(|| "Channel Chat".to_string())
    }

    pub fn disconnect(&mut self) {
        if let Some(ref handle) = self.handle {
            let _ = handle.disconnect();
        }
        self.handle = None;
        self.event_rx = None;
        self.connected = false;
        self.connecting = false;
        self.status_text = "Disconnected".to_string();
    }
}

pub struct Ts3GuiApp {
    pub rt: tokio::runtime::Runtime,
    pub tabs: Vec<ServerTabSession>,
    pub active_tab: usize,
    pub toolbar: ToolbarState,
    pub dialogs: DialogsState,
    pub assets: Ts3Assets,
    pub sound: SoundManager,
    pub palette: Ts3Palette,
    pub connect_tx: mpsc::UnboundedSender<AsyncConnectResult>,
    pub connect_rx: mpsc::UnboundedReceiver<AsyncConnectResult>,
    pub theme_applied: bool,
}

impl Ts3GuiApp {
    pub fn new(cc: &eframe::CreationContext<'_>, cli: StartupCliArgs) -> Self {
        egui_extras::install_image_loaders(&cc.egui_ctx);

        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("failed to build Tokio runtime for Ts3GuiApp");

        let mut dialogs = DialogsState::default();
        dialogs.connect_addr = cli.server_addr.clone();
        dialogs.connect_nickname = cli.nickname.clone();
        dialogs.change_nickname_input = cli.nickname.clone();
        if let Some(ref pw) = cli.server_password {
            dialogs.connect_password = pw.clone();
        }
        if let Some(ref ch) = cli.default_channel {
            dialogs.connect_default_channel = ch.clone();
        }

        let mut assets = Ts3Assets::new();
        assets.load_all(&cc.egui_ctx);

        let sound = SoundManager::new();
        let palette = Ts3Palette::for_mode(dialogs.theme_mode);
        apply_ts3_theme(&cc.egui_ctx, dialogs.theme_mode);

        let (connect_tx, connect_rx) = mpsc::unbounded_channel();

        let mut app = Self {
            rt,
            tabs: vec![ServerTabSession::new_empty(&cli.nickname)],
            active_tab: 0,
            toolbar: ToolbarState::default(),
            dialogs,
            assets,
            sound,
            palette,
            connect_tx,
            connect_rx,
            theme_applied: true,
        };

        if cli.auto_connect {
            app.start_connection(
                0,
                cli.server_addr,
                cli.nickname,
                cli.server_password,
                cli.default_channel,
                None,
            );
        }

        app
    }

    pub fn start_connection(
        &mut self,
        tab_index: usize,
        server_addr: String,
        nickname: String,
        server_password: Option<String>,
        default_channel: Option<String>,
        default_channel_password: Option<String>,
    ) {
        let idx = if tab_index < self.tabs.len() {
            tab_index
        } else {
            self.tabs.push(ServerTabSession::new_empty(&nickname));
            self.tabs.len() - 1
        };
        self.active_tab = idx;

        let tab = &mut self.tabs[idx];
        tab.disconnect();
        tab.server_addr = server_addr.clone();
        tab.nickname = nickname.clone();
        tab.title = server_addr.clone();
        tab.connecting = true;
        tab.status_text = format!("Connecting to {server_addr}...");
        tab.push_server_log(
            ChatLineKind::ServerLog,
            format!("Trying to resolve hostname and connect to {server_addr}..."),
        );

        let identity: IdentityData = self.dialogs.identity.clone();
        let tx = self.connect_tx.clone();

        self.rt.spawn(async move {
            let cfg = ConnectConfig {
                server_addr: server_addr.clone(),
                nickname: nickname.clone(),
                server_password,
                default_channel,
                default_channel_password,
                identity,
            };

            match Ts3Client::connect(cfg).await {
                Ok((handle, event_rx)) => {
                    let _ = tx.send(AsyncConnectResult::Success {
                        tab_index: idx,
                        server_addr,
                        nickname,
                        handle,
                        event_rx,
                    });
                }
                Err(err) => {
                    let _ = tx.send(AsyncConnectResult::Failed {
                        tab_index: idx,
                        server_addr,
                        error: err.to_string(),
                    });
                }
            }
        });
    }

    fn poll_network_events(&mut self) {
        // 1. Drain completed async `Ts3Client::connect` results
        while let Ok(res) = self.connect_rx.try_recv() {
            match res {
                AsyncConnectResult::Success {
                    tab_index,
                    server_addr,
                    nickname,
                    handle,
                    event_rx,
                } => {
                    if let Some(tab) = self.tabs.get_mut(tab_index) {
                        tab.server_addr = server_addr;
                        tab.nickname = nickname;
                        tab.handle = Some(handle);
                        tab.event_rx = Some(event_rx);
                        tab.status_text = "Handshaking...".to_string();
                    }
                }
                AsyncConnectResult::Failed {
                    tab_index,
                    server_addr,
                    error,
                } => {
                    if let Some(tab) = self.tabs.get_mut(tab_index) {
                        tab.connecting = false;
                        tab.connected = false;
                        tab.status_text = format!("Failed to connect to {server_addr}");
                        tab.push_server_log(
                            ChatLineKind::ErrorAlert,
                            format!("Failed to connect to server {server_addr}: {error}"),
                        );
                    }
                    self.sound.play(SoundEvent::ServerError);
                }
            }
        }

        // 2. Drain `TsEvent`s per ServerTabSession
        let mut pending_events: Vec<(usize, TsEvent)> = Vec::new();
        for (idx, tab) in self.tabs.iter_mut().enumerate() {
            if let Some(ref mut rx) = tab.event_rx {
                while let Ok(ev) = rx.try_recv() {
                    pending_events.push((idx, ev));
                }
            }
            // Also refresh point-in-time snapshot from handle when connected
            if let Some(ref handle) = tab.handle {
                let snap = handle.snapshot_state();
                if snap.connected {
                    tab.my_clid = snap.my_clid;
                    tab.my_cid = snap.my_cid;
                }
            }
        }

        for (tab_idx, event) in pending_events {
            self.handle_ts_event(tab_idx, event);
        }

        // Sync global toolbar connected state with active tab
        if let Some(active) = self.tabs.get(self.active_tab) {
            self.toolbar.connected = active.connected;
        } else {
            self.toolbar.connected = false;
        }
    }

    fn handle_ts_event(&mut self, tab_idx: usize, event: TsEvent) {
        let Some(tab) = self.tabs.get_mut(tab_idx) else {
            return;
        };

        match event {
            TsEvent::Connected { my_clid, server } => {
                tab.connected = true;
                tab.connecting = false;
                tab.my_clid = my_clid;
                let srv_name = if server.name.is_empty() {
                    tab.server_addr.clone()
                } else {
                    server.name.clone()
                };
                tab.title = srv_name.clone();
                tab.status_text = format!("Connected to {srv_name}");
                let welcome = server.welcome_message.clone();
                tab.server = server;

                tab.push_server_log(
                    ChatLineKind::ServerLog,
                    format!("Connected to Server: \"{srv_name}\""),
                );
                if !welcome.is_empty() {
                    tab.push_server_log(ChatLineKind::WelcomeMessage, welcome);
                }

                // Sync current mute/away state onto the newly connected server
                if let Some(ref handle) = tab.handle {
                    if self.toolbar.input_muted || self.toolbar.output_muted {
                        let _ = handle.set_mute(self.toolbar.input_muted, self.toolbar.output_muted);
                    }
                    if self.toolbar.away {
                        let msg = if self.toolbar.away_message.is_empty() {
                            None
                        } else {
                            Some(self.toolbar.away_message.as_str())
                        };
                        let _ = handle.set_away(true, msg);
                    }
                }

                self.sound.play(SoundEvent::Connected);
            }
            TsEvent::Disconnected { reason } => {
                let was_connected = tab.connected;
                tab.connected = false;
                tab.connecting = false;
                tab.handle = None;
                tab.event_rx = None;
                tab.status_text = format!("Disconnected ({reason})");
                tab.push_server_log(
                    ChatLineKind::ServerLog,
                    format!("Disconnected from server ({reason})"),
                );
                if was_connected {
                    self.sound.play(SoundEvent::Disconnected);
                } else {
                    self.sound.play(SoundEvent::ConnectionLost);
                }
            }
            TsEvent::ChannelsUpdated(mut channels) => {
                if self.toolbar.subscribed_all {
                    for ch in &mut channels {
                        ch.subscribed = true;
                    }
                }
                tab.channels = channels;
                tab.server.channels_online = tab.channels.len() as u32;
            }
            TsEvent::ClientsUpdated(clients) => {
                if let Some(me) = clients.iter().find(|c| c.clid == tab.my_clid) {
                    tab.my_cid = me.cid;
                }
                tab.clients = clients;
                tab.server.clients_online =
                    tab.clients.iter().filter(|c| c.client_type == 0).count() as u32;
            }
            TsEvent::ClientEntered(client) => {
                let is_same_channel = client.cid == tab.my_cid && tab.my_cid != 0;
                let ch_name = tab
                    .channels
                    .iter()
                    .find(|c| c.cid == client.cid)
                    .map(|c| c.name.clone())
                    .unwrap_or_else(|| format!("Channel #{}", client.cid));

                if client.client_type == 0 {
                    tab.push_server_log(
                        ChatLineKind::ServerLog,
                        format!(
                            "\"{}\" connected to channel \"{}\"",
                            client.nickname, ch_name
                        ),
                    );
                }

                if let Some(existing) = tab.clients.iter_mut().find(|c| c.clid == client.clid) {
                    *existing = client;
                } else {
                    tab.clients.push(client);
                }
                tab.server.clients_online =
                    tab.clients.iter().filter(|c| c.client_type == 0).count() as u32;

                if is_same_channel {
                    self.sound.play(SoundEvent::UserJoinedCurrentChannel);
                } else {
                    self.sound.play(SoundEvent::UserJoinedServer);
                }
            }
            TsEvent::ClientLeft { clid, reason_msg } => {
                let removed = if let Some(pos) = tab.clients.iter().position(|c| c.clid == clid) {
                    Some(tab.clients.remove(pos))
                } else {
                    None
                };
                tab.server.clients_online =
                    tab.clients.iter().filter(|c| c.client_type == 0).count() as u32;

                if let Some(cl) = removed {
                    let was_our_channel = cl.cid == tab.my_cid && tab.my_cid != 0;
                    let reason_suffix = if reason_msg.is_empty() {
                        String::new()
                    } else {
                        format!(" ({reason_msg})")
                    };
                    tab.push_server_log(
                        ChatLineKind::ServerLog,
                        format!("\"{}\" disconnected{}", cl.nickname, reason_suffix),
                    );
                    if was_our_channel {
                        self.sound.play(SoundEvent::UserLeftCurrentChannel);
                    }
                }
            }
            TsEvent::ClientMoved { clid, new_cid } => {
                let old_cid = tab
                    .clients
                    .iter()
                    .find(|c| c.clid == clid)
                    .map(|c| c.cid)
                    .unwrap_or(0);
                let nick = tab
                    .clients
                    .iter()
                    .find(|c| c.clid == clid)
                    .map(|c| c.nickname.clone())
                    .unwrap_or_else(|| format!("client #{clid}"));

                if let Some(cl) = tab.clients.iter_mut().find(|c| c.clid == clid) {
                    cl.cid = new_cid;
                }
                if clid == tab.my_clid {
                    tab.my_cid = new_cid;
                }

                let ch_name = tab
                    .channels
                    .iter()
                    .find(|c| c.cid == new_cid)
                    .map(|c| c.name.clone())
                    .unwrap_or_else(|| format!("Channel #{new_cid}"));

                tab.push_server_log(
                    ChatLineKind::ServerLog,
                    format!("\"{nick}\" switched to channel \"{ch_name}\""),
                );
                tab.push_channel_log(
                    ChatLineKind::ServerLog,
                    format!("\"{nick}\" switched to channel \"{ch_name}\""),
                );

                if clid == tab.my_clid {
                    self.sound.play(SoundEvent::ChannelSwitched);
                } else if new_cid == tab.my_cid && old_cid != tab.my_cid {
                    self.sound.play(SoundEvent::UserSwitchedToCurrentChannel);
                } else if old_cid == tab.my_cid && new_cid != tab.my_cid {
                    self.sound.play(SoundEvent::UserLeftCurrentChannel);
                }
            }
            TsEvent::ChatReceived(chat_msg) => {
                let is_own = chat_msg.invoker_id == tab.my_clid && tab.my_clid != 0;
                match chat_msg.target {
                    ChatTarget::Server => {
                        tab.chat_state.server_lines.push(FormattedChatLine {
                            timestamp: chat_msg.timestamp,
                            kind: ChatLineKind::ServerLog,
                            sender_clid: Some(chat_msg.invoker_id),
                            sender_uid: Some(chat_msg.invoker_uid),
                            sender_name: Some(chat_msg.invoker_name),
                            text: chat_msg.message,
                            is_own_message: is_own,
                        });
                        if tab.chat_state.active_tab != ActiveChatTab::Server {
                            tab.chat_state.server_unread =
                                tab.chat_state.server_unread.saturating_add(1);
                        }
                        if !is_own {
                            self.sound.play(SoundEvent::ChatReceivedServer);
                        }
                    }
                    ChatTarget::Channel => {
                        tab.chat_state.channel_lines.push(FormattedChatLine {
                            timestamp: chat_msg.timestamp,
                            kind: ChatLineKind::ChannelMessage,
                            sender_clid: Some(chat_msg.invoker_id),
                            sender_uid: Some(chat_msg.invoker_uid),
                            sender_name: Some(chat_msg.invoker_name),
                            text: chat_msg.message,
                            is_own_message: is_own,
                        });
                        if tab.chat_state.active_tab != ActiveChatTab::Channel {
                            tab.chat_state.channel_unread =
                                tab.chat_state.channel_unread.saturating_add(1);
                        }
                        if !is_own {
                            self.sound.play(SoundEvent::ChatReceivedChannel);
                        }
                    }
                    ChatTarget::Client(peer_clid) => {
                        let tab_clid = if is_own {
                            peer_clid
                        } else {
                            chat_msg.invoker_id
                        };
                        let peer_nick = if is_own {
                            tab.clients
                                .iter()
                                .find(|c| c.clid == peer_clid)
                                .map(|c| c.nickname.clone())
                                .unwrap_or_else(|| format!("Client #{peer_clid}"))
                        } else {
                            chat_msg.invoker_name.clone()
                        };

                        let priv_tab = if let Some(existing) = tab
                            .chat_state
                            .private_tabs
                            .iter_mut()
                            .find(|pt| pt.clid == tab_clid)
                        {
                            existing
                        } else {
                            tab.chat_state.private_tabs.push(PrivateChatTab {
                                clid: tab_clid,
                                nickname: peer_nick,
                                uid: chat_msg.invoker_uid.clone(),
                                lines: Vec::new(),
                                unread: 0,
                            });
                            tab.chat_state.private_tabs.last_mut().unwrap()
                        };

                        priv_tab.lines.push(FormattedChatLine {
                            timestamp: chat_msg.timestamp,
                            kind: ChatLineKind::PrivateMessage,
                            sender_clid: Some(chat_msg.invoker_id),
                            sender_uid: Some(chat_msg.invoker_uid),
                            sender_name: Some(chat_msg.invoker_name),
                            text: chat_msg.message,
                            is_own_message: is_own,
                        });
                        if tab.chat_state.active_tab != ActiveChatTab::Private(tab_clid) {
                            priv_tab.unread = priv_tab.unread.saturating_add(1);
                        }
                        if !is_own {
                            self.sound.play(SoundEvent::ChatReceivedPrivate);
                        }
                    }
                }
            }
            TsEvent::PokeReceived {
                invoker_id,
                invoker_name,
                message,
            } => {
                let ts = now_hms();
                let poke_line = if message.is_empty() {
                    format!("\"{invoker_name}\" pokes you.")
                } else {
                    format!("\"{invoker_name}\" pokes you: {message}")
                };
                tab.chat_state.server_lines.push(FormattedChatLine {
                    timestamp: ts.clone(),
                    kind: ChatLineKind::PokeAlert,
                    sender_clid: Some(invoker_id),
                    sender_uid: None,
                    sender_name: Some(invoker_name.clone()),
                    text: poke_line,
                    is_own_message: false,
                });

                self.dialogs.show_received_poke = true;
                self.dialogs.received_poke_server = tab.title.clone();
                self.dialogs.received_poke_invoker = invoker_name;
                self.dialogs.received_poke_message = message;
                self.dialogs.received_poke_timestamp = ts;

                self.sound.play(SoundEvent::UserPokedYou);
            }
            TsEvent::VoicePacket { talker_id, .. } => {
                if let Some(cl) = tab.clients.iter_mut().find(|c| c.clid == talker_id) {
                    cl.talking = true;
                }
            }
            TsEvent::ServerError { id, msg } => {
                tab.status_text = format!("Server error {id}: {msg}");
                tab.push_server_log(ChatLineKind::ErrorAlert, format!("<Server Error {id}> {msg}"));
                self.sound.play(SoundEvent::ServerError);
            }
            TsEvent::PingUpdated {
                rtt_ms,
                packet_loss,
            } => {
                tab.rtt_ms = rtt_ms;
                tab.packet_loss = packet_loss;
            }
        }
    }

    fn execute_ui_command(&mut self, ctx: &egui::Context, cmd: UiCommand) {
        match cmd {
            UiCommand::OpenConnectDialog => {
                self.dialogs.show_connect = true;
            }
            UiCommand::DisconnectCurrent => {
                if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                    if tab.connected || tab.connecting {
                        tab.disconnect();
                        tab.push_server_log(
                            ChatLineKind::ServerLog,
                            "Disconnected from server".to_string(),
                        );
                        self.sound.play(SoundEvent::Disconnected);
                    }
                }
            }
            UiCommand::OpenBookmarksDialog => {
                self.dialogs.show_bookmarks = true;
            }
            UiCommand::AddCurrentToBookmarks => {
                if let Some(tab) = self.tabs.get(self.active_tab) {
                    let addr = if tab.server_addr.is_empty() {
                        self.dialogs.connect_addr.clone()
                    } else {
                        tab.server_addr.clone()
                    };
                    let label = if !tab.server.name.is_empty() {
                        tab.server.name.clone()
                    } else {
                        addr.clone()
                    };
                    self.dialogs.bookmarks.push(BookmarkEntry {
                        label,
                        server_addr: addr,
                        nickname: tab.nickname.clone(),
                        server_password: String::new(),
                        default_channel: String::new(),
                        auto_connect: false,
                    });
                    self.dialogs.selected_bookmark =
                        self.dialogs.bookmarks.len().saturating_sub(1);
                    self.dialogs.show_bookmarks = true;
                }
            }
            UiCommand::OpenIdentitiesDialog => {
                self.dialogs.show_identities = true;
            }
            UiCommand::OpenCreateChannelDialog { parent_cid } => {
                self.dialogs.create_channel_parent_cid = parent_cid;
                self.dialogs.show_create_channel = true;
            }
            UiCommand::OpenOptionsDialog => {
                self.dialogs.show_options = true;
            }
            UiCommand::OpenAboutDialog => {
                self.dialogs.show_about = true;
            }
            UiCommand::ToggleAway { away, message } => {
                self.toolbar.away = away;
                self.toolbar.away_message = message.clone().unwrap_or_default();
                if let Some(tab) = self.tabs.get(self.active_tab) {
                    if let Some(ref handle) = tab.handle {
                        let _ = handle.set_away(away, message.as_deref());
                    }
                }
                if away {
                    self.sound.play(SoundEvent::AwayActivated);
                } else {
                    self.sound.play(SoundEvent::AwayDeactivated);
                }
            }
            UiCommand::ToggleMicMute => {
                self.toolbar.input_muted = !self.toolbar.input_muted;
                if let Some(tab) = self.tabs.get(self.active_tab) {
                    if let Some(ref handle) = tab.handle {
                        let _ =
                            handle.set_mute(self.toolbar.input_muted, self.toolbar.output_muted);
                    }
                }
                if self.toolbar.input_muted {
                    self.sound.play(SoundEvent::MicMuted);
                } else {
                    self.sound.play(SoundEvent::MicUnmuted);
                }
            }
            UiCommand::ToggleOutputMute => {
                self.toolbar.output_muted = !self.toolbar.output_muted;
                if let Some(tab) = self.tabs.get(self.active_tab) {
                    if let Some(ref handle) = tab.handle {
                        let _ =
                            handle.set_mute(self.toolbar.input_muted, self.toolbar.output_muted);
                    }
                }
                if self.toolbar.output_muted {
                    self.sound.play(SoundEvent::OutputMuted);
                } else {
                    self.sound.play(SoundEvent::OutputUnmuted);
                }
            }
            UiCommand::SubscribeAllChannels => {
                self.toolbar.subscribed_all = !self.toolbar.subscribed_all;
                if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                    for ch in &mut tab.channels {
                        ch.subscribed = self.toolbar.subscribed_all;
                    }
                    if let Some(ref handle) = tab.handle {
                        let raw = if self.toolbar.subscribed_all {
                            "channelsubscribeall"
                        } else {
                            "channelunsubscribeall"
                        };
                        let _ = handle.send_raw_command(raw);
                    }
                }
            }
            UiCommand::SwitchTheme(mode) => {
                self.toolbar.theme_mode = mode;
                self.dialogs.theme_mode = mode;
                self.palette = Ts3Palette::for_mode(mode);
                apply_ts3_theme(ctx, mode);
            }
            UiCommand::ChangeNicknamePrompt => {
                if let Some(tab) = self.tabs.get(self.active_tab) {
                    self.dialogs.change_nickname_input = tab.nickname.clone();
                }
                self.dialogs.show_change_nickname = true;
            }
            UiCommand::SelectServerTab(idx) => {
                if idx < self.tabs.len() {
                    self.active_tab = idx;
                }
            }
            UiCommand::CloseServerTab(idx) => {
                if idx < self.tabs.len() {
                    self.tabs[idx].disconnect();
                    if self.tabs.len() > 1 {
                        self.tabs.remove(idx);
                        if self.active_tab >= self.tabs.len() {
                            self.active_tab = self.tabs.len() - 1;
                        }
                    } else {
                        let nick = self.dialogs.connect_nickname.clone();
                        self.tabs[0] = ServerTabSession::new_empty(&nick);
                        self.active_tab = 0;
                    }
                }
            }
            UiCommand::NewServerTab => {
                let nick = self.dialogs.connect_nickname.clone();
                self.tabs.push(ServerTabSession::new_empty(&nick));
                self.active_tab = self.tabs.len() - 1;
            }
            UiCommand::SwitchChannel { cid, password } => {
                if let Some(tab) = self.tabs.get(self.active_tab) {
                    if cid != tab.my_cid {
                        if let Some(ref handle) = tab.handle {
                            let _ = handle.switch_channel(cid, password.as_deref());
                        }
                    }
                }
            }
            UiCommand::OpenPrivateChat { clid, nickname } => {
                if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                    let uid = tab
                        .clients
                        .iter()
                        .find(|c| c.clid == clid)
                        .map(|c| c.uid.clone())
                        .unwrap_or_default();
                    if !tab
                        .chat_state
                        .private_tabs
                        .iter()
                        .any(|pt| pt.clid == clid)
                    {
                        tab.chat_state.private_tabs.push(PrivateChatTab {
                            clid,
                            nickname,
                            uid,
                            lines: Vec::new(),
                            unread: 0,
                        });
                    }
                    tab.chat_state.active_tab = ActiveChatTab::Private(clid);
                }
            }
            UiCommand::OpenPokeDialog { clid, nickname } => {
                self.dialogs.poke_target_clid = clid;
                self.dialogs.poke_target_nick = nickname;
                self.dialogs.poke_message.clear();
                self.dialogs.show_poke = true;
            }
            UiCommand::SendRawCommand(raw) => {
                if let Some(tab) = self.tabs.get(self.active_tab) {
                    if let Some(ref handle) = tab.handle {
                        let _ = handle.send_raw_command(&raw);
                    }
                }
            }
        }
    }

    fn execute_dialog_action(&mut self, ctx: &egui::Context, action: DialogAction) {
        match action {
            DialogAction::Connect {
                server_addr,
                nickname,
                server_password,
                default_channel,
                default_channel_password,
                in_new_tab,
            } => {
                let target_idx = if in_new_tab {
                    self.tabs.push(ServerTabSession::new_empty(&nickname));
                    self.tabs.len() - 1
                } else {
                    self.active_tab
                };
                self.start_connection(
                    target_idx,
                    server_addr,
                    nickname,
                    server_password,
                    default_channel,
                    default_channel_password,
                );
            }
            DialogAction::CreateChannel {
                parent_cid,
                name,
                password,
                topic,
                description,
                codec,
                codec_quality,
                is_permanent,
                is_semi_permanent,
            } => {
                let mut cmd = Command::new("channelcreate")
                    .arg("channel_name", name)
                    .arg("cpid", parent_cid)
                    .arg("channel_topic", topic)
                    .arg("channel_description", description)
                    .arg("channel_codec", codec)
                    .arg("channel_codec_quality", codec_quality)
                    .arg_bool("channel_flag_permanent", is_permanent)
                    .arg_bool("channel_flag_semi_permanent", is_semi_permanent);
                if let Some(pw) = password {
                    if !pw.is_empty() {
                        cmd = cmd.arg("channel_password", hash_password(&pw));
                    }
                }
                let built = cmd.build();
                if let Some(tab) = self.tabs.get(self.active_tab) {
                    if let Some(ref handle) = tab.handle {
                        let _ = handle.send_raw_command(&built);
                        self.sound.play(SoundEvent::ChannelCreated);
                    }
                }
            }
            DialogAction::PokeClient { clid, message } => {
                if let Some(tab) = self.tabs.get(self.active_tab) {
                    if let Some(ref handle) = tab.handle {
                        let _ = handle.poke_client(clid, &message);
                    }
                }
            }
            DialogAction::Ui(ui_cmd) => {
                self.execute_ui_command(ctx, ui_cmd);
            }
        }
    }

    fn execute_chat_action(&mut self, ctx: &egui::Context, action: ChatOutgoingAction) {
        match action {
            ChatOutgoingAction::SendMessage { target, message } => {
                if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                    if let Some(ref handle) = tab.handle {
                        let _ = handle.send_chat(target, &message);
                        self.sound.play(SoundEvent::ChatSent);
                    } else {
                        tab.push_server_log(
                            ChatLineKind::ErrorAlert,
                            "Not connected to a server.".to_string(),
                        );
                    }
                }
            }
            ChatOutgoingAction::ExecuteSlashCommand(cmdline) => {
                let trimmed = cmdline.trim().trim_start_matches('/');
                let mut parts = trimmed.splitn(2, ' ');
                let verb = parts.next().unwrap_or("").to_ascii_lowercase();
                let arg = parts.next().unwrap_or("").trim();

                match verb.as_str() {
                    "nick" | "nickname" if !arg.is_empty() => {
                        if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                            tab.nickname = arg.to_string();
                            if let Some(ref handle) = tab.handle {
                                let _ = handle.change_nickname(arg);
                            }
                        }
                    }
                    "away" => {
                        let away = !self.toolbar.away;
                        let msg = if arg.is_empty() {
                            None
                        } else {
                            Some(arg.to_string())
                        };
                        self.execute_ui_command(
                            ctx,
                            UiCommand::ToggleAway {
                                away,
                                message: msg,
                            },
                        );
                    }
                    "disconnect" | "dc" => {
                        self.execute_ui_command(ctx, UiCommand::DisconnectCurrent);
                    }
                    "raw" if !arg.is_empty() => {
                        self.execute_ui_command(ctx, UiCommand::SendRawCommand(arg.to_string()));
                    }
                    _ => {
                        self.execute_ui_command(
                            ctx,
                            UiCommand::SendRawCommand(trimmed.to_string()),
                        );
                    }
                }
            }
            ChatOutgoingAction::Ui(ui_cmd) => {
                self.execute_ui_command(ctx, ui_cmd);
            }
        }
    }

    fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context, commands: &mut Vec<UiCommand>) {
        ctx.input(|i| {
            if i.modifiers.ctrl && i.key_pressed(egui::Key::S) {
                commands.push(UiCommand::OpenConnectDialog);
            }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::D) {
                commands.push(UiCommand::DisconnectCurrent);
            }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::B) {
                commands.push(UiCommand::OpenBookmarksDialog);
            }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::I) {
                commands.push(UiCommand::OpenIdentitiesDialog);
            }
            if i.modifiers.alt && i.key_pressed(egui::Key::P) {
                commands.push(UiCommand::OpenOptionsDialog);
            }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::T) {
                commands.push(UiCommand::NewServerTab);
            }
            if i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::M) {
                commands.push(UiCommand::ToggleMicMute);
            }
        });
    }
}

impl eframe::App for Ts3GuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.sound.enabled = self.dialogs.sound_enabled;
        self.poll_network_events();

        let mut ui_commands: Vec<UiCommand> = Vec::new();
        let mut dialog_actions: Vec<DialogAction> = Vec::new();
        let mut chat_actions: Vec<ChatOutgoingAction> = Vec::new();

        self.handle_keyboard_shortcuts(ctx, &mut ui_commands);

        // 1. Top Panel: MenuBar -> MainToolBar -> ServerTabBar
        egui::TopBottomPanel::top("ts3_top_chrome_panel")
            .frame(
                egui::Frame::NONE
                    .fill(self.palette.window_bg)
                    .inner_margin(egui::Margin::symmetric(4, 2)),
            )
            .show(ctx, |ui| {
                render_menu_bar(
                    ui,
                    &self.toolbar,
                    &self.palette,
                    &self.assets,
                    &mut ui_commands,
                );
                ui.separator();
                render_main_toolbar(
                    ui,
                    &mut self.toolbar,
                    &self.palette,
                    &self.assets,
                    &mut ui_commands,
                );
                ui.separator();

                let summaries: Vec<ServerTabSummary> = self
                    .tabs
                    .iter()
                    .enumerate()
                    .map(|(index, t)| ServerTabSummary {
                        index,
                        title: t.title.clone(),
                        connected: t.connected,
                        unread_count: t.total_unread(),
                    })
                    .collect();

                render_server_tab_bar(
                    ui,
                    &summaries,
                    self.active_tab,
                    &self.palette,
                    &self.assets,
                    &mut ui_commands,
                );
            });

        // 2. Bottom Panel: QStatusBar
        let status_info = if let Some(tab) = self.tabs.get(self.active_tab) {
            StatusBarInfo {
                status_text: tab.status_text.clone(),
                connected: tab.connected,
                server_name: tab.server.name.clone(),
                rtt_ms: tab.rtt_ms,
                packet_loss: tab.packet_loss,
                clients_online: tab.server.clients_online,
                max_clients: tab.server.max_clients,
            }
        } else {
            StatusBarInfo::default()
        };

        egui::TopBottomPanel::bottom("ts3_status_bar_panel")
            .exact_height(24.0)
            .frame(
                egui::Frame::NONE
                    .fill(self.palette.window_bg)
                    .inner_margin(egui::Margin::symmetric(6, 2)),
            )
            .show(ctx, |ui| {
                render_status_bar(ui, &status_info, &self.palette, &self.assets);
            });

        // 3. Bottom Resizable Splitter: MainWindowChatWidget (ChatTabWidget + ChatLineEdit)
        egui::TopBottomPanel::bottom("ts3_chat_splitter_panel")
            .resizable(true)
            .default_height(190.0)
            .min_height(100.0)
            .max_height(450.0)
            .frame(
                egui::Frame::NONE
                    .fill(self.palette.base_bg)
                    .inner_margin(egui::Margin::symmetric(4, 4)),
            )
            .show(ctx, |ui| {
                if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                    let srv_title = if tab.server.name.is_empty() {
                        tab.title.clone()
                    } else {
                        tab.server.name.clone()
                    };
                    let ch_title = tab.current_channel_name();
                    render_chat_pane(
                        ui,
                        &mut tab.chat_state,
                        &srv_title,
                        &ch_title,
                        tab.connected,
                        &self.palette,
                        &self.assets,
                        &mut chat_actions,
                    );
                }
            });

        // 4. Center Horizontal Splitter: Left = ServerTreeView, Right = InfoFrame
        egui::SidePanel::left("ts3_server_tree_splitter_panel")
            .resizable(true)
            .default_width(340.0)
            .min_width(180.0)
            .max_width(680.0)
            .frame(
                egui::Frame::NONE
                    .fill(self.palette.base_bg)
                    .inner_margin(egui::Margin::symmetric(4, 4)),
            )
            .show(ctx, |ui| {
                if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                    render_server_tree(
                        ui,
                        &mut tab.tree_state,
                        tab.connected,
                        tab.my_clid,
                        tab.my_cid,
                        &tab.server,
                        &tab.channels,
                        &tab.clients,
                        &self.palette,
                        &self.assets,
                        &mut ui_commands,
                    );
                }
            });

        egui::CentralPanel::default()
            .frame(
                egui::Frame::NONE
                    .fill(self.palette.base_bg)
                    .inner_margin(egui::Margin::symmetric(6, 4)),
            )
            .show(ctx, |ui| {
                if let Some(tab) = self.tabs.get(self.active_tab) {
                    let selected: &SelectedTreeNode = &tab.tree_state.selected;
                    render_info_pane(
                        ui,
                        selected,
                        tab.connected,
                        &tab.server_addr,
                        tab.my_clid,
                        tab.rtt_ms,
                        tab.packet_loss,
                        &tab.server,
                        &tab.channels,
                        &tab.clients,
                        &self.palette,
                        &self.assets,
                    );
                }
            });

        // 5. Modal Windows (`ConnectDialog`, `BookmarksManager`, `IdentityDialog`, etc.)
        render_dialogs(
            ctx,
            &mut self.dialogs,
            &self.palette,
            &self.assets,
            &mut dialog_actions,
        );

        // Dispatch all collected UI, Chat, and Dialog actions
        for cmd in ui_commands {
            self.execute_ui_command(ctx, cmd);
        }
        for act in chat_actions {
            self.execute_chat_action(ctx, act);
        }
        for d_act in dialog_actions {
            self.execute_dialog_action(ctx, d_act);
        }

        // Keep repainting regularly while any tab is connecting or connected so network events appear immediately
        if self.tabs.iter().any(|t| t.connected || t.connecting) {
            ctx.request_repaint_after(Duration::from_millis(50));
        }
    }
}

fn now_hms() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let tod = secs % 86_400;
    let h = tod / 3_600;
    let m = (tod % 3_600) / 60;
    let s = tod % 60;
    format!("{h:02}:{m:02}:{s:02}")
}

fn main() -> eframe::Result<()> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .try_init();

    let cli = StartupCliArgs::from_env();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("TeamSpeak 3")
            .with_inner_size([1024.0, 720.0])
            .with_min_inner_size([640.0, 480.0]),
        ..Default::default()
    };

    eframe::run_native(
        "TeamSpeak 3",
        native_options,
        Box::new(move |cc| Ok(Box::new(Ts3GuiApp::new(cc, cli)))),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use ts3_client::connection::ChatMessage;

    #[test]
    fn test_server_tab_session_events_and_logs() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let (connect_tx, connect_rx) = mpsc::unbounded_channel();
        let mut app = Ts3GuiApp {
            rt,
            tabs: vec![ServerTabSession::new_empty("Tester")],
            active_tab: 0,
            toolbar: ToolbarState::default(),
            dialogs: DialogsState::default(),
            assets: Ts3Assets::new(),
            sound: SoundManager {
                enabled: false,
                pack_name: "Test".to_string(),
                wav_paths: Default::default(),
            },
            palette: Ts3Palette::light(),
            connect_tx,
            connect_rx,
            theme_applied: true,
        };

        let srv = VirtualServerInfo {
            name: "Los Retrasados Mk.4".to_string(),
            welcome_message: "Welcome to TS3!".to_string(),
            max_clients: 32,
            ..Default::default()
        };
        app.handle_ts_event(
            0,
            TsEvent::Connected {
                my_clid: 5,
                server: srv,
            },
        );
        assert!(app.tabs[0].connected);
        assert_eq!(app.tabs[0].my_clid, 5);
        assert_eq!(app.tabs[0].title, "Los Retrasados Mk.4");
        assert!(app.tabs[0]
            .chat_state
            .server_lines
            .iter()
            .any(|l| l.text.contains("Connected to Server: \"Los Retrasados Mk.4\"")));

        app.handle_ts_event(
            0,
            TsEvent::ChannelsUpdated(vec![ChannelInfo {
                cid: 1,
                pid: 0,
                name: "Lobby".to_string(),
                is_default: true,
                ..Default::default()
            }]),
        );
        assert_eq!(app.tabs[0].channels.len(), 1);

        app.handle_ts_event(
            0,
            TsEvent::ClientsUpdated(vec![ClientInfo {
                clid: 5,
                cid: 1,
                nickname: "Tester".to_string(),
                ..Default::default()
            }]),
        );
        assert_eq!(app.tabs[0].my_cid, 1);

        app.handle_ts_event(
            0,
            TsEvent::ChatReceived(ChatMessage {
                timestamp: "12:00:00".to_string(),
                target: ChatTarget::Channel,
                invoker_id: 5,
                invoker_name: "Tester".to_string(),
                invoker_uid: "uid=".to_string(),
                message: "Hello channel!".to_string(),
            }),
        );
        assert_eq!(app.tabs[0].chat_state.channel_lines.len(), 1);

        app.handle_ts_event(
            0,
            TsEvent::PokeReceived {
                invoker_id: 9,
                invoker_name: "Admin".to_string(),
                message: "Wake up!".to_string(),
            },
        );
        assert!(app.dialogs.show_received_poke);
        assert_eq!(app.dialogs.received_poke_invoker, "Admin");

        app.handle_ts_event(
            0,
            TsEvent::PingUpdated {
                rtt_ms: 23,
                packet_loss: 0.5,
            },
        );
        assert_eq!(app.tabs[0].rtt_ms, 23);

        app.handle_ts_event(
            0,
            TsEvent::Disconnected {
                reason: "server shutdown".to_string(),
            },
        );
        assert!(!app.tabs[0].connected);
    }
}


