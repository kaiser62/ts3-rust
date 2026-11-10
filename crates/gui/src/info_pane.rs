// Owned by Pool Agent 6 (arthoroy28a@gmail.com)
// Right-hand `InfoFrame` implementing the `.tpl` template display for Server Info (`serverinfo.tpl`),
// Channel Info (`channelinfo.tpl`), and Client Info (`clientinfo.tpl`) + top-right `logo-128x128.png` watermark
// from `docs/re/gui/04-info-templates.md`.

use egui::{
    Align, Color32, CornerRadius, FontId, Frame, Grid, ImageSource, Layout, Margin, Mesh, Pos2,
    Rect, RichText, ScrollArea, Shape, Stroke, StrokeKind, TextureHandle, Ui, Vec2,
};
use ts3_client::connection::{ChannelInfo, ClientInfo, VirtualServerInfo};

use crate::assets::Ts3Assets;
use crate::theme::{ThemeMode, Ts3Palette};
use crate::tree_view::SelectedTreeNode;

/// Formats a duration in seconds as `Xd Xh Xm Xs` (or compact sub-units when leading units are zero,
/// always matching standard `Xd Xh Xm Xs` components).
pub fn format_uptime(seconds: u64) -> String {
    let days = seconds / 86_400;
    let hours = (seconds % 86_400) / 3_600;
    let mins = (seconds % 3_600) / 60;
    let secs = seconds % 60;

    if days > 0 {
        format!("{days}d {hours}h {mins}m {secs}s")
    } else if hours > 0 {
        format!("{hours}h {mins}m {secs}s")
    } else if mins > 0 {
        format!("{mins}m {secs}s")
    } else {
        format!("{secs}s")
    }
}

/// Maps a TeamSpeak 3 numeric channel codec identifier to its display string (`channelinfo.tpl` `%%CHANNEL_CODEC%%`).
pub fn codec_name(codec: u8) -> &'static str {
    match codec {
        0 => "Speex Narrowband (0)",
        1 => "Speex Wideband (1)",
        2 => "Speex Ultra-Wideband (2)",
        3 => "CELT Mono (3)",
        4 => "Opus Voice (4)",
        5 => "Opus Music (5)",
        _ => "Unknown Codec",
    }
}

/// Estimates the voice bitrate in KiB/s for a given TS3 codec and quality (0..=10),
/// matching `%%CHANNEL_CODEC_BITRATE%%` in `channelinfo.tpl`.
pub fn estimate_codec_bitrate_kibs(codec: u8, quality: u8) -> f32 {
    let q = quality.min(10) as f32;
    match codec {
        0 => 1.95 + q * 0.25,
        1 => 2.60 + q * 0.42,
        2 => 3.10 + q * 0.55,
        3 => 4.50 + q * 0.70,
        4 => 2.73 + q * 0.50, // Opus Voice: ~2.73 KiB/s (q=0) to ~7.73 KiB/s (q=10), q=6 -> ~5.73 KiB/s
        5 => 3.91 + q * 0.79, // Opus Music: ~3.91 KiB/s (q=0) to ~11.81 KiB/s (q=10)
        _ => 3.00 + q * 0.50,
    }
}

/// Renders the right-hand `QTextEdit#InfoFrame` panel matching `default.qss`, `default_chat.qss`,
/// and `serverinfo.tpl` / `channelinfo.tpl` / `clientinfo.tpl`.
pub fn render_info_pane(
    ui: &mut egui::Ui,
    selected: &SelectedTreeNode,
    connected: bool,
    server_addr: &str,
    my_clid: u16,
    rtt_ms: u32,
    packet_loss: f32,
    server: &VirtualServerInfo,
    channels: &[ChannelInfo],
    clients: &[ClientInfo],
    palette: &Ts3Palette,
    assets: &Ts3Assets,
) {
    let frame = Frame::NONE
        .fill(palette.base_bg)
        .stroke(Stroke::new(1.0_f32, palette.border_color))
        .inner_margin(Margin::same(6));

    frame.show(ui, |ui| {
        let pane_rect = ui.max_rect();

        // 1. Top-right watermark (`styles:default/logo-128x128.png`, `background-position: top right;`)
        paint_top_right_watermark(ui, pane_rect, assets.watermark_logo.as_ref(), palette);

        ScrollArea::vertical()
            .id_salt("ts3_info_frame_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());

                if !connected {
                    render_disconnected_placeholder(ui, server_addr, palette, assets);
                    return;
                }

                match selected {
                    SelectedTreeNode::ServerRoot => {
                        render_server_info(
                            ui,
                            server_addr,
                            my_clid,
                            rtt_ms,
                            packet_loss,
                            server,
                            channels,
                            clients,
                            palette,
                            assets,
                        );
                    }
                    SelectedTreeNode::Channel(cid) => {
                        if let Some(ch) = channels.iter().find(|c| c.cid == *cid) {
                            render_channel_info(ui, ch, clients, palette, assets);
                        } else {
                            render_server_info(
                                ui,
                                server_addr,
                                my_clid,
                                rtt_ms,
                                packet_loss,
                                server,
                                channels,
                                clients,
                                palette,
                                assets,
                            );
                        }
                    }
                    SelectedTreeNode::Client(clid) => {
                        if let Some(cl) = clients.iter().find(|c| c.clid == *clid) {
                            render_client_info(ui, cl, channels, palette, assets);
                        } else {
                            render_server_info(
                                ui,
                                server_addr,
                                my_clid,
                                rtt_ms,
                                packet_loss,
                                server,
                                channels,
                                clients,
                                palette,
                                assets,
                            );
                        }
                    }
                }
            });
    });
}

/// Paints the 128x128 TeamSpeak watermark pinned to the top-right corner of `InfoFrame`
/// (`default.qss:68-74`: `background-image: url('styles:default/logo-128x128.png'); background-position: top right;`).
fn paint_top_right_watermark(
    ui: &mut Ui,
    pane_rect: Rect,
    watermark: Option<&TextureHandle>,
    palette: &Ts3Palette,
) {
    let size = Vec2::splat(128.0);
    let top_right = Pos2::new(
        (pane_rect.right() - size.x - 4.0).max(pane_rect.left()),
        pane_rect.top() + 4.0,
    );
    let target_rect = Rect::from_min_size(top_right, size).intersect(pane_rect);
    if !target_rect.is_positive() {
        return;
    }

    if let Some(tex) = watermark {
        let tint = match palette.mode {
            ThemeMode::Light => Color32::from_white_alpha(220),
            ThemeMode::Dark => Color32::from_white_alpha(90),
        };
        ui.painter().image(
            tex.id(),
            target_rect,
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
            tint,
        );
    } else {
        // Subtle vector watermark fallback if the external TS3 installation directory is absent
        let center = Pos2::new(pane_rect.right() - 48.0, pane_rect.top() + 48.0);
        if pane_rect.contains(center) {
            let stroke_col = match palette.mode {
                ThemeMode::Light => Color32::from_rgba_unmultiplied(47, 113, 175, 22),
                ThemeMode::Dark => Color32::from_rgba_unmultiplied(131, 172, 210, 18),
            };
            ui.painter()
                .circle_stroke(center, 34.0, Stroke::new(4.0_f32, stroke_col));
            ui.painter()
                .circle_stroke(center, 20.0, Stroke::new(2.5_f32, stroke_col));
        }
    }
}

/// Paints the 4-stop vertical linear gradient header banner from `default.qss:40-48`
/// (`#505C78` -> `#8593AE` -> `#586B8C` -> `#6F80AA`) and renders icon + title + subtitle.
fn render_gradient_banner(
    ui: &mut Ui,
    icon: Option<&TextureHandle>,
    fallback_glyph: &str,
    title: &str,
    subtitle: Option<&str>,
    right_badge: Option<(&str, Color32)>,
) {
    let desired_height = if subtitle.is_some_and(|s| !s.is_empty()) {
        48.0
    } else {
        36.0
    };
    let width = ui.available_width().max(120.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, desired_height), egui::Sense::hover());

    if ui.is_rect_visible(rect) {
        paint_four_stop_banner_gradient(ui, rect);
        ui.painter().rect_stroke(
            rect,
            CornerRadius::same(3),
            Stroke::new(1.0_f32, Color32::from_rgb(0x3D, 0x48, 0x63)),
            StrokeKind::Inside,
        );

        let inner = rect.shrink2(Vec2::new(10.0, 5.0));
        let mut child_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(inner)
                .layout(Layout::left_to_right(Align::Center)),
        );

        if let Some(tex) = icon {
            child_ui.add(
                egui::Image::new(ImageSource::Texture(egui::load::SizedTexture::from_handle(
                    tex,
                )))
                .fit_to_exact_size(Vec2::splat(22.0)),
            );
        } else {
            child_ui.label(
                RichText::new(fallback_glyph)
                    .size(16.0)
                    .strong()
                    .color(Color32::WHITE),
            );
        }

        child_ui.add_space(6.0);

        child_ui.vertical(|col| {
            col.spacing_mut().item_spacing.y = 1.0;
            col.horizontal(|row| {
                row.label(
                    RichText::new(title)
                        .font(FontId::proportional(16.5))
                        .strong()
                        .color(Color32::WHITE),
                );
                if let Some((badge_text, badge_bg)) = right_badge {
                    row.add_space(6.0);
                    Frame::NONE
                        .fill(badge_bg)
                        .corner_radius(CornerRadius::same(3))
                        .inner_margin(Margin::symmetric(6, 1))
                        .show(row, |bui| {
                            bui.label(
                                RichText::new(badge_text)
                                    .size(11.0)
                                    .strong()
                                    .color(Color32::WHITE),
                            );
                        });
                }
            });
            if let Some(sub) = subtitle {
                if !sub.is_empty() {
                    col.label(
                        RichText::new(sub)
                            .size(11.5)
                            .color(Color32::from_rgb(0xE0, 0xE8, 0xF6)),
                    );
                }
            }
        });
    }

    ui.add_space(4.0);
    ui.separator();
    ui.add_space(4.0);
}

/// Draws the 4-stop vertical gradient defined in `default.qss:40-44`:
/// - `y = 0.00`: `rgb(80, 92, 120)` (`#505C78`)
/// - `y = 0.08`: `rgb(133, 147, 174)` (`#8593AE`)
/// - `y = 0.32`: `rgb(88, 107, 140)` (`#586B8C`)
/// - `y = 1.00`: `rgb(111, 128, 170)` (`#6F80AA`)
fn paint_four_stop_banner_gradient(ui: &mut Ui, rect: Rect) {
    let stops: [(f32, Color32); 4] = [
        (0.00, Color32::from_rgb(80, 92, 120)),
        (0.08, Color32::from_rgb(133, 147, 174)),
        (0.32, Color32::from_rgb(88, 107, 140)),
        (1.00, Color32::from_rgb(111, 128, 170)),
    ];

    let mut mesh = Mesh::default();
    let h = rect.height().max(1.0);
    for (i, (t, col)) in stops.iter().enumerate() {
        let y = rect.top() + t * h;
        mesh.colored_vertex(Pos2::new(rect.left(), y), *col);
        mesh.colored_vertex(Pos2::new(rect.right(), y), *col);
        if i > 0 {
            let base = ((i - 1) * 2) as u32;
            mesh.add_triangle(base, base + 1, base + 2);
            mesh.add_triangle(base + 1, base + 3, base + 2);
        }
    }
    ui.painter().add(Shape::mesh(mesh));
}

/// Renders a placeholder when no server connection is active.
fn render_disconnected_placeholder(
    ui: &mut Ui,
    server_addr: &str,
    palette: &Ts3Palette,
    assets: &Ts3Assets,
) {
    let icon = assets
        .get_icon("SERVER_GREEN")
        .or_else(|| assets.get_icon("DISCONNECT"));
    let sub = if server_addr.is_empty() {
        "Not connected to a server"
    } else {
        server_addr
    };
    render_gradient_banner(ui, icon, "🌐", "TeamSpeak 3", Some(sub), None);

    ui.label(
        RichText::new("Disconnected. Use Connections → Connect (Ctrl+S) to join a server.")
            .color(palette.text_muted)
            .italics(),
    );
}

/// 1:1 `serverinfo.tpl` view when `SelectedTreeNode::ServerRoot` is selected.
#[allow(clippy::too_many_arguments)]
fn render_server_info(
    ui: &mut Ui,
    server_addr: &str,
    my_clid: u16,
    rtt_ms: u32,
    packet_loss: f32,
    server: &VirtualServerInfo,
    channels: &[ChannelInfo],
    clients: &[ClientInfo],
    palette: &Ts3Palette,
    assets: &Ts3Assets,
) {
    let server_name = if server.name.is_empty() {
        "Virtual Server"
    } else {
        server.name.as_str()
    };
    let my_nick = clients
        .iter()
        .find(|c| c.clid == my_clid)
        .map(|c| format!("Connected as {}", c.nickname))
        .unwrap_or_else(|| {
            if server_addr.is_empty() {
                String::new()
            } else {
                format!("Address: {server_addr}")
            }
        });

    let server_icon = assets
        .get_icon("SERVER_GREEN")
        .or_else(|| assets.get_icon("CONNECT"));
    render_gradient_banner(
        ui,
        server_icon,
        "🖥",
        server_name,
        Some(my_nick.as_str()),
        None,
    );

    let highlight_col = highlight_color(palette);
    let active_col = active_color(palette);

    Grid::new("serverinfo_tpl_grid")
        .num_columns(2)
        .spacing([12.0, 5.0])
        .striped(false)
        .show(ui, |ui| {
            // Server Name
            kv_label(ui, "Server Name:", palette);
            ui.label(
                RichText::new(server_name)
                    .strong()
                    .color(title_link_color(palette)),
            );
            ui.end_row();

            // Server Address
            kv_label(ui, "Server Address:", palette);
            let addr_display = if server_addr.is_empty() {
                "—"
            } else {
                server_addr
            };
            ui.label(RichText::new(addr_display).color(palette.text_primary));
            ui.end_row();

            // Version (server.version on server.platform with OS icon)
            kv_label(ui, "Version:", palette);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                let ver = if server.version.is_empty() {
                    "Unknown"
                } else {
                    server.version.as_str()
                };
                let plat = if server.platform.is_empty() {
                    "Unknown"
                } else {
                    server.platform.as_str()
                };
                ui.label(
                    RichText::new(format!(
                        "{ver} on {}",
                        platform_icon_prefix(plat, plat)
                    ))
                    .color(palette.text_primary),
                );
            });
            ui.end_row();

            // License
            kv_label(ui, "License:", palette);
            let license = infer_server_license(server.max_clients);
            ui.label(RichText::new(license).color(highlight_col));
            ui.end_row();

            // Uptime
            kv_label(ui, "Uptime:", palette);
            ui.label(RichText::new(format_uptime(server.uptime)).color(palette.text_primary));
            ui.end_row();

            // Current Channels
            kv_label(ui, "Current Channels:", palette);
            let ch_count = if !channels.is_empty() {
                channels.len() as u32
            } else {
                server.channels_online
            };
            ui.label(RichText::new(ch_count.to_string()).color(palette.text_primary));
            ui.end_row();

            // Current Clients
            kv_label(ui, "Current Clients:", palette);
            let cl_count = if !clients.is_empty() {
                clients.len() as u32
            } else {
                server.clients_online
            };
            let max_cl = server.max_clients.max(cl_count);
            ui.label(
                RichText::new(format!("{cl_count} / {max_cl}")).color(palette.text_primary),
            );
            ui.end_row();

            // Current Ping
            kv_label(ui, "Current Ping:", palette);
            let ping_col = if rtt_ms < 80 {
                active_col
            } else if rtt_ms < 160 {
                highlight_col
            } else {
                important_color(palette)
            };
            ui.label(
                RichText::new(format!("{rtt_ms} ms ± {packet_loss:.1}%")).color(ping_col),
            );
            ui.end_row();

            // Server Unique ID (with Copy button)
            kv_label(ui, "Server Unique ID:", palette);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                let uid_str = if server.uid.is_empty() {
                    "—"
                } else {
                    server.uid.as_str()
                };
                ui.label(
                    RichText::new(uid_str)
                        .monospace()
                        .size(11.5)
                        .color(palette.text_primary),
                );
                if !server.uid.is_empty()
                    && ui
                        .small_button("📋 Copy")
                        .on_hover_text("Copy Server Unique ID to clipboard")
                        .clicked()
                {
                    ui.ctx().copy_text(server.uid.clone());
                }
            });
            ui.end_row();
        });

    // Welcome Message section (`serverinfo.tpl` `%%?SERVER_WELCOMEMESSAGE%%`)
    if !server.welcome_message.is_empty() {
        ui.add_space(10.0);
        ui.separator();
        ui.add_space(4.0);
        ui.label(
            RichText::new("Welcome Message:")
                .strong()
                .color(palette.text_primary),
        );
        ui.add_space(3.0);
        Frame::NONE
            .fill(palette.alternate_base_bg)
            .stroke(Stroke::new(1.0_f32, palette.border_color))
            .corner_radius(CornerRadius::same(3))
            .inner_margin(Margin::same(8))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                render_bbcode_rich_text(ui, &server.welcome_message, palette);
            });
    }
}

/// 1:1 `channelinfo.tpl` view when `SelectedTreeNode::Channel(cid)` is selected.
fn render_channel_info(
    ui: &mut Ui,
    channel: &ChannelInfo,
    clients: &[ClientInfo],
    palette: &Ts3Palette,
    assets: &Ts3Assets,
) {
    let channel_clients_count = clients.iter().filter(|c| c.cid == channel.cid).count() as u32;
    let effective_count = channel_clients_count.max(channel.total_clients);

    let icon_key = select_channel_icon_key(channel, effective_count);
    let icon = assets.get_icon(icon_key);

    let subtitle = if channel.topic.is_empty() {
        None
    } else {
        Some(channel.topic.as_str())
    };
    let badge = if channel.is_default {
        Some(("Default", Color32::from_rgb(0x2F, 0x71, 0xAF)))
    } else if channel.is_password {
        Some(("Protected", Color32::from_rgb(0xB8, 0x86, 0x0B)))
    } else {
        None
    };

    render_gradient_banner(ui, icon, "📁", &channel.name, subtitle, badge);

    let highlight_col = highlight_color(palette);
    let active_col = active_color(palette);
    let important_col = important_color(palette);

    Grid::new("channelinfo_tpl_grid")
        .num_columns(2)
        .spacing([12.0, 5.0])
        .striped(false)
        .show(ui, |ui| {
            // Channel Name
            kv_label(ui, "Channel Name:", palette);
            ui.label(
                RichText::new(&channel.name)
                    .strong()
                    .color(title_link_color(palette)),
            );
            ui.end_row();

            // Topic
            kv_label(ui, "Topic:", palette);
            if channel.topic.is_empty() {
                ui.label(RichText::new("None").italics().color(palette.text_muted));
            } else {
                ui.label(RichText::new(&channel.topic).color(palette.text_primary));
            }
            ui.end_row();

            // Type (Permanent / Semi-Permanent / Temporary + Default Channel / Password-protected)
            kv_label(ui, "Type:", palette);
            let type_str = format_channel_type_flags(channel);
            ui.label(RichText::new(type_str).color(highlight_col));
            ui.end_row();

            // Codec (Opus Voice (4) / Opus Music (5) + quality bar codec_quality / 10 + estimated bitrate)
            kv_label(ui, "Codec:", palette);
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 3.0;
                let bitrate = estimate_codec_bitrate_kibs(channel.codec, channel.codec_quality);
                ui.horizontal(|ui| {
                    if channel.codec == 5 {
                        if let Some(music_tex) = assets.get_icon("MUSIC") {
                            ui.add(
                                egui::Image::new(ImageSource::Texture(
                                    egui::load::SizedTexture::from_handle(music_tex),
                                ))
                                .fit_to_exact_size(Vec2::splat(14.0)),
                            );
                        }
                    }
                    ui.label(
                        RichText::new(codec_name(channel.codec))
                            .strong()
                            .color(palette.text_primary),
                    );
                });
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    render_quality_bar(ui, channel.codec_quality, palette);
                    ui.label(
                        RichText::new(format!(
                            "Quality: {} / 10 (~{bitrate:.2} KiB/s)",
                            channel.codec_quality.min(10)
                        ))
                        .size(11.5)
                        .color(palette.text_muted),
                    );
                });
            });
            ui.end_row();

            // Current Clients (count / Unlimited or max_clients)
            kv_label(ui, "Current Clients:", palette);
            let max_str = if channel.max_clients < 0 {
                "Unlimited".to_string()
            } else {
                channel.max_clients.to_string()
            };
            ui.label(
                RichText::new(format!("{effective_count} / {max_str}"))
                    .color(palette.text_primary),
            );
            ui.end_row();

            // Needed Talk Power (if > 0)
            if channel.needed_talk_power > 0 {
                kv_label(ui, "Needed Talk Power:", palette);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    if let Some(mod_tex) = assets.get_icon("MODERATED") {
                        ui.add(
                            egui::Image::new(ImageSource::Texture(
                                egui::load::SizedTexture::from_handle(mod_tex),
                            ))
                            .fit_to_exact_size(Vec2::splat(14.0)),
                        );
                    }
                    ui.label(
                        RichText::new(format!("{} (Moderated)", channel.needed_talk_power))
                            .color(important_col),
                    );
                });
                ui.end_row();
            }

            // Subscription Status
            kv_label(ui, "Subscription Status:", palette);
            let (sub_text, sub_col) = if channel.subscribed {
                ("Subscribed", active_col)
            } else {
                ("Unsubscribed", palette.text_muted)
            };
            ui.label(RichText::new(sub_text).color(sub_col));
            ui.end_row();

            // Channel ID
            kv_label(ui, "Channel ID:", palette);
            ui.label(
                RichText::new(channel.cid.to_string())
                    .monospace()
                    .color(palette.text_primary),
            );
            ui.end_row();
        });
}

/// 1:1 `clientinfo.tpl` view when `SelectedTreeNode::Client(clid)` is selected.
fn render_client_info(
    ui: &mut Ui,
    client: &ClientInfo,
    channels: &[ChannelInfo],
    palette: &Ts3Palette,
    assets: &Ts3Assets,
) {
    let icon_key = select_client_icon_key(client);
    let icon = assets.get_icon(icon_key);

    let channel_name = channels
        .iter()
        .find(|c| c.cid == client.cid)
        .map(|c| format!("In channel: {}", c.name));

    let away_badge = if client.away {
        Some(("AWAY", Color32::from_rgb(0xC9, 0x7A, 0x10)))
    } else if client.talking {
        Some(("TALKING", Color32::from_rgb(0x1E, 0x88, 0xE5)))
    } else {
        None
    };

    render_gradient_banner(
        ui,
        icon,
        "👤",
        &client.nickname,
        channel_name.as_deref(),
        away_badge,
    );

    // Optional top country + status strip inside banner area if country or away is present
    if !client.country.is_empty() || client.away {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            if !client.country.is_empty() {
                if let Some(flag_tex) = assets.get_flag(&client.country) {
                    ui.add(
                        egui::Image::new(ImageSource::Texture(
                            egui::load::SizedTexture::from_handle(flag_tex),
                        ))
                        .fit_to_exact_size(Vec2::new(16.0, 11.0)),
                    );
                }
                ui.label(
                    RichText::new(format!(
                        "{} ({})",
                        country_display_name(&client.country),
                        client.country.to_uppercase()
                    ))
                    .size(12.0)
                    .color(palette.text_muted),
                );
            }
            if client.away {
                let away_label = if client.away_message.is_empty() {
                    "Away".to_string()
                } else {
                    format!("Away: {}", client.away_message)
                };
                Frame::NONE
                    .fill(Color32::from_rgb(0xC9, 0x7A, 0x10))
                    .corner_radius(CornerRadius::same(3))
                    .inner_margin(Margin::symmetric(6, 1))
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new(away_label)
                                .size(11.0)
                                .strong()
                                .color(Color32::WHITE),
                        );
                    });
            }
        });
        ui.add_space(4.0);
    }

    Grid::new("clientinfo_tpl_grid")
        .num_columns(2)
        .spacing([12.0, 5.0])
        .striped(false)
        .show(ui, |ui| {
            // Nickname
            kv_label(ui, "Nickname:", palette);
            ui.label(
                RichText::new(&client.nickname)
                    .strong()
                    .color(title_link_color(palette)),
            );
            ui.end_row();

            // Unique ID (with Copy button)
            kv_label(ui, "Unique ID:", palette);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                let uid_str = if client.uid.is_empty() {
                    "—"
                } else {
                    client.uid.as_str()
                };
                ui.label(
                    RichText::new(uid_str)
                        .monospace()
                        .size(11.5)
                        .color(palette.text_primary),
                );
                if !client.uid.is_empty()
                    && ui
                        .small_button("📋 Copy")
                        .on_hover_text("Copy Client Unique ID to clipboard")
                        .clicked()
                {
                    ui.ctx().copy_text(client.uid.clone());
                }
            });
            ui.end_row();

            // Database ID
            kv_label(ui, "Database ID:", palette);
            ui.label(
                RichText::new(client.dbid.to_string())
                    .monospace()
                    .color(palette.text_primary),
            );
            ui.end_row();

            // Client ID
            kv_label(ui, "Client ID:", palette);
            ui.label(
                RichText::new(client.clid.to_string())
                    .monospace()
                    .color(palette.text_primary),
            );
            ui.end_row();

            // Version (client.version on client.platform)
            kv_label(ui, "Version:", palette);
            let ver = if client.version.is_empty() {
                "Unknown"
            } else {
                client.version.as_str()
            };
            let plat = if client.platform.is_empty() {
                "Unknown"
            } else {
                client.platform.as_str()
            };
            ui.label(
                RichText::new(format!("{ver} on {}", platform_icon_prefix(plat, plat)))
                    .color(palette.text_primary),
            );
            ui.end_row();

            // Country
            kv_label(ui, "Country:", palette);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 5.0;
                if client.country.is_empty() {
                    ui.label(RichText::new("Unknown").italics().color(palette.text_muted));
                } else {
                    if let Some(flag_tex) = assets.get_flag(&client.country) {
                        ui.add(
                            egui::Image::new(ImageSource::Texture(
                                egui::load::SizedTexture::from_handle(flag_tex),
                            ))
                            .fit_to_exact_size(Vec2::new(16.0, 11.0)),
                        );
                    }
                    ui.label(
                        RichText::new(format!(
                            "{} ({})",
                            country_display_name(&client.country),
                            client.country.to_uppercase()
                        ))
                        .color(palette.text_primary),
                    );
                }
            });
            ui.end_row();

            // Server Groups (badges for client.server_groups)
            kv_label(ui, "Server Groups:", palette);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(4.0, 4.0);
                if client.server_groups.is_empty() {
                    ui.label(RichText::new("None").italics().color(palette.text_muted));
                } else {
                    for &sgid in &client.server_groups {
                        render_server_group_badge(ui, sgid, palette, assets);
                    }
                }
            });
            ui.end_row();

            // Channel Group (client.channel_group_id)
            kv_label(ui, "Channel Group:", palette);
            render_channel_group_badge(ui, client.channel_group_id, palette, assets);
            ui.end_row();

            // Talk Power (client.talk_power)
            kv_label(ui, "Talk Power:", palette);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                ui.label(
                    RichText::new(client.talk_power.to_string()).color(palette.text_primary),
                );
                if client.is_talker {
                    ui.label(
                        RichText::new("(Priority Speaker / Talker)")
                            .size(11.5)
                            .color(highlight_color(palette)),
                    );
                }
                if client.is_channel_commander {
                    ui.label(
                        RichText::new("[Channel Commander]")
                            .size(11.5)
                            .strong()
                            .color(Color32::from_rgb(0xD9, 0x77, 0x06)),
                    );
                }
            });
            ui.end_row();

            // Status (Online / Away: <msg> / Microphone Muted / Sound Muted)
            kv_label(ui, "Status:", palette);
            let (status_str, status_col) = format_client_status(client, palette);
            ui.label(RichText::new(status_str).strong().color(status_col));
            ui.end_row();
        });
}

fn kv_label(ui: &mut Ui, text: &str, palette: &Ts3Palette) {
    ui.label(RichText::new(text).strong().color(palette.text_primary));
}

fn title_link_color(palette: &Ts3Palette) -> Color32 {
    match palette.mode {
        // `.InfoFrame_Title ... Links { color: midnightblue; }` in light mode, `lightsteelblue` in dark mode
        ThemeMode::Light => Color32::from_rgb(0x19, 0x19, 0x70),
        ThemeMode::Dark => Color32::from_rgb(0xB0, 0xC4, 0xDE),
    }
}

fn highlight_color(palette: &Ts3Palette) -> Color32 {
    // `.Highlight { color: palette(link); }` (`#1C82CC`)
    match palette.mode {
        ThemeMode::Light => Color32::from_rgb(0x1C, 0x82, 0xCC),
        ThemeMode::Dark => Color32::from_rgb(0x5C, 0xAC, 0xEE),
    }
}

fn active_color(palette: &Ts3Palette) -> Color32 {
    // `.Active { color: green; }`
    match palette.mode {
        ThemeMode::Light => Color32::from_rgb(0x00, 0x80, 0x00),
        ThemeMode::Dark => Color32::from_rgb(0x66, 0xCD, 0x00),
    }
}

fn important_color(palette: &Ts3Palette) -> Color32 {
    // `.Important { color: red; }`
    match palette.mode {
        ThemeMode::Light => Color32::from_rgb(0xC9, 0x07, 0x0A),
        ThemeMode::Dark => Color32::from_rgb(0xFF, 0x55, 0x55),
    }
}

fn platform_icon_prefix(platform: &str, label: &str) -> String {
    let lower = platform.to_ascii_lowercase();
    let glyph = if lower.contains("win") {
        "🪟"
    } else if lower.contains("linux") {
        "🐧"
    } else if lower.contains("mac") || lower.contains("os x") || lower.contains("darwin") {
        "🍎"
    } else if lower.contains("freebsd") || lower.contains("bsd") {
        "😈"
    } else if lower.contains("android") || lower.contains("ios") {
        "📱"
    } else {
        "💻"
    };
    format!("{glyph} {label}")
}

fn infer_server_license(max_clients: u32) -> &'static str {
    if max_clients <= 32 {
        "No License (Default 32 Slots)"
    } else if max_clients <= 512 {
        "Non-Profit License (NPL) / Gamer License"
    } else {
        "Authorized TeamSpeak Host Provider (ATHP)"
    }
}

fn select_channel_icon_key(ch: &ChannelInfo, current_clients: u32) -> &'static str {
    let is_full = ch.max_clients >= 0 && current_clients >= ch.max_clients as u32;
    if is_full {
        if ch.subscribed {
            "CHANNEL_RED_SUBSCRIBED"
        } else {
            "CHANNEL_RED"
        }
    } else if ch.is_password {
        if ch.subscribed {
            "CHANNEL_YELLOW_SUBSCRIBED"
        } else {
            "CHANNEL_YELLOW"
        }
    } else if ch.subscribed {
        "CHANNEL_GREEN_SUBSCRIBED"
    } else {
        "CHANNEL_GREEN"
    }
}

fn select_client_icon_key(cl: &ClientInfo) -> &'static str {
    if cl.away {
        "AWAY"
    } else if !cl.output_hardware {
        "HARDWARE_OUTPUT_MUTED"
    } else if cl.output_muted {
        "OUTPUT_MUTED"
    } else if !cl.input_hardware {
        "HARDWARE_INPUT_MUTED"
    } else if cl.input_muted {
        "INPUT_MUTED"
    } else if cl.talking {
        if cl.is_channel_commander {
            "PLAYER_COMMANDER_ON"
        } else {
            "PLAYER_ON"
        }
    } else if cl.is_channel_commander {
        "PLAYER_COMMANDER_OFF"
    } else {
        "PLAYER_OFF"
    }
}

fn format_channel_type_flags(ch: &ChannelInfo) -> String {
    let mut parts = Vec::new();
    if ch.is_permanent {
        parts.push("Permanent");
    } else if ch.is_semi_permanent {
        parts.push("Semi-Permanent");
    } else {
        parts.push("Temporary");
    }
    if ch.is_default {
        parts.push("Default Channel");
    }
    if ch.is_password {
        parts.push("Password-protected");
    }
    if ch.needed_talk_power > 0 {
        parts.push("Moderated");
    }
    parts.join(", ")
}

fn render_quality_bar(ui: &mut Ui, quality: u8, palette: &Ts3Palette) {
    let q = quality.min(10);
    let desired = Vec2::new(90.0, 10.0);
    let (rect, _) = ui.allocate_exact_size(desired, egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(2), palette.alternate_base_bg);
        let fill_w = (rect.width() * (q as f32 / 10.0)).clamp(0.0, rect.width());
        if fill_w > 0.0 {
            let fill_rect = Rect::from_min_size(rect.min, Vec2::new(fill_w, rect.height()));
            let bar_col = if q >= 7 {
                active_color(palette)
            } else if q >= 4 {
                palette.accent_blue
            } else {
                Color32::from_rgb(0xD9, 0x77, 0x06)
            };
            ui.painter()
                .rect_filled(fill_rect, CornerRadius::same(2), bar_col);
        }
        ui.painter().rect_stroke(
            rect,
            CornerRadius::same(2),
            Stroke::new(1.0_f32, palette.border_color),
            StrokeKind::Inside,
        );
    }
}

fn render_server_group_badge(ui: &mut Ui, sgid: u64, palette: &Ts3Palette, assets: &Ts3Assets) {
    let (name, icon_key) = match sgid {
        2 => ("Admin Server Query (2)".to_string(), Some("GROUP_300")),
        6 => ("Server Admin (6)".to_string(), Some("GROUP_300")),
        7 => ("Normal (7)".to_string(), Some("GROUP_500")),
        8 => ("Guest (8)".to_string(), None),
        other => (format!("Group #{other}"), Some("GROUP_500")),
    };

    Frame::NONE
        .fill(palette.alternate_base_bg)
        .stroke(Stroke::new(1.0_f32, palette.border_color))
        .corner_radius(CornerRadius::same(3))
        .inner_margin(Margin::symmetric(6, 2))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                if let Some(key) = icon_key {
                    if let Some(tex) = assets.get_icon(key) {
                        ui.add(
                            egui::Image::new(ImageSource::Texture(
                                egui::load::SizedTexture::from_handle(tex),
                            ))
                            .fit_to_exact_size(Vec2::splat(13.0)),
                        );
                    }
                }
                ui.label(RichText::new(name).size(11.5).color(palette.text_primary));
            });
        });
}

fn render_channel_group_badge(ui: &mut Ui, cgid: u64, palette: &Ts3Palette, assets: &Ts3Assets) {
    let (name, icon_key) = match cgid {
        5 => ("Channel Admin (5)".to_string(), Some("GROUP_100")),
        6 => ("Operator (6)".to_string(), Some("GROUP_200")),
        7 => ("Voice (7)".to_string(), Some("GROUP_600")),
        8 => ("Guest (8)".to_string(), None),
        0 => ("Default (0)".to_string(), None),
        other => (format!("Channel Group #{other}"), None),
    };

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        if let Some(key) = icon_key {
            if let Some(tex) = assets.get_icon(key) {
                ui.add(
                    egui::Image::new(ImageSource::Texture(egui::load::SizedTexture::from_handle(
                        tex,
                    )))
                    .fit_to_exact_size(Vec2::splat(14.0)),
                );
            }
        }
        ui.label(RichText::new(name).color(palette.text_primary));
    });
}

fn format_client_status(client: &ClientInfo, palette: &Ts3Palette) -> (String, Color32) {
    let mut statuses = Vec::new();
    let mut primary_col = active_color(palette);

    if client.away {
        if client.away_message.is_empty() {
            statuses.push("Away".to_string());
        } else {
            statuses.push(format!("Away: {}", client.away_message));
        }
        primary_col = Color32::from_rgb(0xC9, 0x7A, 0x10);
    }
    if client.output_muted || !client.output_hardware {
        statuses.push("Sound Muted".to_string());
        primary_col = important_color(palette);
    }
    if client.input_muted || !client.input_hardware {
        statuses.push("Microphone Muted".to_string());
        if !client.output_muted {
            primary_col = Color32::from_rgb(0xD9, 0x77, 0x06);
        }
    }

    if statuses.is_empty() {
        if client.talking {
            ("Online (Talking)".to_string(), highlight_color(palette))
        } else {
            ("Online".to_string(), primary_col)
        }
    } else {
        (statuses.join(" / "), primary_col)
    }
}

fn country_display_name(code: &str) -> &str {
    match code.to_ascii_uppercase().as_str() {
        "US" => "United States",
        "GB" | "UK" => "United Kingdom",
        "DE" => "Germany",
        "FR" => "France",
        "ES" => "Spain",
        "IT" => "Italy",
        "NL" => "Netherlands",
        "PL" => "Poland",
        "SE" => "Sweden",
        "NO" => "Norway",
        "FI" => "Finland",
        "DK" => "Denmark",
        "CA" => "Canada",
        "AU" => "Australia",
        "BR" => "Brazil",
        "AR" => "Argentina",
        "CL" => "Chile",
        "MX" => "Mexico",
        "JP" => "Japan",
        "KR" => "South Korea",
        "CN" => "China",
        "IN" => "India",
        "RU" => "Russia",
        "UA" => "Ukraine",
        "CH" => "Switzerland",
        "AT" => "Austria",
        "BE" => "Belgium",
        "PT" => "Portugal",
        "CZ" => "Czechia",
        _ => code,
    }
}

/// Renders basic TeamSpeak 3 BBCode (`[b]`, `[i]`, `[u]`, `[color=#RRGGBB]`, `[url=...]`)
/// for the server welcome message (`docs/re/gui/04-info-templates.md` Section 5.3).
fn render_bbcode_rich_text(ui: &mut Ui, raw: &str, palette: &Ts3Palette) {
    let welcome_col = match palette.mode {
        // `.TextMessage_Welcome { color: #0AA537; }` in light, `#11de4d` in dark (`03-qss-themes.md:75,118`)
        ThemeMode::Light => Color32::from_rgb(0x0A, 0xA5, 0x37),
        ThemeMode::Dark => Color32::from_rgb(0x11, 0xDE, 0x4D),
    };
    let stripped = strip_basic_bbcode(raw);
    ui.label(RichText::new(stripped).color(welcome_col));
}

fn strip_basic_bbcode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'[' {
            if let Some(rel_end) = input[i..].find(']') {
                let tag = &input[i + 1..i + rel_end];
                let tag_lower = tag.to_ascii_lowercase();
                let is_known = matches!(
                    tag_lower.as_str(),
                    "b" | "/b" | "i" | "/i" | "u" | "/u" | "/color" | "/size" | "/url" | "img"
                        | "/img"
                ) || tag_lower.starts_with("color=")
                    || tag_lower.starts_with("size=")
                    || tag_lower.starts_with("url=");
                if is_known {
                    i += rel_end + 1;
                    continue;
                }
            }
        }
        if let Some(ch) = input[i..].chars().next() {
            out.push(ch);
            i += ch.len_utf8();
        } else {
            break;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_uptime() {
        assert_eq!(format_uptime(0), "0s");
        assert_eq!(format_uptime(45), "45s");
        assert_eq!(format_uptime(125), "2m 5s");
        assert_eq!(format_uptime(3_661), "1h 1m 1s");
        assert_eq!(format_uptime(86_400), "1d 0h 0m 0s");
        assert_eq!(format_uptime(90_061), "1d 1h 1m 1s");
        assert_eq!(format_uptime(1_217_532), "14d 2h 12m 12s");
    }

    #[test]
    fn test_codec_name() {
        assert_eq!(codec_name(0), "Speex Narrowband (0)");
        assert_eq!(codec_name(1), "Speex Wideband (1)");
        assert_eq!(codec_name(2), "Speex Ultra-Wideband (2)");
        assert_eq!(codec_name(3), "CELT Mono (3)");
        assert_eq!(codec_name(4), "Opus Voice (4)");
        assert_eq!(codec_name(5), "Opus Music (5)");
        assert_eq!(codec_name(99), "Unknown Codec");
    }

    #[test]
    fn test_strip_basic_bbcode() {
        let raw = "[b]Welcome[/b] to [color=#0055ff]Los Retrasados[/color]!";
        assert_eq!(strip_basic_bbcode(raw), "Welcome to Los Retrasados!");
        // Preserve channel spacer brackets that are not BBCode tags
        let spacer = "[cspacer0] --- Lobby ---";
        assert_eq!(strip_basic_bbcode(spacer), "[cspacer0] --- Lobby ---");
    }

    #[test]
    fn test_channel_type_flags_formatting() {
        let mut ch = ChannelInfo::default();
        assert_eq!(format_channel_type_flags(&ch), "Temporary");

        ch.is_permanent = true;
        ch.is_default = true;
        assert_eq!(
            format_channel_type_flags(&ch),
            "Permanent, Default Channel"
        );

        ch.is_permanent = false;
        ch.is_semi_permanent = true;
        ch.is_default = false;
        ch.is_password = true;
        ch.needed_talk_power = 25;
        assert_eq!(
            format_channel_type_flags(&ch),
            "Semi-Permanent, Password-protected, Moderated"
        );
    }
}

