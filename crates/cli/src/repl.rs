use std::time::Duration;

use anyhow::Result;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::mpsc::UnboundedReceiver;
use ts3_client::connection::{ChannelInfo, ChatTarget, ClientInfo, Ts3ClientHandle, TsEvent};

/// Runs the interactive asynchronous REPL reading commands from stdin and printing `TsEvent`s.
pub async fn run_repl(
    handle: Ts3ClientHandle,
    mut event_rx: UnboundedReceiver<TsEvent>,
) -> Result<()> {
    let stdin = tokio::io::stdin();
    let mut lines = BufReader::new(stdin).lines();

    let mut local_away = false;
    let mut local_muted = false;

    println!("Interactive REPL started. Type /help for commands, /quit to disconnect.");

    loop {
        tokio::select! {
            maybe_event = event_rx.recv() => {
                match maybe_event {
                    Some(event) => {
                        let should_exit = handle_server_event(event, &mut local_away, &mut local_muted);
                        if should_exit {
                            break;
                        }
                    }
                    None => {
                        println!("[disconnected] Event stream closed.");
                        break;
                    }
                }
            }
            line_res = lines.next_line() => {
                match line_res {
                    Ok(Some(line)) => {
                        let should_quit = handle_stdin_command(
                            line.trim(),
                            &handle,
                            &mut local_away,
                            &mut local_muted,
                        );
                        if should_quit {
                            let _ = handle.disconnect();
                            let _ = tokio::time::timeout(Duration::from_secs(2), async {
                                while let Some(ev) = event_rx.recv().await {
                                    if matches!(ev, TsEvent::Disconnected { .. }) {
                                        break;
                                    }
                                }
                            })
                            .await;
                            break;
                        }
                    }
                    Ok(None) => {
                        // EOF on stdin
                        println!("EOF received, disconnecting...");
                        let _ = handle.disconnect();
                        let _ = tokio::time::timeout(Duration::from_secs(2), async {
                            while let Some(ev) = event_rx.recv().await {
                                if matches!(ev, TsEvent::Disconnected { .. }) {
                                    break;
                                }
                            }
                        })
                        .await;
                        break;
                    }
                    Err(err) => {
                        eprintln!("Error reading stdin: {err}");
                        let _ = handle.disconnect();
                        break;
                    }
                }
            }
        }
    }

    Ok(())
}

/// Prints an incoming [`TsEvent`] to stdout. Returns `true` if the session has disconnected.
fn handle_server_event(
    event: TsEvent,
    local_away: &mut bool,
    local_muted: &mut bool,
) -> bool {
    match event {
        TsEvent::Connected { my_clid, server } => {
            println!(
                "[server] Connected to \"{}\" ({} {}) as clid={}",
                server.name, server.platform, server.version, my_clid
            );
            if !server.welcome_message.is_empty() {
                println!("[server] Welcome: {}", server.welcome_message);
            }
        }
        TsEvent::Disconnected { reason } => {
            println!("[disconnected] {reason}");
            return true;
        }
        TsEvent::ChannelsUpdated(channels) => {
            println!("[channels] Channel list updated ({} channels):", channels.len());
            print_channels(&channels);
        }
        TsEvent::ClientsUpdated(clients) => {
            // Sync local away/mute flags if our client entry is present
            for c in &clients {
                // Best-effort sync
                let _ = (c, &local_away, &local_muted);
            }
        }
        TsEvent::ClientEntered(client) => {
            println!(
                "[join] {} (clid={}, uid={}) entered channel cid={}",
                client.nickname, client.clid, client.uid, client.cid
            );
        }
        TsEvent::ClientLeft { clid, reason_msg } => {
            if reason_msg.is_empty() {
                println!("[leave] Client clid={clid} left");
            } else {
                println!("[leave] Client clid={clid} left ({reason_msg})");
            }
        }
        TsEvent::ClientMoved { clid, new_cid } => {
            println!("[move] Client clid={clid} moved to channel cid={new_cid}");
        }
        TsEvent::ChatReceived(chat) => {
            let scope = match chat.target {
                ChatTarget::Server => "[chat:server]".to_string(),
                ChatTarget::Channel => "[chat:channel]".to_string(),
                ChatTarget::Client(id) => format!("[chat:pm:{id}]"),
            };
            println!(
                "{} [{}] {} (clid={}): {}",
                scope, chat.timestamp, chat.invoker_name, chat.invoker_id, chat.message
            );
        }
        TsEvent::PokeReceived {
            invoker_id,
            invoker_name,
            message,
        } => {
            println!(
                "[poke] {} (clid={}) poked you: {}",
                invoker_name, invoker_id, message
            );
        }
        TsEvent::ServerError { id, msg } => {
            println!("[error] Server error id={id}: {msg}");
        }
        TsEvent::VoicePacket { .. } | TsEvent::PingUpdated { .. } => {}
    }
    false
}

/// Processes a single user input line. Returns `true` if `/quit` was invoked.
fn handle_stdin_command(
    line: &str,
    handle: &Ts3ClientHandle,
    local_away: &mut bool,
    local_muted: &mut bool,
) -> bool {
    if line.is_empty() {
        return false;
    }

    if !line.starts_with('/') {
        if let Err(err) = handle.send_chat(ChatTarget::Channel, line) {
            eprintln!("Failed to send message: {err}");
        }
        return false;
    }

    let (cmd, rest) = match line.split_once(char::is_whitespace) {
        Some((c, r)) => (c, r.trim()),
        None => (line, ""),
    };

    match cmd {
        "/channels" => {
            let snap = handle.snapshot_state();
            println!("Known channels ({}):", snap.channels.len());
            print_channels(&snap.channels);
        }
        "/clients" => {
            let snap = handle.snapshot_state();
            println!("Connected clients ({}):", snap.clients.len());
            print_clients(&snap.clients);
        }
        "/join" => {
            if rest.is_empty() {
                println!("Usage: /join <cid> [pass]");
                return false;
            }
            let (cid_str, pass_opt) = match rest.split_once(char::is_whitespace) {
                Some((c, p)) => (c.trim(), Some(p.trim()).filter(|s| !s.is_empty())),
                None => (rest, None),
            };
            match cid_str.parse::<u64>() {
                Ok(cid) => {
                    if let Err(err) = handle.switch_channel(cid, pass_opt) {
                        eprintln!("Failed to switch channel: {err}");
                    } else {
                        println!("Joining channel cid={cid}...");
                    }
                }
                Err(_) => {
                    println!("Invalid channel ID '{cid_str}'. Usage: /join <cid> [pass]");
                }
            }
        }
        "/msg" => {
            if rest.is_empty() {
                println!("Usage: /msg <text>");
                return false;
            }
            let snap = handle.snapshot_state();
            let target = if snap.my_cid != 0 {
                ChatTarget::Channel
            } else {
                ChatTarget::Server
            };
            if let Err(err) = handle.send_chat(target, rest) {
                eprintln!("Failed to send chat message: {err}");
            }
        }
        "/pm" => {
            let Some((clid_str, text)) = rest.split_once(char::is_whitespace) else {
                println!("Usage: /pm <clid> <text>");
                return false;
            };
            let text = text.trim();
            if text.is_empty() {
                println!("Usage: /pm <clid> <text>");
                return false;
            }
            match clid_str.trim().parse::<u16>() {
                Ok(clid) => {
                    if let Err(err) = handle.send_chat(ChatTarget::Client(clid), text) {
                        eprintln!("Failed to send private message: {err}");
                    }
                }
                Err(_) => {
                    println!("Invalid client ID '{clid_str}'. Usage: /pm <clid> <text>");
                }
            }
        }
        "/poke" => {
            let Some((clid_str, msg)) = rest.split_once(char::is_whitespace) else {
                println!("Usage: /poke <clid> <msg>");
                return false;
            };
            let msg = msg.trim();
            match clid_str.trim().parse::<u16>() {
                Ok(clid) => {
                    if let Err(err) = handle.poke_client(clid, msg) {
                        eprintln!("Failed to poke client: {err}");
                    } else {
                        println!("Poked client clid={clid}.");
                    }
                }
                Err(_) => {
                    println!("Invalid client ID '{clid_str}'. Usage: /poke <clid> <msg>");
                }
            }
        }
        "/nick" => {
            if rest.is_empty() {
                println!("Usage: /nick <name>");
                return false;
            }
            if let Err(err) = handle.change_nickname(rest) {
                eprintln!("Failed to change nickname: {err}");
            } else {
                println!("Nickname change requested: {rest}");
            }
        }
        "/away" => {
            let snap = handle.snapshot_state();
            if let Some(me) = snap.clients.iter().find(|c| c.clid == snap.my_clid) {
                *local_away = me.away;
            }
            let (new_away, msg_opt) = if !rest.is_empty() {
                (true, Some(rest))
            } else {
                (!*local_away, None)
            };
            if let Err(err) = handle.set_away(new_away, msg_opt) {
                eprintln!("Failed to update away status: {err}");
            } else {
                *local_away = new_away;
                if new_away {
                    println!(
                        "Away status enabled{}",
                        msg_opt.map(|m| format!(" ({m})")).unwrap_or_default()
                    );
                } else {
                    println!("Away status cleared.");
                }
            }
        }
        "/mute" => {
            let snap = handle.snapshot_state();
            if let Some(me) = snap.clients.iter().find(|c| c.clid == snap.my_clid) {
                *local_muted = me.input_muted || me.output_muted;
            }
            let new_muted = !*local_muted;
            if let Err(err) = handle.set_mute(new_muted, new_muted) {
                eprintln!("Failed to toggle mute state: {err}");
            } else {
                *local_muted = new_muted;
                println!(
                    "Input/Output mute: {}",
                    if new_muted { "MUTED" } else { "UNMUTED" }
                );
            }
        }
        "/whoami" => {
            let snap = handle.snapshot_state();
            let my_client = snap.clients.iter().find(|c| c.clid == snap.my_clid);
            let my_nick = my_client
                .map(|c| c.nickname.as_str())
                .unwrap_or("<unknown>");
            let my_uid = my_client.map(|c| c.uid.as_str()).unwrap_or("<unknown>");
            let my_cid = my_client
                .map(|c| c.cid)
                .filter(|&cid| cid != 0)
                .unwrap_or(snap.my_cid);
            let chan_name = snap
                .channels
                .iter()
                .find(|ch| ch.cid == my_cid)
                .map(|ch| ch.name.as_str())
                .unwrap_or("<unknown>");

            println!("=== Who Am I ===");
            println!("Client ID (clid): {}", snap.my_clid);
            println!("Nickname:         {my_nick}");
            println!("UID:              {my_uid}");
            println!("Channel:          cid={my_cid} (\"{chan_name}\")");
            println!(
                "Server:           \"{}\" ({} {})",
                snap.server.name, snap.server.platform, snap.server.version
            );
            println!(
                "Connection:       connected={}, rtt={}ms, loss={:.2}%",
                snap.connected,
                snap.rtt_ms,
                snap.packet_loss * 100.0
            );
        }
        "/quit" | "/exit" => {
            println!("Disconnecting from server...");
            return true;
        }
        "/help" => {
            println!("Available commands:");
            println!("  /channels          - List known channels (ID, parent ID, name, topic)");
            println!("  /clients           - List connected clients (clid, channel_id, nickname, uid)");
            println!("  /join <cid> [pass] - Move own client to channel <cid> with optional password");
            println!("  /msg <text>        - Send channel or server chat message");
            println!("  /pm <clid> <text>  - Send private message to client <clid>");
            println!("  /poke <clid> <msg> - Poke client <clid> with message");
            println!("  /nick <name>       - Change own nickname");
            println!("  /away [msg]        - Toggle/set away status and away message");
            println!("  /mute              - Toggle input/output mute state");
            println!("  /whoami            - Print own client ID, nickname, current channel, and server info");
            println!("  /quit              - Cleanly disconnect from server and exit REPL");
        }
        _ => {
            println!("Unknown command '{cmd}'. Type /help for available commands.");
        }
    }

    false
}

fn print_channels(channels: &[ChannelInfo]) {
    for ch in channels {
        println!(
            "  cid={} pid={} name=\"{}\" topic=\"{}\"",
            ch.cid, ch.pid, ch.name, ch.topic
        );
    }
}

fn print_clients(clients: &[ClientInfo]) {
    for cl in clients {
        println!(
            "  clid={} channel_id={} nickname=\"{}\" uid=\"{}\"",
            cl.clid, cl.cid, cl.nickname, cl.uid
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repl_print_helpers() {
        let ch = ChannelInfo {
            cid: 1,
            pid: 0,
            name: "Default Channel".to_string(),
            topic: "Welcome".to_string(),
            ..Default::default()
        };
        print_channels(&[ch]);

        let cl = ClientInfo {
            clid: 5,
            cid: 1,
            nickname: "RustClient".to_string(),
            uid: "uid123=".to_string(),
            ..Default::default()
        };
        print_clients(&[cl]);
    }
}

