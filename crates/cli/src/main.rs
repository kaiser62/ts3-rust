use std::collections::BTreeMap;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use ts3_client::connection::{
    ChannelInfo, ChatTarget, ClientInfo, ConnectConfig, Ts3Client, TsEvent, VirtualServerInfo,
};
use ts3_proto::identity::IdentityData;

mod repl;

/// TeamSpeak 3 Native Rust CLI Client
#[derive(Debug, Parser)]
#[command(name = "ts3-cli", version, about = "TeamSpeak 3 minimal native Rust CLI client")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Connect to a TeamSpeak 3 server and enter the interactive REPL
    Connect {
        /// Server address (host or host:port)
        #[arg(long, default_value = "192.168.1.140:9987")]
        server: String,

        /// Optional virtual server password
        #[arg(long)]
        password: Option<String>,

        /// Client nickname
        #[arg(long, default_value = "RustClient")]
        nick: String,

        /// Optional default channel name or path to join on connect
        #[arg(long)]
        channel: Option<String>,
    },

    /// Connect, inspect server/channels/clients, optionally send a chat message, and disconnect cleanly
    SmokeTest {
        /// Server address (host or host:port)
        #[arg(long, default_value = "192.168.1.140:9987")]
        server: String,

        /// Optional virtual server password
        #[arg(long)]
        password: Option<String>,

        /// Client nickname
        #[arg(long, default_value = "RustClient")]
        nick: String,

        /// Optional chat message to send before disconnecting
        #[arg(long)]
        say: Option<String>,
    },

    /// Generate a new TeamSpeak 3 P-256 identity and improve its hashcash security level
    Identity {
        /// Target hashcash security level
        #[arg(long, default_value_t = 8)]
        level: u32,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .try_init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Connect {
            server,
            password,
            nick,
            channel,
        } => run_connect(server, password, nick, channel).await,
        Commands::SmokeTest {
            server,
            password,
            nick,
            say,
        } => run_smoke_test(server, password, nick, say).await,
        Commands::Identity { level } => run_identity(level),
    }
}

fn run_identity(level: u32) -> Result<()> {
    let mut identity = IdentityData::generate();
    identity.improve_security_level(level);

    let uid = identity.uid();
    let omega = identity.public_key_omega();
    let actual_level = identity.security_level();
    let exported = identity.export_ts_identity();

    println!("UID:            {uid}");
    println!("Omega:          {omega}");
    println!("Security Level: {actual_level} (offset={})", identity.valid_key_offset);
    println!("Exported:       {exported}");

    Ok(())
}

async fn run_connect(
    server: String,
    password: Option<String>,
    nick: String,
    channel: Option<String>,
) -> Result<()> {
    println!("Generating identity (security level 8)...");
    let identity = IdentityData::generate_with_level(8);
    println!("Identity UID: {}", identity.uid());

    let mut config = ConnectConfig::new(&server, &nick, identity);
    config.server_password = password;
    config.default_channel = channel;

    println!("Connecting to {server} as '{nick}'...");
    let (handle, event_rx) = Ts3Client::connect(config)
        .await
        .with_context(|| format!("failed to initiate connection to {server}"))?;

    repl::run_repl(handle, event_rx).await
}

async fn run_smoke_test(
    server: String,
    password: Option<String>,
    nick: String,
    say: Option<String>,
) -> Result<()> {
    println!("[smoke-test] Generating identity (security level 8)...");
    let identity = IdentityData::generate_with_level(8);
    println!("[smoke-test] Identity UID: {}", identity.uid());

    let mut config = ConnectConfig::new(&server, &nick, identity);
    config.server_password = password;

    println!("[smoke-test] Connecting to {server} as '{nick}'...");
    let (handle, mut event_rx) = Ts3Client::connect(config)
        .await
        .with_context(|| format!("failed to initiate connection to {server}"))?;

    let mut got_initserver = false;
    let mut got_channels = false;
    let mut got_clients = false;

    let mut my_clid: u16 = 0;
    let mut server_info = VirtualServerInfo::default();
    let mut channels: Vec<ChannelInfo> = Vec::new();
    let mut clients: Vec<ClientInfo> = Vec::new();

    let wait_res = tokio::time::timeout(Duration::from_secs(15), async {
        while !(got_initserver && got_channels && got_clients) {
            match event_rx.recv().await {
                Some(TsEvent::Connected {
                    my_clid: clid,
                    server: srv,
                }) => {
                    my_clid = clid;
                    server_info = srv;
                    got_initserver = true;
                }
                Some(TsEvent::ChannelsUpdated(ch_list)) => {
                    channels = ch_list;
                    got_channels = true;
                }
                Some(TsEvent::ClientsUpdated(cl_list)) => {
                    clients = cl_list;
                    if !clients.is_empty() {
                        got_clients = true;
                    }
                }
                Some(TsEvent::ClientEntered(cl)) => {
                    if let Some(existing) = clients.iter_mut().find(|c| c.clid == cl.clid) {
                        *existing = cl;
                    } else {
                        clients.push(cl);
                    }
                    got_clients = true;
                }
                Some(TsEvent::ServerError { id, msg }) => {
                    eprintln!("[smoke-test] Server error {id}: {msg}");
                }
                Some(TsEvent::Disconnected { reason }) => {
                    bail!("disconnected during smoke test initialization: {reason}");
                }
                Some(_) => {}
                None => {
                    bail!("event channel closed before initialization finished");
                }
            }
        }
        Ok::<(), anyhow::Error>(())
    })
    .await;

    if let Err(_) = wait_res {
        // Fallback to snapshot if timeout elapsed after partial state was populated
        let snap = handle.snapshot_state();
        if !got_initserver && snap.connected {
            got_initserver = true;
            my_clid = snap.my_clid;
            server_info = snap.server;
        }
        if channels.is_empty() && !snap.channels.is_empty() {
            channels = snap.channels;
            got_channels = true;
        }
        if clients.is_empty() && !snap.clients.is_empty() {
            clients = snap.clients;
            got_clients = true;
        }
        if !got_initserver {
            let _ = handle.disconnect();
            bail!("timed out (15s) waiting for initserver, channellistfinished, and initial client list");
        }
        eprintln!(
            "[smoke-test] Warning: 15s timeout reached (initserver={got_initserver}, channels={got_channels}, clients={got_clients})"
        );
    } else if let Ok(Err(e)) = wait_res {
        let _ = handle.disconnect();
        return Err(e);
    }

    // Refresh from snapshot to ensure latest state
    let snap = handle.snapshot_state();
    if !snap.server.name.is_empty() {
        server_info = snap.server;
    }
    if !snap.channels.is_empty() {
        channels = snap.channels;
    }
    if !snap.clients.is_empty() {
        clients = snap.clients;
    }

    println!("=== Virtual Server ===");
    println!("Name:     {}", server_info.name);
    println!("Platform: {}", server_info.platform);
    println!("Version:  {}", server_info.version);
    println!("My CLID:  {my_clid}");

    println!("\n=== Channel Tree ({}) ===", channels.len());
    print_channel_tree(&channels, &clients);

    println!("\n=== Connected Clients ({}) ===", clients.len());
    for c in &clients {
        println!(
            "  clid={} cid={} nick=\"{}\" uid=\"{}\"",
            c.clid, c.cid, c.nickname, c.uid
        );
    }

    if let Some(msg) = say {
        println!("\n[smoke-test] Sending message: {msg}");
        handle
            .send_chat(ChatTarget::Channel, &msg)
            .context("failed to send smoke-test chat message")?;
        // Brief pause so the outgoing command packet is flushed over UDP before disconnect
        tokio::time::sleep(Duration::from_millis(250)).await;
    }

    println!("[smoke-test] Disconnecting cleanly...");
    let _ = handle.disconnect();

    let _ = tokio::time::timeout(Duration::from_secs(2), async {
        while let Some(ev) = event_rx.recv().await {
            if matches!(ev, TsEvent::Disconnected { .. }) {
                break;
            }
        }
    })
    .await;

    println!("[smoke-test] Done.");
    Ok(())
}

fn print_channel_tree(channels: &[ChannelInfo], clients: &[ClientInfo]) {
    let mut by_parent: BTreeMap<u64, Vec<&ChannelInfo>> = BTreeMap::new();
    for ch in channels {
        by_parent.entry(ch.pid).or_default().push(ch);
    }

    fn walk(
        pid: u64,
        depth: usize,
        by_parent: &BTreeMap<u64, Vec<&ChannelInfo>>,
        clients: &[ClientInfo],
    ) {
        if let Some(children) = by_parent.get(&pid) {
            for ch in children {
                let indent = "  ".repeat(depth + 1);
                let topic_suffix = if ch.topic.is_empty() {
                    String::new()
                } else {
                    format!(" - topic: \"{}\"", ch.topic)
                };
                println!(
                    "{}[cid={} pid={}] {}{}",
                    indent, ch.cid, ch.pid, ch.name, topic_suffix
                );
                for cl in clients.iter().filter(|c| c.cid == ch.cid) {
                    println!("{}  * [clid={}] {} ({})", indent, cl.clid, cl.nickname, cl.uid);
                }
                walk(ch.cid, depth + 1, by_parent, clients);
            }
        }
    }

    walk(0, 0, &by_parent, clients);
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_cli_parse_connect_defaults() {
        let cli = Cli::try_parse_from(["ts3-cli", "connect"]).unwrap();
        match cli.command {
            Commands::Connect {
                server,
                password,
                nick,
                channel,
            } => {
                assert_eq!(server, "192.168.1.140:9987");
                assert_eq!(password, None);
                assert_eq!(nick, "RustClient");
                assert_eq!(channel, None);
            }
            _ => panic!("expected Connect command"),
        }
    }

    #[test]
    fn test_cli_parse_smoke_test_options() {
        let cli = Cli::try_parse_from([
            "ts3-cli",
            "smoke-test",
            "--server",
            "127.0.0.1:9987",
            "--password",
            "secret",
            "--nick",
            "Tester",
            "--say",
            "Hello world",
        ])
        .unwrap();
        match cli.command {
            Commands::SmokeTest {
                server,
                password,
                nick,
                say,
            } => {
                assert_eq!(server, "127.0.0.1:9987");
                assert_eq!(password.as_deref(), Some("secret"));
                assert_eq!(nick, "Tester");
                assert_eq!(say.as_deref(), Some("Hello world"));
            }
            _ => panic!("expected SmokeTest command"),
        }
    }

    #[test]
    fn test_cli_parse_identity_defaults_and_custom() {
        let cli_def = Cli::try_parse_from(["ts3-cli", "identity"]).unwrap();
        match cli_def.command {
            Commands::Identity { level } => assert_eq!(level, 8),
            _ => panic!("expected Identity command"),
        }

        let cli_custom = Cli::try_parse_from(["ts3-cli", "identity", "--level", "10"]).unwrap();
        match cli_custom.command {
            Commands::Identity { level } => assert_eq!(level, 10),
            _ => panic!("expected Identity command"),
        }
    }

    #[test]
    fn test_run_identity_succeeds() {
        run_identity(8).unwrap();
    }
}

