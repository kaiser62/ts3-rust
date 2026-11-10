// Owned by Pool Agent 7 (arthoroy29@gmail.com)
// Bottom `MainWindowChatWidget` with Server Chat tab, Channel Chat tab, dynamic Private Message tabs,
// `<HH:MM:SS>` timestamps, `TextMessage_*` QSS colors, BBCode (`[b]`, `[i]`, `[u]`, `[color]`, `[url=client://...]`),
// bottom-right `chatlog-128x96.png` watermark, emoticon button, and `ChatLineEdit`
// from `docs/re/gui/08-settings-chat-bbcode.md`.

use egui::{Color32, CornerRadius, Pos2, Rect, RichText, Sense, Stroke, Vec2};
use ts3_client::connection::{ChatMessage, ChatTarget};

use crate::assets::Ts3Assets;
use crate::menus_toolbar::UiCommand;
use crate::theme::Ts3Palette;

/// Standard emoticon shortcuts offered by the `EmoticonButton` popup (`emoticons.txt:1-21`).
pub const EMOTICON_SHORTCUTS: &[(&str, &str)] = &[
    (":)", "Smile"),
    (";)", "Wink"),
    (":D", "Laugh"),
    (":(", "Sad"),
    (":P", "Tongue"),
    ("<3", "Heart"),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatLineKind {
    ServerLog,
    ChannelMessage,
    PrivateMessage,
    PokeAlert,
    ErrorAlert,
    WelcomeMessage,
}

#[derive(Debug, Clone)]
pub struct FormattedChatLine {
    pub timestamp: String,
    pub kind: ChatLineKind,
    pub sender_clid: Option<u16>,
    pub sender_uid: Option<String>,
    pub sender_name: Option<String>,
    pub text: String,
    pub is_own_message: bool,
}

#[derive(Debug, Clone)]
pub struct PrivateChatTab {
    pub clid: u16,
    pub nickname: String,
    pub uid: String,
    pub lines: Vec<FormattedChatLine>,
    pub unread: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ActiveChatTab {
    #[default]
    Server,
    Channel,
    Private(u16),
}

#[derive(Debug, Clone)]
pub struct ChatPaneState {
    pub active_tab: ActiveChatTab,
    pub server_lines: Vec<FormattedChatLine>,
    pub channel_lines: Vec<FormattedChatLine>,
    pub private_tabs: Vec<PrivateChatTab>,
    pub server_unread: usize,
    pub channel_unread: usize,
    pub input_text: String,
    pub show_emoticon_picker: bool,
    pub auto_scroll: bool,
}

impl Default for ChatPaneState {
    fn default() -> Self {
        Self {
            active_tab: ActiveChatTab::Server,
            server_lines: Vec::new(),
            channel_lines: Vec::new(),
            private_tabs: Vec::new(),
            server_unread: 0,
            channel_unread: 0,
            input_text: String::new(),
            show_emoticon_picker: false,
            auto_scroll: true,
        }
    }
}

impl ChatPaneState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends an informational server log entry (`TextMessage_Normal` / `TextMessage_ServerLog`).
    pub fn push_server_log(&mut self, timestamp: String, text: String) {
        self.server_lines.push(FormattedChatLine {
            timestamp,
            kind: ChatLineKind::ServerLog,
            sender_clid: None,
            sender_uid: None,
            sender_name: None,
            text,
            is_own_message: false,
        });
        if self.active_tab != ActiveChatTab::Server {
            self.server_unread += 1;
        }
        self.auto_scroll = true;
    }

    /// Appends a server welcome message (`TextMessage_Welcome`).
    pub fn push_welcome(&mut self, timestamp: String, text: String) {
        self.server_lines.push(FormattedChatLine {
            timestamp,
            kind: ChatLineKind::WelcomeMessage,
            sender_clid: None,
            sender_uid: None,
            sender_name: None,
            text,
            is_own_message: false,
        });
        if self.active_tab != ActiveChatTab::Server {
            self.server_unread += 1;
        }
        self.auto_scroll = true;
    }

    /// Appends a server error alert (`TextMessage_ServerError`).
    pub fn push_error(&mut self, timestamp: String, text: String) {
        self.server_lines.push(FormattedChatLine {
            timestamp,
            kind: ChatLineKind::ErrorAlert,
            sender_clid: None,
            sender_uid: None,
            sender_name: None,
            text,
            is_own_message: false,
        });
        if self.active_tab != ActiveChatTab::Server {
            self.server_unread += 1;
        }
        self.auto_scroll = true;
    }

    /// Appends a client poke alert (`TextMessage_Poke`).
    pub fn push_poke(
        &mut self,
        timestamp: String,
        invoker_id: u16,
        invoker_name: String,
        msg: String,
    ) {
        let formatted_text = if msg.is_empty() {
            format!("\"{}\" pokes you", invoker_name)
        } else {
            format!("\"{}\" pokes you: {}", invoker_name, msg)
        };
        self.server_lines.push(FormattedChatLine {
            timestamp,
            kind: ChatLineKind::PokeAlert,
            sender_clid: Some(invoker_id),
            sender_uid: None,
            sender_name: Some(invoker_name),
            text: formatted_text,
            is_own_message: false,
        });
        if self.active_tab != ActiveChatTab::Server {
            self.server_unread += 1;
        }
        self.auto_scroll = true;
    }

    /// Routes an incoming or echoed `notifytextmessage` into the Server, Channel, or Private tab.
    pub fn push_incoming_chat(&mut self, msg: &ChatMessage, my_clid: u16) {
        let is_own = my_clid != 0 && msg.invoker_id == my_clid;
        match &msg.target {
            ChatTarget::Server => {
                self.server_lines.push(FormattedChatLine {
                    timestamp: msg.timestamp.clone(),
                    kind: ChatLineKind::ServerLog,
                    sender_clid: Some(msg.invoker_id),
                    sender_uid: Some(msg.invoker_uid.clone()),
                    sender_name: Some(msg.invoker_name.clone()),
                    text: msg.message.clone(),
                    is_own_message: is_own,
                });
                if self.active_tab != ActiveChatTab::Server && !is_own {
                    self.server_unread += 1;
                }
            }
            ChatTarget::Channel => {
                self.channel_lines.push(FormattedChatLine {
                    timestamp: msg.timestamp.clone(),
                    kind: ChatLineKind::ChannelMessage,
                    sender_clid: Some(msg.invoker_id),
                    sender_uid: Some(msg.invoker_uid.clone()),
                    sender_name: Some(msg.invoker_name.clone()),
                    text: msg.message.clone(),
                    is_own_message: is_own,
                });
                if self.active_tab != ActiveChatTab::Channel && !is_own {
                    self.channel_unread += 1;
                }
            }
            ChatTarget::Client(target_clid) => {
                let peer_clid = if *target_clid == my_clid && msg.invoker_id != my_clid {
                    msg.invoker_id
                } else {
                    *target_clid
                };

                let line = FormattedChatLine {
                    timestamp: msg.timestamp.clone(),
                    kind: ChatLineKind::PrivateMessage,
                    sender_clid: Some(msg.invoker_id),
                    sender_uid: Some(msg.invoker_uid.clone()),
                    sender_name: Some(msg.invoker_name.clone()),
                    text: msg.message.clone(),
                    is_own_message: is_own,
                };

                if let Some(tab) = self.private_tabs.iter_mut().find(|t| t.clid == peer_clid) {
                    if !is_own {
                        if !msg.invoker_name.is_empty() {
                            tab.nickname = msg.invoker_name.clone();
                        }
                        if !msg.invoker_uid.is_empty() {
                            tab.uid = msg.invoker_uid.clone();
                        }
                    }
                    tab.lines.push(line);
                    if self.active_tab != ActiveChatTab::Private(peer_clid) && !is_own {
                        tab.unread += 1;
                    }
                } else {
                    let nickname = if !is_own && !msg.invoker_name.is_empty() {
                        msg.invoker_name.clone()
                    } else {
                        format!("Client #{}", peer_clid)
                    };
                    let unread = if self.active_tab == ActiveChatTab::Private(peer_clid) || is_own {
                        0
                    } else {
                        1
                    };
                    self.private_tabs.push(PrivateChatTab {
                        clid: peer_clid,
                        nickname,
                        uid: if is_own {
                            String::new()
                        } else {
                            msg.invoker_uid.clone()
                        },
                        lines: vec![line],
                        unread,
                    });
                }
            }
        }
        self.auto_scroll = true;
    }

    /// Opens a new private chat tab for `clid` (or updates and focuses an existing one).
    pub fn open_or_focus_private_tab(&mut self, clid: u16, nickname: String, uid: String) {
        if let Some(tab) = self.private_tabs.iter_mut().find(|t| t.clid == clid) {
            if !nickname.is_empty() {
                tab.nickname = nickname;
            }
            if !uid.is_empty() {
                tab.uid = uid;
            }
            tab.unread = 0;
        } else {
            let display_nick = if nickname.is_empty() {
                format!("Client #{}", clid)
            } else {
                nickname
            };
            self.private_tabs.push(PrivateChatTab {
                clid,
                nickname: display_nick,
                uid,
                lines: Vec::new(),
                unread: 0,
            });
        }
        self.active_tab = ActiveChatTab::Private(clid);
        self.auto_scroll = true;
    }

    /// Closes a private chat tab by `clid` and falls back to the Channel tab if it was active.
    pub fn close_private_tab(&mut self, clid: u16) {
        self.private_tabs.retain(|t| t.clid != clid);
        if self.active_tab == ActiveChatTab::Private(clid) {
            self.active_tab = ActiveChatTab::Channel;
        }
    }
}

pub enum ChatOutgoingAction {
    SendMessage { target: ChatTarget, message: String },
    ExecuteSlashCommand(String),
    Ui(UiCommand),
}

/// A parsed rich-text span produced by [`parse_bbcode`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BbSpan {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
    pub color: Option<Color32>,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct BbStyleState {
    bold_depth: usize,
    italic_depth: usize,
    underline_depth: usize,
    strike_depth: usize,
    color_stack: Vec<Color32>,
    active_url: Option<String>,
}

impl BbStyleState {
    fn to_span(&self, text: impl Into<String>, override_url: Option<String>) -> BbSpan {
        BbSpan {
            text: text.into(),
            bold: self.bold_depth > 0,
            italic: self.italic_depth > 0,
            underline: self.underline_depth > 0,
            strikethrough: self.strike_depth > 0,
            color: self.color_stack.last().copied(),
            url: override_url.or_else(|| self.active_url.clone()),
        }
    }
}

/// Parses a CSS/BBCode color value (`#RRGGBB`, `#RGB`, or named color) into [`Color32`].
pub fn parse_bbcode_color(spec: &str) -> Option<Color32> {
    let s = spec.trim().trim_matches(|c| c == '"' || c == '\'');
    if let Some(hex) = s.strip_prefix('#') {
        match hex.len() {
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                return Some(Color32::from_rgb(r, g, b));
            }
            3 => {
                let r = u8::from_str_radix(&hex[0..1], 16).ok()? * 17;
                let g = u8::from_str_radix(&hex[1..2], 16).ok()? * 17;
                let b = u8::from_str_radix(&hex[2..3], 16).ok()? * 17;
                return Some(Color32::from_rgb(r, g, b));
            }
            _ => return None,
        }
    }

    match s.to_ascii_lowercase().as_str() {
        "red" => Some(Color32::from_rgb(0xFF, 0x00, 0x00)),
        "darkred" | "maroon" => Some(Color32::from_rgb(0x80, 0x00, 0x00)),
        "green" => Some(Color32::from_rgb(0x00, 0x80, 0x00)),
        "lime" => Some(Color32::from_rgb(0x00, 0xFF, 0x00)),
        "darkgreen" => Some(Color32::from_rgb(0x00, 0x64, 0x00)),
        "blue" => Some(Color32::from_rgb(0x00, 0x00, 0xFF)),
        "darkblue" | "navy" => Some(Color32::from_rgb(0x00, 0x00, 0x80)),
        "yellow" => Some(Color32::from_rgb(0xFF, 0xFF, 0x00)),
        "orange" | "gold" => Some(Color32::from_rgb(0xFF, 0xA5, 0x00)),
        "purple" | "violet" => Some(Color32::from_rgb(0x80, 0x00, 0x80)),
        "cyan" | "aqua" => Some(Color32::from_rgb(0x00, 0xFF, 0xFF)),
        "teal" => Some(Color32::from_rgb(0x00, 0x80, 0x80)),
        "magenta" | "fuchsia" => Some(Color32::from_rgb(0xFF, 0x00, 0xFF)),
        "pink" => Some(Color32::from_rgb(0xFF, 0x69, 0xB4)),
        "white" => Some(Color32::from_rgb(0xFF, 0xFF, 0xFF)),
        "black" => Some(Color32::from_rgb(0x00, 0x00, 0x00)),
        "gray" | "grey" => Some(Color32::from_rgb(0x80, 0x80, 0x80)),
        "silver" | "lightgray" | "lightgrey" => Some(Color32::from_rgb(0xC0, 0xC0, 0xC0)),
        "olive" => Some(Color32::from_rgb(0x80, 0x80, 0x00)),
        _ => None,
    }
}

/// Splits a plain text chunk into spans, automatically detecting bare `http://` and `https://` URLs
/// when not already inside a `[url]` block.
fn push_text_with_bare_urls(
    out: &mut Vec<BbSpan>,
    state: &BbStyleState,
    chunk: &str,
) {
    if chunk.is_empty() {
        return;
    }

    if state.active_url.is_some() {
        push_or_merge_span(out, state.to_span(chunk, None));
        return;
    }

    let mut rest = chunk;
    while !rest.is_empty() {
        let http_pos = find_ascii_case_insensitive(rest, "http://");
        let https_pos = find_ascii_case_insensitive(rest, "https://");
        let next_url_pos = match (http_pos, https_pos) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };

        let Some(start) = next_url_pos else {
            push_or_merge_span(out, state.to_span(rest, None));
            break;
        };

        if start > 0 {
            push_or_merge_span(out, state.to_span(&rest[..start], None));
        }

        let after_start = &rest[start..];
        let raw_end = after_start
            .find(|c: char| c.is_whitespace() || matches!(c, '<' | '>' | '"' | '\''))
            .unwrap_or(after_start.len());

        let mut url_slice = &after_start[..raw_end];
        while let Some(last) = url_slice.chars().last() {
            if matches!(last, '.' | ',' | ';' | ':' | '!' | '?' | ')') {
                url_slice = &url_slice[..url_slice.len() - last.len_utf8()];
            } else {
                break;
            }
        }

        if url_slice.len() > "https://".len() || url_slice.len() > "http://".len() {
            push_or_merge_span(
                out,
                state.to_span(url_slice, Some(url_slice.to_string())),
            );
            rest = &after_start[url_slice.len()..];
        } else {
            // Bare prefix with no host; emit prefix as normal text and advance.
            let prefix_len = if after_start.len() >= 8 { 8 } else { after_start.len() };
            push_or_merge_span(out, state.to_span(&after_start[..prefix_len], None));
            rest = &after_start[prefix_len..];
        }
    }
}

fn push_or_merge_span(out: &mut Vec<BbSpan>, span: BbSpan) {
    if span.text.is_empty() {
        return;
    }
    if let Some(last) = out.last_mut() {
        if last.bold == span.bold
            && last.italic == span.italic
            && last.underline == span.underline
            && last.strikethrough == span.strikethrough
            && last.color == span.color
            && last.url == span.url
        {
            last.text.push_str(&span.text);
            return;
        }
    }
    out.push(span);
}

fn find_ascii_case_insensitive(haystack: &str, needle: &str) -> Option<usize> {
    let h_bytes = haystack.as_bytes();
    let n_bytes = needle.as_bytes();
    if n_bytes.is_empty() || h_bytes.len() < n_bytes.len() {
        return None;
    }
    for i in 0..=(h_bytes.len() - n_bytes.len()) {
        if h_bytes[i..i + n_bytes.len()].eq_ignore_ascii_case(n_bytes)
            && haystack.is_char_boundary(i)
        {
            return Some(i);
        }
    }
    None
}

/// Parses TeamSpeak 3 BBCode (`[b]`, `[i]`, `[u]`, `[s]`, `[color=...]`, `[url]`, `[url=...]`,
/// and bare `http://` / `https://` URLs) into a sequence of [`BbSpan`] items (`08-settings-chat-bbcode.md` §6).
pub fn parse_bbcode(input: &str) -> Vec<BbSpan> {
    let mut out = Vec::new();
    let mut state = BbStyleState::default();
    let mut rest = input;

    while !rest.is_empty() {
        let Some(open_idx) = rest.find('[') else {
            push_text_with_bare_urls(&mut out, &state, rest);
            break;
        };

        if open_idx > 0 {
            push_text_with_bare_urls(&mut out, &state, &rest[..open_idx]);
        }

        let after_bracket = &rest[open_idx + 1..];
        let Some(close_rel) = after_bracket.find(']') else {
            // Unclosed `[` - treat rest as literal text.
            push_text_with_bare_urls(&mut out, &state, &rest[open_idx..]);
            break;
        };

        let tag_content = after_bracket[..close_rel].trim();
        let after_tag = &after_bracket[close_rel + 1..];
        let lower_tag = tag_content.to_ascii_lowercase();

        match lower_tag.as_str() {
            "b" => {
                state.bold_depth += 1;
                rest = after_tag;
            }
            "/b" => {
                state.bold_depth = state.bold_depth.saturating_sub(1);
                rest = after_tag;
            }
            "i" => {
                state.italic_depth += 1;
                rest = after_tag;
            }
            "/i" => {
                state.italic_depth = state.italic_depth.saturating_sub(1);
                rest = after_tag;
            }
            "u" => {
                state.underline_depth += 1;
                rest = after_tag;
            }
            "/u" => {
                state.underline_depth = state.underline_depth.saturating_sub(1);
                rest = after_tag;
            }
            "s" => {
                state.strike_depth += 1;
                rest = after_tag;
            }
            "/s" => {
                state.strike_depth = state.strike_depth.saturating_sub(1);
                rest = after_tag;
            }
            "/color" => {
                state.color_stack.pop();
                rest = after_tag;
            }
            "/url" => {
                state.active_url = None;
                rest = after_tag;
            }
            "url" => {
                if let Some(end_rel) = find_ascii_case_insensitive(after_tag, "[/url]") {
                    let inner = &after_tag[..end_rel];
                    let target = inner.trim().to_string();
                    push_or_merge_span(&mut out, state.to_span(inner, Some(target)));
                    rest = &after_tag[end_rel + "[/url]".len()..];
                } else {
                    // Unclosed [url], treat literal tag as text
                    push_text_with_bare_urls(
                        &mut out,
                        &state,
                        &rest[open_idx..open_idx + 1 + close_rel + 1],
                    );
                    rest = after_tag;
                }
            }
            _ => {
                if let Some(color_arg) = tag_content
                    .get(6..)
                    .filter(|_| lower_tag.starts_with("color="))
                {
                    if let Some(color) = parse_bbcode_color(color_arg) {
                        state.color_stack.push(color);
                    }
                    rest = after_tag;
                } else if let Some(url_arg) = tag_content
                    .get(4..)
                    .filter(|_| lower_tag.starts_with("url="))
                {
                    let clean_url = url_arg
                        .trim()
                        .trim_matches(|c| c == '"' || c == '\'')
                        .to_string();
                    state.active_url = Some(clean_url);
                    rest = after_tag;
                } else {
                    // Unknown bracket expression (e.g. `[spacer]`, `[12:00]`): keep literal text.
                    let raw_tag = &rest[open_idx..open_idx + 1 + close_rel + 1];
                    push_text_with_bare_urls(&mut out, &state, raw_tag);
                    rest = after_tag;
                }
            }
        }
    }

    out
}

/// Parses a TeamSpeak `client://<clid>/<uid>~<nickname>` URI into `(clid, uid, nickname)`.
fn parse_ts3_client_uri(uri: &str) -> Option<(u16, String, String)> {
    let rest = uri
        .strip_prefix("client://")
        .or_else(|| uri.strip_prefix("CLIENT://"))?;
    let (clid_str, after_clid) = rest.split_once('/')?;
    let clid = clid_str.parse::<u16>().ok()?;
    if let Some((uid, nick)) = after_clid.split_once('~') {
        let decoded_nick = nick.replace("%20", " ");
        Some((clid, uid.to_string(), decoded_nick))
    } else {
        Some((clid, after_clid.to_string(), format!("Client #{}", clid)))
    }
}

/// Parses a TeamSpeak `channelid://<cid>` URI into a channel ID.
fn parse_ts3_channel_uri(uri: &str) -> Option<u64> {
    let rest = uri
        .strip_prefix("channelid://")
        .or_else(|| uri.strip_prefix("CHANNELID://"))?;
    rest.trim().parse::<u64>().ok()
}

pub fn render_chat_pane(
    ui: &mut egui::Ui,
    chat: &mut ChatPaneState,
    server_tab_title: &str,
    channel_tab_title: &str,
    _connected: bool,
    palette: &Ts3Palette,
    assets: &Ts3Assets,
    actions: &mut Vec<ChatOutgoingAction>,
) {
    let total_height = ui.available_height();
    let tab_bar_height = 24.0;
    let input_row_height = 26.0;
    let spacing_y = 4.0;
    let transcript_height =
        (total_height - tab_bar_height - input_row_height - spacing_y * 2.0).max(36.0);

    // 1. Transcript area (`QTextEdit#ChatTab` with 1px solid mid border & bottom-right watermark)
    let transcript_frame = egui::Frame::NONE
        .fill(palette.base_bg)
        .stroke(Stroke::new(1.0_f32, palette.border_color))
        .inner_margin(egui::Margin::same(4));

    let mut pending_focus_tab: Option<(u16, String, String)> = None;

    transcript_frame.show(ui, |ui| {
        ui.set_min_height(transcript_height);
        ui.set_max_height(transcript_height);
        ui.set_width(ui.available_width());

        let viewport_rect = ui.max_rect();

        // Bottom-right watermark (`default.qss:76-82` `styles:default/chatlog-128x96.png`)
        if let Some(watermark) = &assets.watermark_chatlog {
            let wm_size = Vec2::new(128.0, 96.0);
            let wm_pos = Pos2::new(
                (viewport_rect.max.x - wm_size.x - 2.0).max(viewport_rect.min.x),
                (viewport_rect.max.y - wm_size.y - 2.0).max(viewport_rect.min.y),
            );
            let wm_rect = Rect::from_min_size(wm_pos, wm_size);
            ui.painter().image(
                watermark.id(),
                wm_rect,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );
        }

        let lines: &[FormattedChatLine] = match &chat.active_tab {
            ActiveChatTab::Server => &chat.server_lines,
            ActiveChatTab::Channel => &chat.channel_lines,
            ActiveChatTab::Private(clid) => chat
                .private_tabs
                .iter()
                .find(|t| t.clid == *clid)
                .map(|t| t.lines.as_slice())
                .unwrap_or(&[]),
        };

        let scroll_area = egui::ScrollArea::vertical()
            .id_salt("ts3_chat_transcript_scroll")
            .auto_shrink([false, false])
            .stick_to_bottom(true)
            .max_height(transcript_height);

        scroll_area.show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            for line in lines {
                render_chat_line(ui, line, palette, actions, &mut pending_focus_tab);
            }
            if chat.auto_scroll {
                ui.scroll_to_cursor(Some(egui::Align::BOTTOM));
                chat.auto_scroll = false;
            }
        });
    });

    if let Some((clid, nick, uid)) = pending_focus_tab {
        chat.open_or_focus_private_tab(clid, nick, uid);
    }

    ui.add_space(2.0);

    // 2. Bottom `ImprovedTabBar` (Server tab, Channel tab, dynamic Private Message tabs)
    render_chat_tab_bar(
        ui,
        chat,
        server_tab_title,
        channel_tab_title,
        palette,
        assets,
    );

    ui.add_space(2.0);

    // 3. Bottom Input Row (`ChatLineEdit` + `EmoticonButton` + `Send` button)
    render_chat_input_row(ui, chat, palette, actions);
}

fn render_chat_line(
    ui: &mut egui::Ui,
    line: &FormattedChatLine,
    palette: &Ts3Palette,
    actions: &mut Vec<ChatOutgoingAction>,
    pending_focus_tab: &mut Option<(u16, String, String)>,
) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;

        // `<HH:MM:SS>` timestamp prefix (`default_chat.qss:168` `.TextMessage_Time`)
        if !line.timestamp.is_empty() {
            let ts_formatted = if line.timestamp.starts_with('<') && line.timestamp.ends_with('>') {
                line.timestamp.clone()
            } else {
                format!("<{}>", line.timestamp)
            };
            ui.label(RichText::new(ts_formatted).color(palette.chat_time));
        }

        // Clickable `"<SenderName>:"` in `palette.chat_client` (`default_chat.qss:99` `.TextMessage_UserLink`)
        if line.kind != ChatLineKind::PokeAlert {
            if let Some(sender_name) = &line.sender_name {
                if !sender_name.is_empty() {
                    let sender_label = format!("{}:", sender_name);
                    let resp = ui.add(
                        egui::Label::new(
                            RichText::new(sender_label)
                                .color(palette.chat_client)
                                .strong(),
                        )
                        .sense(Sense::click()),
                    );

                    let clid_opt = line.sender_clid;
                    let uid_str = line.sender_uid.clone().unwrap_or_default();
                    let nick_str = sender_name.clone();

                    if resp.clicked() {
                        if let Some(clid) = clid_opt {
                            *pending_focus_tab = Some((clid, nick_str.clone(), uid_str.clone()));
                            actions.push(ChatOutgoingAction::Ui(UiCommand::OpenPrivateChat {
                                clid,
                                nickname: nick_str.clone(),
                            }));
                        }
                    }

                    resp.context_menu(|ui| {
                        if let Some(clid) = clid_opt {
                            if ui.button("Open Text Chat").clicked() {
                                *pending_focus_tab =
                                    Some((clid, nick_str.clone(), uid_str.clone()));
                                actions.push(ChatOutgoingAction::Ui(UiCommand::OpenPrivateChat {
                                    clid,
                                    nickname: nick_str.clone(),
                                }));
                                ui.close_menu();
                            }
                            if ui.button("Poke Client").clicked() {
                                actions.push(ChatOutgoingAction::Ui(UiCommand::OpenPokeDialog {
                                    clid,
                                    nickname: nick_str.clone(),
                                }));
                                ui.close_menu();
                            }
                        }
                        if ui.button("Copy Nickname").clicked() {
                            ui.ctx().copy_text(nick_str.clone());
                            ui.close_menu();
                        }
                    });
                }
            }
        }

        // Base color & weight according to `default_chat.qss` primary message class
        let (base_color, force_bold) = match line.kind {
            ChatLineKind::ServerLog => {
                if line.sender_name.is_some() {
                    (palette.chat_normal, false)
                } else {
                    (palette.chat_server, false)
                }
            }
            ChatLineKind::WelcomeMessage => (palette.chat_channel, false),
            ChatLineKind::ChannelMessage | ChatLineKind::PrivateMessage => {
                (palette.chat_normal, false)
            }
            ChatLineKind::PokeAlert => (palette.chat_poke, false),
            ChatLineKind::ErrorAlert => (palette.chat_error, true),
        };

        // Render BBCode spans in message body
        let spans = parse_bbcode(&line.text);
        for span in spans {
            render_bb_span(
                ui,
                &span,
                base_color,
                force_bold,
                palette,
                actions,
                pending_focus_tab,
            );
        }
    });
}

fn render_bb_span(
    ui: &mut egui::Ui,
    span: &BbSpan,
    base_color: Color32,
    force_bold: bool,
    palette: &Ts3Palette,
    actions: &mut Vec<ChatOutgoingAction>,
    pending_focus_tab: &mut Option<(u16, String, String)>,
) {
    let mut rt = RichText::new(&span.text);
    if span.bold || force_bold {
        rt = rt.strong();
    }
    if span.italic {
        rt = rt.italics();
    }
    if span.underline {
        rt = rt.underline();
    }
    if span.strikethrough {
        rt = rt.strikethrough();
    }

    if let Some(url) = &span.url {
        if let Some((clid, uid, nick)) = parse_ts3_client_uri(url) {
            let link_rt = rt.color(span.color.unwrap_or(palette.chat_client)).strong();
            let resp = ui.add(egui::Label::new(link_rt).sense(Sense::click()));
            if resp.clicked() {
                let display_nick = if span.text.is_empty() {
                    nick.clone()
                } else {
                    span.text.clone()
                };
                *pending_focus_tab = Some((clid, display_nick.clone(), uid));
                actions.push(ChatOutgoingAction::Ui(UiCommand::OpenPrivateChat {
                    clid,
                    nickname: display_nick,
                }));
            }
            resp.context_menu(|ui| {
                let display_nick = if span.text.is_empty() {
                    nick.clone()
                } else {
                    span.text.clone()
                };
                if ui.button("Open Text Chat").clicked() {
                    *pending_focus_tab = Some((clid, display_nick.clone(), String::new()));
                    actions.push(ChatOutgoingAction::Ui(UiCommand::OpenPrivateChat {
                        clid,
                        nickname: display_nick.clone(),
                    }));
                    ui.close_menu();
                }
                if ui.button("Poke Client").clicked() {
                    actions.push(ChatOutgoingAction::Ui(UiCommand::OpenPokeDialog {
                        clid,
                        nickname: display_nick,
                    }));
                    ui.close_menu();
                }
            });
        } else if let Some(cid) = parse_ts3_channel_uri(url) {
            let link_rt = rt
                .color(span.color.unwrap_or(palette.chat_channel))
                .strong();
            let resp = ui.add(egui::Label::new(link_rt).sense(Sense::click()));
            if resp.clicked() && cid > 0 {
                actions.push(ChatOutgoingAction::Ui(UiCommand::SwitchChannel {
                    cid,
                    password: None,
                }));
            }
        } else {
            let link_rt = rt.color(span.color.unwrap_or(palette.chat_link)).underline();
            let resp = ui
                .add(egui::Label::new(link_rt).sense(Sense::click()))
                .on_hover_text(url);
            if resp.clicked() {
                let _ = open::that(url);
            }
        }
    } else {
        rt = rt.color(span.color.unwrap_or(base_color));
        ui.label(rt);
    }
}

fn render_chat_tab_bar(
    ui: &mut egui::Ui,
    chat: &mut ChatPaneState,
    server_tab_title: &str,
    channel_tab_title: &str,
    palette: &Ts3Palette,
    assets: &Ts3Assets,
) {
    let mut tab_to_close: Option<u16> = None;

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;

        // 1. Server Tab
        let server_selected = chat.active_tab == ActiveChatTab::Server;
        let server_title = if server_tab_title.is_empty() {
            "Server"
        } else {
            server_tab_title
        };
        let server_label = if chat.server_unread > 0 {
            format!("{} ({})", server_title, chat.server_unread)
        } else {
            server_title.to_string()
        };
        let server_icon = assets
            .get_icon("SERVER_GREEN")
            .or_else(|| assets.get_icon("CONNECT"));
        if render_improved_tab_button(
            ui,
            server_icon,
            &server_label,
            server_selected,
            chat.server_unread > 0,
            palette,
        )
        .clicked()
        {
            chat.active_tab = ActiveChatTab::Server;
            chat.server_unread = 0;
            chat.auto_scroll = true;
        }

        // 2. Channel Tab
        let channel_selected = chat.active_tab == ActiveChatTab::Channel;
        let channel_title = if channel_tab_title.is_empty() {
            "Channel"
        } else {
            channel_tab_title
        };
        let channel_label = if chat.channel_unread > 0 {
            format!("{} ({})", channel_title, chat.channel_unread)
        } else {
            channel_title.to_string()
        };
        let channel_icon = assets
            .get_icon("CHANNEL_GREEN_SUBSCRIBED")
            .or_else(|| assets.get_icon("CHANNEL_GREEN"));
        if render_improved_tab_button(
            ui,
            channel_icon,
            &channel_label,
            channel_selected,
            chat.channel_unread > 0,
            palette,
        )
        .clicked()
        {
            chat.active_tab = ActiveChatTab::Channel;
            chat.channel_unread = 0;
            chat.auto_scroll = true;
        }

        // 3. Dynamic Private Message Tabs
        let client_icon = assets
            .get_icon("PLAYER_CHAT")
            .or_else(|| assets.get_icon("PLAYER_OFF"));
        let mut clicked_private_clid: Option<u16> = None;

        for tab in &chat.private_tabs {
            let is_selected = chat.active_tab == ActiveChatTab::Private(tab.clid);
            let label = if tab.unread > 0 {
                format!("{} ({})", tab.nickname, tab.unread)
            } else {
                tab.nickname.clone()
            };

            let fill = if is_selected {
                palette.base_bg
            } else {
                palette.alternate_base_bg
            };

            let tab_frame = egui::Frame::NONE
                .fill(fill)
                .stroke(Stroke::new(1.0_f32, palette.border_color))
                .corner_radius(CornerRadius {
                    nw: 0,
                    ne: 0,
                    sw: 6,
                    se: 6,
                })
                .inner_margin(egui::Margin::symmetric(6, 2));

            tab_frame.show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    if let Some(tex) = client_icon {
                        ui.image((tex.id(), Vec2::new(14.0, 14.0)));
                    }
                    let mut text = RichText::new(label).color(if tab.unread > 0 && !is_selected {
                        palette.accent_blue
                    } else {
                        palette.text_primary
                    });
                    if is_selected || tab.unread > 0 {
                        text = text.strong();
                    }
                    if ui
                        .add(egui::Label::new(text).sense(Sense::click()))
                        .clicked()
                    {
                        clicked_private_clid = Some(tab.clid);
                    }
                    if ui
                        .small_button("×")
                        .on_hover_text("Close Tab")
                        .clicked()
                    {
                        tab_to_close = Some(tab.clid);
                    }
                });
            });
        }

        if let Some(clid) = clicked_private_clid {
            chat.active_tab = ActiveChatTab::Private(clid);
            if let Some(tab) = chat.private_tabs.iter_mut().find(|t| t.clid == clid) {
                tab.unread = 0;
            }
            chat.auto_scroll = true;
        }
    });

    if let Some(clid) = tab_to_close {
        chat.close_private_tab(clid);
    }
}

fn render_improved_tab_button(
    ui: &mut egui::Ui,
    icon: Option<&egui::TextureHandle>,
    label: &str,
    selected: bool,
    has_unread: bool,
    palette: &Ts3Palette,
) -> egui::Response {
    let fill = if selected {
        palette.base_bg
    } else {
        palette.alternate_base_bg
    };

    let frame = egui::Frame::NONE
        .fill(fill)
        .stroke(Stroke::new(1.0_f32, palette.border_color))
        .corner_radius(CornerRadius {
            nw: 0,
            ne: 0,
            sw: 6,
            se: 6,
        })
        .inner_margin(egui::Margin::symmetric(8, 2));

    let inner = frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            if let Some(tex) = icon {
                ui.image((tex.id(), Vec2::new(14.0, 14.0)));
            }
            let mut rt = RichText::new(label).color(if has_unread && !selected {
                palette.accent_blue
            } else {
                palette.text_primary
            });
            if selected || has_unread {
                rt = rt.strong();
            }
            ui.add(egui::Label::new(rt).sense(Sense::click()))
        })
        .inner
    });

    inner.inner.union(inner.response)
}

fn render_chat_input_row(
    ui: &mut egui::Ui,
    chat: &mut ChatPaneState,
    palette: &Ts3Palette,
    actions: &mut Vec<ChatOutgoingAction>,
) {
    let mut should_send = false;

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;

        let reserved_right = 88.0;
        let input_width = (ui.available_width() - reserved_right).max(60.0);

        let edit_resp = ui.add(
            egui::TextEdit::singleline(&mut chat.input_text)
                .desired_width(input_width)
                .hint_text(
                    RichText::new("Enter chat message or /raw <command>...")
                        .color(Color32::from_rgb(0x90, 0x90, 0x90)),
                ),
        );

        if edit_resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            should_send = true;
            edit_resp.request_focus();
        }

        // EmoticonButton (`emoticons.txt`)
        let emo_btn = ui
            .button(":)")
            .on_hover_text("Insert Emoticon");
        if emo_btn.clicked() {
            chat.show_emoticon_picker = !chat.show_emoticon_picker;
        }

        if chat.show_emoticon_picker {
            egui::Area::new(egui::Id::new("ts3_emoticon_picker_popup"))
                .order(egui::Order::Foreground)
                .fixed_pos(Pos2::new(
                    emo_btn.rect.min.x - 120.0,
                    (emo_btn.rect.min.y - 34.0).max(0.0),
                ))
                .show(ui.ctx(), |ui| {
                    egui::Frame::popup(ui.style())
                        .fill(palette.base_bg)
                        .stroke(Stroke::new(1.0_f32, palette.border_color))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                for (shortcut, tooltip) in EMOTICON_SHORTCUTS {
                                    if ui.button(*shortcut).on_hover_text(*tooltip).clicked() {
                                        if !chat.input_text.is_empty()
                                            && !chat.input_text.ends_with(' ')
                                        {
                                            chat.input_text.push(' ');
                                        }
                                        chat.input_text.push_str(shortcut);
                                        chat.show_emoticon_picker = false;
                                    }
                                }
                            });
                        });
                });
        }

        if ui.button("Send").clicked() {
            should_send = true;
        }
    });

    if should_send {
        submit_chat_input(chat, actions);
    }
}

/// Dispatches the current `chat.input_text` as either a `/raw <cmd>` slash command or a
/// `ChatOutgoingAction::SendMessage` targeting the active tab (`Server`, `Channel`, or `Client(clid)`).
pub fn submit_chat_input(chat: &mut ChatPaneState, actions: &mut Vec<ChatOutgoingAction>) {
    let trimmed = chat.input_text.trim();
    if trimmed.is_empty() {
        return;
    }

    if let Some(raw_cmd) = trimmed.strip_prefix("/raw ") {
        let cmd = raw_cmd.trim().to_string();
        if !cmd.is_empty() {
            actions.push(ChatOutgoingAction::ExecuteSlashCommand(cmd));
        }
    } else {
        let target = match chat.active_tab {
            ActiveChatTab::Server => ChatTarget::Server,
            ActiveChatTab::Channel => ChatTarget::Channel,
            ActiveChatTab::Private(clid) => ChatTarget::Client(clid),
        };
        actions.push(ChatOutgoingAction::SendMessage {
            target,
            message: trimmed.to_string(),
        });
    }

    chat.input_text.clear();
    chat.auto_scroll = true;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bbcode_basic_formatting() {
        let spans = parse_bbcode("Hello [b]bold [i]italic[/i][/b] [u]under[/u] [s]strike[/s]!");
        assert_eq!(spans.len(), 8);
        assert_eq!(spans[0].text, "Hello ");
        assert!(!spans[0].bold && !spans[0].italic);

        assert_eq!(spans[1].text, "bold ");
        assert!(spans[1].bold && !spans[1].italic);

        assert_eq!(spans[2].text, "italic");
        assert!(spans[2].bold && spans[2].italic);

        assert_eq!(spans[3].text, " ");
        assert!(!spans[3].bold && !spans[3].italic);

        assert_eq!(spans[4].text, "under");
        assert!(spans[4].underline);

        assert_eq!(spans[5].text, " ");
        assert!(!spans[5].underline);

        assert_eq!(spans[6].text, "strike");
        assert!(spans[6].strikethrough);

        assert_eq!(spans[7].text, "!");
        assert!(!spans[7].strikethrough);
    }

    #[test]
    fn test_parse_bbcode_colors_and_urls() {
        let spans = parse_bbcode(
            "[COLOR=#ff0000]Error:[/COLOR] [color=green]OK[/color] [url=https://teamspeak.com]TS3[/url] [url]http://example.org[/url] and https://rust-lang.org.",
        );
        assert_eq!(spans[0].text, "Error:");
        assert_eq!(spans[0].color, Some(Color32::from_rgb(0xFF, 0x00, 0x00)));

        assert_eq!(spans[2].text, "OK");
        assert_eq!(spans[2].color, Some(Color32::from_rgb(0x00, 0x80, 0x00)));

        assert_eq!(spans[4].text, "TS3");
        assert_eq!(spans[4].url.as_deref(), Some("https://teamspeak.com"));

        assert_eq!(spans[6].text, "http://example.org");
        assert_eq!(spans[6].url.as_deref(), Some("http://example.org"));

        assert_eq!(spans[8].text, "https://rust-lang.org");
        assert_eq!(spans[8].url.as_deref(), Some("https://rust-lang.org"));

        assert_eq!(spans[9].text, ".");
        assert!(spans[9].url.is_none());
    }

    #[test]
    fn test_parse_bbcode_ts3_custom_client_uri() {
        let spans = parse_bbcode(
            "[URL=client://202/jGKJGPXdhY9wr4MN5zd5ADMjI3A=~Alice]Alice[/URL] joined",
        );
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].text, "Alice");
        assert_eq!(
            spans[0].url.as_deref(),
            Some("client://202/jGKJGPXdhY9wr4MN5zd5ADMjI3A=~Alice")
        );
        assert_eq!(
            parse_ts3_client_uri(spans[0].url.as_deref().unwrap()),
            Some((
                202,
                "jGKJGPXdhY9wr4MN5zd5ADMjI3A=".to_string(),
                "Alice".to_string()
            ))
        );
    }

    #[test]
    fn test_chat_pane_state_helpers_and_unread_badges() {
        let mut state = ChatPaneState::new();
        assert_eq!(state.active_tab, ActiveChatTab::Server);

        // Server log while on Server tab -> unread remains 0
        state.push_server_log("12:00:01".to_string(), "Connected".to_string());
        assert_eq!(state.server_lines.len(), 1);
        assert_eq!(state.server_unread, 0);

        // Incoming channel message while on Server tab -> channel_unread increments
        state.push_incoming_chat(
            &ChatMessage {
                timestamp: "12:00:02".to_string(),
                target: ChatTarget::Channel,
                invoker_id: 5,
                invoker_name: "Alice".to_string(),
                invoker_uid: "uid_alice=".to_string(),
                message: "Hello channel!".to_string(),
            },
            1,
        );
        assert_eq!(state.channel_lines.len(), 1);
        assert_eq!(state.channel_unread, 1);

        // Incoming private message creates a PrivateChatTab and increments its unread count
        state.push_incoming_chat(
            &ChatMessage {
                timestamp: "12:00:03".to_string(),
                target: ChatTarget::Client(5),
                invoker_id: 5,
                invoker_name: "Alice".to_string(),
                invoker_uid: "uid_alice=".to_string(),
                message: "Secret PM".to_string(),
            },
            1,
        );
        assert_eq!(state.private_tabs.len(), 1);
        assert_eq!(state.private_tabs[0].clid, 5);
        assert_eq!(state.private_tabs[0].nickname, "Alice");
        assert_eq!(state.private_tabs[0].unread, 1);

        // Focusing the private tab clears unread and sets active_tab
        state.open_or_focus_private_tab(5, "Alice".to_string(), "uid_alice=".to_string());
        assert_eq!(state.active_tab, ActiveChatTab::Private(5));
        assert_eq!(state.private_tabs[0].unread, 0);

        // Server error and poke while on Private tab increment server_unread
        state.push_error("12:00:04".to_string(), "Permission denied".to_string());
        state.push_poke(
            "12:00:05".to_string(),
            7,
            "Bob".to_string(),
            "Wake up!".to_string(),
        );
        assert_eq!(state.server_lines.len(), 3);
        assert_eq!(state.server_unread, 2);
        assert_eq!(state.server_lines[1].kind, ChatLineKind::ErrorAlert);
        assert_eq!(state.server_lines[2].kind, ChatLineKind::PokeAlert);
        assert_eq!(state.server_lines[2].text, "\"Bob\" pokes you: Wake up!");
    }

    #[test]
    fn test_submit_chat_input_targets_and_slash_raw() {
        let mut state = ChatPaneState::new();
        let mut actions = Vec::new();

        state.active_tab = ActiveChatTab::Channel;
        state.input_text = "Hello :)".to_string();
        submit_chat_input(&mut state, &mut actions);
        assert!(state.input_text.is_empty());
        assert!(matches!(
            &actions[0],
            ChatOutgoingAction::SendMessage {
                target: ChatTarget::Channel,
                message
            } if message == "Hello :)"
        ));

        state.input_text = "/raw clientlist -uid".to_string();
        submit_chat_input(&mut state, &mut actions);
        assert!(matches!(
            &actions[1],
            ChatOutgoingAction::ExecuteSlashCommand(cmd) if cmd == "clientlist -uid"
        ));
    }
}
