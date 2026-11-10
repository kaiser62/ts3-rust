// Owned by Pool Agent 1 (arthoroy.in@gmail.com)
// Exact QSS Light (`default.qss`) & Dark (`default_darkmode.qss`) themes, `Ts3Palette`,
// banner 4-stop vertical gradient, splitter handle painter, and typography from `docs/re/gui/03-qss-themes.md`.

use egui::{
    epaint::{Vertex, WHITE_UV},
    style::{Selection, WidgetVisuals},
    vec2, Color32, Context, CornerRadius, FontId, Margin, Mesh, Rect, Response, Sense,
    Shape, Stroke, StrokeKind, Ui, Visuals,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeMode {
    #[default]
    Light,
    Dark,
}

/// 4-stop vertical banner gradient definition from `default.qss:40-44`
pub const BANNER_GRADIENT_STOPS: [(f32, Color32); 4] = [
    (0.00_f32, Color32::from_rgb(0x50, 0x5C, 0x78)), // stop 0: 0.00  rgb(80, 92, 120)  #505C78
    (0.08_f32, Color32::from_rgb(0x85, 0x93, 0xAE)), // stop 1: 0.08  rgb(133, 147, 174) #8593AE
    (0.32_f32, Color32::from_rgb(0x58, 0x6B, 0x8C)), // stop 2: 0.32  rgb(88, 107, 140)  #586B8C
    (1.00_f32, Color32::from_rgb(0x6F, 0x80, 0xAA)), // stop 3: 1.00  rgb(111, 128, 170) #6F80AA
];

/// Exact TeamSpeak 3 Chat and InfoFrame Color Palette (`default_chat.qss` / `default_chat_darkmode.qss`)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ts3ChatPalette {
    pub link: Color32,
    pub user_link: Color32,
    pub channel_link: Color32,
    pub server_link: Color32,
    pub offline_link: Color32,
    pub host_name: Color32,
    pub welcome: Color32,
    pub host_msg: Color32,
    pub poke: Color32,
    pub normal: Color32,
    pub connected: Color32,
    pub disconnected: Color32,
    pub client_joined: Color32,
    pub client_left: Color32,
    pub client_switched: Color32,
    pub client_dropped: Color32,
    pub client_kicked: Color32,
    pub client_banned: Color32,
    pub channel_event: Color32,
    pub group_event: Color32,
    pub talk_power_event: Color32,
    pub time_stamp: Color32,
    pub history_marker: Color32,
    pub server_error: Color32,
    pub info_title_link: Color32,
    pub info_custom_nick: Color32,
    pub highlight: Color32,
    pub important: Color32,
    pub active: Color32,
    pub inactive: Color32,
    pub input_help_text: Color32,
    pub input_normal_text: Color32,
    pub client_friend: Color32,
    pub client_blocked: Color32,
    pub client_recording: Color32,
}

pub const TS3_LIGHT_CHAT_PALETTE: Ts3ChatPalette = Ts3ChatPalette {
    link: Color32::from_rgb(0x1C, 0x82, 0xCC),         // default.qss:7 #1c82cc
    user_link: Color32::from_rgb(0x00, 0x2F, 0x5D),    // default_chat.qss:99 #002F5D
    channel_link: Color32::from_rgb(0x00, 0x2F, 0x5D), // default_chat.qss:107 #002F5D
    server_link: Color32::from_rgb(0x00, 0x2F, 0x5D),  // default_chat.qss:108 #002F5D
    offline_link: Color32::from_rgb(0x00, 0x2F, 0x5D), // default_chat.qss:106 #002F5D
    host_name: Color32::from_rgb(0x33, 0x59, 0x7D),    // default_chat.qss:117 #33597D
    welcome: Color32::from_rgb(0x0A, 0xA5, 0x37),      // default_chat.qss:124 #0AA537
    host_msg: Color32::from_rgb(0x0A, 0xA5, 0x37),     // default_chat.qss:124 #0AA537
    poke: Color32::from_rgb(0x0A, 0xA5, 0x37),         // default_chat.qss:125 #0AA537
    normal: Color32::from_rgb(0x33, 0x59, 0x7D),       // default_chat.qss:131 #33597D
    connected: Color32::from_rgb(0x33, 0x59, 0x7D),    // default_chat.qss:132 #33597D
    disconnected: Color32::from_rgb(0x33, 0x59, 0x7D), // default_chat.qss:135 #33597D
    client_joined: Color32::from_rgb(0x33, 0x59, 0x7D),// default_chat.qss:134 #33597D
    client_left: Color32::from_rgb(0x33, 0x59, 0x7D),  // default_chat.qss:138 #33597D
    client_switched: Color32::from_rgb(0x33, 0x59, 0x7D), // default_chat.qss:137 #33597D
    client_dropped: Color32::from_rgb(0xC9, 0x07, 0x0A),  // default_chat.qss:144 #C9070A
    client_kicked: Color32::from_rgb(0xC9, 0x07, 0x0A),   // default_chat.qss:144 #C9070A
    client_banned: Color32::from_rgb(0xC9, 0x07, 0x0A),   // default_chat.qss:145 #C9070A
    channel_event: Color32::from_rgb(0x33, 0x59, 0x7D),   // default_chat.qss:147-151 #33597D
    group_event: Color32::from_rgb(0x33, 0x59, 0x7D),     // default_chat.qss:155-156 #33597D
    talk_power_event: Color32::from_rgb(0x33, 0x59, 0x7D),// default_chat.qss:157-159 #33597D
    time_stamp: Color32::from_rgb(0x33, 0x59, 0x7D),      // default_chat.qss:168 #33597D
    history_marker: Color32::from_rgb(0x70, 0x74, 0x75),  // default_chat.qss:174 #707475
    server_error: Color32::from_rgb(0xC9, 0x07, 0x0A),    // default_chat.qss:184 #C9070A
    info_title_link: Color32::from_rgb(0x19, 0x19, 0x70), // default_chat.qss:199 #191970
    info_custom_nick: Color32::from_rgb(0x1C, 0x82, 0xCC),// default_chat.qss:204 #1C82CC
    highlight: Color32::from_rgb(0x1C, 0x82, 0xCC),       // default_chat.qss:218 #1C82CC
    important: Color32::from_rgb(0xFF, 0x00, 0x00),       // default_chat.qss:223 #FF0000
    active: Color32::from_rgb(0x00, 0x80, 0x00),          // default_chat.qss:228 #008000
    inactive: Color32::from_rgb(0x80, 0x80, 0x80),        // default_chat.qss:233 #808080
    input_help_text: Color32::from_rgb(0x90, 0x90, 0x90), // default.qss:56 #909090
    input_normal_text: Color32::from_rgb(0x00, 0x00, 0x00),
    client_friend: Color32::from_rgb(0x00, 0x80, 0x00),   // default.qss:10 #008000
    client_blocked: Color32::from_rgb(0xFF, 0x00, 0x00),  // default.qss:11 #ff0000
    client_recording: Color32::from_rgb(0x80, 0x00, 0x00),// default.qss:12 #800000
};

pub const TS3_DARK_CHAT_PALETTE: Ts3ChatPalette = Ts3ChatPalette {
    link: Color32::from_rgb(0x00, 0x6A, 0xD2),         // default_chat_darkmode.qss:12 #006ad2
    user_link: Color32::from_rgb(0x00, 0x6A, 0xD2),    // default_chat_darkmode.qss:12 #006ad2
    channel_link: Color32::from_rgb(0x00, 0x6A, 0xD2), // default_chat_darkmode.qss:12 #006ad2
    server_link: Color32::from_rgb(0x00, 0x6A, 0xD2),  // default_chat_darkmode.qss:12 #006ad2
    offline_link: Color32::from_rgb(0x00, 0x6A, 0xD2), // default_chat_darkmode.qss:12 #006ad2
    host_name: Color32::from_rgb(0x83, 0xAC, 0xD2),    // default_chat_darkmode.qss:47 #83acd2
    welcome: Color32::from_rgb(0x11, 0xDE, 0x4D),      // default_chat_darkmode.qss:18 #11de4d
    host_msg: Color32::from_rgb(0x11, 0xDE, 0x4D),     // default_chat_darkmode.qss:18 #11de4d
    poke: Color32::from_rgb(0x11, 0xDE, 0x4D),         // default_chat_darkmode.qss:19 #11de4d
    normal: Color32::from_rgb(0x83, 0xAC, 0xD2),       // default_chat_darkmode.qss:47 #83acd2
    connected: Color32::from_rgb(0x83, 0xAC, 0xD2),    // default_chat_darkmode.qss:47 #83acd2
    disconnected: Color32::from_rgb(0x83, 0xAC, 0xD2), // default_chat_darkmode.qss:47 #83acd2
    client_joined: Color32::from_rgb(0x83, 0xAC, 0xD2),// default_chat_darkmode.qss:47 #83acd2
    client_left: Color32::from_rgb(0x83, 0xAC, 0xD2),  // default_chat_darkmode.qss:47 #83acd2
    client_switched: Color32::from_rgb(0x83, 0xAC, 0xD2), // default_chat_darkmode.qss:47 #83acd2
    client_dropped: Color32::from_rgb(0xEC, 0x21, 0x24),  // default_chat_darkmode.qss:24 #ec2124
    client_kicked: Color32::from_rgb(0xEC, 0x21, 0x24),   // default_chat_darkmode.qss:24 #ec2124
    client_banned: Color32::from_rgb(0xEC, 0x21, 0x24),   // default_chat_darkmode.qss:25 #ec2124
    channel_event: Color32::from_rgb(0x83, 0xAC, 0xD2),   // default_chat_darkmode.qss:47 #83acd2
    group_event: Color32::from_rgb(0x83, 0xAC, 0xD2),     // default_chat_darkmode.qss:47 #83acd2
    talk_power_event: Color32::from_rgb(0x83, 0xAC, 0xD2),// default_chat_darkmode.qss:47 #83acd2
    time_stamp: Color32::from_rgb(0x83, 0xAC, 0xD2),      // default_chat_darkmode.qss:47 #83acd2
    history_marker: Color32::from_rgb(0xBC, 0xBC, 0xBC),  // default_chat_darkmode.qss:52 #bcbcbc
    server_error: Color32::from_rgb(0xF7, 0x3C, 0x3F),    // default_chat_darkmode.qss:56 #f73c3f
    info_title_link: Color32::from_rgb(0xB0, 0xC4, 0xDE), // default_chat_darkmode.qss:62 #B0C4DE
    info_custom_nick: Color32::from_rgb(0x00, 0x6A, 0xD2),// default_chat_darkmode.qss:66 #006ad2
    highlight: Color32::from_rgb(0x00, 0x6A, 0xD2),       // default_chat_darkmode.qss:71 #006ad2
    important: Color32::from_rgb(0xFF, 0x00, 0x00),       // default_chat_darkmode.qss:76 #FF0000
    active: Color32::from_rgb(0x90, 0xEE, 0x90),          // default_chat_darkmode.qss:81 #90EE90
    inactive: Color32::from_rgb(0xD3, 0xD3, 0xD3),        // default_chat_darkmode.qss:86 #D3D3D3
    input_help_text: Color32::from_rgb(0x70, 0x70, 0x70),
    input_normal_text: Color32::from_rgb(0xDF, 0xDF, 0xDF),
    client_friend: Color32::from_rgb(0x2E, 0xC0, 0x2E),
    client_blocked: Color32::from_rgb(0xFF, 0x4D, 0x4D),
    client_recording: Color32::from_rgb(0xD9, 0x53, 0x4F),
};

#[derive(Debug, Clone)]
pub struct Ts3Palette {
    pub mode: ThemeMode,
    pub window_bg: Color32,
    pub base_bg: Color32,
    pub alternate_base_bg: Color32,
    pub text_primary: Color32,
    pub text_muted: Color32,
    pub accent_blue: Color32,
    pub border_color: Color32,
    pub selection_bg: Color32,
    pub selection_fg: Color32,
    pub hover_bg: Color32,
    pub chat_time: Color32,
    pub chat_normal: Color32,
    pub chat_server: Color32,
    pub chat_channel: Color32,
    pub chat_client: Color32,
    pub chat_error: Color32,
    pub chat_poke: Color32,
    pub chat_link: Color32,
    pub chat_disconnected: Color32,

    // Expanded QSS chat palette & roles
    pub chat: Ts3ChatPalette,
    pub splitter_handle: Color32,
    pub msg_user_link: Color32,
    pub msg_channel_link: Color32,
    pub msg_server_link: Color32,
    pub msg_offline_link: Color32,
    pub msg_host_name: Color32,
    pub msg_welcome: Color32,
    pub msg_host_message: Color32,
    pub msg_poke: Color32,
    pub msg_normal: Color32,
    pub msg_connected: Color32,
    pub msg_disconnected: Color32,
    pub msg_client_joined: Color32,
    pub msg_client_left: Color32,
    pub msg_client_switched: Color32,
    pub msg_client_dropped: Color32,
    pub msg_client_kicked: Color32,
    pub msg_client_banned: Color32,
    pub msg_channel_created: Color32,
    pub msg_channel_deleted: Color32,
    pub msg_channel_moved: Color32,
    pub msg_channel_edited: Color32,
    pub msg_server_edited: Color32,
    pub msg_server_error: Color32,
    pub msg_group_assigned: Color32,
    pub msg_group_revoked: Color32,
    pub msg_talk_power_assigned: Color32,
    pub msg_talk_power_revoked: Color32,
    pub msg_talk_power_requested: Color32,
    pub msg_time: Color32,
    pub msg_begin_history: Color32,
    pub msg_end_history: Color32,
    pub info_title_link: Color32,
    pub info_custom_nick: Color32,
    pub highlight: Color32,
    pub important: Color32,
    pub active: Color32,
    pub inactive: Color32,
    pub input_help_text: Color32,
    pub client_friend: Color32,
    pub client_blocked: Color32,
    pub client_recording: Color32,
}

impl Ts3Palette {
    pub fn light() -> Self {
        let chat = TS3_LIGHT_CHAT_PALETTE;
        Self {
            mode: ThemeMode::Light,
            window_bg: Color32::from_rgb(0xEC, 0xEC, 0xEC),       // default.qss #ececec
            base_bg: Color32::from_rgb(0xFF, 0xFF, 0xFF),         // base view #ffffff
            alternate_base_bg: Color32::from_rgb(0xF6, 0xF6, 0xF6), // alt rows #f6f6f6
            text_primary: Color32::from_rgb(0x1E, 0x1E, 0x1E),    // text #1e1e1e
            text_muted: Color32::from_rgb(0x6E, 0x6E, 0x6E),      // muted #6e6e6e
            accent_blue: Color32::from_rgb(0x2F, 0x71, 0xAF),     // brand blue #2f71af
            border_color: Color32::from_rgb(0xC8, 0xC8, 0xC8),    // border #c8c8c8
            selection_bg: Color32::from_rgb(0x33, 0x99, 0xFF),    // selection #3399ff
            selection_fg: Color32::from_rgb(0xFF, 0xFF, 0xFF),    // sel fg #ffffff
            hover_bg: Color32::from_rgb(0xE5, 0xF3, 0xFF),        // hover #e5f3ff
            chat_time: chat.time_stamp,                           // #33597D
            chat_normal: chat.normal,                             // #33597D
            chat_server: chat.server_link,                        // #002F5D
            chat_channel: chat.welcome,                           // #0AA537
            chat_client: chat.user_link,                          // #002F5D
            chat_error: chat.server_error,                        // #C9070A
            chat_poke: chat.poke,                                 // #0AA537
            chat_link: chat.link,                                 // #1C82CC
            chat_disconnected: chat.disconnected,                 // #33597D

            chat,
            splitter_handle: Color32::from_rgb(0xB4, 0xB4, 0xB4), // splitter handle
            msg_user_link: chat.user_link,
            msg_channel_link: chat.channel_link,
            msg_server_link: chat.server_link,
            msg_offline_link: chat.offline_link,
            msg_host_name: chat.host_name,
            msg_welcome: chat.welcome,
            msg_host_message: chat.host_msg,
            msg_poke: chat.poke,
            msg_normal: chat.normal,
            msg_connected: chat.connected,
            msg_disconnected: chat.disconnected,
            msg_client_joined: chat.client_joined,
            msg_client_left: chat.client_left,
            msg_client_switched: chat.client_switched,
            msg_client_dropped: chat.client_dropped,
            msg_client_kicked: chat.client_kicked,
            msg_client_banned: chat.client_banned,
            msg_channel_created: chat.channel_event,
            msg_channel_deleted: chat.channel_event,
            msg_channel_moved: chat.channel_event,
            msg_channel_edited: chat.channel_event,
            msg_server_edited: chat.channel_event,
            msg_server_error: chat.server_error,
            msg_group_assigned: chat.group_event,
            msg_group_revoked: chat.group_event,
            msg_talk_power_assigned: chat.talk_power_event,
            msg_talk_power_revoked: chat.talk_power_event,
            msg_talk_power_requested: chat.talk_power_event,
            msg_time: chat.time_stamp,
            msg_begin_history: chat.history_marker,
            msg_end_history: chat.history_marker,
            info_title_link: chat.info_title_link,
            info_custom_nick: chat.info_custom_nick,
            highlight: chat.highlight,
            important: chat.important,
            active: chat.active,
            inactive: chat.inactive,
            input_help_text: chat.input_help_text,
            client_friend: chat.client_friend,
            client_blocked: chat.client_blocked,
            client_recording: chat.client_recording,
        }
    }

    pub fn dark() -> Self {
        let chat = TS3_DARK_CHAT_PALETTE;
        Self {
            mode: ThemeMode::Dark,
            window_bg: Color32::from_rgb(0x1A, 0x1A, 0x1A),       // default_darkmode.qss #1a1a1a
            base_bg: Color32::from_rgb(0x36, 0x36, 0x36),         // base view #363636
            alternate_base_bg: Color32::from_rgb(0x2D, 0x2D, 0x2D), // alt rows #2d2d2d
            text_primary: Color32::from_rgb(0xCE, 0xCE, 0xCE),    // text #cecece
            text_muted: Color32::from_rgb(0x8E, 0x8E, 0x8E),      // muted #8e8e8e
            accent_blue: Color32::from_rgb(0x3A, 0x82, 0xC4),     // blue #3a82c4
            border_color: Color32::from_rgb(0x46, 0x46, 0x46),    // border #464646
            selection_bg: Color32::from_rgb(0x2F, 0x71, 0xAF),    // selection #2f71af
            selection_fg: Color32::from_rgb(0xFF, 0xFF, 0xFF),    // sel fg #ffffff
            hover_bg: Color32::from_rgb(0x42, 0x42, 0x42),        // hover #424242
            chat_time: chat.time_stamp,                           // #83ACD2
            chat_normal: chat.normal,                             // #83ACD2
            chat_server: chat.server_link,                        // #006AD2
            chat_channel: chat.welcome,                           // #11DE4D
            chat_client: chat.user_link,                          // #006AD2
            chat_error: chat.server_error,                        // #F73C3F
            chat_poke: chat.poke,                                 // #11DE4D
            chat_link: chat.link,                                 // #006AD2
            chat_disconnected: chat.disconnected,                 // #83ACD2

            chat,
            splitter_handle: Color32::from_rgb(0x55, 0x55, 0x55), // splitter handle
            msg_user_link: chat.user_link,
            msg_channel_link: chat.channel_link,
            msg_server_link: chat.server_link,
            msg_offline_link: chat.offline_link,
            msg_host_name: chat.host_name,
            msg_welcome: chat.welcome,
            msg_host_message: chat.host_msg,
            msg_poke: chat.poke,
            msg_normal: chat.normal,
            msg_connected: chat.connected,
            msg_disconnected: chat.disconnected,
            msg_client_joined: chat.client_joined,
            msg_client_left: chat.client_left,
            msg_client_switched: chat.client_switched,
            msg_client_dropped: chat.client_dropped,
            msg_client_kicked: chat.client_kicked,
            msg_client_banned: chat.client_banned,
            msg_channel_created: chat.channel_event,
            msg_channel_deleted: chat.channel_event,
            msg_channel_moved: chat.channel_event,
            msg_channel_edited: chat.channel_event,
            msg_server_edited: chat.channel_event,
            msg_server_error: chat.server_error,
            msg_group_assigned: chat.group_event,
            msg_group_revoked: chat.group_event,
            msg_talk_power_assigned: chat.talk_power_event,
            msg_talk_power_revoked: chat.talk_power_event,
            msg_talk_power_requested: chat.talk_power_event,
            msg_time: chat.time_stamp,
            msg_begin_history: chat.history_marker,
            msg_end_history: chat.history_marker,
            info_title_link: chat.info_title_link,
            info_custom_nick: chat.info_custom_nick,
            highlight: chat.highlight,
            important: chat.important,
            active: chat.active,
            inactive: chat.inactive,
            input_help_text: chat.input_help_text,
            client_friend: chat.client_friend,
            client_blocked: chat.client_blocked,
            client_recording: chat.client_recording,
        }
    }

    pub fn for_mode(mode: ThemeMode) -> Self {
        match mode {
            ThemeMode::Light => Self::light(),
            ThemeMode::Dark => Self::dark(),
        }
    }
}

/// 1. Paint the 4-stop vertical header/banner gradient (`default.qss:40-44`)
pub fn paint_banner_gradient(ui: &mut Ui, rect: Rect, _palette: &Ts3Palette) {
    if !ui.is_rect_visible(rect) || rect.height() <= 0.0_f32 || rect.width() <= 0.0_f32 {
        return;
    }
    let height = rect.height();
    let stops = BANNER_GRADIENT_STOPS;

    let mut mesh = Mesh::default();
    for i in 0..stops.len() - 1 {
        let (y0_ratio, c0) = stops[i];
        let (y1_ratio, c1) = stops[i + 1];
        let y0 = rect.min.y + y0_ratio * height;
        let y1 = rect.min.y + y1_ratio * height;
        let idx = mesh.vertices.len() as u32;
        mesh.vertices.push(Vertex {
            pos: egui::pos2(rect.min.x, y0),
            uv: WHITE_UV,
            color: c0,
        });
        mesh.vertices.push(Vertex {
            pos: egui::pos2(rect.max.x, y0),
            uv: WHITE_UV,
            color: c0,
        });
        mesh.vertices.push(Vertex {
            pos: egui::pos2(rect.max.x, y1),
            uv: WHITE_UV,
            color: c1,
        });
        mesh.vertices.push(Vertex {
            pos: egui::pos2(rect.min.x, y1),
            uv: WHITE_UV,
            color: c1,
        });
        mesh.indices.extend([idx, idx + 1, idx + 2, idx, idx + 2, idx + 3]);
    }
    ui.painter().add(Shape::mesh(mesh));
}

/// 2. Paint the 1:1 splitter grip handle (`default.qss:21-36`)
pub fn paint_splitter_handle(ui: &mut Ui, rect: Rect, vertical: bool, palette: &Ts3Palette) {
    if !ui.is_rect_visible(rect) {
        return;
    }
    let painter = ui.painter();

    // 2px thick bar centered in rect
    let bar_rect = if vertical {
        Rect::from_center_size(rect.center(), vec2(2.0_f32, rect.height().max(0.0_f32)))
    } else {
        Rect::from_center_size(rect.center(), vec2(rect.width().max(0.0_f32), 2.0_f32))
    };
    painter.rect_filled(bar_rect, CornerRadius::ZERO, palette.border_color);

    // 5-dot grip handle in center
    let dot_size = vec2(2.0_f32, 2.0_f32);
    let grip_color = palette.splitter_handle;
    for i in -2..=2 {
        let dot_pos = if vertical {
            egui::pos2(rect.center().x, rect.center().y + (i as f32) * 4.0_f32)
        } else {
            egui::pos2(rect.center().x + (i as f32) * 4.0_f32, rect.center().y)
        };
        painter.rect_filled(
            Rect::from_center_size(dot_pos, dot_size),
            CornerRadius::ZERO,
            grip_color,
        );
    }
}

/// 3. Paint tree row selection/hover background (`default.qss:88-91` + Windows Vista tree metrics)
pub fn paint_tree_row_bg(
    ui: &mut Ui,
    rect: Rect,
    selected: bool,
    hovered: bool,
    palette: &Ts3Palette,
) {
    if !ui.is_rect_visible(rect) {
        return;
    }
    let painter = ui.painter();
    let cr = CornerRadius::same(2);
    if selected {
        painter.rect_filled(rect, cr, palette.selection_bg);
        if hovered {
            painter.rect_stroke(
                rect,
                cr,
                Stroke::new(1.0_f32, palette.selection_fg.gamma_multiply(0.35_f32)),
                StrokeKind::Inside,
            );
        }
    } else if hovered {
        painter.rect_filled(rect, cr, palette.hover_bg);
    }
}

/// 4. Paint badge pill frames (`default_chat.qss:245-247`)
pub fn paint_group_badge(
    ui: &mut Ui,
    text: &str,
    color: Color32,
    palette: &Ts3Palette,
) -> Response {
    let font_id = FontId::monospace(10.5_f32);
    let galley = ui.painter().layout_no_wrap(text.to_string(), font_id, color);
    let padding = vec2(5.0_f32, 1.5_f32);
    let desired_size = galley.size() + padding * 2.0_f32;
    let (rect, response) = ui.allocate_exact_size(desired_size, Sense::hover());

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let bg_alpha = match palette.mode {
            ThemeMode::Light => 26,
            ThemeMode::Dark => 45,
        };
        let bg_color = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), bg_alpha);
        let cr = CornerRadius::same(3);
        painter.rect_filled(rect, cr, bg_color);
        painter.rect_stroke(
            rect,
            cr,
            Stroke::new(1.0_f32, color),
            StrokeKind::Inside,
        );
        let text_pos = egui::pos2(rect.min.x + padding.x, rect.min.y + padding.y);
        painter.galley(text_pos, galley, color);
    }

    response
}

/// Apply complete TS3 theme to `egui::Context`
pub fn apply_ts3_theme(ctx: &Context, mode: ThemeMode) {
    let pal = Ts3Palette::for_mode(mode);
    let mut visuals = match mode {
        ThemeMode::Light => Visuals::light(),
        ThemeMode::Dark => Visuals::dark(),
    };

    visuals.dark_mode = mode == ThemeMode::Dark;
    visuals.panel_fill = pal.window_bg;
    visuals.window_fill = pal.window_bg;
    visuals.extreme_bg_color = pal.base_bg;
    visuals.faint_bg_color = pal.alternate_base_bg;
    visuals.striped = true;
    visuals.window_corner_radius = CornerRadius::same(2);
    visuals.menu_corner_radius = CornerRadius::same(2);
    visuals.window_stroke = Stroke::new(1.0_f32, pal.border_color);
    visuals.selection = Selection {
        bg_fill: pal.selection_bg,
        stroke: Stroke::new(1.0_f32, pal.accent_blue),
    };
    visuals.hyperlink_color = pal.chat_link;
    visuals.error_fg_color = pal.chat_error;
    visuals.warn_fg_color = Color32::from_rgb(255, 140, 0);

    let cr = CornerRadius::same(2);
    match mode {
        ThemeMode::Light => {
            visuals.widgets.noninteractive = WidgetVisuals {
                bg_fill: Color32::from_rgb(244, 244, 244),
                weak_bg_fill: Color32::from_rgb(244, 244, 244),
                bg_stroke: Stroke::new(1.0_f32, Color32::from_rgb(220, 220, 220)),
                corner_radius: cr,
                fg_stroke: Stroke::new(1.0_f32, pal.text_muted),
                expansion: 0.0_f32,
            };
            visuals.widgets.inactive = WidgetVisuals {
                bg_fill: Color32::from_rgb(225, 225, 225),
                weak_bg_fill: Color32::from_rgb(225, 225, 225),
                bg_stroke: Stroke::new(1.0_f32, Color32::from_rgb(173, 173, 173)),
                corner_radius: cr,
                fg_stroke: Stroke::new(1.0_f32, pal.text_primary),
                expansion: 0.0_f32,
            };
            visuals.widgets.hovered = WidgetVisuals {
                bg_fill: Color32::from_rgb(229, 241, 251),
                weak_bg_fill: Color32::from_rgb(229, 241, 251),
                bg_stroke: Stroke::new(1.0_f32, Color32::from_rgb(0, 120, 215)),
                corner_radius: cr,
                fg_stroke: Stroke::new(1.0_f32, pal.text_primary),
                expansion: 0.0_f32,
            };
            visuals.widgets.active = WidgetVisuals {
                bg_fill: Color32::from_rgb(204, 228, 247),
                weak_bg_fill: Color32::from_rgb(204, 228, 247),
                bg_stroke: Stroke::new(1.0_f32, Color32::from_rgb(0, 84, 153)),
                corner_radius: cr,
                fg_stroke: Stroke::new(1.0_f32, pal.text_primary),
                expansion: 0.0_f32,
            };
            visuals.widgets.open = WidgetVisuals {
                bg_fill: Color32::WHITE,
                weak_bg_fill: Color32::WHITE,
                bg_stroke: Stroke::new(1.0_f32, Color32::from_rgb(0, 120, 215)),
                corner_radius: cr,
                fg_stroke: Stroke::new(1.0_f32, pal.text_primary),
                expansion: 0.0_f32,
            };
        }
        ThemeMode::Dark => {
            visuals.widgets.noninteractive = WidgetVisuals {
                bg_fill: Color32::from_rgb(35, 35, 35),
                weak_bg_fill: Color32::from_rgb(35, 35, 35),
                bg_stroke: Stroke::new(1.0_f32, Color32::from_rgb(60, 60, 60)),
                corner_radius: cr,
                fg_stroke: Stroke::new(1.0_f32, pal.text_muted),
                expansion: 0.0_f32,
            };
            visuals.widgets.inactive = WidgetVisuals {
                bg_fill: Color32::from_rgb(56, 56, 56),
                weak_bg_fill: Color32::from_rgb(56, 56, 56),
                bg_stroke: Stroke::new(1.0_f32, Color32::from_rgb(70, 70, 70)),
                corner_radius: cr,
                fg_stroke: Stroke::new(1.0_f32, pal.text_primary),
                expansion: 0.0_f32,
            };
            visuals.widgets.hovered = WidgetVisuals {
                bg_fill: Color32::from_rgb(68, 68, 68),
                weak_bg_fill: Color32::from_rgb(68, 68, 68),
                bg_stroke: Stroke::new(1.0_f32, pal.accent_blue),
                corner_radius: cr,
                fg_stroke: Stroke::new(1.0_f32, Color32::WHITE),
                expansion: 0.0_f32,
            };
            visuals.widgets.active = WidgetVisuals {
                bg_fill: Color32::from_rgb(30, 30, 30),
                weak_bg_fill: Color32::from_rgb(30, 30, 30),
                bg_stroke: Stroke::new(1.0_f32, pal.accent_blue),
                corner_radius: cr,
                fg_stroke: Stroke::new(1.0_f32, Color32::WHITE),
                expansion: 0.0_f32,
            };
            visuals.widgets.open = WidgetVisuals {
                bg_fill: Color32::from_rgb(35, 35, 35),
                weak_bg_fill: Color32::from_rgb(35, 35, 35),
                bg_stroke: Stroke::new(1.0_f32, pal.accent_blue),
                corner_radius: cr,
                fg_stroke: Stroke::new(1.0_f32, Color32::WHITE),
                expansion: 0.0_f32,
            };
        }
    }

    let mut style = (*ctx.style()).clone();
    style.visuals = visuals;
    style.spacing.item_spacing = vec2(4.0_f32, 4.0_f32);
    style.spacing.button_padding = vec2(9.0_f32, 3.0_f32);
    style.spacing.window_margin = Margin::same(4);
    style.spacing.menu_margin = Margin::same(2);
    style.spacing.indent = 16.0_f32;
    style.spacing.interact_size = vec2(24.0_f32, 24.0_f32);
    ctx.set_style(style);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_light_palette_hex_values() {
        let pal = Ts3Palette::light();
        assert_eq!(pal.mode, ThemeMode::Light);
        assert_eq!(pal.window_bg, Color32::from_rgb(0xEC, 0xEC, 0xEC));       // #ececec
        assert_eq!(pal.base_bg, Color32::from_rgb(0xFF, 0xFF, 0xFF));         // #ffffff
        assert_eq!(pal.alternate_base_bg, Color32::from_rgb(0xF6, 0xF6, 0xF6));// #f6f6f6
        assert_eq!(pal.text_primary, Color32::from_rgb(0x1E, 0x1E, 0x1E));    // #1e1e1e
        assert_eq!(pal.text_muted, Color32::from_rgb(0x6E, 0x6E, 0x6E));      // #6e6e6e
        assert_eq!(pal.accent_blue, Color32::from_rgb(0x2F, 0x71, 0xAF));     // #2f71af
        assert_eq!(pal.border_color, Color32::from_rgb(0xC8, 0xC8, 0xC8));    // #c8c8c8
        assert_eq!(pal.selection_bg, Color32::from_rgb(0x33, 0x99, 0xFF));    // #3399ff
        assert_eq!(pal.selection_fg, Color32::from_rgb(0xFF, 0xFF, 0xFF));    // #ffffff
        assert_eq!(pal.hover_bg, Color32::from_rgb(0xE5, 0xF3, 0xFF));        // #e5f3ff

        // default_chat.qss colors
        assert_eq!(pal.msg_user_link, Color32::from_rgb(0x00, 0x2F, 0x5D));   // #002f5d
        assert_eq!(pal.msg_welcome, Color32::from_rgb(0x0A, 0xA5, 0x37));     // #0aa537
        assert_eq!(pal.msg_poke, Color32::from_rgb(0x0A, 0xA5, 0x37));        // #0aa537
        assert_eq!(pal.msg_normal, Color32::from_rgb(0x33, 0x59, 0x7D));      // #33597d
        assert_eq!(pal.msg_client_dropped, Color32::from_rgb(0xC9, 0x07, 0x0A));// #c9070a
        assert_eq!(pal.msg_server_error, Color32::from_rgb(0xC9, 0x07, 0x0A));  // #c9070a
        assert_eq!(pal.msg_time, Color32::from_rgb(0x33, 0x59, 0x7D));        // #33597d
        assert_eq!(pal.msg_begin_history, Color32::from_rgb(0x70, 0x74, 0x75));// #707475
        assert_eq!(pal.chat.link, Color32::from_rgb(0x1C, 0x82, 0xCC));       // #1c82cc
        assert_eq!(pal.client_friend, Color32::from_rgb(0x00, 0x80, 0x00));   // #008000
        assert_eq!(pal.client_blocked, Color32::from_rgb(0xFF, 0x00, 0x00));  // #ff0000
        assert_eq!(pal.client_recording, Color32::from_rgb(0x80, 0x00, 0x00));// #800000
    }

    #[test]
    fn test_dark_palette_hex_values() {
        let pal = Ts3Palette::dark();
        assert_eq!(pal.mode, ThemeMode::Dark);
        assert_eq!(pal.window_bg, Color32::from_rgb(0x1A, 0x1A, 0x1A));       // #1a1a1a
        assert_eq!(pal.base_bg, Color32::from_rgb(0x36, 0x36, 0x36));         // #363636
        assert_eq!(pal.alternate_base_bg, Color32::from_rgb(0x2D, 0x2D, 0x2D));// #2d2d2d
        assert_eq!(pal.text_primary, Color32::from_rgb(0xCE, 0xCE, 0xCE));    // #cecece
        assert_eq!(pal.text_muted, Color32::from_rgb(0x8E, 0x8E, 0x8E));      // #8e8e8e
        assert_eq!(pal.accent_blue, Color32::from_rgb(0x3A, 0x82, 0xC4));     // #3a82c4
        assert_eq!(pal.border_color, Color32::from_rgb(0x46, 0x46, 0x46));    // #464646
        assert_eq!(pal.selection_bg, Color32::from_rgb(0x2F, 0x71, 0xAF));    // #2f71af
        assert_eq!(pal.selection_fg, Color32::from_rgb(0xFF, 0xFF, 0xFF));    // #ffffff
        assert_eq!(pal.hover_bg, Color32::from_rgb(0x42, 0x42, 0x42));        // #424242

        // default_chat_darkmode.qss colors
        assert_eq!(pal.msg_user_link, Color32::from_rgb(0x00, 0x6A, 0xD2));   // #006ad2
        assert_eq!(pal.msg_welcome, Color32::from_rgb(0x11, 0xDE, 0x4D));     // #11de4d
        assert_eq!(pal.msg_poke, Color32::from_rgb(0x11, 0xDE, 0x4D));        // #11de4d
        assert_eq!(pal.msg_normal, Color32::from_rgb(0x83, 0xAC, 0xD2));      // #83acd2
        assert_eq!(pal.msg_client_dropped, Color32::from_rgb(0xEC, 0x21, 0x24));// #ec2124
        assert_eq!(pal.msg_server_error, Color32::from_rgb(0xF7, 0x3C, 0x3F));  // #f73c3f
        assert_eq!(pal.msg_time, Color32::from_rgb(0x83, 0xAC, 0xD2));        // #83acd2
        assert_eq!(pal.msg_begin_history, Color32::from_rgb(0xBC, 0xBC, 0xBC));// #bcbcbc
        assert_eq!(pal.chat.link, Color32::from_rgb(0x00, 0x6A, 0xD2));       // #006ad2
    }

    #[test]
    fn test_theme_switching() {
        let ctx = Context::default();

        // 1. Light theme application
        apply_ts3_theme(&ctx, ThemeMode::Light);
        assert!(!ctx.style().visuals.dark_mode);
        assert_eq!(ctx.style().visuals.window_fill, Color32::from_rgb(0xEC, 0xEC, 0xEC));
        assert_eq!(ctx.style().visuals.selection.bg_fill, Color32::from_rgb(0x33, 0x99, 0xFF));
        assert_eq!(ctx.style().spacing.item_spacing, vec2(4.0_f32, 4.0_f32));
        assert_eq!(ctx.style().spacing.button_padding, vec2(9.0_f32, 3.0_f32));
        assert_eq!(ctx.style().spacing.window_margin, Margin::same(4));
        assert_eq!(ctx.style().visuals.window_corner_radius, CornerRadius::same(2));
        assert_eq!(ctx.style().visuals.menu_corner_radius, CornerRadius::same(2));
        assert!(ctx.style().visuals.striped);

        // 2. Dark theme application
        apply_ts3_theme(&ctx, ThemeMode::Dark);
        assert!(ctx.style().visuals.dark_mode);
        assert_eq!(ctx.style().visuals.window_fill, Color32::from_rgb(0x1A, 0x1A, 0x1A));
        assert_eq!(ctx.style().visuals.selection.bg_fill, Color32::from_rgb(0x2F, 0x71, 0xAF));
        assert_eq!(ctx.style().visuals.extreme_bg_color, Color32::from_rgb(0x36, 0x36, 0x36));
        assert_eq!(ctx.style().visuals.faint_bg_color, Color32::from_rgb(0x2D, 0x2D, 0x2D));
        assert_eq!(ctx.style().spacing.item_spacing, vec2(4.0_f32, 4.0_f32));
        assert_eq!(ctx.style().spacing.button_padding, vec2(9.0_f32, 3.0_f32));
        assert!(ctx.style().visuals.striped);
    }

    #[test]
    fn test_banner_gradient_stops() {
        assert_eq!(BANNER_GRADIENT_STOPS[0].0, 0.00_f32);
        assert_eq!(BANNER_GRADIENT_STOPS[0].1, Color32::from_rgb(80, 92, 120)); // #505c78
        assert_eq!(BANNER_GRADIENT_STOPS[1].0, 0.08_f32);
        assert_eq!(BANNER_GRADIENT_STOPS[1].1, Color32::from_rgb(133, 147, 174)); // #8593ae
        assert_eq!(BANNER_GRADIENT_STOPS[2].0, 0.32_f32);
        assert_eq!(BANNER_GRADIENT_STOPS[2].1, Color32::from_rgb(88, 107, 140)); // #586b8c
        assert_eq!(BANNER_GRADIENT_STOPS[3].0, 1.00_f32);
        assert_eq!(BANNER_GRADIENT_STOPS[3].1, Color32::from_rgb(111, 128, 170)); // #6f80aa
    }

    #[test]
    fn test_for_mode_selection() {
        let light = Ts3Palette::for_mode(ThemeMode::Light);
        assert_eq!(light.mode, ThemeMode::Light);
        let dark = Ts3Palette::for_mode(ThemeMode::Dark);
        assert_eq!(dark.mode, ThemeMode::Dark);
    }

    #[test]
    fn test_painter_helpers_do_not_panic() {
        let ctx = Context::default();
        let pal_light = Ts3Palette::light();
        let pal_dark = Ts3Palette::dark();
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let rect = Rect::from_min_size(egui::pos2(0.0_f32, 0.0_f32), vec2(200.0_f32, 32.0_f32));
                paint_banner_gradient(ui, rect, &pal_light);
                paint_banner_gradient(ui, rect, &pal_dark);

                paint_splitter_handle(ui, rect, true, &pal_light);
                paint_splitter_handle(ui, rect, false, &pal_dark);

                paint_tree_row_bg(ui, rect, true, true, &pal_light);
                paint_tree_row_bg(ui, rect, true, false, &pal_dark);
                paint_tree_row_bg(ui, rect, false, true, &pal_light);
                paint_tree_row_bg(ui, rect, false, false, &pal_dark);

                let _ = paint_group_badge(ui, "Server Admin", Color32::RED, &pal_light);
                let _ = paint_group_badge(ui, "SA", Color32::BLUE, &pal_dark);
            });
        });
    }
}
