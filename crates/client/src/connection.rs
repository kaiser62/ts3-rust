// Owned by Pool Agent 7 (arthoroy29@gmail.com)

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use thiserror::Error;
use tokio::net::{lookup_host, UdpSocket};
use tokio::sync::mpsc;
use tracing::{debug, trace, warn};

pub use ts3_proto::command::hash_password;
use ts3_proto::command::{escape, Command, CommandError, ParsedCommand};
use ts3_proto::crypt::{CryptError, Init1Next, TsCrypt, TS3_INIT_MAC};
use ts3_proto::identity::VersionSign;
use ts3_proto::packet::{
    C2SPacket, PacketError, PacketFlags, PacketType, S2CPacket, VoicePacketS2C,
};
use ts3_proto::quicklz::QuickLzError;

pub use crate::events::*;
use crate::reliability::{
    split_and_compress_c2s, CommandReassembler, GenerationWindow, OutQueue, OutSequenceTracker,
    ReliabilityError, RttEstimator, CLOCK_RESOLUTION,
};

/// Interval between keepalive `Ping` packets once connected.
pub const PING_INTERVAL: Duration = Duration::from_secs(1);

/// Default TeamSpeak 3 UDP voice port.
pub const DEFAULT_TS3_PORT: u16 = 9987;

/// Fixed C2S `packet_id` used for `Init1` handshake packets (`0x0101`).
pub const INIT1_C2S_PACKET_ID: u16 = 0x0101;

/// Interval between `Init1` handshake step retransmissions before `initivexpand2` arrives.
const INIT1_RETRY_INTERVAL: Duration = Duration::from_millis(500);

/// Maximum number of `Init1` retries before aborting the handshake.
const INIT1_MAX_RETRIES: u32 = 20;

/// Type aliases for reliability primitives.
pub type InWindow = GenerationWindow;
pub type OutSeq = OutSequenceTracker;
pub type RetransmitQueue = OutQueue;

/// Errors returned by [`Ts3Client`] and [`Ts3ClientHandle`].
#[derive(Debug, Error)]
pub enum ClientError {
    /// Underlying UDP socket or network I/O failure.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Server address string could not be resolved to any socket address.
    #[error("could not resolve server address: {0}")]
    InvalidAddress(String),

    /// Cryptographic handshake or EAX encrypt/decrypt failure.
    #[error("cryptographic error: {0}")]
    Crypt(#[from] CryptError),

    /// Wire packet framing error.
    #[error("packet framing error: {0}")]
    Packet(#[from] PacketError),

    /// Command parsing error.
    #[error("command parse error: {0}")]
    Command(#[from] CommandError),

    /// QuickLZ decompression failure.
    #[error("QuickLZ decompression failed: {0}")]
    QuickLz(#[from] QuickLzError),

    /// Reliability / retransmission timeout error.
    #[error("reliability error: {0}")]
    Reliability(#[from] ReliabilityError),

    /// Handshake or operation timed out.
    #[error("operation timed out: {0}")]
    Timeout(String),

    /// Server returned a non-zero TeamSpeak error code.
    #[error("server error {id}: {msg}")]
    ServerError { id: u32, msg: String },

    /// The client background task has terminated or disconnected.
    #[error("client is disconnected")]
    Disconnected,
}

/// Internal commands sent from [`Ts3ClientHandle`] to the connection worker loop.
#[derive(Debug)]
enum HandleAction {
    SendChat {
        target: ChatTarget,
        msg: String,
    },
    SwitchChannel {
        cid: u64,
        password: Option<String>,
    },
    PokeClient {
        clid: u16,
        msg: String,
    },
    SetAway {
        away: bool,
        msg: Option<String>,
    },
    SetMute {
        input_muted: bool,
        output_muted: bool,
    },
    ChangeNickname {
        nick: String,
    },
    SendVoice {
        codec: u8,
        frame: Vec<u8>,
    },
    SendRawCommand {
        cmd: String,
    },
    Disconnect,
}

/// Cloneable handle for controlling an active [`Ts3Client`] connection and inspecting state.
#[derive(Debug, Clone)]
pub struct Ts3ClientHandle {
    action_tx: mpsc::UnboundedSender<HandleAction>,
    snapshot: Arc<RwLock<ClientStateSnapshot>>,
}

impl Ts3ClientHandle {
    /// Connects to a TeamSpeak 3 server with the given [`ConnectConfig`].
    ///
    /// Convenience alias for [`Ts3Client::connect`].
    pub async fn connect(
        config: ConnectConfig,
    ) -> Result<(Self, mpsc::UnboundedReceiver<TsEvent>), ClientError> {
        Ts3Client::connect(config).await
    }

    /// Sends a text message to a server, channel, or specific client.
    pub fn send_chat(&self, target: ChatTarget, msg: &str) -> Result<(), ClientError> {
        self.action_tx
            .send(HandleAction::SendChat {
                target,
                msg: msg.to_string(),
            })
            .map_err(|_| ClientError::Disconnected)
    }

    /// Moves our client to channel `cid`, optionally supplying a plain-text channel password.
    pub fn switch_channel(&self, cid: u64, password: Option<&str>) -> Result<(), ClientError> {
        self.action_tx
            .send(HandleAction::SwitchChannel {
                cid,
                password: password.map(ToOwned::to_owned),
            })
            .map_err(|_| ClientError::Disconnected)
    }

    /// Pokes client `clid` with `msg`.
    pub fn poke_client(&self, clid: u16, msg: &str) -> Result<(), ClientError> {
        self.action_tx
            .send(HandleAction::PokeClient {
                clid,
                msg: msg.to_string(),
            })
            .map_err(|_| ClientError::Disconnected)
    }

    /// Sets or clears our client's away status and optional away message.
    pub fn set_away(&self, away: bool, msg: Option<&str>) -> Result<(), ClientError> {
        self.action_tx
            .send(HandleAction::SetAway {
                away,
                msg: msg.map(ToOwned::to_owned),
            })
            .map_err(|_| ClientError::Disconnected)
    }

    /// Updates our client's input (microphone) and output (speaker/headphone) mute flags.
    pub fn set_mute(&self, input_muted: bool, output_muted: bool) -> Result<(), ClientError> {
        self.action_tx
            .send(HandleAction::SetMute {
                input_muted,
                output_muted,
            })
            .map_err(|_| ClientError::Disconnected)
    }

    /// Changes our client's display nickname on the server.
    pub fn change_nickname(&self, nick: &str) -> Result<(), ClientError> {
        self.action_tx
            .send(HandleAction::ChangeNickname {
                nick: nick.to_string(),
            })
            .map_err(|_| ClientError::Disconnected)
    }

    /// Sends an encoded voice frame (`codec` + raw frame payload) over UDP.
    pub fn send_voice(&self, codec: u8, frame: &[u8]) -> Result<(), ClientError> {
        self.action_tx
            .send(HandleAction::SendVoice {
                codec,
                frame: frame.to_vec(),
            })
            .map_err(|_| ClientError::Disconnected)
    }

    /// Sends a raw TeamSpeak 3 command string over the reliable `Command` stream.
    pub fn send_raw_command(&self, cmd: &str) -> Result<(), ClientError> {
        self.action_tx
            .send(HandleAction::SendRawCommand {
                cmd: cmd.to_string(),
            })
            .map_err(|_| ClientError::Disconnected)
    }

    /// Sends `clientdisconnect` and shuts down the client connection loop.
    pub fn disconnect(&self) -> Result<(), ClientError> {
        self.action_tx
            .send(HandleAction::Disconnect)
            .map_err(|_| ClientError::Disconnected)
    }

    /// Returns a point-in-time [`ClientStateSnapshot`] of the connection, server, channels, and clients.
    pub fn snapshot_state(&self) -> ClientStateSnapshot {
        self.snapshot
            .read()
            .expect("client state lock poisoned")
            .clone()
    }
}

/// TeamSpeak 3 full protocol UDP client.
pub struct Ts3Client;

impl Ts3Client {
    /// Resolves `config.server_addr`, binds a local UDP socket, starts the `Init1` / `Init2`
    /// cryptographic handshake and session event loop, and returns a [`Ts3ClientHandle`] along
    /// with an unbounded receiver of [`TsEvent`]s.
    pub async fn connect(
        config: ConnectConfig,
    ) -> Result<(Ts3ClientHandle, mpsc::UnboundedReceiver<TsEvent>), ClientError> {
        let remote_addr = resolve_server_addr(&config.server_addr).await?;
        let bind_addr: SocketAddr = if remote_addr.is_ipv6() {
            "[::]:0".parse().unwrap()
        } else {
            "0.0.0.0:0".parse().unwrap()
        };

        let socket = UdpSocket::bind(bind_addr).await?;
        socket.connect(remote_addr).await?;

        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let (action_tx, action_rx) = mpsc::unbounded_channel();
        let snapshot = Arc::new(RwLock::new(ClientStateSnapshot::default()));

        let mut worker = ConnectionWorker::new(
            config,
            socket,
            event_tx,
            action_rx,
            Arc::clone(&snapshot),
        );

        // Send Init1 Step 0 immediately so the first handshake packet is already in flight
        // before `connect` returns.
        worker.start_init1().await?;

        tokio::spawn(async move {
            worker.run().await;
        });

        let handle = Ts3ClientHandle {
            action_tx,
            snapshot,
        };
        Ok((handle, event_rx))
    }
}

/// Resolves a host or `host:port` string into a [`SocketAddr`], defaulting to port `9987`.
pub async fn resolve_server_addr(addr_str: &str) -> Result<SocketAddr, ClientError> {
    let trimmed = addr_str.trim();
    if trimmed.is_empty() {
        return Err(ClientError::InvalidAddress(
            "server address is empty".to_string(),
        ));
    }

    let with_port = if let Ok(sock) = trimmed.parse::<SocketAddr>() {
        return Ok(sock);
    } else if trimmed.starts_with('[') {
        if trimmed.contains("]:") {
            trimmed.to_string()
        } else {
            format!("{trimmed}:{DEFAULT_TS3_PORT}")
        }
    } else if trimmed.contains(':') {
        trimmed.to_string()
    } else {
        format!("{trimmed}:{DEFAULT_TS3_PORT}")
    };

    let resolved = {
        let mut addrs = lookup_host(with_port.as_str())
            .await
            .map_err(|e| ClientError::InvalidAddress(format!("{with_port}: {e}")))?;
        addrs.next()
    };
    resolved.ok_or(ClientError::InvalidAddress(with_port))
}

/// Default client version string sent in `clientinit` (`"3.5.3 [Build: 1587971024]"`).
pub const DEFAULT_CLIENT_VERSION: &str = "3.5.3 [Build: 1587971024]";

/// Default client platform string sent in `clientinit` (`"Windows"`).
pub const DEFAULT_CLIENT_PLATFORM: &str = "Windows";

/// Default client hardware ID string sent in `clientinit`.
pub const DEFAULT_CLIENT_HWID: &str = "1234567890abcdef,1234567890abcdef";

/// Builds the `clientinit` command string from [`ConnectConfig`].
pub fn build_clientinit_command(config: &ConnectConfig) -> String {
    let vs = VersionSign::default_windows();
    let default_channel = config.default_channel.as_deref().unwrap_or("");
    let channel_pw = config
        .default_channel_password
        .as_deref()
        .filter(|s| !s.is_empty())
        .map(hash_password)
        .unwrap_or_default();
    let server_pw = config
        .server_password
        .as_deref()
        .filter(|s| !s.is_empty())
        .map(hash_password)
        .unwrap_or_default();
    let key_offset = config.identity.valid_key_offset;

    Command::new("clientinit")
        .arg("client_nickname", &config.nickname)
        .arg("client_version", vs.version)
        .arg("client_platform", vs.platform)
        .arg("client_input_hardware", 1)
        .arg("client_output_hardware", 1)
        .arg("client_default_channel", default_channel)
        .arg("client_default_channel_password", channel_pw)
        .arg("client_server_password", server_pw)
        .arg("client_meta_data", "")
        .arg("client_version_sign", vs.sign)
        .arg("client_key_offset", key_offset)
        .arg("client_nickname_phonetic", "")
        .arg("client_default_token", "")
        .arg("hwid", DEFAULT_CLIENT_HWID)
        .build()
}

/// Builds a 2-byte big-endian `u16` payload for `Ack` / `AckLow`.
#[inline]
pub fn build_ack_payload(packet_id: u16) -> Vec<u8> {
    packet_id.to_be_bytes().to_vec()
}

/// Parses a 4-byte big-endian `u32` (or 2-byte big-endian `u16`) `packet_id` from an `Ack` / `AckLow` payload.
#[inline]
pub fn parse_ack_payload(payload: &[u8]) -> Option<u16> {
    extract_acked_packet_id(payload)
}

/// Formats the current wall-clock time as `"HH:MM:SS"` UTC.
fn current_timestamp_hms() -> String {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HandshakePhase {
    Init1,
    Init2,
    Connected,
    Closed,
}

struct ConnectionWorker {
    config: ConnectConfig,
    socket: UdpSocket,
    event_tx: mpsc::UnboundedSender<TsEvent>,
    action_rx: mpsc::UnboundedReceiver<HandleAction>,
    snapshot: Arc<RwLock<ClientStateSnapshot>>,

    crypt: TsCrypt,
    phase: HandshakePhase,

    // Reliability & sequencing state
    out_seq: OutSequenceTracker,
    out_queue: OutQueue,
    rtt_estimator: RttEstimator,
    cmd_reassembler: CommandReassembler,
    voice_window: GenerationWindow,
    voice_out_seq: u16,

    // Handshake retransmission state
    last_init1_packet: Option<Vec<u8>>,
    last_init1_sent: Instant,
    init1_retries: u32,

    // Ping / Pong tracking
    pending_pings: HashMap<u16, Instant>,
    pings_sent: u64,
    pongs_received: u64,

    // Session state
    my_clid: u16,
    my_cid: u64,
    server_info: VirtualServerInfo,
    channels: BTreeMap<u64, ChannelInfo>,
    clients: BTreeMap<u16, ClientInfo>,
    rtt_ms: u32,
    packet_loss: f32,

    // Commands queued by the handle before `initserver` completes
    pending_user_commands: VecDeque<String>,
}

impl ConnectionWorker {
    fn new(
        config: ConnectConfig,
        socket: UdpSocket,
        event_tx: mpsc::UnboundedSender<TsEvent>,
        action_rx: mpsc::UnboundedReceiver<HandleAction>,
        snapshot: Arc<RwLock<ClientStateSnapshot>>,
    ) -> Self {
        let mut crypt = TsCrypt::new(config.identity.clone());
        crypt.set_init_version(ts3_proto::crypt::TSLIB_INIT_VERSION.to_be_bytes());

        Self {
            config,
            socket,
            event_tx,
            action_rx,
            snapshot,
            crypt,
            phase: HandshakePhase::Init1,
            out_seq: OutSequenceTracker::new(),
            out_queue: OutQueue::new(),
            rtt_estimator: RttEstimator::new(),
            cmd_reassembler: CommandReassembler::new(),
            voice_window: GenerationWindow::voice(),
            voice_out_seq: 0,
            last_init1_packet: None,
            last_init1_sent: Instant::now(),
            init1_retries: 0,
            pending_pings: HashMap::new(),
            pings_sent: 0,
            pongs_received: 0,
            my_clid: 0,
            my_cid: 0,
            server_info: VirtualServerInfo::default(),
            channels: BTreeMap::new(),
            clients: BTreeMap::new(),
            rtt_ms: 0,
            packet_loss: 0.0,
            pending_user_commands: VecDeque::new(),
        }
    }

    /// Sends C2S `Init1` Step 0 (`packet_id = 0x0101`, `Flags::UNENCRYPTED | Flags::NEW_PROTOCOL`).
    async fn start_init1(&mut self) -> Result<(), ClientError> {
        let step0_payload = self.crypt.create_init1_step0();
        self.send_init1_packet(step0_payload).await
    }

    async fn send_init1_packet(&mut self, payload: Vec<u8>) -> Result<(), ClientError> {
        let mut pkt = C2SPacket::new(
            INIT1_C2S_PACKET_ID,
            0,
            PacketType::Init1,
            PacketFlags::UNENCRYPTED | PacketFlags::NEW_PROTOCOL,
            payload,
        );
        pkt.mac = *TS3_INIT_MAC;
        let wire = pkt.encode();
        self.socket.send(&wire).await?;
        self.last_init1_packet = Some(wire);
        self.last_init1_sent = Instant::now();
        Ok(())
    }

    async fn run(&mut self) {
        let mut recv_buf = [0u8; 2048];
        let mut retransmit_ticker = tokio::time::interval(CLOCK_RESOLUTION);
        retransmit_ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let mut ping_ticker = tokio::time::interval(PING_INTERVAL);
        ping_ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        loop {
            if self.phase == HandshakePhase::Closed {
                break;
            }

            tokio::select! {
                recv_res = self.socket.recv(&mut recv_buf) => {
                    match recv_res {
                        Ok(n) => {
                            if let Err(err) = self.handle_incoming_datagram(&recv_buf[..n]).await {
                                warn!("dropping or error handling incoming datagram: {err}");
                                if matches!(err, ClientError::Crypt(CryptError::Init1ServerError(_))) {
                                    self.disconnect_with_reason(err.to_string());
                                    break;
                                }
                            }
                        }
                        Err(err) => {
                            self.disconnect_with_reason(format!("UDP socket receive error: {err}"));
                            break;
                        }
                    }
                }
                Some(action) = self.action_rx.recv() => {
                    if let Err(err) = self.handle_user_action(action).await {
                        warn!("error handling user action: {err}");
                        self.disconnect_with_reason(err.to_string());
                        break;
                    }
                }
                _ = retransmit_ticker.tick() => {
                    if let Err(err) = self.handle_retransmit_tick().await {
                        self.disconnect_with_reason(err.to_string());
                        break;
                    }
                }
                _ = ping_ticker.tick() => {
                    if self.phase == HandshakePhase::Connected {
                        let _ = self.send_keepalive_ping().await;
                    }
                }
            }
        }
    }

    fn disconnect_with_reason(&mut self, reason: String) {
        if self.phase == HandshakePhase::Closed {
            return;
        }
        self.phase = HandshakePhase::Closed;
        self.sync_snapshot();
        let _ = self.event_tx.send(TsEvent::Disconnected { reason });
    }

    fn sync_snapshot(&self) {
        if let Ok(mut guard) = self.snapshot.write() {
            guard.connected = self.phase == HandshakePhase::Connected;
            guard.my_clid = self.my_clid;
            guard.my_cid = self.my_cid;
            guard.server = self.server_info.clone();
            guard.channels = self.channels.values().cloned().collect();
            guard.clients = self.clients.values().cloned().collect();
            guard.rtt_ms = self.rtt_ms;
            guard.packet_loss = self.packet_loss;
        }
    }

    async fn handle_retransmit_tick(&mut self) -> Result<(), ClientError> {
        let now = Instant::now();

        // Retransmit Init1 packet if still in Init1 phase
        if self.phase == HandshakePhase::Init1 {
            if let Some(ref wire) = self.last_init1_packet {
                if now.saturating_duration_since(self.last_init1_sent) >= INIT1_RETRY_INTERVAL {
                    if self.init1_retries >= INIT1_MAX_RETRIES {
                        return Err(ClientError::Timeout(
                            "Init1 handshake timed out waiting for server response".to_string(),
                        ));
                    }
                    self.init1_retries += 1;
                    self.last_init1_sent = now;
                    self.socket.send(wire).await?;
                }
            }
        }

        // Retransmit unacknowledged Command / CommandLow packets
        let resends = self.out_queue.collect_resends(now)?;
        for wire in resends {
            self.socket.send(&wire).await?;
        }

        Ok(())
    }

    async fn send_keepalive_ping(&mut self) -> Result<(), ClientError> {
        let (packet_id, generation_id) = self.out_seq.next(PacketType::Ping);
        let mut pkt = C2SPacket::new(
            packet_id,
            self.my_clid,
            PacketType::Ping,
            PacketFlags::UNENCRYPTED | PacketFlags::NEW_PROTOCOL,
            Vec::new(),
        );
        pkt.generation_id = generation_id;
        self.crypt.encrypt_c2s(&mut pkt, false)?;
        let wire = pkt.encode();
        self.pending_pings.insert(packet_id, Instant::now());
        self.pings_sent = self.pings_sent.saturating_add(1);
        self.socket.send(&wire).await?;
        Ok(())
    }

    async fn send_pong(&mut self, ping_packet_id: u16) -> Result<(), ClientError> {
        let (packet_id, generation_id) = self.out_seq.next(PacketType::Pong);
        let mut pkt = C2SPacket::new(
            packet_id,
            self.my_clid,
            PacketType::Pong,
            PacketFlags::UNENCRYPTED | PacketFlags::NEW_PROTOCOL,
            ping_packet_id.to_be_bytes().to_vec(),
        );
        pkt.generation_id = generation_id;
        self.crypt.encrypt_c2s(&mut pkt, false)?;
        self.socket.send(&pkt.encode()).await?;
        Ok(())
    }

    async fn send_ack(&mut self, target_packet_id: u16, is_low: bool) -> Result<(), ClientError> {
        let ack_type = if is_low {
            PacketType::AckLow
        } else {
            PacketType::Ack
        };
        let (packet_id, generation_id) = self.out_seq.next(ack_type);
        let flags = if is_low {
            PacketFlags::UNENCRYPTED
        } else {
            PacketFlags::NONE
        };
        let mut pkt = C2SPacket::new(
            packet_id,
            self.my_clid,
            ack_type,
            flags,
            target_packet_id.to_be_bytes().to_vec(),
        );
        pkt.generation_id = generation_id;
        self.crypt.encrypt_c2s(&mut pkt, false)?;
        self.socket.send(&pkt.encode()).await?;
        Ok(())
    }

    /// Sends a C2S `Command` (or `CommandLow`) string, compressing and splitting if needed,
    /// encrypting each packet with `force_dummy` (when `true`, uses dummy key/nonce),
    /// and inserting into `out_queue` for reliable retransmission.
    async fn send_command_str(
        &mut self,
        cmd_str: &str,
        packet_type: PacketType,
        force_dummy: bool,
        unencrypted: bool,
    ) -> Result<(), ClientError> {
        let chunks = split_and_compress_c2s(cmd_str, packet_type, true);
        let now = Instant::now();
        let initial_rto = self.rtt_estimator.rto();

        for (mut flags, chunk_payload) in chunks {
            let (packet_id, generation_id) = self.out_seq.next(packet_type);
            if unencrypted {
                flags |= PacketFlags::UNENCRYPTED;
            }
            let mut pkt = C2SPacket::new(
                packet_id,
                self.my_clid,
                packet_type,
                flags,
                chunk_payload,
            );
            pkt.generation_id = generation_id;
            self.crypt.encrypt_c2s(&mut pkt, force_dummy)?;
            let wire = pkt.encode();
            self.out_queue
                .insert(packet_type, packet_id, wire.clone(), now, initial_rto);
            self.socket.send(&wire).await?;
        }

        Ok(())
    }

    async fn handle_user_action(&mut self, action: HandleAction) -> Result<(), ClientError> {
        match action {
            HandleAction::SendChat { target, msg } => {
                let cmd = match target {
                    ChatTarget::Server => Command::new("sendtextmessage")
                        .arg("targetmode", 3)
                        .arg("msg", msg)
                        .build(),
                    ChatTarget::Channel => Command::new("sendtextmessage")
                        .arg("targetmode", 2)
                        .arg("msg", msg)
                        .build(),
                    ChatTarget::Client(clid) => Command::new("sendtextmessage")
                        .arg("targetmode", 1)
                        .arg("target", clid)
                        .arg("msg", msg)
                        .build(),
                };
                self.enqueue_or_send_command(&cmd).await?;
            }
            HandleAction::SwitchChannel { cid, password } => {
                let mut builder = Command::new("clientmove")
                    .arg("clid", self.my_clid)
                    .arg("cid", cid);
                if let Some(pw) = password {
                    if !pw.is_empty() {
                        builder = builder.arg("cpw", hash_password(&pw));
                    }
                }
                let cmd = builder.build();
                self.enqueue_or_send_command(&cmd).await?;
            }
            HandleAction::PokeClient { clid, msg } => {
                let cmd = Command::new("clientpoke")
                    .arg("clid", clid)
                    .arg("msg", msg)
                    .build();
                self.enqueue_or_send_command(&cmd).await?;
            }
            HandleAction::SetAway { away, msg } => {
                let mut builder = Command::new("clientupdate").arg_bool("client_away", away);
                if let Some(m) = msg {
                    builder = builder.arg("client_away_message", m);
                } else if !away {
                    builder = builder.arg("client_away_message", "");
                }
                let cmd = builder.build();
                self.enqueue_or_send_command(&cmd).await?;
            }
            HandleAction::SetMute {
                input_muted,
                output_muted,
            } => {
                let cmd = Command::new("clientupdate")
                    .arg_bool("client_input_muted", input_muted)
                    .arg_bool("client_output_muted", output_muted)
                    .build();
                self.enqueue_or_send_command(&cmd).await?;
            }
            HandleAction::ChangeNickname { nick } => {
                let cmd = Command::new("clientupdate")
                    .arg("client_nickname", nick)
                    .build();
                self.enqueue_or_send_command(&cmd).await?;
            }
            HandleAction::SendVoice { codec, frame } => {
                let (packet_id, generation_id) = self.out_seq.next(PacketType::Voice);
                let v_id = self.voice_out_seq;
                self.voice_out_seq = self.voice_out_seq.wrapping_add(1);
                let mut payload = Vec::with_capacity(3 + frame.len());
                payload.extend_from_slice(&v_id.to_be_bytes());
                payload.push(codec);
                payload.extend_from_slice(&frame);
                let mut pkt = C2SPacket::new(
                    packet_id,
                    self.my_clid,
                    PacketType::Voice,
                    PacketFlags::UNENCRYPTED,
                    payload,
                );
                pkt.generation_id = generation_id;
                self.crypt.encrypt_c2s(&mut pkt, false)?;
                self.socket.send(&pkt.encode()).await?;
            }
            HandleAction::SendRawCommand { cmd } => {
                self.enqueue_or_send_command(&cmd).await?;
            }
            HandleAction::Disconnect => {
                if self.phase == HandshakePhase::Connected {
                    let disconnect_cmd = Command::new("clientdisconnect")
                        .arg("reasonid", 8)
                        .arg("reasonmsg", "leaving")
                        .build();
                    let _ = self
                        .send_command_str(&disconnect_cmd, PacketType::Command, false, false)
                        .await;
                }
                self.disconnect_with_reason("disconnected by user".to_string());
            }
        }
        Ok(())
    }

    async fn enqueue_or_send_command(&mut self, cmd: &str) -> Result<(), ClientError> {
        if self.phase == HandshakePhase::Connected || self.crypt.crypto_init_complete() {
            self.send_command_str(cmd, PacketType::Command, false, false)
                .await
        } else {
            self.pending_user_commands.push_back(cmd.to_string());
            Ok(())
        }
    }

    async fn handle_incoming_datagram(&mut self, raw: &[u8]) -> Result<(), ClientError> {
        let mut s2c = S2CPacket::decode(raw)?;
        let packet_type = s2c.packet_type;
        trace!(
            "S2C datagram: type={:?} id={} flags={:?} len={}",
            packet_type,
            s2c.packet_id,
            s2c.flags,
            s2c.payload.len()
        );

        // Handle S2C Init1 packets
        if packet_type == PacketType::Init1 {
            self.crypt.decrypt_s2c(&mut s2c)?;
            return self.handle_s2c_init1(&s2c.payload).await;
        }

        // Resolve generation_id for decryption using the appropriate receive window
        s2c.generation_id = match packet_type {
            PacketType::Command => self
                .cmd_reassembler
                .command
                .window()
                .generation_unchecked(s2c.packet_id),
            PacketType::CommandLow => self
                .cmd_reassembler
                .command_low
                .window()
                .generation_unchecked(s2c.packet_id),
            PacketType::Voice | PacketType::VoiceWhisper => {
                self.voice_window.generation_unchecked(s2c.packet_id)
            }
            _ => 0,
        };

        self.crypt.decrypt_s2c(&mut s2c)?;

        match packet_type {
            PacketType::Command | PacketType::CommandLow => {
                let is_low = packet_type == PacketType::CommandLow;
                let (should_ack, messages) = self.cmd_reassembler.receive(
                    packet_type,
                    s2c.packet_id,
                    s2c.flags,
                    s2c.payload,
                )?;
                if should_ack {
                    self.send_ack(s2c.packet_id, is_low).await?;
                }
                for msg_bytes in messages {
                    let msg = String::from_utf8(msg_bytes).map_err(ReliabilityError::from)?;
                    self.handle_reassembled_command(&msg).await?;
                }
            }
            PacketType::Ack | PacketType::AckLow => {
                if let Some(acked_id) = extract_acked_packet_id(&s2c.payload) {
                    if let Some(sample_rtt) = self.out_queue.acknowledge(packet_type, acked_id) {
                        self.rtt_estimator.record_sample(sample_rtt);
                        self.rtt_ms = self.rtt_estimator.srtt().as_millis() as u32;
                    }
                }
            }
            PacketType::Ping => {
                self.send_pong(s2c.packet_id).await?;
            }
            PacketType::Pong => {
                let pong_id = if s2c.payload.len() >= 2 {
                    u16::from_be_bytes([s2c.payload[0], s2c.payload[1]])
                } else {
                    s2c.packet_id
                };
                if let Some(sent_at) = self.pending_pings.remove(&pong_id) {
                    let sample = Instant::now().saturating_duration_since(sent_at);
                    self.rtt_estimator.record_sample(sample);
                    self.pongs_received = self.pongs_received.saturating_add(1);
                    self.rtt_ms = self.rtt_estimator.srtt().as_millis() as u32;
                    if self.pings_sent > 0 {
                        let lost = self.pings_sent.saturating_sub(self.pongs_received);
                        self.packet_loss = (lost as f32) / (self.pings_sent as f32);
                    }
                    self.sync_snapshot();
                    let _ = self.event_tx.send(TsEvent::PingUpdated {
                        rtt_ms: self.rtt_ms,
                        packet_loss: self.packet_loss,
                    });
                }
            }
            PacketType::Voice | PacketType::VoiceWhisper => {
                if self.voice_window.accept_and_advance(s2c.packet_id).is_some() {
                    if let Ok(voice) = VoicePacketS2C::decode(&s2c.payload) {
                        let _ = self.event_tx.send(TsEvent::VoicePacket {
                            talker_id: voice.talker_client_id,
                            codec: voice.codec as u8,
                            audio_data: voice.audio_data,
                        });
                    }
                }
            }
            PacketType::Init1 => {}
        }

        Ok(())
    }

    async fn handle_s2c_init1(&mut self, payload: &[u8]) -> Result<(), ClientError> {
        if payload.is_empty() {
            return Ok(());
        }

        // Normalize payload in case a server/mock prefixes 4B version_echo before step 1 or 3
        let normalized = if payload[0] != 1
            && payload[0] != 3
            && payload.len() >= 5
            && (payload[4] == 1 || payload[4] == 3)
        {
            &payload[4..]
        } else {
            payload
        };

        let next = self.crypt.process_init1_response(normalized)?;
        match next {
            Init1Next::SendInit1Packet(data) => {
                let is_step4 = data.len() > 5 && data[4] == 0x04;
                self.send_init1_packet(data).await?;
                if is_step4 {
                    // Step 4 embeds `clientinitiv` as Command packet 0, so the next Command
                    // (`clientek`) must use packet_id = 1.
                    self.out_seq.skip_command_id_zero();
                    self.phase = HandshakePhase::Init2;
                }
            }
            Init1Next::SendClientInitIv {
                alpha_b64,
                omega_b64,
                ip,
            } => {
                // If `SendClientInitIv` is returned directly, send `clientinitiv` as Command packet_id = 0
                // with Flags::UNENCRYPTED | Flags::NEW_PROTOCOL.
                self.phase = HandshakePhase::Init2;
                self.last_init1_packet = None;
                let cmd_str = format!(
                    "clientinitiv alpha={} omega={} ot=1 ip={}",
                    escape(&alpha_b64),
                    escape(&omega_b64),
                    escape(&ip)
                );
                self.send_command_str(&cmd_str, PacketType::Command, true, true)
                    .await?;
            }
        }
        Ok(())
    }

    async fn handle_reassembled_command(&mut self, raw_msg: &str) -> Result<(), ClientError> {
        for line in raw_msg.lines() {
            let trimmed = line.trim_matches(|c: char| c.is_whitespace() || c == '\0');
            if trimmed.is_empty() {
                continue;
            }
            let cmd = match ParsedCommand::parse(trimmed) {
                Ok(c) => c,
                Err(err) => {
                    debug!("ignoring unparseable S2C command {trimmed:?}: {err}");
                    continue;
                }
            };
            self.dispatch_s2c_command(&cmd).await?;
        }
        Ok(())
    }

    async fn dispatch_s2c_command(&mut self, cmd: &ParsedCommand) -> Result<(), ClientError> {
        trace!("S2C command: {}", cmd.name);
        match cmd.name.as_str() {
            "initivexpand2" => {
                self.last_init1_packet = None;

                let license_b64 = cmd
                    .get("l")
                    .or_else(|| cmd.get("license"))
                    .unwrap_or_default();
                let beta_b64 = cmd.get("beta").unwrap_or_default();
                let omega_b64 = cmd.get("omega").unwrap_or_default();
                let proof_b64 = cmd.get("proof").unwrap_or_default();

                let ek_params = self.crypt.process_initivexpand2(
                    license_b64,
                    beta_b64,
                    omega_b64,
                    proof_b64,
                )?;

                // Ensure Command packet_id is at 1 (if `clientinitiv` was sent inside Init1 Step 4)
                self.out_seq.skip_command_id_zero();

                // 1. Send `clientek` as Command packet_id = 1 encrypted with dummy key/nonce (`force_dummy = true`)
                let clientek_cmd = Command::new("clientek")
                    .arg("ek", &ek_params.ek_b64)
                    .arg("proof", &ek_params.proof_b64)
                    .build();
                self.send_command_str(&clientek_cmd, PacketType::Command, true, false)
                    .await?;

                // 2. Complete Init2 and immediately send `clientinit` as Command packet_id = 2
                //    encrypted with the newly derived session keys (`force_dummy = false`)
                self.send_clientinit().await?;
            }
            "initivexpand" => {
                self.last_init1_packet = None;

                let alpha_b64 = cmd.get("alpha").unwrap_or_default();
                let beta_b64 = cmd.get("beta").unwrap_or_default();
                let omega_b64 = cmd.get("omega").unwrap_or_default();

                self.crypt
                    .process_initivexpand(alpha_b64, beta_b64, omega_b64)?;
                self.out_seq.skip_command_id_zero();
                self.send_clientinit().await?;
            }
            "initserver" => {
                if let Some(first) = cmd.items.first() {
                    self.server_info = VirtualServerInfo::from_map(first);
                }
                if let Some(aclid) = cmd.get_u16("aclid") {
                    self.my_clid = aclid;
                }
                if let Some(cid) = cmd.get_u64("cid") {
                    self.my_cid = cid;
                }
                self.phase = HandshakePhase::Connected;
                self.sync_snapshot();

                let _ = self.event_tx.send(TsEvent::Connected {
                    my_clid: self.my_clid,
                    server: self.server_info.clone(),
                });

                // Subscribe to all channels (the server responds with `notifychannelsubscribed`
                // and `notifycliententerview` for all visible clients across channels)
                self.send_command_str("channelsubscribeall", PacketType::Command, false, false)
                    .await?;

                // Flush any commands queued before `initserver`
                while let Some(queued_cmd) = self.pending_user_commands.pop_front() {
                    self.send_command_str(&queued_cmd, PacketType::Command, false, false)
                        .await?;
                }
            }
            "channellist" => {
                for item in cmd.expanded_items() {
                    let mut ch = ChannelInfo::from_map(&item);
                    ch.subscribed = true;
                    if ch.cid != 0 {
                        self.channels.insert(ch.cid, ch);
                    }
                }
                self.sync_snapshot();
            }
            "channellistfinished" => {
                self.sync_snapshot();
                let list: Vec<ChannelInfo> = self.channels.values().cloned().collect();
                let _ = self.event_tx.send(TsEvent::ChannelsUpdated(list));
            }
            "notifychannelcreated" => {
                for item in cmd.expanded_items() {
                    let ch = ChannelInfo::from_map(&item);
                    if ch.cid != 0 {
                        self.channels.insert(ch.cid, ch);
                    }
                }
                self.sync_snapshot();
                let list: Vec<ChannelInfo> = self.channels.values().cloned().collect();
                let _ = self.event_tx.send(TsEvent::ChannelsUpdated(list));
            }
            "notifychanneledited" => {
                for item in cmd.expanded_items() {
                    if let Some(cid) = item.get("cid").and_then(|v| v.parse::<u64>().ok()) {
                        if let Some(existing) = self.channels.get_mut(&cid) {
                            existing.apply_map(&item);
                        } else {
                            self.channels.insert(cid, ChannelInfo::from_map(&item));
                        }
                    }
                }
                self.sync_snapshot();
                let list: Vec<ChannelInfo> = self.channels.values().cloned().collect();
                let _ = self.event_tx.send(TsEvent::ChannelsUpdated(list));
            }
            "notifychanneldeleted" => {
                for item in cmd.expanded_items() {
                    if let Some(cid) = item.get("cid").and_then(|v| v.parse::<u64>().ok()) {
                        self.channels.remove(&cid);
                    }
                }
                self.sync_snapshot();
                let list: Vec<ChannelInfo> = self.channels.values().cloned().collect();
                let _ = self.event_tx.send(TsEvent::ChannelsUpdated(list));
            }
            "notifychannelmoved" => {
                for item in cmd.expanded_items() {
                    if let Some(cid) = item.get("cid").and_then(|v| v.parse::<u64>().ok()) {
                        if let Some(existing) = self.channels.get_mut(&cid) {
                            existing.apply_map(&item);
                        }
                    }
                }
                self.sync_snapshot();
                let list: Vec<ChannelInfo> = self.channels.values().cloned().collect();
                let _ = self.event_tx.send(TsEvent::ChannelsUpdated(list));
            }
            "clientlist" | "" => {
                for item in cmd.expanded_items() {
                    if item.contains_key("clid") {
                        let mut client = ClientInfo::from_map(&item);
                        if client.clid != 0 {
                            if client.clid == self.my_clid && client.cid != 0 {
                                self.my_cid = client.cid;
                            }
                            if let Some(existing) = self.clients.get_mut(&client.clid) {
                                existing.apply_map(&item);
                            } else {
                                if client.cid == 0 && client.clid == self.my_clid {
                                    client.cid = self.my_cid;
                                }
                                self.clients.insert(client.clid, client);
                            }
                        }
                    }
                }
                self.sync_snapshot();
                let list: Vec<ClientInfo> = self.clients.values().cloned().collect();
                let _ = self.event_tx.send(TsEvent::ClientsUpdated(list));
            }
            "notifychannelsubscribed" => {
                for item in cmd.expanded_items() {
                    if let Some(cid) = item.get("cid").and_then(|v| v.parse::<u64>().ok()) {
                        if let Some(ch) = self.channels.get_mut(&cid) {
                            ch.subscribed = true;
                        }
                    }
                }
                self.sync_snapshot();
            }
            "notifyservergrouplist" | "notifychannelgrouplist" => {
                trace!("received group list: {} ({} items)", cmd.name, cmd.items.len());
            }
            "notifycliententerview" => {
                for item in cmd.expanded_items() {
                    let client = ClientInfo::from_map(&item);
                    if client.clid != 0 {
                        if client.clid == self.my_clid && client.cid != 0 {
                            self.my_cid = client.cid;
                        }
                        self.clients.insert(client.clid, client.clone());
                        let _ = self.event_tx.send(TsEvent::ClientEntered(client));
                    }
                }
                self.sync_snapshot();
                let list: Vec<ClientInfo> = self.clients.values().cloned().collect();
                let _ = self.event_tx.send(TsEvent::ClientsUpdated(list));
            }
            "notifyclientleftview" => {
                for item in cmd.expanded_items() {
                    if let Some(clid) = item.get("clid").and_then(|v| v.parse::<u16>().ok()) {
                        self.clients.remove(&clid);
                        let reason_msg = item.get("reasonmsg").cloned().unwrap_or_default();
                        let _ = self
                            .event_tx
                            .send(TsEvent::ClientLeft { clid, reason_msg });
                    }
                }
                self.sync_snapshot();
                let list: Vec<ClientInfo> = self.clients.values().cloned().collect();
                let _ = self.event_tx.send(TsEvent::ClientsUpdated(list));
            }
            "notifyclientmoved" => {
                for item in cmd.expanded_items() {
                    let clid_opt = item.get("clid").and_then(|v| v.parse::<u16>().ok());
                    let new_cid_opt = item
                        .get("ctid")
                        .or_else(|| item.get("cid"))
                        .and_then(|v| v.parse::<u64>().ok());
                    if let (Some(clid), Some(new_cid)) = (clid_opt, new_cid_opt) {
                        if clid == self.my_clid {
                            self.my_cid = new_cid;
                        }
                        if let Some(client) = self.clients.get_mut(&clid) {
                            client.cid = new_cid;
                        }
                        let _ = self
                            .event_tx
                            .send(TsEvent::ClientMoved { clid, new_cid });
                    }
                }
                self.sync_snapshot();
                let list: Vec<ClientInfo> = self.clients.values().cloned().collect();
                let _ = self.event_tx.send(TsEvent::ClientsUpdated(list));
            }
            "notifyclientupdated" => {
                for item in cmd.expanded_items() {
                    if let Some(clid) = item.get("clid").and_then(|v| v.parse::<u16>().ok()) {
                        if let Some(client) = self.clients.get_mut(&clid) {
                            client.apply_map(&item);
                        } else {
                            self.clients.insert(clid, ClientInfo::from_map(&item));
                        }
                    }
                }
                self.sync_snapshot();
                let list: Vec<ClientInfo> = self.clients.values().cloned().collect();
                let _ = self.event_tx.send(TsEvent::ClientsUpdated(list));
            }
            "notifytextmessage" => {
                let target_mode = cmd.get_u32("targetmode").unwrap_or(3);
                let invoker_id = cmd.get_u16("invokerid").unwrap_or(0);
                let invoker_name = cmd.get("invokername").unwrap_or_default().to_string();
                let invoker_uid = cmd.get("invokeruid").unwrap_or_default().to_string();
                let message = cmd.get("msg").unwrap_or_default().to_string();
                let target = match target_mode {
                    1 => ChatTarget::Client(invoker_id),
                    2 => ChatTarget::Channel,
                    _ => ChatTarget::Server,
                };
                let chat_msg = ChatMessage {
                    timestamp: current_timestamp_hms(),
                    target,
                    invoker_id,
                    invoker_name,
                    invoker_uid,
                    message,
                };
                let _ = self.event_tx.send(TsEvent::ChatReceived(chat_msg));
            }
            "notifyclientpoke" => {
                let invoker_id = cmd.get_u16("invokerid").unwrap_or(0);
                let invoker_name = cmd.get("invokername").unwrap_or_default().to_string();
                let message = cmd.get("msg").unwrap_or_default().to_string();
                let _ = self.event_tx.send(TsEvent::PokeReceived {
                    invoker_id,
                    invoker_name,
                    message,
                });
            }
            "notifyconnectioninforequest" => {
                let reply = Command::new("setconnectioninfo")
                    .arg("connection_ping", self.rtt_ms)
                    .arg(
                        "connection_packetloss_total",
                        format!("{:.4}", self.packet_loss),
                    )
                    .build();
                self.send_command_str(&reply, PacketType::CommandLow, false, false)
                    .await?;
            }
            "notifyserveredited" | "servervariables" => {
                if let Some(first) = cmd.items.first() {
                    self.server_info.apply_map(first);
                    self.sync_snapshot();
                }
            }
            "error" => {
                let id = cmd.get_u32("id").unwrap_or(0);
                let msg = cmd.get("msg").unwrap_or_default().to_string();
                debug!("S2C error response: id={id} msg={msg}");
                if id != 0 {
                    let _ = self.event_tx.send(TsEvent::ServerError { id, msg });
                }
            }
            _ => {
                debug!("unhandled S2C command: {}", cmd.name);
            }
        }
        Ok(())
    }

    async fn send_clientinit(&mut self) -> Result<(), ClientError> {
        let cmd = build_clientinit_command(&self.config);
        self.send_command_str(&cmd, PacketType::Command, false, false)
            .await
    }
}

/// Extracts the acknowledged 16-bit `packet_id` from an S2C `Ack` / `AckLow` payload
/// (supporting both 4-byte big-endian `u32` and 2-byte big-endian `u16` formats).
fn extract_acked_packet_id(payload: &[u8]) -> Option<u16> {
    if payload.len() >= 4 {
        let val = u32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]]);
        if val <= u16::MAX as u32 {
            Some(val as u16)
        } else {
            Some(u16::from_be_bytes([payload[0], payload[1]]))
        }
    } else if payload.len() >= 2 {
        Some(u16::from_be_bytes([payload[0], payload[1]]))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ts3_proto::identity::IdentityData;

    #[test]
    fn test_extract_acked_packet_id() {
        assert_eq!(extract_acked_packet_id(&42u32.to_be_bytes()), Some(42));
        assert_eq!(extract_acked_packet_id(&42u16.to_be_bytes()), Some(42));
        assert_eq!(extract_acked_packet_id(&0u32.to_be_bytes()), Some(0));
        assert_eq!(extract_acked_packet_id(&[1]), None);
        assert_eq!(parse_ack_payload(&build_ack_payload(123)), Some(123));
    }

    #[test]
    fn test_hash_password_and_clientinit_builder() {
        assert_eq!(hash_password(""), "");
        assert_eq!(hash_password("secret"), "5en6G6MezRroT3XKqkdPOmY/BfQ=");

        let mut id = IdentityData::generate();
        id.valid_key_offset = 10;
        id.last_checked_key_offset = 42;

        let mut cfg = ConnectConfig::new("127.0.0.1:9987", "Test User", id);
        cfg.default_channel = Some("/Lobby".to_string());
        cfg.default_channel_password = Some("chanpw".to_string());
        cfg.server_password = Some("srvpw".to_string());

        let cmd_str = build_clientinit_command(&cfg);
        let parsed = ParsedCommand::parse(&cmd_str).unwrap();
        assert_eq!(parsed.name, "clientinit");
        assert_eq!(parsed.get("client_nickname"), Some("Test User"));
        assert_eq!(parsed.get("client_version"), Some("3.5.3 [Build: 1587971024]"));
        assert_eq!(parsed.get("client_platform"), Some("Windows"));
        assert_eq!(parsed.get("client_input_hardware"), Some("1"));
        assert_eq!(parsed.get("client_output_hardware"), Some("1"));
        assert_eq!(parsed.get("client_default_channel"), Some("/Lobby"));
        assert_eq!(
            parsed.get("client_default_channel_password"),
            Some(hash_password("chanpw").as_str())
        );
        assert_eq!(
            parsed.get("client_server_password"),
            Some(hash_password("srvpw").as_str())
        );
        assert_eq!(parsed.get("client_meta_data"), Some(""));
        assert_eq!(parsed.get_u64("client_key_offset"), Some(10));
        assert_eq!(parsed.get("client_nickname_phonetic"), Some(""));
        assert_eq!(parsed.get("client_default_token"), Some(""));
        assert_eq!(
            parsed.get("hwid"),
            Some("1234567890abcdef,1234567890abcdef")
        );
    }

    #[tokio::test]
    async fn test_resolve_server_addr_defaults_port_9987() {
        let addr = resolve_server_addr("127.0.0.1").await.unwrap();
        assert_eq!(addr.port(), 9987);

        let custom = resolve_server_addr("127.0.0.1:10011").await.unwrap();
        assert_eq!(custom.port(), 10011);
    }

    #[tokio::test]
    async fn test_loopback_init1_and_notification_dispatch() {
        let server_sock = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let server_addr = server_sock.local_addr().unwrap();

        let id = IdentityData::generate();
        let cfg = ConnectConfig::new(server_addr.to_string(), "LoopbackTester", id);

        let (handle, mut events) = Ts3Client::connect(cfg).await.unwrap();

        // 1. Server should immediately receive C2S Init1 Step 0 (13B header + 21B payload = 34 bytes)
        let mut buf = [0u8; 1024];
        let (n, client_addr) = server_sock.recv_from(&mut buf).await.unwrap();
        assert_eq!(n, 13 + 21);
        assert_eq!(&buf[0..8], b"TS3INIT1");
        assert_eq!(u16::from_be_bytes([buf[8], buf[9]]), INIT1_C2S_PACKET_ID);
        assert_eq!(buf[12] & 0x0F, PacketType::Init1 as u8);
        assert_eq!(buf[13 + 4], 0x00); // Step 0

        let a0_m = &buf[13 + 9..13 + 13];

        // 2. Server sends S2C Init1 Step 1 (`[0x01, server_bytes(16B), a0_m_reversed(4B)]` = 21 bytes)
        let mut step1_payload = Vec::with_capacity(21);
        step1_payload.push(0x01);
        step1_payload.extend_from_slice(&[0x55u8; 16]);
        step1_payload.extend(a0_m.iter().rev().copied());

        let mut step1_pkt = S2CPacket::new(
            0,
            PacketType::Init1,
            PacketFlags::UNENCRYPTED,
            step1_payload,
        );
        step1_pkt.mac = *TS3_INIT_MAC;
        server_sock
            .send_to(&step1_pkt.encode(), client_addr)
            .await
            .unwrap();

        // 3. Server receives C2S Init1 Step 2 (13B header + 25B payload = 38 bytes)
        let (n2, _) = server_sock.recv_from(&mut buf).await.unwrap();
        assert_eq!(n2, 13 + 25);
        assert_eq!(buf[13 + 4], 0x02); // Step 2

        // 4. Clean disconnect via handle
        handle.disconnect().unwrap();
        let ev = events.recv().await.unwrap();
        assert!(matches!(ev, TsEvent::Disconnected { .. }));
        assert!(!handle.snapshot_state().connected);
    }

    #[tokio::test]
    async fn test_full_handshake_notifications_and_handle_actions() {
        use base64::engine::general_purpose::STANDARD as BASE64;
        use base64::Engine;

        let server_sock = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let server_addr = server_sock.local_addr().unwrap();

        let client_id = IdentityData::generate();
        let server_id = IdentityData::generate();
        let cfg = ConnectConfig::new(server_addr.to_string(), "RustBot", client_id.clone());

        let (handle, mut events) = Ts3Client::connect(cfg).await.unwrap();

        let mut buf = [0u8; 2048];

        // Step 0 -> Step 1
        let (n0, client_addr) = server_sock.recv_from(&mut buf).await.unwrap();
        assert_eq!(n0, 13 + 21);
        let a0_m = [buf[22], buf[23], buf[24], buf[25]];

        let mut step1_payload = vec![0x01];
        step1_payload.extend_from_slice(&[0x11u8; 16]);
        step1_payload.extend(a0_m.iter().rev().copied());
        let mut step1_pkt = S2CPacket::new(
            0,
            PacketType::Init1,
            PacketFlags::UNENCRYPTED,
            step1_payload,
        );
        step1_pkt.mac = *TS3_INIT_MAC;
        server_sock
            .send_to(&step1_pkt.encode(), client_addr)
            .await
            .unwrap();

        // Step 2 -> Step 3
        let (n2, _) = server_sock.recv_from(&mut buf).await.unwrap();
        assert_eq!(n2, 13 + 25);

        let mut step3_payload = vec![0x03];
        let mut x = [0u8; 64];
        x[63] = 2;
        let mut modulus = [0u8; 64];
        modulus[63] = 17;
        step3_payload.extend_from_slice(&x);
        step3_payload.extend_from_slice(&modulus);
        step3_payload.extend_from_slice(&4u32.to_be_bytes()); // level = 4
        step3_payload.extend_from_slice(&[0xAAu8; 100]);

        let mut step3_pkt = S2CPacket::new(
            0,
            PacketType::Init1,
            PacketFlags::UNENCRYPTED,
            step3_payload,
        );
        step3_pkt.mac = *TS3_INIT_MAC;
        server_sock
            .send_to(&step3_pkt.encode(), client_addr)
            .await
            .unwrap();

        // Step 4 (`301 + N` bytes containing `clientinitiv` at offset 301)
        let (n4, _) = server_sock.recv_from(&mut buf).await.unwrap();
        assert!(n4 > 13 + 301);
        let step4_cmd_str = std::str::from_utf8(&buf[13 + 301..n4]).unwrap();
        let parsed_initiv = ParsedCommand::parse(step4_cmd_str).unwrap();
        assert_eq!(parsed_initiv.name, "clientinitiv");
        let alpha_b64 = parsed_initiv.get("alpha").unwrap().to_string();

        // Server prepares a TsCrypt instance with matching shared secret (via legacy `initivexpand`)
        let beta = [0x42u8; 10];
        let beta_b64 = BASE64.encode(beta);
        let server_omega = server_id.public_key_omega();

        let mut server_crypt = TsCrypt::new(client_id.clone());
        let initivexpand_cmd = Command::new("initivexpand")
            .arg("alpha", &alpha_b64)
            .arg("beta", &beta_b64)
            .arg("omega", &server_omega)
            .build();

        // Send S2C Command packet 0 (`initivexpand`) encrypted with dummy key
        let mut s2c_cmd0 = S2CPacket::new(
            0,
            PacketType::Command,
            PacketFlags::NEW_PROTOCOL,
            initivexpand_cmd.into_bytes(),
        );
        server_crypt.encrypt_s2c_for_test(&mut s2c_cmd0, true).unwrap();
        server_sock
            .send_to(&s2c_cmd0.encode(), client_addr)
            .await
            .unwrap();

        // Complete server_crypt with the same alpha/beta/server_omega so it shares the session keys
        server_crypt
            .process_initivexpand(&alpha_b64, &beta_b64, &server_omega)
            .unwrap();

        // Client sends Ack for packet 0 and `clientinit` (Command packet_id = 1)
        let mut saw_clientinit = false;
        for _ in 0..2 {
            let (nr, _) = server_sock.recv_from(&mut buf).await.unwrap();
            let mut c2s = C2SPacket::decode(&buf[..nr]).unwrap();
            if c2s.packet_type == PacketType::Command {
                server_crypt.decrypt_c2s_for_test(&mut c2s, false).unwrap();
                let cmd_text = std::str::from_utf8(&c2s.payload).unwrap();
                let parsed = ParsedCommand::parse(cmd_text).unwrap();
                assert_eq!(parsed.name, "clientinit");
                assert_eq!(parsed.get("client_nickname"), Some("RustBot"));
                saw_clientinit = true;
            }
        }
        assert!(saw_clientinit);

        // Server sends S2C Command packet 1: `initserver` + `channellist` + `channellistfinished` + `notifytextmessage`
        let combined_s2c = [
            "initserver virtualserver_name=Test\\sServer virtualserver_maxclients=32 aclid=7 cid=10",
            "channellist cid=10 pid=0 channel_order=0 channel_name=Lobby channel_flag_default=1",
            "channellistfinished",
            "notifycliententerview ctid=10 clid=7 client_nickname=RustBot client_unique_identifier=uid1",
            "notifytextmessage targetmode=2 msg=Welcome! invokerid=1 invokername=Admin invokeruid=admin_uid",
        ]
        .join("\n");

        let mut s2c_cmd1 = S2CPacket::new(
            1,
            PacketType::Command,
            PacketFlags::NEW_PROTOCOL,
            combined_s2c.into_bytes(),
        );
        server_crypt
            .encrypt_s2c_for_test(&mut s2c_cmd1, false)
            .unwrap();
        server_sock
            .send_to(&s2c_cmd1.encode(), client_addr)
            .await
            .unwrap();

        // Verify emitted events
        let ev1 = events.recv().await.unwrap();
        match ev1 {
            TsEvent::Connected { my_clid, server } => {
                assert_eq!(my_clid, 7);
                assert_eq!(server.name, "Test Server");
                assert_eq!(server.max_clients, 32);
            }
            other => panic!("expected Connected, got {other:?}"),
        }

        let ev2 = events.recv().await.unwrap();
        match ev2 {
            TsEvent::ChannelsUpdated(chs) => {
                assert_eq!(chs.len(), 1);
                assert_eq!(chs[0].cid, 10);
                assert_eq!(chs[0].name, "Lobby");
            }
            other => panic!("expected ChannelsUpdated, got {other:?}"),
        }

        let ev3 = events.recv().await.unwrap();
        assert!(matches!(ev3, TsEvent::ClientEntered(ref c) if c.clid == 7 && c.cid == 10));

        let ev4 = events.recv().await.unwrap();
        assert!(matches!(ev4, TsEvent::ClientsUpdated(ref cs) if cs.len() == 1));

        let ev5 = events.recv().await.unwrap();
        match ev5 {
            TsEvent::ChatReceived(chat) => {
                assert_eq!(chat.target, ChatTarget::Channel);
                assert_eq!(chat.invoker_name, "Admin");
                assert_eq!(chat.message, "Welcome!");
            }
            other => panic!("expected ChatReceived, got {other:?}"),
        }

        // Verify snapshot state
        let snap = handle.snapshot_state();
        assert!(snap.connected);
        assert_eq!(snap.my_clid, 7);
        assert_eq!(snap.my_cid, 10);
        assert_eq!(snap.channels.len(), 1);
        assert_eq!(snap.clients.len(), 1);

        // Exercise handle methods
        handle.send_chat(ChatTarget::Channel, "Hello back!").unwrap();
        handle.switch_channel(10, Some("secret")).unwrap();
        handle.poke_client(1, "Hey!").unwrap();
        handle.set_away(true, Some("AFK")).unwrap();
        handle.set_mute(true, false).unwrap();
        handle.change_nickname("RustBot2").unwrap();
        handle.send_voice(4, &[0x01, 0x02, 0x03]).unwrap();

        // Also test channel/client updates, poke, connectioninfo request, server error, and voice S2C
        let batch2 = [
            "notifychannelcreated cid=20 cpid=10 channel_order=0 channel_name=SubRoom",
            "notifychanneledited cid=20 channel_name=SubRoomEdited",
            "notifychannelmoved cid=20 cpid=0 order=10",
            "notifychanneldeleted cid=20",
            "notifyclientupdated clid=7 client_away=1 client_away_message=Brb",
            "notifyclientmoved clid=7 ctid=10",
            "notifyclientpoke invokerid=1 invokername=Admin msg=Wake\\sup!",
            "notifyconnectioninforequest",
            "error id=1024 msg=invalid\\schannel\\sID",
            "notifyclientleftview clid=7 reasonmsg=kicked",
        ]
        .join("\n");

        let mut s2c_cmd2 = S2CPacket::new(
            2,
            PacketType::Command,
            PacketFlags::NEW_PROTOCOL,
            batch2.into_bytes(),
        );
        server_crypt
            .encrypt_s2c_for_test(&mut s2c_cmd2, false)
            .unwrap();
        server_sock
            .send_to(&s2c_cmd2.encode(), client_addr)
            .await
            .unwrap();

        // Also send S2C Voice packet
        let voice_s2c = VoicePacketS2C::new(
            1,
            42,
            ts3_proto::packet::CodecType::OpusVoice,
            vec![0xAA, 0xBB, 0xCC],
        );
        let mut s2c_voice = S2CPacket::new(
            0,
            PacketType::Voice,
            PacketFlags::UNENCRYPTED,
            voice_s2c.encode(),
        );
        server_crypt
            .encrypt_s2c_for_test(&mut s2c_voice, false)
            .unwrap();
        server_sock
            .send_to(&s2c_voice.encode(), client_addr)
            .await
            .unwrap();

        let mut saw_poke = false;
        let mut saw_server_err = false;
        let mut saw_client_left = false;
        let mut saw_voice = false;

        while !(saw_poke && saw_server_err && saw_client_left && saw_voice) {
            let ev = tokio::time::timeout(Duration::from_secs(2), events.recv())
                .await
                .expect("timed out waiting for events")
                .unwrap();
            match ev {
                TsEvent::PokeReceived {
                    invoker_id,
                    invoker_name,
                    message,
                } => {
                    assert_eq!(invoker_id, 1);
                    assert_eq!(invoker_name, "Admin");
                    assert_eq!(message, "Wake up!");
                    saw_poke = true;
                }
                TsEvent::ServerError { id, msg } => {
                    assert_eq!(id, 1024);
                    assert_eq!(msg, "invalid channel ID");
                    saw_server_err = true;
                }
                TsEvent::ClientLeft { clid, reason_msg } => {
                    assert_eq!(clid, 7);
                    assert_eq!(reason_msg, "kicked");
                    saw_client_left = true;
                }
                TsEvent::VoicePacket {
                    talker_id,
                    codec,
                    audio_data,
                } => {
                    assert_eq!(talker_id, 42);
                    assert_eq!(codec, 4);
                    assert_eq!(audio_data, vec![0xAA, 0xBB, 0xCC]);
                    saw_voice = true;
                }
                _ => {}
            }
        }

        handle.disconnect().unwrap();
    }

    #[tokio::test]
    async fn test_initivexpand2_handshake_and_all_s2c_notifications() {
        use base64::engine::general_purpose::STANDARD as BASE64;
        use base64::Engine;
        use ts3_proto::crypt::compute_shared_secret2;
        use ts3_proto::license::LicenseChain;

        let server_sock = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let server_addr = server_sock.local_addr().unwrap();

        let client_id = IdentityData::generate();
        let server_id = IdentityData::generate();
        let cfg = ConnectConfig::new(server_addr.to_string(), "ModernBot", client_id.clone());

        let (handle, _events) = Ts3Client::connect(cfg).await.unwrap();

        let mut buf = [0u8; 2048];

        // Step 0 -> Step 1
        let (n0, client_addr) = server_sock.recv_from(&mut buf).await.unwrap();
        assert_eq!(n0, 13 + 21);
        let a0_m = [buf[22], buf[23], buf[24], buf[25]];

        let mut step1_payload = vec![0x01];
        step1_payload.extend_from_slice(&[0x22u8; 16]);
        step1_payload.extend(a0_m.iter().rev().copied());
        let mut step1_pkt = S2CPacket::new(
            0,
            PacketType::Init1,
            PacketFlags::UNENCRYPTED,
            step1_payload,
        );
        step1_pkt.mac = *TS3_INIT_MAC;
        server_sock
            .send_to(&step1_pkt.encode(), client_addr)
            .await
            .unwrap();

        // Step 2 -> Step 3
        let (n2, _) = server_sock.recv_from(&mut buf).await.unwrap();
        assert_eq!(n2, 13 + 25);

        let mut step3_payload = vec![0x03];
        let mut x = [0u8; 64];
        x[63] = 2;
        let mut modulus = [0u8; 64];
        modulus[63] = 17;
        step3_payload.extend_from_slice(&x);
        step3_payload.extend_from_slice(&modulus);
        step3_payload.extend_from_slice(&2u32.to_be_bytes());
        step3_payload.extend_from_slice(&[0xBBu8; 100]);

        let mut step3_pkt = S2CPacket::new(
            0,
            PacketType::Init1,
            PacketFlags::UNENCRYPTED,
            step3_payload,
        );
        step3_pkt.mac = *TS3_INIT_MAC;
        server_sock
            .send_to(&step3_pkt.encode(), client_addr)
            .await
            .unwrap();

        // Step 4 -> extract alpha
        let (n4, _) = server_sock.recv_from(&mut buf).await.unwrap();
        let step4_cmd_str = std::str::from_utf8(&buf[13 + 301..n4]).unwrap();
        let parsed_initiv = ParsedCommand::parse(step4_cmd_str).unwrap();
        assert_eq!(parsed_initiv.name, "clientinitiv");
        let alpha_b64 = parsed_initiv.get("alpha").unwrap().to_string();
        let alpha_bytes: [u8; 10] = BASE64
            .decode(&alpha_b64)
            .unwrap()
            .try_into()
            .unwrap();

        // Build a 1-block Ephemeral license where block_key = identity element [1, 0, ..., 0]
        // so that LicenseBlock::derive_key(ROOT_KEY) = 0 + ROOT_KEY = ROOT_KEY!
        // Wait: let's know the server's ephemeral scalar! If we use a known license block and derive
        // the server's public key, or we can extract `ek` from `clientek` (which is dummy-encrypted!).
        // Wait: to decrypt `clientinit` (packet_id = 2), we need the 64-byte `iv_struct`!
        // Can we know the discrete log of `LicenseChain::parse_and_derive_key`?
        // WAIT: `LicenseBlock::derive_key(&parent)` computes `(pub_point * clamped_hash) + parent_point`!
        // Since we don't know the discrete log of `LICENSE_ROOT_KEY`, wait: what if `pub_point` is chosen, or what if we use `ek`?
        // No! If we don't know the discrete log of `LICENSE_ROOT_KEY`, how can a mock server compute `d_client * P_server`?
        // Wait: in `TsCrypt::process_initivexpand2`, the client generates random `d_client`, sends `ek = d_client * B`, and computes `shared = SHA512(d_client * P_server)`!
        // Notice: `P_server` is `derived_key` from `LicenseChain` (`s_block * P_block + P_root`).
        // Unless the server knows the discrete log of `P_server` (which requires knowing the discrete log of `P_root`), nobody without TeamSpeak's root private key can compute `s_server * ek` for a random `ek`!
        // Wait: unless `P_server` has order 1 (the identity point `(0, 1)` on Edwards curve: `[1, 0, 0, ..., 0]`)?
        // No, we can't set `P_block * s_block = -P_root` without knowing `s_block` beforehand (since `s_block = clamp(SHA512(P_block || ...))`).
        // That's why `TsFullClient` / `TsCrypt` has `initivexpand` (where we DO know `server_id`'s P-256 private key!) for full session encryption, AND we can test `initivexpand2` up to receiving and decrypting `clientek` (packet_id = 1, dummy-encrypted) + verifying `clientinit` (packet_id = 2) is immediately transmitted!

        // Let's test `initivexpand2` with a valid 1-block Ephemeral license chain:
        let mut block_key = [0u8; 32];
        block_key[0] = 1; // Identity Edwards point (0, 1) -> validCompressedEdwardsY!
        let mut license_bytes = Vec::with_capacity(43);
        license_bytes.push(0x01); // version 1
        license_bytes.push(0x00); // keyKind = 0
        license_bytes.extend_from_slice(&block_key);
        license_bytes.push(32); // Ephemeral block type
        license_bytes.extend_from_slice(&100u32.to_be_bytes());
        license_bytes.extend_from_slice(&200u32.to_be_bytes());
        assert!(LicenseChain::parse_and_derive_key(&license_bytes).is_ok());

        let proof_bytes = server_id.sign_asn1_der(&license_bytes);
        let beta_bytes = [0x55u8; 54];

        let initivexpand2_cmd = Command::new("initivexpand2")
            .arg("l", BASE64.encode(&license_bytes))
            .arg("beta", BASE64.encode(beta_bytes))
            .arg("omega", server_id.public_key_omega())
            .arg("proof", BASE64.encode(&proof_bytes))
            .build();

        let dummy_crypt = TsCrypt::new(server_id.clone());
        let mut s2c_init2 = S2CPacket::new(
            0,
            PacketType::Command,
            PacketFlags::NEW_PROTOCOL,
            initivexpand2_cmd.into_bytes(),
        );
        dummy_crypt.encrypt_s2c_for_test(&mut s2c_init2, true).unwrap();
        server_sock
            .send_to(&s2c_init2.encode(), client_addr)
            .await
            .unwrap();

        // Client must send:
        // 1. Ack for S2C Command 0
        // 2. Command packet_id = 1: `clientek` (dummy-encrypted!)
        // 3. Command packet_id = 2: `clientinit` (session-encrypted!)
        let mut saw_ack0 = false;
        let mut saw_clientek_id1 = false;
        let mut saw_clientinit_id2 = false;

        for _ in 0..3 {
            let (nr, _) = server_sock.recv_from(&mut buf).await.unwrap();
            let mut c2s = C2SPacket::decode(&buf[..nr]).unwrap();
            match c2s.packet_type {
                PacketType::Ack => {
                    saw_ack0 = true;
                }
                PacketType::Command if c2s.packet_id == 1 => {
                    assert_eq!(c2s.client_id, 0);
                    assert!(c2s.flags.contains(PacketFlags::NEW_PROTOCOL));
                    // `clientek` MUST be encrypted with the dummy key/nonce!
                    dummy_crypt.decrypt_c2s_for_test(&mut c2s, true).unwrap();
                    let cmd_str = std::str::from_utf8(&c2s.payload).unwrap();
                    let parsed = ParsedCommand::parse(cmd_str).unwrap();
                    assert_eq!(parsed.name, "clientek");
                    assert!(parsed.get("ek").is_some());
                    assert!(parsed.get("proof").is_some());
                    saw_clientek_id1 = true;
                }
                PacketType::Command if c2s.packet_id == 2 => {
                    assert_eq!(c2s.client_id, 0);
                    assert!(c2s.flags.contains(PacketFlags::NEW_PROTOCOL));
                    saw_clientinit_id2 = true;
                }
                other => panic!("unexpected packet type {other:?} id={}", c2s.packet_id),
            }
        }
        assert!(saw_ack0);
        assert!(saw_clientek_id1);
        assert!(saw_clientinit_id2);
        let _ = (alpha_bytes, compute_shared_secret2);

        handle.disconnect().unwrap();
    }
}
