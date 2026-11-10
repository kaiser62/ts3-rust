// Owned by Pool Agent 5 (arthoroy28@gmail.com)
// 1:1 `ServerTreeView` rendering virtualserver root, hierarchical channels, spacer channels
// (`[spacer]`, `[cspacer]`, `[rspacer]`, `[*spacer]`, `---`), channel status icons, client icons,
// right-aligned badges, double-click to switch channel, and right-click context menus
// from `docs/re/gui/02-mainwindow-layout.md` & `docs/re/gui/06-dialogs-menus.md`.

use std::collections::{HashMap, HashSet};
use egui::{
    Align, Color32, FontId, Id, Layout, Pos2, Rect, Response, RichText, Sense, Stroke,
    TextureHandle, Ui, Vec2,
};
use ts3_client::connection::{ChannelInfo, ClientInfo, VirtualServerInfo};

use crate::assets::Ts3Assets;
use crate::menus_toolbar::UiCommand;
use crate::theme::Ts3Palette;

/// Selected node in the `ServerTreeView` (`QTreeView#ServerTreeView`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum SelectedTreeNode {
    #[default]
    ServerRoot,
    Channel(u64),
    Client(u16),
}

/// Persistent UI state for `ServerTreeView`.
#[derive(Debug, Clone, Default)]
pub struct TreeViewState {
    pub selected: SelectedTreeNode,
    pub collapsed_channels: HashSet<u64>,
    pub server_root_collapsed: bool,
    pub password_prompt_channel: Option<(u64, String)>,
    pub password_input: String,
}

/// Parsed TeamSpeak 3 top-level Spacer Channel variant (`docs/re/gui/02-mainwindow-layout.md` §5.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpacerKind {
    /// `[spacer...]<label>` — left-aligned text (or blank gap if `<label>` is empty) without a channel folder icon.
    Left(String),
    /// `[cspacer...]<label>` — centered text across the full row width without a channel folder icon.
    Center(String),
    /// `[rspacer...]<label>` — right-aligned text to the right boundary without a channel folder icon.
    Right(String),
    /// `[*spacer...]<pattern>` — repeats `<pattern>` across the full row width (or renders a line for `---`, `===`, `___`, `-.-`, `...`).
    Repeat(String),
}

/// Parses a TeamSpeak 3 spacer channel name according to `^\[([*rRcC]?spacer[^\]]*)\](.*)$`.
///
/// Only top-level channels (`pid == 0`) are eligible to be spacer channels.
pub fn parse_spacer_channel(name: &str, pid: u64) -> Option<SpacerKind> {
    if pid != 0 {
        return None;
    }
    let rest = name.strip_prefix('[')?;
    let bracket_end = rest.find(']')?;
    let tag = &rest[..bracket_end];
    let payload = &rest[bracket_end + 1..];

    let tag_lower = tag.to_ascii_lowercase();
    if tag_lower.starts_with("*spacer") {
        Some(SpacerKind::Repeat(payload.to_string()))
    } else if tag_lower.starts_with("cspacer") {
        Some(SpacerKind::Center(payload.to_string()))
    } else if tag_lower.starts_with("rspacer") {
        Some(SpacerKind::Right(payload.to_string()))
    } else if tag_lower.starts_with("spacer") {
        Some(SpacerKind::Left(payload.to_string()))
    } else {
        None
    }
}

/// Sorts sibling channels sharing `parent_cid` in TeamSpeak 3 linked-list order (`channel_order`):
/// 1. Starts from `order == 0` (the head of the sibling chain).
/// 2. Follows each channel whose `order == prev_cid`.
/// 3. Appends any remaining orphan channels (broken `order` references or cycles) in ascending `cid` order.
pub fn sort_channels_linked_list(channels: &[ChannelInfo], parent_cid: u64) -> Vec<ChannelInfo> {
    let mut siblings: Vec<&ChannelInfo> =
        channels.iter().filter(|c| c.pid == parent_cid).collect();
    if siblings.is_empty() {
        return Vec::new();
    }

    // Sort by cid ascending first for deterministic tie-breaking and orphan fallback.
    siblings.sort_by_key(|c| c.cid);

    let mut by_order: HashMap<u64, Vec<&ChannelInfo>> = HashMap::with_capacity(siblings.len());
    for ch in &siblings {
        by_order.entry(ch.order).or_default().push(*ch);
    }

    let mut result = Vec::with_capacity(siblings.len());
    let mut visited = HashSet::with_capacity(siblings.len());
    let mut stack: Vec<&ChannelInfo> = Vec::new();

    if let Some(heads) = by_order.get(&0) {
        for &head in heads.iter().rev() {
            stack.push(head);
        }
    }

    while let Some(curr) = stack.pop() {
        if !visited.insert(curr.cid) {
            continue;
        }
        result.push(curr.clone());
        if let Some(nexts) = by_order.get(&curr.cid) {
            for &next in nexts.iter().rev() {
                if !visited.contains(&next.cid) {
                    stack.push(next);
                }
            }
        }
    }

    // Fallback for any orphans whose `order` points to a non-existent sibling or cycle.
    for ch in siblings {
        if !visited.contains(&ch.cid) {
            stack.push(ch);
            while let Some(curr) = stack.pop() {
                if !visited.insert(curr.cid) {
                    continue;
                }
                result.push(curr.clone());
                if let Some(nexts) = by_order.get(&curr.cid) {
                    for &next in nexts.iter().rev() {
                        if !visited.contains(&next.cid) {
                            stack.push(next);
                        }
                    }
                }
            }
        }
    }

    result
}

/// Sorts clients inside a channel (`cid`) according to TeamSpeak 3 rules:
/// - Regular clients only (`client_type == 0`; skips ServerQuery `client_type == 1`).
/// - Ordered by `talk_power` descending, then `nickname` case-insensitive ascending (with `clid` tie-breaker).
pub fn sort_channel_clients(clients: &[ClientInfo], cid: u64) -> Vec<ClientInfo> {
    let mut list: Vec<ClientInfo> = clients
        .iter()
        .filter(|c| c.cid == cid && c.client_type == 0)
        .cloned()
        .collect();

    list.sort_by(|a, b| {
        b.talk_power
            .cmp(&a.talk_power)
            .then_with(|| {
                a.nickname
                    .to_lowercase()
                    .cmp(&b.nickname.to_lowercase())
            })
            .then_with(|| a. nickname.cmp(&b.nickname))
            .then_with(|| a.clid.cmp(&b.clid))
    });

    list
}

/// Returns the logical icon key in `Ts3Assets` for a channel's primary status icon.
pub fn channel_status_icon_key(ch: &ChannelInfo) -> &'static str {
    let is_full = ch.max_clients >= 0 && (ch.total_clients as i32) >= ch.max_clients;
    let sub = ch.subscribed || true; // Default to subscribed icon set per spec (`CHANNEL_*_SUBSCRIBED`)
    if is_full {
        if sub {
            "CHANNEL_RED_SUBSCRIBED"
        } else {
            "CHANNEL_RED"
        }
    } else if ch.is_password {
        if sub {
            "CHANNEL_YELLOW_SUBSCRIBED"
        } else {
            "CHANNEL_YELLOW"
        }
    } else if sub {
        "CHANNEL_GREEN_SUBSCRIBED"
    } else {
        "CHANNEL_GREEN"
    }
}

/// Returns the logical icon key in `Ts3Assets` for a client's primary status lamp/icon.
pub fn client_status_icon_key(client: &ClientInfo) -> &'static str {
    if client.away {
        "AWAY"
    } else if !client.output_hardware {
        "HARDWARE_OUTPUT_MUTED"
    } else if client.output_muted {
        "OUTPUT_MUTED"
    } else if !client.input_hardware {
        "HARDWARE_INPUT_MUTED"
    } else if client.input_muted {
        "INPUT_MUTED"
    } else if client.talking {
        if client.is_channel_commander {
            "PLAYER_COMMANDER_ON"
        } else {
            "PLAYER_ON"
        }
    } else if client.is_channel_commander {
        "PLAYER_COMMANDER_OFF"
    } else {
        "PLAYER_OFF"
    }
}

/// Renders the TeamSpeak 3 `ServerTreeView` (`QTreeView#ServerTreeView`).
#[allow(clippy::too_many_arguments)]
pub fn render_server_tree(
    ui: &mut egui::Ui,
    tree_state: &mut TreeViewState,
    connected: bool,
    my_clid: u16,
    my_cid: u64,
    server: &VirtualServerInfo,
    channels: &[ChannelInfo],
    clients: &[ClientInfo],
    palette: &Ts3Palette,
    assets: &Ts3Assets,
    commands: &mut Vec<UiCommand>,
) {
    ui.spacing_mut().item_spacing = Vec2::new(4.0, 1.0);

    if !connected && server.name.is_empty() && channels.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(24.0);
            ui.label(
                RichText::new("Not connected to a server")
                    .color(palette.text_muted)
                    .italics(),
            );
        });
        return;
    }

    // Pre-build parent -> sorted children map and channel -> sorted clients map.
    let mut parent_ids: HashSet<u64> = HashSet::new();
    parent_ids.insert(0);
    for ch in channels {
        parent_ids.insert(ch.pid);
    }

    let mut children_map: HashMap<u64, Vec<ChannelInfo>> = HashMap::new();
    for pid in parent_ids {
        let sorted = sort_channels_linked_list(channels, pid);
        if !sorted.is_empty() {
            children_map.insert(pid, sorted);
        }
    }

    let mut clients_by_cid: HashMap<u64, Vec<ClientInfo>> = HashMap::new();
    for ch in channels {
        let sorted_clients = sort_channel_clients(clients, ch.cid);
        if !sorted_clients.is_empty() {
            clients_by_cid.insert(ch.cid, sorted_clients);
        }
    }

    let all_channel_ids: Vec<u64> = channels.iter().map(|c| c.cid).collect();

    egui::ScrollArea::vertical()
        .id_salt("ts3_server_tree_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            // 1. Render VirtualServer Root Node
            render_server_root_row(
                ui,
                tree_state,
                server,
                &all_channel_ids,
                palette,
                assets,
                commands,
            );

            // 2. Render Channel Hierarchy if root is expanded
            if !tree_state.server_root_collapsed {
                if let Some(root_channels) = children_map.get(&0) {
                    for ch in root_channels {
                        render_channel_subtree(
                            ui,
                            tree_state,
                            ch,
                            0,
                            my_clid,
                            my_cid,
                            &children_map,
                            &clients_by_cid,
                            palette,
                            assets,
                            commands,
                        );
                    }
                }
            }
        });

    // 3. Render inline password modal if user double-clicked a password-protected channel
    render_channel_password_modal(ui, tree_state, palette, commands);
}

fn render_server_root_row(
    ui: &mut Ui,
    tree_state: &mut TreeViewState,
    server: &VirtualServerInfo,
    all_channel_ids: &[u64],
    palette: &Ts3Palette,
    assets: &Ts3Assets,
    commands: &mut Vec<UiCommand>,
) {
    let is_selected = tree_state.selected == SelectedTreeNode::ServerRoot;
    let server_title = if server.name.is_empty() {
        "TeamSpeak 3 Server"
    } else {
        server.name.as_str()
    };

    let row_height = 20.0;
    let avail_width = ui.available_width().max(120.0);
    let (row_rect, response) =
        ui.allocate_exact_size(Vec2::new(avail_width, row_height), Sense::click());

    // Selection / hover background
    paint_row_background(ui, row_rect, is_selected, response.hovered(), palette);

    let fg_color = if is_selected {
        palette.selection_fg
    } else {
        palette.text_primary
    };

    // Expand/collapse arrow hit-box
    let arrow_rect = Rect::from_min_size(
        Pos2::new(row_rect.left() + 2.0, row_rect.top() + 2.0),
        Vec2::new(14.0, 16.0),
    );
    let arrow_symbol = if tree_state.server_root_collapsed {
        "▸"
    } else {
        "▾"
    };
    ui.painter().text(
        arrow_rect.center(),
        egui::Align2::CENTER_CENTER,
        arrow_symbol,
        FontId::proportional(11.0),
        fg_color,
    );

    // Server icon (`SERVER_GREEN`)
    let icon_rect = Rect::from_min_size(
        Pos2::new(arrow_rect.right() + 2.0, row_rect.center().y - 8.0),
        Vec2::new(16.0, 16.0),
    );
    draw_icon_or_fallback(
        ui,
        icon_rect,
        assets.get_icon("SERVER_GREEN"),
        Color32::from_rgb(0x2E, 0xA0, 0x43),
        "S",
    );

    // Server name (bold)
    let text_pos = Pos2::new(icon_rect.right() + 5.0, row_rect.center().y);
    ui.painter().text(
        text_pos,
        egui::Align2::LEFT_CENTER,
        server_title,
        FontId::proportional(13.0),
        fg_color,
    );

    if response.clicked() {
        if let Some(pos) = response.interact_pointer_pos() {
            if arrow_rect.expand(2.0).contains(pos) {
                tree_state.server_root_collapsed = !tree_state.server_root_collapsed;
            }
        }
        tree_state.selected = SelectedTreeNode::ServerRoot;
    }
    if response.double_clicked() {
        tree_state.server_root_collapsed = !tree_state.server_root_collapsed;
    }

    // Server Root Context Menu (`docs/re/gui/06-dialogs-menus.md` §8.1)
    response.context_menu(|ui| {
        tree_state.selected = SelectedTreeNode::ServerRoot;
        if menu_item_with_icon(ui, assets, "CHANNEL_CREATE", "Create Channel").clicked() {
            commands.push(UiCommand::OpenCreateChannelDialog { parent_cid: 0 });
            ui.close_menu();
        }
        ui.separator();
        if menu_item_with_icon(
            ui,
            assets,
            "SUBSCRIBE_TO_ALL_CHANNELS",
            "Subscribe to All Channels",
        )
        .clicked()
        {
            commands.push(UiCommand::SubscribeAllChannels);
            ui.close_menu();
        }
        ui.separator();
        if menu_item_with_icon(ui, assets, "CHANNEL_EXPAND_ALL", "Expand All").clicked() {
            tree_state.server_root_collapsed = false;
            tree_state.collapsed_channels.clear();
            ui.close_menu();
        }
        if menu_item_with_icon(ui, assets, "CHANNEL_COLLAPSE_ALL", "Collapse All").clicked() {
            for &cid in all_channel_ids {
                tree_state.collapsed_channels.insert(cid);
            }
            ui.close_menu();
        }
        ui.separator();
        if menu_item_with_icon(ui, assets, "CHANGE_NICKNAME", "Change Nickname").clicked() {
            commands.push(UiCommand::ChangeNicknamePrompt);
            ui.close_menu();
        }
        if menu_item_with_icon(ui, assets, "COPY", "Copy Server Name").clicked() {
            ui.ctx().copy_text(server_title.to_string());
            ui.close_menu();
        }
    });
}

#[allow(clippy::too_many_arguments)]
fn render_channel_subtree(
    ui: &mut Ui,
    tree_state: &mut TreeViewState,
    ch: &ChannelInfo,
    depth: usize,
    my_clid: u16,
    my_cid: u64,
    children_map: &HashMap<u64, Vec<ChannelInfo>>,
    clients_by_cid: &HashMap<u64, Vec<ClientInfo>>,
    palette: &Ts3Palette,
    assets: &Ts3Assets,
    commands: &mut Vec<UiCommand>,
) {
    let subchannels = children_map.get(&ch.cid);
    let ch_clients = clients_by_cid.get(&ch.cid);
    let has_children = subchannels.is_some_and(|s| !s.is_empty())
        || ch_clients.is_some_and(|c| !c.is_empty());
    let is_collapsed = tree_state.collapsed_channels.contains(&ch.cid);

    let response = if let Some(spacer) = parse_spacer_channel(&ch.name, ch.pid) {
        render_spacer_row(ui, tree_state, ch, &spacer, palette)
    } else {
        render_normal_channel_row(
            ui,
            tree_state,
            ch,
            depth,
            has_children,
            is_collapsed,
            palette,
            assets,
        )
    };

    // Single-click selects `Channel(cid)` (updating `InfoFrame`)
    if response.clicked() {
        tree_state.selected = SelectedTreeNode::Channel(ch.cid);
    }

    // Double-click switches channel (or opens password prompt if `ch.is_password && ch.cid != my_cid`)
    if response.double_clicked() && ch.cid != my_cid {
        request_channel_switch(tree_state, ch, commands);
    }

    // Channel Context Menu (`docs/re/gui/06-dialogs-menus.md` §8.2)
    response.context_menu(|ui| {
        tree_state.selected = SelectedTreeNode::Channel(ch.cid);
        if menu_item_with_icon(ui, assets, "CHANNEL_SWITCH", "Switch to Channel").clicked() {
            if ch.cid != my_cid {
                request_channel_switch(tree_state, ch, commands);
            }
            ui.close_menu();
        }
        ui.separator();
        if menu_item_with_icon(ui, assets, "CHANNEL_CREATE", "Create Channel").clicked() {
            commands.push(UiCommand::OpenCreateChannelDialog { parent_cid: 0 });
            ui.close_menu();
        }
        if menu_item_with_icon(ui, assets, "CHANNEL_CREATE_SUB", "Create Sub-Channel").clicked() {
            commands.push(UiCommand::OpenCreateChannelDialog { parent_cid: ch.cid });
            ui.close_menu();
        }
        ui.separator();
        if menu_item_with_icon(ui, assets, "COPY", "Copy Channel Name").clicked() {
            ui.ctx().copy_text(ch.name.clone());
            ui.close_menu();
        }
        if has_children {
            ui.separator();
            if menu_item_with_icon(ui, assets, "CHANNEL_EXPAND_SUB", "Expand Subchannels")
                .clicked()
            {
                expand_subtree_recursive(ch.cid, children_map, &mut tree_state.collapsed_channels);
                ui.close_menu();
            }
            if menu_item_with_icon(ui, assets, "CHANNEL_COLLAPSE_SUB", "Collapse Subchannels")
                .clicked()
            {
                collapse_subtree_recursive(
                    ch.cid,
                    children_map,
                    &mut tree_state.collapsed_channels,
                );
                ui.close_menu();
            }
        }
    });

    if !is_collapsed {
        // In TeamSpeak 3, clients in a channel are rendered before sub-channels by default.
        if let Some(client_list) = ch_clients {
            for client in client_list {
                render_client_row(
                    ui,
                    tree_state,
                    client,
                    depth + 1,
                    my_clid,
                    my_cid,
                    palette,
                    assets,
                    commands,
                );
            }
        }

        if let Some(subs) = subchannels {
            for sub in subs {
                render_channel_subtree(
                    ui,
                    tree_state,
                    sub,
                    depth + 1,
                    my_clid,
                    my_cid,
                    children_map,
                    clients_by_cid,
                    palette,
                    assets,
                    commands,
                );
            }
        }
    }
}

fn request_channel_switch(
    tree_state: &mut TreeViewState,
    ch: &ChannelInfo,
    commands: &mut Vec<UiCommand>,
) {
    if ch.is_password {
        tree_state.password_prompt_channel = Some((ch.cid, ch.name.clone()));
        tree_state.password_input.clear();
    } else {
        commands.push(UiCommand::SwitchChannel {
            cid: ch.cid,
            password: None,
        });
    }
}

fn expand_subtree_recursive(
    cid: u64,
    children_map: &HashMap<u64, Vec<ChannelInfo>>,
    collapsed: &mut HashSet<u64>,
) {
    collapsed.remove(&cid);
    if let Some(subs) = children_map.get(&cid) {
        for sub in subs {
            expand_subtree_recursive(sub.cid, children_map, collapsed);
        }
    }
}

fn collapse_subtree_recursive(
    cid: u64,
    children_map: &HashMap<u64, Vec<ChannelInfo>>,
    collapsed: &mut HashSet<u64>,
) {
    collapsed.insert(cid);
    if let Some(subs) = children_map.get(&cid) {
        for sub in subs {
            collapse_subtree_recursive(sub.cid, children_map, collapsed);
        }
    }
}

fn render_spacer_row(
    ui: &mut Ui,
    tree_state: &TreeViewState,
    ch: &ChannelInfo,
    spacer: &SpacerKind,
    palette: &Ts3Palette,
) -> Response {
    let is_selected = tree_state.selected == SelectedTreeNode::Channel(ch.cid);
    let row_height = 18.0;
    let avail_width = ui.available_width().max(120.0);
    let (row_rect, response) =
        ui.allocate_exact_size(Vec2::new(avail_width, row_height), Sense::click());

    paint_row_background(ui, row_rect, is_selected, response.hovered(), palette);

    let fg_color = if is_selected {
        palette.selection_fg
    } else {
        palette.text_muted
    };
    let line_color = if is_selected {
        palette.selection_fg
    } else {
        palette.border_color
    };

    let inner_rect = row_rect.shrink2(Vec2::new(6.0, 0.0));
    match spacer {
        SpacerKind::Left(label) => {
            if !label.is_empty() {
                ui.painter().with_clip_rect(inner_rect).text(
                    Pos2::new(inner_rect.left() + 12.0, inner_rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    label,
                    FontId::proportional(12.5),
                    fg_color,
                );
            }
        }
        SpacerKind::Center(label) => {
            if !label.is_empty() {
                ui.painter().with_clip_rect(inner_rect).text(
                    inner_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    label,
                    FontId::proportional(12.5),
                    fg_color,
                );
            }
        }
        SpacerKind::Right(label) => {
            if !label.is_empty() {
                ui.painter().with_clip_rect(inner_rect).text(
                    Pos2::new(inner_rect.right() - 4.0, inner_rect.center().y),
                    egui::Align2::RIGHT_CENTER,
                    label,
                    FontId::proportional(12.5),
                    fg_color,
                );
            }
        }
        SpacerKind::Repeat(pattern) => {
            let trimmed = pattern.trim();
            let y = inner_rect.center().y;
            let x_start = inner_rect.left() + 4.0;
            let x_end = inner_rect.right() - 4.0;

            match trimmed {
                "---" | "___" => {
                    // Continuous solid horizontal dividing rule
                    ui.painter().line_segment(
                        [Pos2::new(x_start, y), Pos2::new(x_end, y)],
                        Stroke::new(1.0_f32, line_color),
                    );
                }
                "===" => {
                    // Continuous double-line dividing rule
                    ui.painter().line_segment(
                        [Pos2::new(x_start, y - 1.5), Pos2::new(x_end, y - 1.5)],
                        Stroke::new(1.0_f32, line_color),
                    );
                    ui.painter().line_segment(
                        [Pos2::new(x_start, y + 1.5), Pos2::new(x_end, y + 1.5)],
                        Stroke::new(1.0_f32, line_color),
                    );
                }
                "-.-" => {
                    // Decorative dash-dot divider line
                    draw_dash_dot_line(ui, x_start, x_end, y, line_color);
                }
                "..." => {
                    // Dotted separator line across the channel tree
                    draw_dotted_line(ui, x_start, x_end, y, line_color);
                }
                _ if !pattern.is_empty() => {
                    // Repeat the pattern across the full row width
                    let char_count = pattern.chars().count().max(1);
                    let approx_pattern_px = (char_count as f32 * 6.5).max(6.5);
                    let repeats = ((inner_rect.width() / approx_pattern_px).ceil() as usize)
                        .clamp(1, 256);
                    let repeated: String = pattern.repeat(repeats);
                    ui.painter().with_clip_rect(inner_rect).text(
                        Pos2::new(x_start, y),
                        egui::Align2::LEFT_CENTER,
                        repeated,
                        FontId::monospace(12.0),
                        fg_color,
                    );
                }
                _ => {}
            }
        }
    }

    response
}

fn draw_dash_dot_line(ui: &Ui, x_start: f32, x_end: f32, y: f32, color: Color32) {
    let painter = ui.painter();
    let mut x = x_start;
    let stroke = Stroke::new(1.0_f32, color);
    while x < x_end {
        let dash_end = (x + 6.0).min(x_end);
        painter.line_segment([Pos2::new(x, y), Pos2::new(dash_end, y)], stroke);
        x += 9.0;
        if x + 1.5 <= x_end {
            painter.circle_filled(Pos2::new(x, y), 1.0, color);
        }
        x += 4.0;
    }
}

fn draw_dotted_line(ui: &Ui, x_start: f32, x_end: f32, y: f32, color: Color32) {
    let painter = ui.painter();
    let mut x = x_start;
    while x < x_end {
        painter.circle_filled(Pos2::new(x, y), 1.0, color);
        x += 4.0;
    }
}

#[allow(clippy::too_many_arguments)]
fn render_normal_channel_row(
    ui: &mut Ui,
    tree_state: &mut TreeViewState,
    ch: &ChannelInfo,
    depth: usize,
    has_children: bool,
    is_collapsed: bool,
    palette: &Ts3Palette,
    assets: &Ts3Assets,
) -> Response {
    let is_selected = tree_state.selected == SelectedTreeNode::Channel(ch.cid);
    let row_height = 20.0;
    let avail_width = ui.available_width().max(120.0);
    let (row_rect, response) =
        ui.allocate_exact_size(Vec2::new(avail_width, row_height), Sense::click());

    paint_row_background(ui, row_rect, is_selected, response.hovered(), palette);

    let fg_color = if is_selected {
        palette.selection_fg
    } else {
        palette.text_primary
    };

    let indent = 16.0 + (depth as f32) * 16.0;
    let arrow_rect = Rect::from_min_size(
        Pos2::new(row_rect.left() + indent, row_rect.top() + 2.0),
        Vec2::new(14.0, 16.0),
    );

    if has_children {
        let arrow_symbol = if is_collapsed { "▸" } else { "▾" };
        ui.painter().text(
            arrow_rect.center(),
            egui::Align2::CENTER_CENTER,
            arrow_symbol,
            FontId::proportional(11.0),
            fg_color,
        );
        if response.clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                if arrow_rect.expand(2.0).contains(pos) {
                    if is_collapsed {
                        tree_state.collapsed_channels.remove(&ch.cid);
                    } else {
                        tree_state.collapsed_channels.insert(ch.cid);
                    }
                }
            }
        }
    }

    // Channel status icon (`CHANNEL_GREEN_SUBSCRIBED`, `CHANNEL_YELLOW_SUBSCRIBED`, `CHANNEL_RED_SUBSCRIBED`)
    let icon_key = channel_status_icon_key(ch);
    let fallback_color = match icon_key {
        "CHANNEL_RED_SUBSCRIBED" | "CHANNEL_RED" => Color32::from_rgb(0xD9, 0x38, 0x3A),
        "CHANNEL_YELLOW_SUBSCRIBED" | "CHANNEL_YELLOW" => Color32::from_rgb(0xE0, 0xA8, 0x2E),
        _ => Color32::from_rgb(0x2E, 0xA0, 0x43),
    };
    let icon_rect = Rect::from_min_size(
        Pos2::new(arrow_rect.right() + 2.0, row_rect.center().y - 7.0),
        Vec2::new(14.0, 14.0),
    );
    draw_icon_or_fallback(
        ui,
        icon_rect,
        assets.get_icon(icon_key),
        fallback_color,
        "C",
    );

    // Right-aligned badges (`DEFAULT`, `REGISTER`, `MODERATED`, `MUSIC`)
    let mut right_cursor = row_rect.right() - 4.0;
    let badge_size = Vec2::new(14.0, 14.0);

    // Collect active badges in right-to-left or left-to-right order at the right edge
    let mut badges: Vec<(&str, Color32, &str)> = Vec::new();
    if ch.is_default {
        badges.push(("DEFAULT", Color32::from_rgb(0x2F, 0x71, 0xAF), "H"));
    }
    if ch.is_password {
        badges.push(("REGISTER", Color32::from_rgb(0xE0, 0xA8, 0x2E), "P"));
    }
    if ch.needed_talk_power > 0 {
        badges.push(("MODERATED", Color32::from_rgb(0xC9, 0x6A, 0x28), "M"));
    }
    if ch.codec == 5 {
        badges.push(("MUSIC", Color32::from_rgb(0x7E, 0x57, 0xC2), "♫"));
    }

    for (badge_key, badge_color, badge_letter) in badges.into_iter().rev() {
        right_cursor -= badge_size.x;
        let b_rect = Rect::from_min_size(
            Pos2::new(right_cursor, row_rect.center().y - 7.0),
            badge_size,
        );
        draw_icon_or_fallback(
            ui,
            b_rect,
            assets.get_icon(badge_key),
            badge_color,
            badge_letter,
        );
        right_cursor -= 3.0;
    }

    // Channel Name clipped between icon_rect and right_cursor
    let text_left = icon_rect.right() + 5.0;
    let text_right = (right_cursor - 2.0).max(text_left + 10.0);
    let text_clip = Rect::from_min_max(
        Pos2::new(text_left, row_rect.top()),
        Pos2::new(text_right, row_rect.bottom()),
    );
    ui.painter().with_clip_rect(text_clip).text(
        Pos2::new(text_left, row_rect.center().y),
        egui::Align2::LEFT_CENTER,
        &ch.name,
        FontId::proportional(13.0),
        fg_color,
    );

    response
}

#[allow(clippy::too_many_arguments)]
fn render_client_row(
    ui: &mut Ui,
    tree_state: &mut TreeViewState,
    client: &ClientInfo,
    depth: usize,
    my_clid: u16,
    my_cid: u64,
    palette: &Ts3Palette,
    assets: &Ts3Assets,
    commands: &mut Vec<UiCommand>,
) {
    let is_selected = tree_state.selected == SelectedTreeNode::Client(client.clid);
    let is_self = client.clid == my_clid && my_clid != 0;
    let row_height = 20.0;
    let avail_width = ui.available_width().max(120.0);
    let (row_rect, response) =
        ui.allocate_exact_size(Vec2::new(avail_width, row_height), Sense::click());

    paint_row_background(ui, row_rect, is_selected, response.hovered(), palette);

    let fg_color = if is_selected {
        palette.selection_fg
    } else if is_self {
        palette.accent_blue
    } else {
        palette.text_primary
    };

    // Align client icon beneath child level
    let indent = 16.0 + (depth as f32) * 16.0 + 16.0;
    let icon_key = client_status_icon_key(client);
    let fallback_color = match icon_key {
        "PLAYER_ON" => Color32::from_rgb(0x33, 0x99, 0xFF),
        "PLAYER_COMMANDER_ON" => Color32::from_rgb(0xFF, 0x98, 0x00),
        "PLAYER_COMMANDER_OFF" => Color32::from_rgb(0xC6, 0x71, 0x00),
        "AWAY" => Color32::from_rgb(0xE0, 0xA8, 0x2E),
        "OUTPUT_MUTED" | "INPUT_MUTED" | "HARDWARE_INPUT_MUTED" | "HARDWARE_OUTPUT_MUTED" => {
            Color32::from_rgb(0xD9, 0x38, 0x3A)
        }
        _ => Color32::from_rgb(0x2F, 0x71, 0xAF),
    };

    let icon_rect = Rect::from_min_size(
        Pos2::new(row_rect.left() + indent, row_rect.center().y - 7.0),
        Vec2::new(14.0, 14.0),
    );
    draw_icon_or_fallback(
        ui,
        icon_rect,
        assets.get_icon(icon_key),
        fallback_color,
        "•",
    );

    // Right-aligned decorations: Country Flag (`assets.get_flag(&client.country)`) + Server Group tag badges
    let mut right_cursor = row_rect.right() - 4.0;

    // Server group tag badges
    for &sgid in client.server_groups.iter().rev() {
        let group_icon_key = match sgid {
            6 => Some(("GROUP_300", "[S]")),
            7 => Some(("GROUP_500", "[N]")),
            8 => None, // Guest typically has no badge icon unless configured, but show tag pill
            100 => Some(("GROUP_100", "[C]")),
            200 => Some(("GROUP_200", "[O]")),
            300 => Some(("GROUP_300", "[S]")),
            500 => Some(("GROUP_500", "[G]")),
            600 => Some(("GROUP_600", "[V]")),
            _ => None,
        };

        if let Some((g_key, fallback_tag)) = group_icon_key {
            if let Some(tex) = assets.get_icon(g_key) {
                right_cursor -= 14.0;
                let g_rect = Rect::from_min_size(
                    Pos2::new(right_cursor, row_rect.center().y - 7.0),
                    Vec2::new(14.0, 14.0),
                );
                ui.painter().image(
                    tex.id(),
                    g_rect,
                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );
                right_cursor -= 3.0;
                continue;
            }
            right_cursor = draw_group_pill(ui, right_cursor, row_rect.center().y, fallback_tag, palette, is_selected);
        } else {
            let tag_label = format!("[{sgid}]");
            right_cursor = draw_group_pill(ui, right_cursor, row_rect.center().y, &tag_label, palette, is_selected);
        }
    }

    // Priority speaker badge (`IS_TALKER`) if active
    if client.is_talker {
        right_cursor -= 14.0;
        let t_rect = Rect::from_min_size(
            Pos2::new(right_cursor, row_rect.center().y - 7.0),
            Vec2::new(14.0, 14.0),
        );
        draw_icon_or_fallback(
            ui,
            t_rect,
            assets.get_icon("IS_TALKER"),
            Color32::from_rgb(0x33, 0x99, 0xFF),
            "T",
        );
        right_cursor -= 3.0;
    }

    // Country flag (`assets.get_flag(&client.country)`)
    if !client.country.is_empty() {
        if let Some(flag_tex) = assets.get_flag(&client.country) {
            right_cursor -= 16.0;
            let flag_rect = Rect::from_min_size(
                Pos2::new(right_cursor, row_rect.center().y - 5.5),
                Vec2::new(16.0, 11.0),
            );
            ui.painter().image(
                flag_tex.id(),
                flag_rect,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );
            right_cursor -= 4.0;
        } else {
            let cc = client.country.to_uppercase();
            right_cursor = draw_group_pill(ui, right_cursor, row_rect.center().y, &cc, palette, is_selected);
        }
    }

    // Client Nickname (+ optional away message in muted brackets, bold highlight for our own client `clid == my_clid`)
    let display_name = if client.away && !client.away_message.is_empty() {
        format!("{} [{}]", client.nickname, client.away_message)
    } else {
        client.nickname.clone()
    };

    let text_left = icon_rect.right() + 5.0;
    let text_right = (right_cursor - 2.0).max(text_left + 10.0);
    let text_clip = Rect::from_min_max(
        Pos2::new(text_left, row_rect.top()),
        Pos2::new(text_right, row_rect.bottom()),
    );

    let painter = ui.painter().with_clip_rect(text_clip);
    let font = FontId::proportional(13.0);
    painter.text(
        Pos2::new(text_left, row_rect.center().y),
        egui::Align2::LEFT_CENTER,
        &display_name,
        font.clone(),
        fg_color,
    );
    // Faux-bold pass for our own client (`clid == my_clid`)
    if is_self {
        painter.text(
            Pos2::new(text_left + 0.6, row_rect.center().y),
            egui::Align2::LEFT_CENTER,
            &display_name,
            font,
            fg_color,
        );
    }

    // Single-click selects `Client(clid)`
    if response.clicked() {
        tree_state.selected = SelectedTreeNode::Client(client.clid);
    }

    // Double-click on a client emits `UiCommand::OpenPrivateChat`
    if response.double_clicked() {
        commands.push(UiCommand::OpenPrivateChat {
            clid: client.clid,
            nickname: client.nickname.clone(),
        });
    }

    // Client Context Menu (`docs/re/gui/06-dialogs-menus.md` §8.3)
    response.context_menu(|ui| {
        tree_state.selected = SelectedTreeNode::Client(client.clid);
        if menu_item_with_icon(ui, assets, "PLAYER_CHAT", "Open Text Chat").clicked() {
            commands.push(UiCommand::OpenPrivateChat {
                clid: client.clid,
                nickname: client.nickname.clone(),
            });
            ui.close_menu();
        }
        if menu_item_with_icon(ui, assets, "POKE", "Poke Client").clicked() {
            commands.push(UiCommand::OpenPokeDialog {
                clid: client.clid,
                nickname: client.nickname.clone(),
            });
            ui.close_menu();
        }
        if is_self {
            ui.separator();
            if menu_item_with_icon(ui, assets, "CHANGE_NICKNAME", "Change Nickname").clicked() {
                commands.push(UiCommand::ChangeNicknamePrompt);
                ui.close_menu();
            }
        } else if client.cid != my_cid {
            ui.separator();
            if menu_item_with_icon(ui, assets, "CHANNEL_SWITCH", "Join Channel of Client")
                .clicked()
            {
                commands.push(UiCommand::SwitchChannel {
                    cid: client.cid,
                    password: None,
                });
                ui.close_menu();
            }
        }
        ui.separator();
        if menu_item_with_icon(ui, assets, "COPY", "Copy Unique ID").clicked() {
            ui.ctx().copy_text(client.uid.clone());
            ui.close_menu();
        }
        if menu_item_with_icon(ui, assets, "COPY", "Copy Nickname").clicked() {
            ui.ctx().copy_text(client.nickname.clone());
            ui.close_menu();
        }
    });
}

fn render_channel_password_modal(
    ui: &mut Ui,
    tree_state: &mut TreeViewState,
    palette: &Ts3Palette,
    commands: &mut Vec<UiCommand>,
) {
    let Some((target_cid, target_name)) = tree_state.password_prompt_channel.clone() else {
        return;
    };

    let mut open = true;
    let mut submit = false;
    let mut cancel = false;

    egui::Window::new("Channel Password")
        .id(Id::new("ts3_channel_password_modal"))
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
        .open(&mut open)
        .show(ui.ctx(), |ui| {
            ui.set_min_width(280.0);
            ui.label(
                RichText::new(format!("Enter password for channel \"{target_name}\":"))
                    .color(palette.text_primary),
            );
            ui.add_space(4.0);
            let edit_resp = ui.add(
                egui::TextEdit::singleline(&mut tree_state.password_input)
                    .password(true)
                    .desired_width(f32::INFINITY)
                    .hint_text("Channel password"),
            );
            if edit_resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                submit = true;
            }
            ui.add_space(8.0);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.button("Cancel").clicked() {
                    cancel = true;
                }
                if ui.button("OK").clicked() {
                    submit = true;
                }
            });
        });

    if submit {
        let pw = tree_state.password_input.clone();
        commands.push(UiCommand::SwitchChannel {
            cid: target_cid,
            password: Some(pw),
        });
        tree_state.password_prompt_channel = None;
        tree_state.password_input.clear();
    } else if cancel || !open {
        tree_state.password_prompt_channel = None;
        tree_state.password_input.clear();
    }
}

fn paint_row_background(
    ui: &Ui,
    rect: Rect,
    is_selected: bool,
    is_hovered: bool,
    palette: &Ts3Palette,
) {
    if is_selected {
        ui.painter().rect_filled(rect, 2.0, palette.selection_bg);
    } else if is_hovered {
        ui.painter().rect_filled(rect, 2.0, palette.hover_bg);
    }
}

fn draw_icon_or_fallback(
    ui: &Ui,
    rect: Rect,
    texture: Option<&TextureHandle>,
    fallback_color: Color32,
    fallback_glyph: &str,
) {
    if let Some(tex) = texture {
        ui.painter().image(
            tex.id(),
            rect,
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
            Color32::WHITE,
        );
    } else {
        ui.painter()
            .circle_filled(rect.center(), rect.width() * 0.42, fallback_color);
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            fallback_glyph,
            FontId::proportional(9.0),
            Color32::WHITE,
        );
    }
}

fn draw_group_pill(
    ui: &Ui,
    right_cursor: f32,
    center_y: f32,
    label: &str,
    palette: &Ts3Palette,
    is_selected: bool,
) -> f32 {
    let font = FontId::proportional(10.0);
    let text_color = if is_selected {
        palette.selection_fg
    } else {
        palette.text_muted
    };
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), font, text_color);
    let width = galley.size().x + 4.0;
    let new_cursor = right_cursor - width;
    ui.painter().galley(
        Pos2::new(new_cursor + 2.0, center_y - galley.size().y * 0.5),
        galley,
        text_color,
    );
    new_cursor - 2.0
}

fn menu_item_with_icon(
    ui: &mut Ui,
    assets: &Ts3Assets,
    icon_key: &str,
    label: &str,
) -> Response {
    if let Some(tex) = assets.get_icon(icon_key) {
        ui.add(egui::Button::image_and_text(
            (tex.id(), Vec2::new(14.0, 14.0)),
            label,
        ))
    } else {
        ui.button(label)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_channel(cid: u64, pid: u64, order: u64, name: &str) -> ChannelInfo {
        ChannelInfo {
            cid,
            pid,
            order,
            name: name.to_string(),
            ..ChannelInfo::default()
        }
    }

    #[test]
    fn test_parse_spacer_channel_variants() {
        // Left-aligned spacers (`[spacer...]`)
        assert_eq!(
            parse_spacer_channel("[spacer10]", 0),
            Some(SpacerKind::Left(String::new()))
        );
        assert_eq!(
            parse_spacer_channel("[spacer_rules]Welcome Rules", 0),
            Some(SpacerKind::Left("Welcome Rules".to_string()))
        );

        // Center-aligned spacers (`[cspacer...]`)
        assert_eq!(
            parse_spacer_channel("[cspacer]--- Lobby ---", 0),
            Some(SpacerKind::Center("--- Lobby ---".to_string()))
        );
        assert_eq!(
            parse_spacer_channel("[cspacer0]", 0),
            Some(SpacerKind::Center(String::new()))
        );
        assert_eq!(
            parse_spacer_channel("[CSPACER42]Centered Header", 0),
            Some(SpacerKind::Center("Centered Header".to_string()))
        );

        // Right-aligned spacers (`[rspacer...]`)
        assert_eq!(
            parse_spacer_channel("[rspacer99]Staff Only", 0),
            Some(SpacerKind::Right("Staff Only".to_string()))
        );

        // Repeating character spacers (`[*spacer...]`)
        assert_eq!(
            parse_spacer_channel("[*spacer]---", 0),
            Some(SpacerKind::Repeat("---".to_string()))
        );
        assert_eq!(
            parse_spacer_channel("[*spacer1]===", 0),
            Some(SpacerKind::Repeat("===".to_string()))
        );
        assert_eq!(
            parse_spacer_channel("[*spacer_dots]...", 0),
            Some(SpacerKind::Repeat("...".to_string()))
        );
        assert_eq!(
            parse_spacer_channel("[*spacer_dashdot]-.-", 0),
            Some(SpacerKind::Repeat("-.-".to_string()))
        );

        // Non-root channels (`pid != 0`) are never parsed as spacers
        assert_eq!(parse_spacer_channel("[cspacer]Subchannel", 1), None);
        assert_eq!(parse_spacer_channel("[*spacer]---", 5), None);

        // Normal channel names are not spacers
        assert_eq!(parse_spacer_channel("Default Channel", 0), None);
        assert_eq!(parse_spacer_channel("[VIP] Lounge", 0), None);
    }

    #[test]
    fn test_channel_order_linked_list_sorting() {
        // Channels given out of order:
        // Head is cid=30 (order=0) -> cid=10 (order=30) -> cid=20 (order=10) -> cid=40 (order=20)
        let channels = vec![
            make_channel(20, 0, 10, "Third"),
            make_channel(40, 0, 20, "Fourth"),
            make_channel(10, 0, 30, "Second"),
            make_channel(30, 0, 0, "First"),
            // Subchannels of cid=10: cid=102 (order=0) -> cid=101 (order=102)
            make_channel(101, 10, 102, "Sub Second"),
            make_channel(102, 10, 0, "Sub First"),
        ];

        let sorted_root = sort_channels_linked_list(&channels, 0);
        let root_cids: Vec<u64> = sorted_root.iter().map(|c| c.cid).collect();
        assert_eq!(root_cids, vec![30, 10, 20, 40]);

        let sorted_sub = sort_channels_linked_list(&channels, 10);
        let sub_cids: Vec<u64> = sorted_sub.iter().map(|c| c.cid).collect();
        assert_eq!(sub_cids, vec![102, 101]);
    }

    #[test]
    fn test_channel_order_with_orphans_fallback() {
        // cid=2 has order=0 (valid head), cid=1 has order=999 (missing predecessor), cid=3 has order=888 (missing)
        let channels = vec![
            make_channel(3, 0, 888, "Orphan B"),
            make_channel(2, 0, 0, "Head"),
            make_channel(1, 0, 999, "Orphan A"),
        ];

        let sorted = sort_channels_linked_list(&channels, 0);
        let cids: Vec<u64> = sorted.iter().map(|c| c.cid).collect();
        // Head first, then orphans in ascending cid order (1, 3)
        assert_eq!(cids, vec![2, 1, 3]);
    }

    #[test]
    fn test_sort_channel_clients_talk_power_and_nickname() {
        let clients = vec![
            ClientInfo {
                clid: 1,
                cid: 10,
                nickname: "charlie".to_string(),
                talk_power: 50,
                client_type: 0,
                ..ClientInfo::default()
            },
            ClientInfo {
                clid: 2,
                cid: 10,
                nickname: "Alice".to_string(),
                talk_power: 75,
                client_type: 0,
                ..ClientInfo::default()
            },
            ClientInfo {
                clid: 3,
                cid: 10,
                nickname: "bob".to_string(),
                talk_power: 50,
                client_type: 0,
                ..ClientInfo::default()
            },
            ClientInfo {
                clid: 4,
                cid: 10,
                nickname: "serveradmin".to_string(),
                talk_power: 100,
                client_type: 1, // ServerQuery should be skipped
                ..ClientInfo::default()
            },
        ];

        let sorted = sort_channel_clients(&clients, 10);
        let clids: Vec<u16> = sorted.iter().map(|c| c.clid).collect();
        assert_eq!(clids, vec![2, 3, 1]);
    }
}
