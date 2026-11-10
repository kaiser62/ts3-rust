// Owned by Pool Agent 8 (arthoroy29a@gmail.com)
// 1:1 modal dialogs (`ConnectDialog`, `BookmarksManager`, `IdentityDialog`, `CreateChannelDialog`,
// `PokeDialog`, `OptionsDialog`, `AboutDialog`) from `docs/re/gui/06-dialogs-menus.md`.

use egui::{Align, Color32, Layout, RichText, ScrollArea, Vec2};
use ts3_proto::identity::{IdentityData, VersionSign};

use crate::assets::Ts3Assets;
use crate::menus_toolbar::UiCommand;
use crate::theme::{ThemeMode, Ts3Palette};

#[derive(Debug, Clone)]
pub struct BookmarkEntry {
    pub label: String,
    pub server_addr: String,
    pub nickname: String,
    pub server_password: String,
    pub default_channel: String,
    pub auto_connect: bool,
}

#[derive(Debug, Clone)]
pub struct DialogsState {
    pub show_connect: bool,
    pub connect_more_expanded: bool,
    pub connect_addr: String,
    pub connect_password: String,
    pub connect_nickname: String,
    pub connect_default_channel: String,
    pub connect_channel_password: String,
    pub connect_in_new_tab: bool,

    pub show_bookmarks: bool,
    pub bookmarks: Vec<BookmarkEntry>,
    pub selected_bookmark: usize,

    pub show_identities: bool,
    pub identity: IdentityData,
    pub identity_target_level: u32,
    pub identity_export_str: String,
    pub identity_import_str: String,
    pub identity_status_msg: String,

    pub show_create_channel: bool,
    pub create_channel_tab: usize,
    pub create_channel_parent_cid: u64,
    pub create_channel_name: String,
    pub create_channel_password: String,
    pub create_channel_topic: String,
    pub create_channel_description: String,
    pub create_channel_codec: u8,
    pub create_channel_quality: u8,
    pub create_channel_type: u8,
    pub create_channel_default: bool,
    pub create_channel_needed_talk_power: i32,
    pub create_channel_max_clients: i32,
    pub create_channel_max_clients_limited: bool,
    pub create_channel_delete_delay: u32,
    pub create_channel_encrypted: bool,

    pub show_poke: bool,
    pub poke_target_clid: u16,
    pub poke_target_nick: String,
    pub poke_message: String,

    pub show_received_poke: bool,
    pub received_poke_server: String,
    pub received_poke_invoker: String,
    pub received_poke_message: String,
    pub received_poke_timestamp: String,

    pub show_change_nickname: bool,
    pub change_nickname_input: String,

    pub show_options: bool,
    pub options_category: usize,
    pub theme_mode: ThemeMode,
    pub sound_enabled: bool,
    pub voice_activation_threshold_db: f32,
    pub warn_on_close_connected: bool,
    pub disable_poke_dialog: bool,
    pub show_country_flags: bool,
    pub show_server_query_clients: bool,
    pub master_volume_db: f32,
    pub comfort_noise_enabled: bool,
    pub echo_cancellation: bool,
    pub noise_reduction: bool,
    pub max_chat_lines: u32,
    pub log_channel_chat: bool,
    pub log_server_chat: bool,
    pub block_whispers: bool,

    pub show_about: bool,
}

impl Default for DialogsState {
    fn default() -> Self {
        let identity = IdentityData::generate_with_level(8);
        let identity_export_str = identity.export_ts_identity();
        Self {
            show_connect: false,
            connect_more_expanded: false,
            connect_addr: "192.168.1.140:9987".to_string(),
            connect_password: String::new(),
            connect_nickname: "TeamSpeakUser".to_string(),
            connect_default_channel: String::new(),
            connect_channel_password: String::new(),
            connect_in_new_tab: false,

            show_bookmarks: false,
            bookmarks: vec![BookmarkEntry {
                label: "Los Retrasados Mk.4 (Local Test Server)".to_string(),
                server_addr: "192.168.1.140:9987".to_string(),
                nickname: "TeamSpeakUser".to_string(),
                server_password: String::new(),
                default_channel: String::new(),
                auto_connect: false,
            }],
            selected_bookmark: 0,

            show_identities: false,
            identity,
            identity_target_level: 10,
            identity_export_str,
            identity_import_str: String::new(),
            identity_status_msg: String::new(),

            show_create_channel: false,
            create_channel_tab: 0,
            create_channel_parent_cid: 0,
            create_channel_name: String::new(),
            create_channel_password: String::new(),
            create_channel_topic: String::new(),
            create_channel_description: String::new(),
            create_channel_codec: 4,
            create_channel_quality: 6,
            create_channel_type: 0,
            create_channel_default: false,
            create_channel_needed_talk_power: 0,
            create_channel_max_clients: -1,
            create_channel_max_clients_limited: false,
            create_channel_delete_delay: 0,
            create_channel_encrypted: true,

            show_poke: false,
            poke_target_clid: 0,
            poke_target_nick: String::new(),
            poke_message: String::new(),

            show_received_poke: false,
            received_poke_server: String::new(),
            received_poke_invoker: String::new(),
            received_poke_message: String::new(),
            received_poke_timestamp: String::new(),

            show_change_nickname: false,
            change_nickname_input: "TeamSpeakUser".to_string(),

            show_options: false,
            options_category: 0,
            theme_mode: ThemeMode::Light,
            sound_enabled: true,
            voice_activation_threshold_db: -40.0,
            warn_on_close_connected: true,
            disable_poke_dialog: false,
            show_country_flags: true,
            show_server_query_clients: false,
            master_volume_db: 0.0,
            comfort_noise_enabled: false,
            echo_cancellation: true,
            noise_reduction: true,
            max_chat_lines: 500,
            log_channel_chat: true,
            log_server_chat: true,
            block_whispers: false,

            show_about: false,
        }
    }
}

pub enum DialogAction {
    Connect {
        server_addr: String,
        nickname: String,
        server_password: Option<String>,
        default_channel: Option<String>,
        default_channel_password: Option<String>,
        in_new_tab: bool,
    },
    CreateChannel {
        parent_cid: u64,
        name: String,
        password: Option<String>,
        topic: String,
        description: String,
        codec: u8,
        codec_quality: u8,
        is_permanent: bool,
        is_semi_permanent: bool,
    },
    PokeClient {
        clid: u16,
        message: String,
    },
    Ui(UiCommand),
}

pub const OPTIONS_CATEGORIES: &[&str] = &[
    "Application",
    "Design",
    "Security",
    "Playback",
    "Capture",
    "Chat",
    "Whisper",
    "Hotkeys",
    "Messages",
    "Downloads",
    "Notifications",
    "ServerQuery",
    "Addons",
];

pub fn render_dialogs(
    ctx: &egui::Context,
    dialogs: &mut DialogsState,
    palette: &Ts3Palette,
    assets: &Ts3Assets,
    actions: &mut Vec<DialogAction>,
) {
    if dialogs.show_connect {
        render_connect_dialog(ctx, dialogs, palette, assets, actions);
    }
    if dialogs.show_bookmarks {
        render_bookmarks_dialog(ctx, dialogs, palette, assets, actions);
    }
    if dialogs.show_identities {
        render_identities_dialog(ctx, dialogs, palette, assets);
    }
    if dialogs.show_create_channel {
        render_create_channel_dialog(ctx, dialogs, palette, assets, actions);
    }
    if dialogs.show_poke {
        render_poke_dialog(ctx, dialogs, palette, actions);
    }
    if dialogs.show_received_poke && !dialogs.disable_poke_dialog {
        render_received_poke_dialog(ctx, dialogs, palette);
    }
    if dialogs.show_change_nickname {
        render_change_nickname_dialog(ctx, dialogs, actions);
    }
    if dialogs.show_options {
        render_options_dialog(ctx, dialogs, palette, actions);
    }
    if dialogs.show_about {
        render_about_dialog(ctx, dialogs, palette, assets);
    }
}

fn render_connect_dialog(
    ctx: &egui::Context,
    dialogs: &mut DialogsState,
    palette: &Ts3Palette,
    assets: &Ts3Assets,
    actions: &mut Vec<DialogAction>,
) {
    let mut open = dialogs.show_connect;
    let mut close_requested = false;

    egui::Window::new("Connect")
        .id(egui::Id::new("ts3_connect_dialog"))
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .default_width(380.0)
        .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
        .show(ctx, |ui| {
            ui.spacing_mut().item_spacing = Vec2::new(8.0, 6.0);

            egui::Grid::new("connect_dialog_main_grid")
                .num_columns(2)
                .spacing([10.0, 6.0])
                .striped(false)
                .show(ui, |ui| {
                    ui.label("Server Nickname or Address:");
                    ui.add(
                        egui::TextEdit::singleline(&mut dialogs.connect_addr)
                            .desired_width(230.0)
                            .hint_text("192.168.1.140:9987"),
                    );
                    ui.end_row();

                    ui.label("Server Password:");
                    ui.add(
                        egui::TextEdit::singleline(&mut dialogs.connect_password)
                            .password(true)
                            .desired_width(230.0),
                    );
                    ui.end_row();

                    ui.label("Nickname:");
                    ui.vertical(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut dialogs.connect_nickname)
                                .desired_width(230.0),
                        );
                        if dialogs.connect_nickname.trim().len() < 3 {
                            ui.colored_label(
                                palette.chat_error,
                                RichText::new("[Too Short - min 3 chars]").small(),
                            );
                        }
                    });
                    ui.end_row();
                });

            if dialogs.connect_more_expanded {
                ui.separator();
                egui::Grid::new("connect_dialog_advanced_grid")
                    .num_columns(2)
                    .spacing([10.0, 6.0])
                    .show(ui, |ui| {
                        ui.label("Default Channel:");
                        ui.add(
                            egui::TextEdit::singleline(&mut dialogs.connect_default_channel)
                                .desired_width(230.0)
                                .hint_text("Optional channel name or /path"),
                        );
                        ui.end_row();

                        ui.label("Channel Password:");
                        ui.add(
                            egui::TextEdit::singleline(&mut dialogs.connect_channel_password)
                                .password(true)
                                .desired_width(230.0),
                        );
                        ui.end_row();

                        ui.label("Identity:");
                        let uid_preview = dialogs.identity.uid();
                        let sec_lvl = dialogs.identity.security_level();
                        egui::ComboBox::from_id_salt("connect_identity_combo")
                            .width(230.0)
                            .selected_text(format!("Default (Level {sec_lvl}, {uid_preview})"))
                            .show_ui(ui, |ui| {
                                let _ = ui.selectable_label(
                                    true,
                                    format!("Default (Level {sec_lvl}, {uid_preview})"),
                                );
                            });
                        ui.end_row();

                        ui.label("Playback Profile:");
                        egui::ComboBox::from_id_salt("connect_playback_combo")
                            .width(230.0)
                            .selected_text("Default")
                            .show_ui(ui, |ui| {
                                let _ = ui.selectable_label(true, "Default");
                            });
                        ui.end_row();

                        ui.label("Capture Profile:");
                        egui::ComboBox::from_id_salt("connect_capture_combo")
                            .width(230.0)
                            .selected_text("Default")
                            .show_ui(ui, |ui| {
                                let _ = ui.selectable_label(true, "Default");
                            });
                        ui.end_row();

                        ui.label("Hotkey Profile:");
                        egui::ComboBox::from_id_salt("connect_hotkey_combo")
                            .width(230.0)
                            .selected_text("Default")
                            .show_ui(ui, |ui| {
                                let _ = ui.selectable_label(true, "Default");
                            });
                        ui.end_row();
                    });
            }

            ui.separator();
            ui.horizontal(|ui| {
                let more_label = if dialogs.connect_more_expanded {
                    "Less"
                } else {
                    "More"
                };
                if ui.button(more_label).clicked() {
                    dialogs.connect_more_expanded = !dialogs.connect_more_expanded;
                }

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.button("Cancel").clicked() {
                        close_requested = true;
                    }

                    let can_connect = !dialogs.connect_addr.trim().is_empty()
                        && dialogs.connect_nickname.trim().len() >= 3;

                    if ui
                        .add_enabled(can_connect, egui::Button::new("In New Tab"))
                        .clicked()
                    {
                        let _ = assets.loaded_from_disk;
                        actions.push(build_connect_action(dialogs, true));
                        close_requested = true;
                    }

                    if ui
                        .add_enabled(can_connect, egui::Button::new("Connect"))
                        .clicked()
                    {
                        actions.push(build_connect_action(dialogs, false));
                        close_requested = true;
                    }
                });
            });
        });

    if close_requested {
        open = false;
    }
    dialogs.show_connect = open;
}

fn build_connect_action(dialogs: &DialogsState, in_new_tab: bool) -> DialogAction {
    DialogAction::Connect {
        server_addr: dialogs.connect_addr.trim().to_string(),
        nickname: dialogs.connect_nickname.trim().to_string(),
        server_password: if dialogs.connect_password.is_empty() {
            None
        } else {
            Some(dialogs.connect_password.clone())
        },
        default_channel: if dialogs.connect_default_channel.trim().is_empty() {
            None
        } else {
            Some(dialogs.connect_default_channel.trim().to_string())
        },
        default_channel_password: if dialogs.connect_channel_password.is_empty() {
            None
        } else {
            Some(dialogs.connect_channel_password.clone())
        },
        in_new_tab,
    }
}

fn render_bookmarks_dialog(
    ctx: &egui::Context,
    dialogs: &mut DialogsState,
    palette: &Ts3Palette,
    _assets: &Ts3Assets,
    actions: &mut Vec<DialogAction>,
) {
    let mut open = dialogs.show_bookmarks;
    let mut close_requested = false;

    egui::Window::new("Manage Bookmarks")
        .id(egui::Id::new("ts3_bookmarks_manager_dialog"))
        .open(&mut open)
        .collapsible(false)
        .resizable(true)
        .default_size([620.0, 360.0])
        .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
        .show(ctx, |ui| {
            ui.columns(2, |cols| {
                // Left column: Bookmark list
                cols[0].label(RichText::new("Local Bookmarks").strong());
                egui::Frame::group(cols[0].style())
                    .fill(palette.base_bg)
                    .show(&mut cols[0], |ui| {
                        ScrollArea::vertical()
                            .max_height(240.0)
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                for (idx, bm) in dialogs.bookmarks.iter().enumerate() {
                                    let is_selected = idx == dialogs.selected_bookmark;
                                    let title = if bm.label.trim().is_empty() {
                                        bm.server_addr.as_str()
                                    } else {
                                        bm.label.as_str()
                                    };
                                    let resp = ui.selectable_label(
                                        is_selected,
                                        format!("📌 {title} ({})", bm.server_addr),
                                    );
                                    if resp.clicked() {
                                        dialogs.selected_bookmark = idx;
                                    }
                                    if resp.double_clicked() {
                                        dialogs.selected_bookmark = idx;
                                        actions.push(DialogAction::Connect {
                                            server_addr: bm.server_addr.trim().to_string(),
                                            nickname: bm.nickname.trim().to_string(),
                                            server_password: if bm.server_password.is_empty() {
                                                None
                                            } else {
                                                Some(bm.server_password.clone())
                                            },
                                            default_channel: if bm
                                                .default_channel
                                                .trim()
                                                .is_empty()
                                            {
                                                None
                                            } else {
                                                Some(bm.default_channel.trim().to_string())
                                            },
                                            default_channel_password: None,
                                            in_new_tab: false,
                                        });
                                        close_requested = true;
                                    }
                                }
                            });
                    });

                // Right column: Bookmark detail form
                cols[1].label(RichText::new("Bookmark Details").strong());
                if dialogs.selected_bookmark >= dialogs.bookmarks.len()
                    && !dialogs.bookmarks.is_empty()
                {
                    dialogs.selected_bookmark = dialogs.bookmarks.len() - 1;
                }

                if let Some(bm) = dialogs.bookmarks.get_mut(dialogs.selected_bookmark) {
                    egui::Grid::new("bookmark_details_grid")
                        .num_columns(2)
                        .spacing([8.0, 6.0])
                        .show(&mut cols[1], |ui| {
                            ui.label("Label:");
                            ui.add(
                                egui::TextEdit::singleline(&mut bm.label).desired_width(180.0),
                            );
                            ui.end_row();

                            ui.label("Address:");
                            ui.add(
                                egui::TextEdit::singleline(&mut bm.server_addr)
                                    .desired_width(180.0),
                            );
                            ui.end_row();

                            ui.label("Nickname:");
                            ui.add(
                                egui::TextEdit::singleline(&mut bm.nickname).desired_width(180.0),
                            );
                            ui.end_row();

                            ui.label("Server Password:");
                            ui.add(
                                egui::TextEdit::singleline(&mut bm.server_password)
                                    .password(true)
                                    .desired_width(180.0),
                            );
                            ui.end_row();

                            ui.label("Default Channel:");
                            ui.add(
                                egui::TextEdit::singleline(&mut bm.default_channel)
                                    .desired_width(180.0),
                            );
                            ui.end_row();

                            ui.label("");
                            ui.checkbox(&mut bm.auto_connect, "Connect on Startup");
                            ui.end_row();
                        });
                } else {
                    cols[1].colored_label(palette.text_muted, "No bookmark selected.");
                }
            });

            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Add Bookmark").clicked() {
                    dialogs.bookmarks.push(BookmarkEntry {
                        label: "New Server".to_string(),
                        server_addr: "127.0.0.1:9987".to_string(),
                        nickname: dialogs.connect_nickname.clone(),
                        server_password: String::new(),
                        default_channel: String::new(),
                        auto_connect: false,
                    });
                    dialogs.selected_bookmark = dialogs.bookmarks.len() - 1;
                }

                let can_remove = !dialogs.bookmarks.is_empty();
                if ui
                    .add_enabled(can_remove, egui::Button::new("Remove"))
                    .clicked()
                {
                    if dialogs.selected_bookmark < dialogs.bookmarks.len() {
                        dialogs.bookmarks.remove(dialogs.selected_bookmark);
                        if dialogs.selected_bookmark >= dialogs.bookmarks.len()
                            && !dialogs.bookmarks.is_empty()
                        {
                            dialogs.selected_bookmark = dialogs.bookmarks.len() - 1;
                        }
                    }
                }

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.button("Close").clicked() {
                        close_requested = true;
                    }

                    let has_valid_bm = dialogs
                        .bookmarks
                        .get(dialogs.selected_bookmark)
                        .map(|b| !b.server_addr.trim().is_empty())
                        .unwrap_or(false);

                    if ui
                        .add_enabled(has_valid_bm, egui::Button::new("Connect in New Tab"))
                        .clicked()
                    {
                        if let Some(bm) = dialogs.bookmarks.get(dialogs.selected_bookmark) {
                            actions.push(DialogAction::Connect {
                                server_addr: bm.server_addr.trim().to_string(),
                                nickname: bm.nickname.trim().to_string(),
                                server_password: if bm.server_password.is_empty() {
                                    None
                                } else {
                                    Some(bm.server_password.clone())
                                },
                                default_channel: if bm.default_channel.trim().is_empty() {
                                    None
                                } else {
                                    Some(bm.default_channel.trim().to_string())
                                },
                                default_channel_password: None,
                                in_new_tab: true,
                            });
                            close_requested = true;
                        }
                    }

                    if ui
                        .add_enabled(has_valid_bm, egui::Button::new("Connect"))
                        .clicked()
                    {
                        if let Some(bm) = dialogs.bookmarks.get(dialogs.selected_bookmark) {
                            actions.push(DialogAction::Connect {
                                server_addr: bm.server_addr.trim().to_string(),
                                nickname: bm.nickname.trim().to_string(),
                                server_password: if bm.server_password.is_empty() {
                                    None
                                } else {
                                    Some(bm.server_password.clone())
                                },
                                default_channel: if bm.default_channel.trim().is_empty() {
                                    None
                                } else {
                                    Some(bm.default_channel.trim().to_string())
                                },
                                default_channel_password: None,
                                in_new_tab: false,
                            });
                            close_requested = true;
                        }
                    }
                });
            });
        });

    if close_requested {
        open = false;
    }
    dialogs.show_bookmarks = open;
}

fn render_identities_dialog(
    ctx: &egui::Context,
    dialogs: &mut DialogsState,
    palette: &Ts3Palette,
    _assets: &Ts3Assets,
) {
    let mut open = dialogs.show_identities;
    let mut close_requested = false;

    egui::Window::new("Identities (Ctrl+I)")
        .id(egui::Id::new("ts3_identities_dialog"))
        .open(&mut open)
        .collapsible(false)
        .resizable(true)
        .default_width(520.0)
        .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
        .show(ctx, |ui| {
            let mut uid = dialogs.identity.uid();
            let mut omega = dialogs.identity.public_key_omega();
            let current_level = dialogs.identity.security_level();
            let valid_offset = dialogs.identity.valid_key_offset;

            egui::Grid::new("identities_info_grid")
                .num_columns(2)
                .spacing([10.0, 6.0])
                .show(ui, |ui| {
                    ui.label("Identity Name:");
                    ui.label(RichText::new("Default Identity (NIST P-256 / secp256r1)").strong());
                    ui.end_row();

                    ui.label("Unique ID:");
                    ui.add(
                        egui::TextEdit::singleline(&mut uid)
                            .desired_width(360.0)
                            .interactive(true),
                    );
                    ui.end_row();

                    ui.label("Public Key (Omega):");
                    ui.add(
                        egui::TextEdit::singleline(&mut omega)
                            .desired_width(360.0)
                            .interactive(true),
                    );
                    ui.end_row();

                    ui.label("Security Level:");
                    let level_color = if current_level >= 20 {
                        Color32::from_rgb(0x00, 0x88, 0x00)
                    } else if current_level >= 8 {
                        palette.accent_blue
                    } else {
                        palette.chat_error
                    };
                    ui.colored_label(
                        level_color,
                        RichText::new(format!("Level {current_level}")).strong(),
                    );
                    ui.end_row();

                    ui.label("Valid Key Offset:");
                    ui.monospace(format!(
                        "{valid_offset} (checked: {})",
                        dialogs.identity.last_checked_key_offset
                    ));
                    ui.end_row();
                });

            ui.separator();
            ui.label(RichText::new("Identity Security Level (Hashcash)").strong());
            ui.horizontal(|ui| {
                ui.label("Requested Security Level:");
                ui.add(
                    egui::DragValue::new(&mut dialogs.identity_target_level)
                        .range(1..=32)
                        .speed(1),
                );
                if ui.button("Improve Security Level").clicked() {
                    let target = dialogs.identity_target_level.min(24);
                    dialogs.identity.improve_security_level(target);
                    dialogs.identity_export_str = dialogs.identity.export_ts_identity();
                    dialogs.identity_status_msg = format!(
                        "Improved security level to {} (offset {})",
                        dialogs.identity.security_level(),
                        dialogs.identity.valid_key_offset
                    );
                }
                if ui.button("Generate New").clicked() {
                    dialogs.identity = IdentityData::generate_with_level(8);
                    dialogs.identity_export_str = dialogs.identity.export_ts_identity();
                    dialogs.identity_status_msg =
                        format!("Generated new P-256 identity ({})", dialogs.identity.uid());
                }
            });

            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Export").clicked() {
                    dialogs.identity_export_str = dialogs.identity.export_ts_identity();
                    ctx.copy_text(dialogs.identity_export_str.clone());
                    dialogs.identity_status_msg =
                        "Exported obfuscated TS3 identity string & copied to clipboard.".to_string();
                }
                ui.label("Export String (Obfuscated INI format):");
            });
            if dialogs.identity_export_str.is_empty() {
                dialogs.identity_export_str = dialogs.identity.export_ts_identity();
            }
            let mut export_preview = dialogs.identity_export_str.clone();
            ui.add(
                egui::TextEdit::singleline(&mut export_preview)
                    .desired_width(f32::INFINITY)
                    .font(egui::TextStyle::Monospace),
            );

            ui.horizontal(|ui| {
                ui.label("Import:");
                ui.add(
                    egui::TextEdit::singleline(&mut dialogs.identity_import_str)
                        .desired_width(310.0)
                        .hint_text("Paste <offset>V<b64> or ASN.1 DER base64..."),
                );
                if ui.button("Import Identity").clicked() {
                    match IdentityData::import_ts_identity(&dialogs.identity_import_str) {
                        Ok(id) => {
                            dialogs.identity_export_str = id.export_ts_identity();
                            dialogs.identity_status_msg = format!(
                                "Imported identity {} (Level {})",
                                id.uid(),
                                id.security_level()
                            );
                            dialogs.identity = id;
                        }
                        Err(e) => {
                            dialogs.identity_status_msg = format!("Import error: {e}");
                        }
                    }
                }
            });

            if !dialogs.identity_status_msg.is_empty() {
                ui.colored_label(palette.accent_blue, &dialogs.identity_status_msg);
            }

            ui.separator();
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.button("Close").clicked() {
                    close_requested = true;
                }
            });
        });

    if close_requested {
        open = false;
    }
    dialogs.show_identities = open;
}

fn render_create_channel_dialog(
    ctx: &egui::Context,
    dialogs: &mut DialogsState,
    palette: &Ts3Palette,
    _assets: &Ts3Assets,
    actions: &mut Vec<DialogAction>,
) {
    let mut open = dialogs.show_create_channel;
    let mut close_requested = false;

    egui::Window::new("Create Channel")
        .id(egui::Id::new("ts3_create_channel_dialog"))
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .default_width(430.0)
        .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
        .show(ctx, |ui| {
            // 4-tab bar: Standard | Audio | Permissions | Advanced
            ui.horizontal(|ui| {
                for (idx, tab_name) in ["Standard", "Audio", "Permissions", "Advanced"]
                    .iter()
                    .enumerate()
                {
                    if ui
                        .selectable_label(dialogs.create_channel_tab == idx, *tab_name)
                        .clicked()
                    {
                        dialogs.create_channel_tab = idx;
                    }
                }
            });
            ui.separator();

            match dialogs.create_channel_tab {
                0 => {
                    // Standard tab
                    egui::Grid::new("create_channel_standard_grid")
                        .num_columns(2)
                        .spacing([10.0, 6.0])
                        .show(ui, |ui| {
                            ui.label("Name:");
                            ui.add(
                                egui::TextEdit::singleline(&mut dialogs.create_channel_name)
                                    .desired_width(270.0),
                            );
                            ui.end_row();

                            ui.label("Password:");
                            ui.add(
                                egui::TextEdit::singleline(&mut dialogs.create_channel_password)
                                    .password(true)
                                    .desired_width(270.0),
                            );
                            ui.end_row();

                            ui.label("Topic:");
                            ui.add(
                                egui::TextEdit::singleline(&mut dialogs.create_channel_topic)
                                    .desired_width(270.0),
                            );
                            ui.end_row();

                            ui.label("Description:");
                            ui.add(
                                egui::TextEdit::multiline(&mut dialogs.create_channel_description)
                                    .desired_rows(3)
                                    .desired_width(270.0),
                            );
                            ui.end_row();
                        });

                    ui.separator();
                    ui.label(RichText::new("Channel Type").strong());
                    ui.horizontal(|ui| {
                        ui.radio_value(&mut dialogs.create_channel_type, 0, "Temporary");
                        ui.radio_value(&mut dialogs.create_channel_type, 1, "Semi-Permanent");
                        ui.radio_value(&mut dialogs.create_channel_type, 2, "Permanent");
                    });
                    ui.checkbox(&mut dialogs.create_channel_default, "Default Channel");
                }
                1 => {
                    // Audio tab
                    ui.label(RichText::new("Codec Details").strong());
                    egui::Grid::new("create_channel_audio_grid")
                        .num_columns(2)
                        .spacing([10.0, 8.0])
                        .show(ui, |ui| {
                            ui.label("Codec:");
                            ui.horizontal(|ui| {
                                ui.radio_value(&mut dialogs.create_channel_codec, 4, "Opus Voice");
                                ui.radio_value(&mut dialogs.create_channel_codec, 5, "Opus Music");
                            });
                            ui.end_row();

                            ui.label("Quality:");
                            ui.add(
                                egui::Slider::new(&mut dialogs.create_channel_quality, 0..=10)
                                    .text("/ 10"),
                            );
                            ui.end_row();

                            ui.label("Estimated Bandwidth:");
                            let bw_kib = match dialogs.create_channel_codec {
                                5 => 3.5 + f32::from(dialogs.create_channel_quality) * 1.15,
                                _ => 2.4 + f32::from(dialogs.create_channel_quality) * 0.55,
                            };
                            ui.colored_label(palette.accent_blue, format!("{bw_kib:.2} KiB/s"));
                            ui.end_row();
                        });
                }
                2 => {
                    // Permissions tab
                    ui.label(RichText::new("Channel Permissions").strong());
                    egui::Grid::new("create_channel_perms_grid")
                        .num_columns(2)
                        .spacing([10.0, 6.0])
                        .show(ui, |ui| {
                            ui.label("Needed Talk Power:");
                            ui.add(
                                egui::DragValue::new(
                                    &mut dialogs.create_channel_needed_talk_power,
                                )
                                .range(0..=100),
                            );
                            ui.end_row();
                        });
                    ui.colored_label(
                        palette.text_muted,
                        "Detailed permission powers (Join, Subscribe, Modify, Delete, File Transfer) are managed by the VirtualServer after creation.",
                    );
                }
                _ => {
                    // Advanced tab
                    ui.label(RichText::new("Advanced Channel Options").strong());
                    ui.checkbox(
                        &mut dialogs.create_channel_encrypted,
                        "Channel voice data encryption",
                    );
                    ui.horizontal(|ui| {
                        ui.label("Delete delay for temporary channels (s):");
                        ui.add_enabled(
                            dialogs.create_channel_type == 0,
                            egui::DragValue::new(&mut dialogs.create_channel_delete_delay)
                                .range(0..=86400),
                        );
                    });
                    ui.separator();
                    ui.label(RichText::new("Max Users").strong());
                    ui.horizontal(|ui| {
                        ui.radio_value(
                            &mut dialogs.create_channel_max_clients_limited,
                            false,
                            "Unlimited",
                        );
                        ui.radio_value(
                            &mut dialogs.create_channel_max_clients_limited,
                            true,
                            "Limited",
                        );
                        if dialogs.create_channel_max_clients_limited {
                            if dialogs.create_channel_max_clients < 0 {
                                dialogs.create_channel_max_clients = 16;
                            }
                            ui.add(
                                egui::DragValue::new(&mut dialogs.create_channel_max_clients)
                                    .range(0..=1024),
                            );
                        }
                    });
                }
            }

            ui.separator();
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.button("Cancel").clicked() {
                    close_requested = true;
                }
                let can_create = !dialogs.create_channel_name.trim().is_empty();
                if ui
                    .add_enabled(can_create, egui::Button::new("OK"))
                    .clicked()
                {
                    let is_permanent =
                        dialogs.create_channel_type == 2 || dialogs.create_channel_default;
                    let is_semi_permanent =
                        dialogs.create_channel_type == 1 && !dialogs.create_channel_default;
                    actions.push(DialogAction::CreateChannel {
                        parent_cid: dialogs.create_channel_parent_cid,
                        name: dialogs.create_channel_name.trim().to_string(),
                        password: if dialogs.create_channel_password.is_empty() {
                            None
                        } else {
                            Some(dialogs.create_channel_password.clone())
                        },
                        topic: dialogs.create_channel_topic.trim().to_string(),
                        description: dialogs.create_channel_description.trim().to_string(),
                        codec: dialogs.create_channel_codec,
                        codec_quality: dialogs.create_channel_quality,
                        is_permanent,
                        is_semi_permanent,
                    });
                    dialogs.create_channel_name.clear();
                    dialogs.create_channel_password.clear();
                    dialogs.create_channel_topic.clear();
                    dialogs.create_channel_description.clear();
                    close_requested = true;
                }
            });
        });

    if close_requested {
        open = false;
    }
    dialogs.show_create_channel = open;
}

fn render_poke_dialog(
    ctx: &egui::Context,
    dialogs: &mut DialogsState,
    _palette: &Ts3Palette,
    actions: &mut Vec<DialogAction>,
) {
    let mut open = dialogs.show_poke;
    let mut close_requested = false;

    egui::Window::new("Poke")
        .id(egui::Id::new("ts3_poke_dialog"))
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .default_width(340.0)
        .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
        .show(ctx, |ui| {
            ui.label(format!(
                "Target Client: {} (clid={})",
                dialogs.poke_target_nick, dialogs.poke_target_clid
            ));
            ui.label("Poke Reason:");
            let resp = ui.add(
                egui::TextEdit::singleline(&mut dialogs.poke_message)
                    .desired_width(310.0)
                    .hint_text("Enter poke message..."),
            );
            let enter_pressed = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));

            ui.separator();
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.button("Cancel").clicked() {
                    close_requested = true;
                }
                if ui.button("OK").clicked() || enter_pressed {
                    actions.push(DialogAction::PokeClient {
                        clid: dialogs.poke_target_clid,
                        message: dialogs.poke_message.trim().to_string(),
                    });
                    dialogs.poke_message.clear();
                    close_requested = true;
                }
            });
        });

    if close_requested {
        open = false;
    }
    dialogs.show_poke = open;
}

fn render_received_poke_dialog(
    ctx: &egui::Context,
    dialogs: &mut DialogsState,
    palette: &Ts3Palette,
) {
    let mut open = dialogs.show_received_poke;
    let mut close_requested = false;

    egui::Window::new("You Have Been Poked")
        .id(egui::Id::new("ts3_received_poke_dialog"))
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .default_width(380.0)
        .anchor(egui::Align2::CENTER_CENTER, Vec2::new(0.0, -30.0))
        .show(ctx, |ui| {
            ui.label(RichText::new(&dialogs.received_poke_server).strong());
            ui.separator();
            egui::Frame::group(ui.style())
                .fill(palette.base_bg)
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.colored_label(
                            palette.chat_time,
                            format!("<{}>", dialogs.received_poke_timestamp),
                        );
                        ui.colored_label(
                            palette.chat_poke,
                            RichText::new(format!(
                                "\"{}\" pokes you: {}",
                                dialogs.received_poke_invoker, dialogs.received_poke_message
                            ))
                            .strong(),
                        );
                    });
                });
            ui.separator();
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.button("OK").clicked() {
                    close_requested = true;
                }
            });
        });

    if close_requested {
        open = false;
    }
    dialogs.show_received_poke = open;
}

fn render_change_nickname_dialog(
    ctx: &egui::Context,
    dialogs: &mut DialogsState,
    actions: &mut Vec<DialogAction>,
) {
    let mut open = dialogs.show_change_nickname;
    let mut close_requested = false;

    egui::Window::new("Change Nickname")
        .id(egui::Id::new("ts3_change_nickname_dialog"))
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .default_width(320.0)
        .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
        .show(ctx, |ui| {
            ui.label("Enter new nickname:");
            let resp = ui.add(
                egui::TextEdit::singleline(&mut dialogs.change_nickname_input).desired_width(290.0),
            );
            let enter = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            ui.separator();
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.button("Cancel").clicked() {
                    close_requested = true;
                }
                let valid = dialogs.change_nickname_input.trim().len() >= 3;
                if ui.add_enabled(valid, egui::Button::new("OK")).clicked() || (enter && valid) {
                    let new_nick = dialogs.change_nickname_input.trim().to_string();
                    dialogs.connect_nickname = new_nick.clone();
                    let escaped = ts3_proto::command::escape(&new_nick);
                    actions.push(DialogAction::Ui(UiCommand::SendRawCommand(format!(
                        "clientupdate client_nickname={escaped}"
                    ))));
                    close_requested = true;
                }
            });
        });

    if close_requested {
        open = false;
    }
    dialogs.show_change_nickname = open;
}

fn render_options_dialog(
    ctx: &egui::Context,
    dialogs: &mut DialogsState,
    palette: &Ts3Palette,
    actions: &mut Vec<DialogAction>,
) {
    let mut open = dialogs.show_options;
    let mut close_requested = false;

    egui::Window::new("Options (Alt+P)")
        .id(egui::Id::new("ts3_options_dialog"))
        .open(&mut open)
        .collapsible(false)
        .resizable(true)
        .default_size([600.0, 400.0])
        .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                // 13-category left sidebar (`ContentsWidget`)
                egui::Frame::group(ui.style())
                    .fill(palette.base_bg)
                    .show(ui, |ui| {
                        ui.set_width(145.0);
                        ui.set_min_height(310.0);
                        for (idx, &cat) in OPTIONS_CATEGORIES.iter().enumerate() {
                            if ui
                                .selectable_label(dialogs.options_category == idx, cat)
                                .clicked()
                            {
                                dialogs.options_category = idx;
                            }
                        }
                    });

                ui.separator();

                // Stacked page area (`m_pagesList`)
                ui.vertical(|ui| {
                    ui.set_min_height(310.0);
                    ui.set_min_width(400.0);
                    let cat_title = OPTIONS_CATEGORIES
                        .get(dialogs.options_category)
                        .copied()
                        .unwrap_or("Application");
                    ui.heading(cat_title);
                    ui.separator();

                    match dialogs.options_category {
                        0 => {
                            // Application
                            ui.checkbox(
                                &mut dialogs.warn_on_close_connected,
                                "Warn when closing TeamSpeak with active connections",
                            );
                            ui.checkbox(
                                &mut dialogs.disable_poke_dialog,
                                "Never show Poke Dialog (avoid fullscreen interruptions)",
                            );
                            ui.checkbox(
                                &mut dialogs.show_server_query_clients,
                                "Show ServerQuery Clients in Server Tree",
                            );
                        }
                        1 => {
                            // Design
                            ui.label(RichText::new("Theme / Style (QSS):").strong());
                            let prev_theme = dialogs.theme_mode;
                            ui.horizontal(|ui| {
                                ui.radio_value(
                                    &mut dialogs.theme_mode,
                                    ThemeMode::Light,
                                    "Default Light (default.qss)",
                                );
                                ui.radio_value(
                                    &mut dialogs.theme_mode,
                                    ThemeMode::Dark,
                                    "Dark Mode (default_darkmode.qss)",
                                );
                            });
                            if dialogs.theme_mode != prev_theme {
                                actions.push(DialogAction::Ui(UiCommand::SwitchTheme(
                                    dialogs.theme_mode,
                                )));
                            }
                            ui.separator();
                            ui.label(RichText::new("Server Tree Display:").strong());
                            ui.checkbox(
                                &mut dialogs.show_country_flags,
                                "Display country flags on clients",
                            );
                        }
                        2 => {
                            // Security
                            ui.label("Save Passwords & Certificate Trust:");
                            ui.label(format!(
                                "Active Identity Security Level: {}",
                                dialogs.identity.security_level()
                            ));
                        }
                        3 => {
                            // Playback
                            ui.add(
                                egui::Slider::new(&mut dialogs.master_volume_db, -40.0..=20.0)
                                    .text("Voice Volume Adjustment (dB)"),
                            );
                            ui.checkbox(
                                &mut dialogs.comfort_noise_enabled,
                                "Generate Comfort Noise",
                            );
                            ui.checkbox(
                                &mut dialogs.sound_enabled,
                                "Enable Sound Pack Notifications (Default Female)",
                            );
                        }
                        4 => {
                            // Capture
                            ui.label(RichText::new("Voice Activation Detection (VAD):").strong());
                            ui.add(
                                egui::Slider::new(
                                    &mut dialogs.voice_activation_threshold_db,
                                    -50.0..=10.0,
                                )
                                .text("VAD Threshold (dB)"),
                            );
                            ui.separator();
                            ui.checkbox(&mut dialogs.echo_cancellation, "Echo Cancellation");
                            ui.checkbox(&mut dialogs.noise_reduction, "Remove Background Noise");
                        }
                        5 => {
                            // Chat
                            ui.horizontal(|ui| {
                                ui.label("Maximum Chat History Lines:");
                                ui.add(
                                    egui::DragValue::new(&mut dialogs.max_chat_lines)
                                        .range(50..=5000),
                                );
                            });
                            ui.checkbox(&mut dialogs.log_server_chat, "Automatically log Server Chat");
                            ui.checkbox(
                                &mut dialogs.log_channel_chat,
                                "Automatically log Channel Chat",
                            );
                        }
                        6 => {
                            // Whisper
                            ui.checkbox(&mut dialogs.block_whispers, "Block incoming whispers");
                        }
                        10 => {
                            // Notifications
                            ui.checkbox(
                                &mut dialogs.sound_enabled,
                                "Play Sound Pack WAV cues (Default Sound Pack Female)",
                            );
                        }
                        _ => {
                            ui.colored_label(
                                palette.text_muted,
                                format!("{cat_title} settings configured to TS3 3.6.2 defaults."),
                            );
                        }
                    }
                });
            });

            ui.separator();
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.button("OK").clicked() {
                    close_requested = true;
                }
                if ui.button("Apply").clicked() {
                    actions.push(DialogAction::Ui(UiCommand::SwitchTheme(dialogs.theme_mode)));
                }
            });
        });

    if close_requested {
        open = false;
    }
    dialogs.show_options = open;
}

fn render_about_dialog(
    ctx: &egui::Context,
    dialogs: &mut DialogsState,
    palette: &Ts3Palette,
    _assets: &Ts3Assets,
) {
    let mut open = dialogs.show_about;
    let mut close_requested = false;

    egui::Window::new("About TeamSpeak 3")
        .id(egui::Id::new("ts3_about_dialog"))
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .default_width(420.0)
        .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
        .show(ctx, |ui| {
            ui.heading("TeamSpeak 3 Client");
            ui.colored_label(
                palette.accent_blue,
                RichText::new("Version: 3.6.2 / Native Rust 1:1 Edition").strong(),
            );
            let vs = VersionSign::default_windows();
            ui.label(format!("Protocol Version Sign: {} ({})", vs.version, vs.platform));
            ui.separator();
            ui.label("Protocol & Cryptographic Stack:");
            ui.monospace("• Transport: UDP 5- step Init1 Cookie + ECDH P-256 / Curve25519");
            ui.monospace("• Cipher: AES-128-EAX (16B key/nonce, 8B MAC tag)");
            ui.monospace("• Compression: QuickLZ Level 1 (0x80 header framing)");
            ui.monospace("• Identity: LibTomCrypt ASN.1 DER P-256 + SHA-1 Hashcash");
            ui.monospace("• Audio Codecs: Opus Voice (4) / Opus Music (5) @ 48 kHz");
            ui.separator();
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.button("Close").clicked() {
                    close_requested = true;
                }
            });
        });

    if close_requested {
        open = false;
    }
    dialogs.show_about = open;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dialogs_state_defaults() {
        let mut state = DialogsState::default();
        assert!(!state.show_connect);
        assert_eq!(state.connect_addr, "192.168.1.140:9987");
        assert_eq!(state.connect_nickname, "TeamSpeakUser");
        assert_eq!(state.bookmarks.len(), 1);
        assert!(state.identity.security_level() >= 8);
        assert!(!state.identity.uid().is_empty());
        assert!(!state.identity.public_key_omega().is_empty());
        assert_eq!(OPTIONS_CATEGORIES.len(), 13);

        state.identity.improve_security_level(9);
        assert!(state.identity.security_level() >= 9);

        let exported = state.identity.export_ts_identity();
        let reimported = IdentityData::import_ts_identity(&exported).expect("reimport identity");
        assert_eq!(reimported.uid(), state.identity.uid());
    }

    #[test]
    fn test_render_all_dialogs_headless() {
        let ctx = egui::Context::default();
        let mut dialogs = DialogsState::default();
        dialogs.show_connect = true;
        dialogs.connect_more_expanded = true;
        dialogs.show_bookmarks = true;
        dialogs.show_identities = true;
        dialogs.show_create_channel = true;
        dialogs.show_poke = true;
        dialogs.show_received_poke = true;
        dialogs.show_change_nickname = true;
        dialogs.show_options = true;
        dialogs.show_about = true;

        let palette = Ts3Palette::light();
        let assets = Ts3Assets::new();
        let mut actions = Vec::new();

        for tab in 0..4 {
            dialogs.create_channel_tab = tab;
            let _ = ctx.run(egui::RawInput::default(), |ctx| {
                render_dialogs(ctx, &mut dialogs, &palette, &assets, &mut actions);
            });
        }
    }
}


