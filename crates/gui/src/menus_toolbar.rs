// Owned by Pool Agent 4 (arthoroy27a@gmail.com)
// 1:1 Menu Bar (`Connections`, `Bookmarks`, `Self`, `Permissions`, `Tools`, `Plugins`, `Help`),
// Main Toolbar, Server Tab Bar (`ImprovedTabBar`), and Status Bar from `docs/re/gui/02-mainwindow-layout.md`.

use crate::assets::Ts3Assets;
use crate::theme::{ThemeMode, Ts3Palette};
use egui::{
    Align, Color32, CornerRadius, FontId, Id, Key, Layout, Modifiers, Pos2, Rect, RichText, Sense,
    Stroke, StrokeKind, Ui, Vec2,
};

pub const TS3_CLIENT_VERSION_LABEL: &str = "TS3 3.6.2 (Qt 5.15.2)";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiCommand {
    OpenConnectDialog,
    DisconnectCurrent,
    OpenBookmarksDialog,
    AddCurrentToBookmarks,
    OpenIdentitiesDialog,
    OpenCreateChannelDialog { parent_cid: u64 },
    OpenOptionsDialog,
    OpenAboutDialog,
    ToggleAway { away: bool, message: Option<String> },
    ToggleMicMute,
    ToggleOutputMute,
    SubscribeAllChannels,
    SwitchTheme(ThemeMode),
    ChangeNicknamePrompt,
    SelectServerTab(usize),
    CloseServerTab(usize),
    NewServerTab,
    SwitchChannel { cid: u64, password: Option<String> },
    OpenPrivateChat { clid: u16, nickname: String },
    OpenPokeDialog { clid: u16, nickname: String },
    SendRawCommand(String),
}

#[derive(Debug, Clone)]
pub struct ToolbarState {
    pub connected: bool,
    pub away: bool,
    pub away_message: String,
    pub input_muted: bool,
    pub output_muted: bool,
    pub subscribed_all: bool,
    pub push_to_talk_active: bool,
    pub theme_mode: ThemeMode,
}

impl Default for ToolbarState {
    fn default() -> Self {
        Self {
            connected: false,
            away: false,
            away_message: String::new(),
            input_muted: false,
            output_muted: false,
            subscribed_all: true,
            push_to_talk_active: false,
            theme_mode: ThemeMode::Light,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ServerTabSummary {
    pub index: usize,
    pub title: String,
    pub connected: bool,
    pub unread_count: usize,
}

#[derive(Debug, Clone, Default)]
pub struct StatusBarInfo {
    pub status_text: String,
    pub connected: bool,
    pub server_name: String,
    pub rtt_ms: u32,
    pub packet_loss: f32,
    pub clients_online: u32,
    pub max_clients: u32,
}

/// Renders a menu item with an optional 16x16 icon, checkmark, label, and right-aligned keyboard shortcut.
#[allow(clippy::too_many_arguments)]
fn menu_item_with_shortcut(
    ui: &mut Ui,
    assets: &Ts3Assets,
    icon_key: Option<&str>,
    checked: Option<bool>,
    label: &str,
    shortcut: &str,
    enabled: bool,
    palette: &Ts3Palette,
) -> bool {
    let desired_width = 250.0;
    let desired_height = 22.0;
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(desired_width, desired_height),
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );

    if ui.is_rect_visible(rect) {
        let hovered = enabled && response.hovered();
        if hovered {
            ui.painter()
                .rect_filled(rect, CornerRadius::same(2), palette.selection_bg);
        }

        let fg = if !enabled {
            palette.text_muted
        } else if hovered {
            palette.selection_fg
        } else {
            palette.text_primary
        };

        let shortcut_fg = if !enabled {
            palette.text_muted
        } else if hovered {
            palette.selection_fg
        } else {
            palette.text_muted
        };

        let icon_rect = Rect::from_min_size(
            Pos2::new(rect.left() + 4.0, rect.center().y - 8.0),
            Vec2::splat(16.0),
        );

        if let Some(true) = checked {
            ui.painter().rect_stroke(
                icon_rect.expand(1.0),
                CornerRadius::same(2),
                Stroke::new(1.0_f32, palette.accent_blue),
                StrokeKind::Inside,
            );
            ui.painter().text(
                icon_rect.center(),
                egui::Align2::CENTER_CENTER,
                "✓",
                FontId::proportional(12.0),
                fg,
            );
        } else if let Some(key) = icon_key {
            if let Some(tex) = assets.get_icon(key) {
                let tint = if enabled {
                    Color32::WHITE
                } else {
                    Color32::from_white_alpha(110)
                };
                ui.painter().image(
                    tex.id(),
                    icon_rect,
                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                    tint,
                );
            } else {
                let dot_color = if enabled {
                    palette.accent_blue
                } else {
                    palette.text_muted
                };
                ui.painter()
                    .circle_filled(icon_rect.center(), 2.5, dot_color);
            }
        }

        let text_x = rect.left() + 26.0;
        ui.painter().text(
            Pos2::new(text_x, rect.center().y),
            egui::Align2::LEFT_CENTER,
            label,
            FontId::proportional(12.5),
            fg,
        );

        if !shortcut.is_empty() {
            ui.painter().text(
                Pos2::new(rect.right() - 6.0, rect.center().y),
                egui::Align2::RIGHT_CENTER,
                shortcut,
                FontId::proportional(11.5),
                shortcut_fg,
            );
        }
    }

    if enabled && response.clicked() {
        ui.close_menu();
        true
    } else {
        false
    }
}

/// Renders the exact 7 top-level menus matching the Windows TS3 Client (`MainWindow::setupMenus()`).
pub fn render_menu_bar(
    ui: &mut egui::Ui,
    state: &ToolbarState,
    palette: &Ts3Palette,
    assets: &Ts3Assets,
    commands: &mut Vec<UiCommand>,
) {
    egui::menu::bar(ui, |ui| {
        ui.spacing_mut().button_padding = Vec2::new(6.0, 3.0);

        // 1. &Connections
        ui.menu_button("Connections", |ui| {
            if menu_item_with_shortcut(
                ui,
                assets,
                Some("CONNECT"),
                None,
                "Connect",
                "Ctrl+S",
                true,
                palette,
            ) {
                commands.push(UiCommand::OpenConnectDialog);
            }
            if menu_item_with_shortcut(
                ui,
                assets,
                Some("DISCONNECT"),
                None,
                "Disconnect from Current",
                "Ctrl+D",
                state.connected,
                palette,
            ) {
                commands.push(UiCommand::DisconnectCurrent);
            }
            if menu_item_with_shortcut(
                ui,
                assets,
                Some("DISCONNECT_ALL"),
                None,
                "Disconnect from All",
                "Ctrl+Shift+D",
                state.connected,
                palette,
            ) {
                commands.push(UiCommand::DisconnectCurrent);
            }
            ui.separator();
            if menu_item_with_shortcut(
                ui,
                assets,
                Some("SERVER_QUERY"),
                None,
                "ServerQuery Login",
                "",
                state.connected,
                palette,
            ) {
                commands.push(UiCommand::SendRawCommand(
                    "clientsetserverquerylogin".to_string(),
                ));
            }
            ui.separator();
            if menu_item_with_shortcut(
                ui,
                assets,
                Some("QUIT"),
                None,
                "Quit",
                "Ctrl+Q",
                true,
                palette,
            ) {
                commands.push(UiCommand::SendRawCommand("clientquit".to_string()));
            }
        });

        // 2. &Bookmarks
        ui.menu_button("Bookmarks", |ui| {
            if menu_item_with_shortcut(
                ui,
                assets,
                Some("BOOKMARK_MANAGER"),
                None,
                "Bookmarks Manager",
                "Ctrl+B",
                true,
                palette,
            ) {
                commands.push(UiCommand::OpenBookmarksDialog);
            }
            if menu_item_with_shortcut(
                ui,
                assets,
                Some("BOOKMARK_ADD"),
                None,
                "Add to Bookmarks",
                "Ctrl+Shift+B",
                state.connected,
                palette,
            ) {
                commands.push(UiCommand::AddCurrentToBookmarks);
            }
            ui.separator();
            if menu_item_with_shortcut(
                ui,
                assets,
                Some("SERVER_GREEN"),
                None,
                "TeamSpeak Public Server",
                "",
                true,
                palette,
            ) {
                commands.push(UiCommand::OpenConnectDialog);
            }
        });

        // 3. S&elf
        ui.menu_button("Self", |ui| {
            if menu_item_with_shortcut(
                ui,
                assets,
                Some("AWAY"),
                Some(state.away),
                "Set Away",
                "Alt+A",
                true,
                palette,
            ) {
                let next_away = !state.away;
                let msg = if next_away && !state.away_message.is_empty() {
                    Some(state.away_message.clone())
                } else {
                    None
                };
                commands.push(UiCommand::ToggleAway {
                    away: next_away,
                    message: msg,
                });
            }
            if menu_item_with_shortcut(
                ui,
                assets,
                Some("AWAY"),
                None,
                "Set Away Status...",
                "",
                true,
                palette,
            ) {
                commands.push(UiCommand::ToggleAway {
                    away: true,
                    message: Some("AFK".to_string()),
                });
            }
            if menu_item_with_shortcut(
                ui,
                assets,
                Some("AWAY"),
                None,
                "Set Globally Away",
                "",
                true,
                palette,
            ) {
                commands.push(UiCommand::ToggleAway {
                    away: true,
                    message: if state.away_message.is_empty() {
                        None
                    } else {
                        Some(state.away_message.clone())
                    },
                });
            }
            if menu_item_with_shortcut(
                ui,
                assets,
                Some("PRESENT"),
                Some(!state.away),
                "Online",
                "",
                state.away,
                palette,
            ) {
                commands.push(UiCommand::ToggleAway {
                    away: false,
                    message: None,
                });
            }
            ui.separator();
            if menu_item_with_shortcut(
                ui,
                assets,
                Some("ACTIVATE_MICROPHONE"),
                Some(!state.input_muted),
                "Activate Microphone",
                "Ctrl+M",
                true,
                palette,
            ) {
                if state.input_muted {
                    commands.push(UiCommand::ToggleMicMute);
                }
            }
            if menu_item_with_shortcut(
                ui,
                assets,
                Some("INPUT_MUTED"),
                Some(state.input_muted),
                "Mute Microphone",
                "Ctrl+Shift+M",
                true,
                palette,
            ) {
                commands.push(UiCommand::ToggleMicMute);
            }
            if menu_item_with_shortcut(
                ui,
                assets,
                Some("OUTPUT_MUTED"),
                Some(state.output_muted),
                "Mute Speakers/Headphones",
                "Ctrl+Shift+D",
                true,
                palette,
            ) {
                commands.push(UiCommand::ToggleOutputMute);
            }
            if menu_item_with_shortcut(
                ui,
                assets,
                Some("HARDWARE_OUTPUT_MUTED"),
                None,
                "Mute Speakers on All Servers",
                "Ctrl+Shift+S",
                true,
                palette,
            ) {
                commands.push(UiCommand::ToggleOutputMute);
            }
            ui.separator();
            if menu_item_with_shortcut(
                ui,
                assets,
                Some("CHANGE_NICKNAME"),
                None,
                "Change Nickname",
                "",
                state.connected,
                palette,
            ) {
                commands.push(UiCommand::ChangeNicknamePrompt);
            }
            if menu_item_with_shortcut(
                ui,
                assets,
                Some("PHONETICSNICKNAME"),
                None,
                "Set Phonetic Nickname",
                "",
                state.connected,
                palette,
            ) {
                commands.push(UiCommand::ChangeNicknamePrompt);
            }
            if menu_item_with_shortcut(
                ui,
                assets,
                Some("CHANNEL_COMMANDER"),
                None,
                "Channel Commander",
                "",
                state.connected,
                palette,
            ) {
                commands.push(UiCommand::SendRawCommand(
                    "clientupdate client_is_channel_commander=1".to_string(),
                ));
            }
            ui.separator();
            if menu_item_with_shortcut(
                ui,
                assets,
                Some("INFO"),
                None,
                "Connection Info",
                "Ctrl+I",
                state.connected,
                palette,
            ) {
                commands.push(UiCommand::SendRawCommand("getconnectioninfo".to_string()));
            }
        });

        // 4. Per&missions
        ui.menu_button("Permissions", |ui| {
            if menu_item_with_shortcut(
                ui,
                assets,
                Some("PERMISSIONS_SERVER_GROUPS"),
                None,
                "Server Groups",
                "Ctrl+F1",
                state.connected,
                palette,
            ) {
                commands.push(UiCommand::SendRawCommand("servergrouplist".to_string()));
            }
            if menu_item_with_shortcut(
                ui,
                assets,
                Some("PERMISSIONS_CHANNEL_GROUPS"),
                None,
                "Channel Groups",
                "Ctrl+F2",
                state.connected,
                palette,
            ) {
                commands.push(UiCommand::SendRawCommand("channelgrouplist".to_string()));
            }
            if menu_item_with_shortcut(
                ui,
                assets,
                Some("PERMISSIONS_CHANNELS"),
                None,
                "Channel Permissions",
                "Ctrl+F3",
                state.connected,
                palette,
            ) {
                commands.push(UiCommand::SendRawCommand("channelpermlist".to_string()));
            }
            if menu_item_with_shortcut(
                ui,
                assets,
                Some("PERMISSIONS_CLIENTS"),
                None,
                "Client Permissions",
                "Ctrl+F4",
                state.connected,
                palette,
            ) {
                commands.push(UiCommand::SendRawCommand("clientpermlist".to_string()));
            }
            if menu_item_with_shortcut(
                ui,
                assets,
                Some("PERMISSIONS_CHANNEL_CLIENT"),
                None,
                "Channel Client Permissions",
                "Ctrl+F5",
                state.connected,
                palette,
            ) {
                commands.push(UiCommand::SendRawCommand(
                    "channelclientpermlist".to_string(),
                ));
            }
            ui.separator();
            if menu_item_with_shortcut(
                ui,
                assets,
                Some("TOKEN"),
                None,
                "Privilege Keys",
                "Ctrl+Shift+K",
                state.connected,
                palette,
            ) {
                commands.push(UiCommand::SendRawCommand("privilegekeylist".to_string()));
            }
        });

        // 5. &Tools
        ui.menu_button("Tools", |ui| {
            if menu_item_with_shortcut(
                ui,
                assets,
                Some("IDENTITY_MANAGER"),
                None,
                "Identities",
                "Ctrl+I",
                true,
                palette,
            ) {
                commands.push(UiCommand::OpenIdentitiesDialog);
            }
            if menu_item_with_shortcut(
                ui,
                assets,
                Some("CONTACT"),
                None,
                "Contacts",
                "Ctrl+Shift+O",
                true,
                palette,
            ) {
                commands.push(UiCommand::SendRawCommand("contacts".to_string()));
            }
            if menu_item_with_shortcut(
                ui,
                assets,
                Some("LISTVIEW"),
                None,
                "List All Clients",
                "Ctrl+Shift+L",
                state.connected,
                palette,
            ) {
                commands.push(UiCommand::SendRawCommand("clientlist".to_string()));
            }
            if menu_item_with_shortcut(
                ui,
                assets,
                Some("BAN_LIST"),
                None,
                "Ban List",
                "Ctrl+Shift+B",
                state.connected,
                palette,
            ) {
                commands.push(UiCommand::SendRawCommand("banlist".to_string()));
            }
            if menu_item_with_shortcut(
                ui,
                assets,
                Some("COMPLAINT_LIST"),
                None,
                "Complaints List",
                "Ctrl+Shift+C",
                state.connected,
                palette,
            ) {
                commands.push(UiCommand::SendRawCommand("complainlist".to_string()));
            }
            ui.separator();
            if menu_item_with_shortcut(
                ui,
                assets,
                Some("FILETRANSFER"),
                None,
                "File Browser",
                "Ctrl+T",
                true,
                palette,
            ) {
                commands.push(UiCommand::SendRawCommand("ftlist".to_string()));
            }
            if menu_item_with_shortcut(
                ui,
                assets,
                Some("SERVER_LOG"),
                None,
                "Server Log",
                "Ctrl+L",
                state.connected,
                palette,
            ) {
                commands.push(UiCommand::SendRawCommand("logview".to_string()));
            }
            if menu_item_with_shortcut(
                ui,
                assets,
                Some("MESSAGES"),
                None,
                "Client Log",
                "Ctrl+Shift+C",
                true,
                palette,
            ) {
                commands.push(UiCommand::SendRawCommand("clientlog".to_string()));
            }
            ui.separator();
            if menu_item_with_shortcut(
                ui,
                assets,
                Some("WHISPERLISTS"),
                None,
                "Whisper Lists",
                "Ctrl+Shift+W",
                true,
                palette,
            ) {
                commands.push(UiCommand::SendRawCommand("whisperlists".to_string()));
            }
            if menu_item_with_shortcut(
                ui,
                assets,
                Some("WHISPERHISTORY"),
                None,
                "Whisper History",
                "Ctrl+Shift+H",
                true,
                palette,
            ) {
                commands.push(UiCommand::SendRawCommand("whisperhistory".to_string()));
            }
            if menu_item_with_shortcut(
                ui,
                assets,
                Some("INVITE_BUDDY"),
                None,
                "Invite Buddy",
                "",
                state.connected,
                palette,
            ) {
                commands.push(UiCommand::SendRawCommand("invitebuddy".to_string()));
            }
            if menu_item_with_shortcut(
                ui,
                assets,
                Some("URLCATCHER"),
                None,
                "URLs",
                "Ctrl+U",
                true,
                palette,
            ) {
                commands.push(UiCommand::SendRawCommand("urlcatcher".to_string()));
            }
            ui.separator();
            if menu_item_with_shortcut(
                ui,
                assets,
                Some("SETTINGS"),
                None,
                "Options",
                "Alt+P",
                true,
                palette,
            ) {
                commands.push(UiCommand::OpenOptionsDialog);
            }
        });

        // 6. P&lugins
        ui.menu_button("Plugins", |ui| {
            if menu_item_with_shortcut(
                ui,
                assets,
                Some("PLUGINS"),
                None,
                "Manage Plugins (Addons)...",
                "",
                true,
                palette,
            ) {
                commands.push(UiCommand::OpenOptionsDialog);
            }
            ui.separator();
            let _ = menu_item_with_shortcut(
                ui,
                assets,
                Some("ADDON"),
                Some(true),
                "ClientQuery",
                "",
                false,
                palette,
            );
            let _ = menu_item_with_shortcut(
                ui,
                assets,
                Some("ADDON"),
                Some(false),
                "Gamepad and Joystick Hotkey Support",
                "",
                false,
                palette,
            );
        });

        // 7. &Help
        ui.menu_button("Help", |ui| {
            if menu_item_with_shortcut(
                ui,
                assets,
                Some("CHANGELOG"),
                None,
                "View Changelog",
                "",
                true,
                palette,
            ) {
                commands.push(UiCommand::OpenAboutDialog);
            }
            if menu_item_with_shortcut(
                ui,
                assets,
                Some("INFO"),
                None,
                "View Licenses",
                "",
                true,
                palette,
            ) {
                commands.push(UiCommand::OpenAboutDialog);
            }
            if menu_item_with_shortcut(
                ui,
                assets,
                Some("CHECK_UPDATE"),
                None,
                "Check for Update",
                "",
                true,
                palette,
            ) {
                commands.push(UiCommand::SendRawCommand("checkupdate".to_string()));
            }
            ui.separator();
            if menu_item_with_shortcut(
                ui,
                assets,
                Some("MYTS_ACCOUNT"),
                None,
                "myTeamSpeak Account",
                "",
                true,
                palette,
            ) {
                commands.push(UiCommand::OpenOptionsDialog);
            }
            if menu_item_with_shortcut(
                ui,
                assets,
                Some("ABOUT"),
                None,
                "About TeamSpeak 3",
                "",
                true,
                palette,
            ) {
                commands.push(UiCommand::OpenAboutDialog);
            }
        });
    });
}

/// Helper to draw a 24x24 `ImprovedToolBar` icon button with active/toggled borders, fallback vector glyphs, and tooltip.
#[allow(clippy::too_many_arguments)]
fn toolbar_icon_button(
    ui: &mut Ui,
    assets: &Ts3Assets,
    icon_key: &str,
    fallback_glyph: &str,
    tooltip: &str,
    toggled: bool,
    enabled: bool,
    palette: &Ts3Palette,
) -> egui::Response {
    let btn_size = Vec2::splat(28.0);
    let icon_size = Vec2::splat(24.0);
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(btn_size, sense);

    if ui.is_rect_visible(rect) {
        let hovered = enabled && response.hovered();
        if toggled {
            let fill = if hovered {
                palette.hover_bg
            } else {
                palette.selection_bg.gamma_multiply(0.28)
            };
            ui.painter()
                .rect_filled(rect, CornerRadius::same(3), fill);
            ui.painter().rect_stroke(
                rect,
                CornerRadius::same(3),
                Stroke::new(1.5_f32, palette.accent_blue),
                StrokeKind::Inside,
            );
        } else if hovered {
            ui.painter()
                .rect_filled(rect, CornerRadius::same(3), palette.hover_bg);
            ui.painter().rect_stroke(
                rect,
                CornerRadius::same(3),
                Stroke::new(1.0_f32, palette.border_color),
                StrokeKind::Inside,
            );
        }

        let icon_rect = Rect::from_center_size(rect.center(), icon_size);
        if let Some(tex) = assets.get_icon(icon_key) {
            let tint = if enabled {
                Color32::WHITE
            } else {
                Color32::from_white_alpha(95)
            };
            ui.painter().image(
                tex.id(),
                icon_rect,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                tint,
            );
        } else {
            let fg = if !enabled {
                palette.text_muted
            } else if toggled {
                palette.accent_blue
            } else {
                palette.text_primary
            };
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                fallback_glyph,
                FontId::proportional(14.0),
                fg,
            );
        }
    }

    response.on_hover_text(tooltip)
}

/// Renders the 24x24 icon main toolbar (`ImprovedToolBar`) matching Windows TS3 Client.
pub fn render_main_toolbar(
    ui: &mut egui::Ui,
    state: &mut ToolbarState,
    palette: &Ts3Palette,
    assets: &Ts3Assets,
    commands: &mut Vec<UiCommand>,
) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(4.0, 2.0);
        ui.set_min_height(30.0);

        // 1. Connect
        if toolbar_icon_button(
            ui,
            assets,
            "CONNECT",
            "⚡",
            "Connect to a Server (Ctrl+S)",
            false,
            true,
            palette,
        )
        .clicked()
        {
            commands.push(UiCommand::OpenConnectDialog);
        }

        // 2. Disconnect
        if toolbar_icon_button(
            ui,
            assets,
            "DISCONNECT",
            "✖",
            "Disconnect from Current Server (Ctrl+D)",
            false,
            state.connected,
            palette,
        )
        .clicked()
            && state.connected
        {
            commands.push(UiCommand::DisconnectCurrent);
        }

        ui.separator();

        // 3. Bookmarks
        if toolbar_icon_button(
            ui,
            assets,
            "BOOKMARK_MANAGER",
            "★",
            "Bookmarks Manager (Ctrl+B)",
            false,
            true,
            palette,
        )
        .clicked()
        {
            commands.push(UiCommand::OpenBookmarksDialog);
        }

        ui.separator();

        // 4. Away toggle + away message indicator
        let away_tooltip = if state.away {
            if state.away_message.is_empty() {
                "Away Status: Active (Alt+A)".to_string()
            } else {
                format!("Away Status: {} (Alt+A)", state.away_message)
            }
        } else {
            "Set Away Status (Alt+A)".to_string()
        };
        if toolbar_icon_button(
            ui,
            assets,
            "AWAY",
            "🕒",
            &away_tooltip,
            state.away,
            true,
            palette,
        )
        .clicked()
        {
            state.away = !state.away;
            let msg = if state.away && !state.away_message.is_empty() {
                Some(state.away_message.clone())
            } else {
                None
            };
            commands.push(UiCommand::ToggleAway {
                away: state.away,
                message: msg,
            });
        }

        if state.away && !state.away_message.is_empty() {
            ui.label(
                RichText::new(format!("[{}]", state.away_message))
                    .size(11.5)
                    .color(palette.text_muted),
            )
            .on_hover_text("Current Away Message");
        }

        // 5. Activate / Mute Microphone
        let mic_icon = if state.input_muted {
            "INPUT_MUTED"
        } else {
            "ACTIVATE_MICROPHONE"
        };
        let mic_tooltip = if state.input_muted {
            "Unmute Microphone (Ctrl+M / Ctrl+Shift+M)"
        } else {
            "Mute Microphone (Ctrl+Shift+M)"
        };
        if toolbar_icon_button(
            ui,
            assets,
            mic_icon,
            "🎤",
            mic_tooltip,
            state.input_muted,
            true,
            palette,
        )
        .clicked()
        {
            state.input_muted = !state.input_muted;
            commands.push(UiCommand::ToggleMicMute);
        }

        // 6. Mute Speakers/Headphones
        let spk_tooltip = if state.output_muted {
            "Unmute Speakers/Headphones (Ctrl+Shift+D)"
        } else {
            "Mute Speakers/Headphones (Ctrl+Shift+D)"
        };
        if toolbar_icon_button(
            ui,
            assets,
            "OUTPUT_MUTED",
            "🔇",
            spk_tooltip,
            state.output_muted,
            true,
            palette,
        )
        .clicked()
        {
            state.output_muted = !state.output_muted;
            commands.push(UiCommand::ToggleOutputMute);
        }

        ui.separator();

        // 7. Subscribe to all channels
        let sub_icon = if state.subscribed_all {
            "SUBSCRIBE_TO_ALL_CHANNELS"
        } else {
            "UNSUBSCRIBE_FROM_ALL_CHANNELS"
        };
        let sub_tooltip = if state.subscribed_all {
            "Subscribed to All Channels (Click to toggle)"
        } else {
            "Subscribe to All Channels"
        };
        if toolbar_icon_button(
            ui,
            assets,
            sub_icon,
            "👁",
            sub_tooltip,
            state.subscribed_all,
            true,
            palette,
        )
        .clicked()
        {
            state.subscribed_all = !state.subscribed_all;
            commands.push(UiCommand::SubscribeAllChannels);
        }

        ui.separator();

        // 8. Light / Dark theme switch & Options button
        let is_dark = state.theme_mode == ThemeMode::Dark;
        let theme_glyph = if is_dark { "☀" } else { "🌙" };
        let theme_tooltip = if is_dark {
            "Switch to Light Theme (default.qss)"
        } else {
            "Switch to Dark Theme (default_darkmode.qss)"
        };
        if toolbar_icon_button(
            ui,
            assets,
            "ADDON_THEME",
            theme_glyph,
            theme_tooltip,
            is_dark,
            true,
            palette,
        )
        .clicked()
        {
            let next_mode = match state.theme_mode {
                ThemeMode::Light => ThemeMode::Dark,
                ThemeMode::Dark => ThemeMode::Light,
            };
            state.theme_mode = next_mode;
            commands.push(UiCommand::SwitchTheme(next_mode));
        }

        if toolbar_icon_button(
            ui,
            assets,
            "SETTINGS",
            "⚙",
            "Options (Alt+P)",
            false,
            true,
            palette,
        )
        .clicked()
        {
            commands.push(UiCommand::OpenOptionsDialog);
        }
    });
}

/// Renders the 1:1 `ImprovedTabBar` for server connection tabs (`MainWindowServerTabsWidget`).
pub fn render_server_tab_bar(
    ui: &mut egui::Ui,
    tabs: &[ServerTabSummary],
    active_tab: usize,
    palette: &Ts3Palette,
    assets: &Ts3Assets,
    commands: &mut Vec<UiCommand>,
) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(2.0, 0.0);
        ui.set_min_height(26.0);

        for tab in tabs {
            let is_active = tab.index == active_tab;
            let title_text = if tab.unread_count > 0 {
                format!("({}) {}", tab.unread_count, tab.title)
            } else {
                tab.title.clone()
            };

            let text_width = (title_text.chars().count() as f32 * 7.2).clamp(72.0, 210.0);
            let tab_width = text_width + 44.0;
            let tab_height = 24.0;

            let tab_id = Id::new(("ts3_server_tab", tab.index));
            let (tab_rect, tab_resp) =
                ui.allocate_exact_size(Vec2::new(tab_width, tab_height), Sense::click());

            let close_rect = Rect::from_min_size(
                Pos2::new(tab_rect.right() - 18.0, tab_rect.center().y - 7.0),
                Vec2::splat(14.0),
            );
            let close_resp = ui.interact(close_rect, tab_id.with("close"), Sense::click());

            if ui.is_rect_visible(tab_rect) {
                let bg = if is_active {
                    palette.base_bg
                } else if tab_resp.hovered() {
                    palette.hover_bg
                } else {
                    palette.alternate_base_bg
                };

                ui.painter().rect_filled(
                    tab_rect,
                    CornerRadius {
                        nw: 3,
                        ne: 3,
                        sw: 0,
                        se: 0,
                    },
                    bg,
                );
                ui.painter().rect_stroke(
                    tab_rect,
                    CornerRadius {
                        nw: 3,
                        ne: 3,
                        sw: 0,
                        se: 0,
                    },
                    Stroke::new(1.0_f32, palette.border_color),
                    StrokeKind::Inside,
                );

                // Top active indicator line matching Qt QTabBar highlight
                if is_active {
                    ui.painter().line_segment(
                        [
                            Pos2::new(tab_rect.left() + 1.0, tab_rect.top() + 1.0),
                            Pos2::new(tab_rect.right() - 1.0, tab_rect.top() + 1.0),
                        ],
                        Stroke::new(2.0_f32, palette.accent_blue),
                    );
                }

                // Server status icon (`SERVER_GREEN` or painted circle fallback)
                let status_center = Pos2::new(tab_rect.left() + 12.0, tab_rect.center().y);
                let icon_rect = Rect::from_center_size(status_center, Vec2::splat(14.0));
                if tab.connected {
                    if let Some(tex) = assets.get_icon("SERVER_GREEN") {
                        ui.painter().image(
                            tex.id(),
                            icon_rect,
                            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                            Color32::WHITE,
                        );
                    } else {
                        ui.painter().circle_filled(
                            status_center,
                            5.0,
                            Color32::from_rgb(0x2E, 0xCC, 0x71),
                        );
                    }
                } else {
                    ui.painter().circle_filled(
                        status_center,
                        5.0,
                        Color32::from_rgb(0xE7, 0x4C, 0x3C),
                    );
                }

                // Server Title & Unread Highlight
                let text_color = if tab.unread_count > 0 {
                    palette.chat_poke
                } else if is_active {
                    palette.text_primary
                } else {
                    palette.text_muted
                };

                ui.painter().text(
                    Pos2::new(tab_rect.left() + 24.0, tab_rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    &title_text,
                    FontId::proportional(12.0),
                    text_color,
                );

                // Close Button ('×') (`ImprovedTabBarCloseButton`)
                if close_resp.hovered() {
                    ui.painter().rect_filled(
                        close_rect,
                        CornerRadius::same(2),
                        Color32::from_rgb(0xD9, 0x53, 0x4F),
                    );
                }
                if let Some(tex) = assets.get_icon("TAB_CLOSE_BUTTON") {
                    ui.painter().image(
                        tex.id(),
                        close_rect,
                        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                        Color32::WHITE,
                    );
                } else {
                    let close_fg = if close_resp.hovered() {
                        Color32::WHITE
                    } else {
                        palette.text_muted
                    };
                    ui.painter().text(
                        close_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "×",
                        FontId::proportional(13.0),
                        close_fg,
                    );
                }
            }

            if close_resp.clicked() || tab_resp.middle_clicked() {
                commands.push(UiCommand::CloseServerTab(tab.index));
            } else if tab_resp.clicked() && !is_active {
                commands.push(UiCommand::SelectServerTab(tab.index));
            }

            // Tab Context Menu (`onServerTabContextMenu`)
            tab_resp.context_menu(|ui| {
                if ui.button("Connect to New Tab").clicked() {
                    commands.push(UiCommand::NewServerTab);
                    ui.close_menu();
                }
                if ui
                    .add_enabled(tab.connected, egui::Button::new("Disconnect from Server"))
                    .clicked()
                {
                    if tab.index != active_tab {
                        commands.push(UiCommand::SelectServerTab(tab.index));
                    }
                    commands.push(UiCommand::DisconnectCurrent);
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Close Tab").clicked() {
                    commands.push(UiCommand::CloseServerTab(tab.index));
                    ui.close_menu();
                }
            });
        }

        // '+' New Server Tab button
        let (plus_rect, plus_resp) =
            ui.allocate_exact_size(Vec2::new(24.0, 24.0), Sense::click());
        if ui.is_rect_visible(plus_rect) {
            let bg = if plus_resp.hovered() {
                palette.hover_bg
            } else {
                palette.alternate_base_bg
            };
            ui.painter()
                .rect_filled(plus_rect, CornerRadius::same(3), bg);
            ui.painter().rect_stroke(
                plus_rect,
                CornerRadius::same(3),
                Stroke::new(1.0_f32, palette.border_color),
                StrokeKind::Inside,
            );
            ui.painter().text(
                plus_rect.center(),
                egui::Align2::CENTER_CENTER,
                "+",
                FontId::proportional(15.0),
                palette.text_primary,
            );
        }
        if plus_resp.on_hover_text("Open New Server Tab (Ctrl+N)").clicked() {
            commands.push(UiCommand::NewServerTab);
        }
    });
}

/// Selects the appropriate `PING_*` icon key based on connection state and RTT.
fn ping_icon_key(info: &StatusBarInfo) -> &'static str {
    if !info.connected {
        "PING_DISCONNECTED"
    } else if info.rtt_ms == 0 {
        "PING_CALCULATING"
    } else if info.rtt_ms < 60 {
        "PING_4"
    } else if info.rtt_ms < 120 {
        "PING_3"
    } else if info.rtt_ms < 200 {
        "PING_2"
    } else {
        "PING_1"
    }
}

/// Renders the bottom status bar (`QStatusBar`: `ServerStatusWidget`, `PingWidget`, and TS3 version label).
pub fn render_status_bar(
    ui: &mut egui::Ui,
    info: &StatusBarInfo,
    palette: &Ts3Palette,
    assets: &Ts3Assets,
) {
    ui.horizontal(|ui| {
        ui.set_min_height(20.0);
        ui.spacing_mut().item_spacing = Vec2::new(8.0, 0.0);

        // Status dot / icon
        let (dot_rect, _) = ui.allocate_exact_size(Vec2::splat(14.0), Sense::hover());
        if info.connected {
            if let Some(tex) = assets.get_icon("SERVER_GREEN") {
                ui.painter().image(
                    tex.id(),
                    dot_rect,
                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );
            } else {
                ui.painter().circle_filled(
                    dot_rect.center(),
                    4.5,
                    Color32::from_rgb(0x2E, 0xCC, 0x71),
                );
            }
        } else {
            ui.painter()
                .circle_filled(dot_rect.center(), 4.5, palette.chat_disconnected);
        }

        // Left connection status text
        let left_status = if !info.status_text.is_empty() {
            info.status_text.clone()
        } else if info.connected {
            if info.server_name.is_empty() {
                "Connected".to_string()
            } else {
                format!("Connected to {}", info.server_name)
            }
        } else {
            "Disconnected".to_string()
        };

        ui.label(
            RichText::new(left_status)
                .size(11.5)
                .color(palette.text_primary),
        );

        // Right-aligned diagnostics (`PingWidget`, `ServerStatusWidget`, TS3 version)
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(
                RichText::new(TS3_CLIENT_VERSION_LABEL)
                    .size(11.0)
                    .color(palette.text_muted),
            );

            ui.separator();

            let ping_color = if !info.connected {
                palette.text_muted
            } else if info.rtt_ms < 80 && info.packet_loss < 1.0 {
                Color32::from_rgb(0x2E, 0xCC, 0x71)
            } else if info.rtt_ms < 160 && info.packet_loss < 5.0 {
                Color32::from_rgb(0xF3, 0x9C, 0x12)
            } else {
                palette.chat_error
            };

            let ping_text = if info.connected {
                format!("Ping: {} ms | Loss: {:.2}%", info.rtt_ms, info.packet_loss)
            } else {
                "Ping: -- ms | Loss: 0.00%".to_string()
            };

            ui.label(RichText::new(ping_text).size(11.5).color(ping_color));

            if let Some(tex) = assets.get_icon(ping_icon_key(info)) {
                let (rect, _) = ui.allocate_exact_size(Vec2::splat(14.0), Sense::hover());
                ui.painter().image(
                    tex.id(),
                    rect,
                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );
            }

            if info.connected {
                ui.separator();

                ui.label(
                    RichText::new(format!(
                        "Clients: {}/{}",
                        info.clients_online, info.max_clients
                    ))
                    .size(11.5)
                    .color(palette.text_primary),
                );
            }
        });
    });
}

/// Global keyboard shortcut handler matching Windows TS3 Client (`MainWindow` accelerators).
pub fn handle_global_shortcuts(
    ctx: &egui::Context,
    state: &ToolbarState,
    commands: &mut Vec<UiCommand>,
) {
    ctx.input_mut(|input| {
        let ctrl_shift = Modifiers {
            ctrl: true,
            shift: true,
            ..Default::default()
        };
        let ctrl = Modifiers {
            ctrl: true,
            ..Default::default()
        };
        let alt = Modifiers {
            alt: true,
            ..Default::default()
        };

        // Check Ctrl+Shift combinations first so they are not shadowed by plain Ctrl
        if input.consume_key(ctrl_shift, Key::B) {
            if state.connected {
                commands.push(UiCommand::AddCurrentToBookmarks);
            }
        } else if input.consume_key(ctrl_shift, Key::M) {
            commands.push(UiCommand::ToggleMicMute);
        } else if input.consume_key(ctrl_shift, Key::D) || input.consume_key(ctrl_shift, Key::S) {
            commands.push(UiCommand::ToggleOutputMute);
        } else if input.consume_key(ctrl_shift, Key::K) {
            if state.connected {
                commands.push(UiCommand::SendRawCommand("privilegekeylist".to_string()));
            }
        } else if input.consume_key(ctrl_shift, Key::O) {
            commands.push(UiCommand::SendRawCommand("contacts".to_string()));
        } else if input.consume_key(ctrl_shift, Key::L) {
            if state.connected {
                commands.push(UiCommand::SendRawCommand("clientlist".to_string()));
            }
        } else if input.consume_key(ctrl_shift, Key::C) {
            commands.push(UiCommand::SendRawCommand("complainlist".to_string()));
        } else if input.consume_key(ctrl_shift, Key::W) {
            commands.push(UiCommand::SendRawCommand("whisperlists".to_string()));
        } else if input.consume_key(ctrl_shift, Key::H) {
            commands.push(UiCommand::SendRawCommand("whisperhistory".to_string()));
        }
        // Plain Ctrl combinations
        else if input.consume_key(ctrl, Key::S) {
            commands.push(UiCommand::OpenConnectDialog);
        } else if input.consume_key(ctrl, Key::D) {
            if state.connected {
                commands.push(UiCommand::DisconnectCurrent);
            }
        } else if input.consume_key(ctrl, Key::B) {
            commands.push(UiCommand::OpenBookmarksDialog);
        } else if input.consume_key(ctrl, Key::I) {
            commands.push(UiCommand::OpenIdentitiesDialog);
        } else if input.consume_key(ctrl, Key::P) {
            commands.push(UiCommand::OpenOptionsDialog);
        } else if input.consume_key(ctrl, Key::M) {
            commands.push(UiCommand::ToggleMicMute);
        } else if input.consume_key(ctrl, Key::Q) {
            commands.push(UiCommand::SendRawCommand("clientquit".to_string()));
        } else if input.consume_key(ctrl, Key::N) {
            commands.push(UiCommand::NewServerTab);
        } else if input.consume_key(ctrl, Key::T) {
            commands.push(UiCommand::SendRawCommand("ftlist".to_string()));
        } else if input.consume_key(ctrl, Key::L) {
            if state.connected {
                commands.push(UiCommand::SendRawCommand("logview".to_string()));
            }
        } else if input.consume_key(ctrl, Key::U) {
            commands.push(UiCommand::SendRawCommand("urlcatcher".to_string()));
        } else if input.consume_key(ctrl, Key::F1) {
            if state.connected {
                commands.push(UiCommand::SendRawCommand("servergrouplist".to_string()));
            }
        } else if input.consume_key(ctrl, Key::F2) {
            if state.connected {
                commands.push(UiCommand::SendRawCommand("channelgrouplist".to_string()));
            }
        } else if input.consume_key(ctrl, Key::F3) {
            if state.connected {
                commands.push(UiCommand::SendRawCommand("channelpermlist".to_string()));
            }
        } else if input.consume_key(ctrl, Key::F4) {
            if state.connected {
                commands.push(UiCommand::SendRawCommand("clientpermlist".to_string()));
            }
        } else if input.consume_key(ctrl, Key::F5) {
            if state.connected {
                commands.push(UiCommand::SendRawCommand(
                    "channelclientpermlist".to_string(),
                ));
            }
        }
        // Alt combinations
        else if input.consume_key(alt, Key::P) {
            commands.push(UiCommand::OpenOptionsDialog);
        } else if input.consume_key(alt, Key::A) {
            let next_away = !state.away;
            let msg = if next_away && !state.away_message.is_empty() {
                Some(state.away_message.clone())
            } else {
                None
            };
            commands.push(UiCommand::ToggleAway {
                away: next_away,
                message: msg,
            });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toolbar_state_default_matches_ts3_client() {
        let state = ToolbarState::default();
        assert!(!state.connected);
        assert!(!state.away);
        assert!(state.away_message.is_empty());
        assert!(!state.input_muted);
        assert!(!state.output_muted);
        assert!(state.subscribed_all);
        assert!(!state.push_to_talk_active);
        assert_eq!(state.theme_mode, ThemeMode::Light);
    }

    #[test]
    fn status_bar_info_and_ping_icon_selection() {
        let mut info = StatusBarInfo::default();
        assert_eq!(ping_icon_key(&info), "PING_DISCONNECTED");

        info.connected = true;
        info.rtt_ms = 0;
        assert_eq!(ping_icon_key(&info), "PING_CALCULATING");

        info.rtt_ms = 24;
        assert_eq!(ping_icon_key(&info), "PING_4");

        info.rtt_ms = 85;
        assert_eq!(ping_icon_key(&info), "PING_3");

        info.rtt_ms = 150;
        assert_eq!(ping_icon_key(&info), "PING_2");

        info.rtt_ms = 260;
        assert_eq!(ping_icon_key(&info), "PING_1");
    }

    #[test]
    fn render_headless_menu_toolbar_tabs_and_statusbar() {
        let ctx = egui::Context::default();
        let mut state = ToolbarState {
            connected: true,
            away: true,
            away_message: "AFK".to_string(),
            ..ToolbarState::default()
        };
        let palette = Ts3Palette::light();
        let assets = Ts3Assets::new();
        let mut commands = Vec::new();
        let tabs = vec![
            ServerTabSummary {
                index: 0,
                title: "TeamSpeak Public".to_string(),
                connected: true,
                unread_count: 0,
            },
            ServerTabSummary {
                index: 1,
                title: "Clan Server".to_string(),
                connected: false,
                unread_count: 3,
            },
        ];
        let status_info = StatusBarInfo {
            status_text: "Connected".to_string(),
            connected: true,
            server_name: "TeamSpeak Public".to_string(),
            rtt_ms: 24,
            packet_loss: 0.0,
            clients_online: 42,
            max_clients: 128,
        };

        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            handle_global_shortcuts(ctx, &state, &mut commands);
            egui::CentralPanel::default().show(ctx, |ui| {
                render_menu_bar(ui, &state, &palette, &assets, &mut commands);
                render_main_toolbar(ui, &mut state, &palette, &assets, &mut commands);
                render_server_tab_bar(ui, &tabs, 0, &palette, &assets, &mut commands);
                render_status_bar(ui, &status_info, &palette, &assets);
            });
        });

        assert!(commands.is_empty());
    }

    fn dispatch_shortcut(
        ctx: &egui::Context,
        state: &ToolbarState,
        key: Key,
        modifiers: Modifiers,
    ) -> Vec<UiCommand> {
        let mut commands = Vec::new();
        let mut raw_input = egui::RawInput::default();
        raw_input.modifiers = modifiers;
        raw_input.events.push(egui::Event::Key {
            key,
            physical_key: Some(key),
            pressed: true,
            repeat: false,
            modifiers,
        });

        let _ = ctx.run(raw_input, |ctx| {
            handle_global_shortcuts(ctx, state, &mut commands);
        });
        commands
    }

    #[test]
    fn global_shortcuts_emit_expected_ui_commands() {
        let ctx = egui::Context::default();
        let mut state = ToolbarState {
            connected: true,
            away: false,
            away_message: "Lunch".to_string(),
            ..ToolbarState::default()
        };

        let ctrl = Modifiers {
            ctrl: true,
            ..Default::default()
        };
        let ctrl_shift = Modifiers {
            ctrl: true,
            shift: true,
            ..Default::default()
        };
        let alt = Modifiers {
            alt: true,
            ..Default::default()
        };

        assert_eq!(
            dispatch_shortcut(&ctx, &state, Key::S, ctrl),
            vec![UiCommand::OpenConnectDialog]
        );
        assert_eq!(
            dispatch_shortcut(&ctx, &state, Key::D, ctrl),
            vec![UiCommand::DisconnectCurrent]
        );
        assert_eq!(
            dispatch_shortcut(&ctx, &state, Key::B, ctrl),
            vec![UiCommand::OpenBookmarksDialog]
        );
        assert_eq!(
            dispatch_shortcut(&ctx, &state, Key::B, ctrl_shift),
            vec![UiCommand::AddCurrentToBookmarks]
        );
        assert_eq!(
            dispatch_shortcut(&ctx, &state, Key::I, ctrl),
            vec![UiCommand::OpenIdentitiesDialog]
        );
        assert_eq!(
            dispatch_shortcut(&ctx, &state, Key::P, ctrl),
            vec![UiCommand::OpenOptionsDialog]
        );
        assert_eq!(
            dispatch_shortcut(&ctx, &state, Key::P, alt),
            vec![UiCommand::OpenOptionsDialog]
        );
        assert_eq!(
            dispatch_shortcut(&ctx, &state, Key::M, ctrl),
            vec![UiCommand::ToggleMicMute]
        );
        assert_eq!(
            dispatch_shortcut(&ctx, &state, Key::M, ctrl_shift),
            vec![UiCommand::ToggleMicMute]
        );
        assert_eq!(
            dispatch_shortcut(&ctx, &state, Key::D, ctrl_shift),
            vec![UiCommand::ToggleOutputMute]
        );
        assert_eq!(
            dispatch_shortcut(&ctx, &state, Key::A, alt),
            vec![UiCommand::ToggleAway {
                away: true,
                message: Some("Lunch".to_string()),
            }]
        );
        assert_eq!(
            dispatch_shortcut(&ctx, &state, Key::F1, ctrl),
            vec![UiCommand::SendRawCommand("servergrouplist".to_string())]
        );
        assert_eq!(
            dispatch_shortcut(&ctx, &state, Key::F5, ctrl),
            vec![UiCommand::SendRawCommand(
                "channelclientpermlist".to_string()
            )]
        );

        // Disconnected state suppresses server-bound shortcuts
        state.connected = false;
        assert!(dispatch_shortcut(&ctx, &state, Key::D, ctrl).is_empty());
        assert!(dispatch_shortcut(&ctx, &state, Key::B, ctrl_shift).is_empty());
        assert!(dispatch_shortcut(&ctx, &state, Key::F1, ctrl).is_empty());
    }
}
