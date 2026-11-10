// Owned by Pool Agent 2 (arthoroy26a@gmail.com)
// Runtime loader for `C:\Program Files\TeamSpeak 3 Client\gfx\default_colored_2014.zip`
// (parsing `settings.ini` -> SVG rasterization via `resvg` into `egui::TextureHandle`),
// `countries.zip` PNG flags, and `styles/default/` watermark PNGs from `docs/re/gui/05-gfx-icons.md`.

use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use egui::{Color32, ColorImage, Context, Pos2, Rect, Response, Sense, Stroke, TextureHandle, TextureOptions, Ui, Vec2};

pub const DEFAULT_TS3_INSTALL_DIR: &str = r"C:\Program Files\TeamSpeak 3 Client";
pub const ICON_RASTER_SIZE: u32 = 24;

pub struct Ts3Assets {
    pub icons: HashMap<String, TextureHandle>,
    pub flags: HashMap<String, TextureHandle>,
    pub watermark_logo: Option<TextureHandle>,
    pub watermark_chatlog: Option<TextureHandle>,
    pub loaded_from_disk: bool,
}

impl Default for Ts3Assets {
    fn default() -> Self {
        Self::new()
    }
}

impl Ts3Assets {
    pub fn new() -> Self {
        Self {
            icons: HashMap::new(),
            flags: HashMap::new(),
            watermark_logo: None,
            watermark_chatlog: None,
            loaded_from_disk: false,
        }
    }

    pub fn load_all(&mut self, ctx: &Context) {
        self.load_from_dir(ctx, Path::new(DEFAULT_TS3_INSTALL_DIR));
    }

    pub fn load_from_dir(&mut self, ctx: &Context, base_dir: &Path) {
        let mut any_disk_loaded = false;

        let icon_zip_path = base_dir.join("gfx").join("default_colored_2014.zip");
        if icon_zip_path.is_file() {
            if self.load_icon_pack_zip(ctx, &icon_zip_path) {
                any_disk_loaded = true;
            }
        }

        let legacy_zip_path = base_dir.join("gfx").join("default.zip");
        if legacy_zip_path.is_file() {
            self.load_legacy_fallback_zip(ctx, &legacy_zip_path);
        }

        // Ensure fallback textures exist for core keys if the zip was absent or incomplete
        self.populate_procedural_fallback_textures(ctx);

        let countries_zip_path = base_dir.join("gfx").join("countries.zip");
        if countries_zip_path.is_file() {
            if self.load_countries_zip(ctx, &countries_zip_path) {
                any_disk_loaded = true;
            }
        }

        let logo_path = base_dir.join("styles").join("default").join("logo-128x128.png");
        if let Some(tex) = load_png_file_texture(ctx, &logo_path, "ts3_watermark_logo") {
            self.watermark_logo = Some(tex);
            any_disk_loaded = true;
        }

        let chatlog_path = base_dir.join("styles").join("default").join("chatlog-128x96.png");
        if let Some(tex) = load_png_file_texture(ctx, &chatlog_path, "ts3_watermark_chatlog") {
            self.watermark_chatlog = Some(tex);
            any_disk_loaded = true;
        }

        self.loaded_from_disk = any_disk_loaded;
    }

    fn load_icon_pack_zip(&mut self, ctx: &Context, zip_path: &Path) -> bool {
        let Ok((ini_map, svg_entries)) = read_icon_pack_zip_data(zip_path) else {
            return false;
        };

        // Rasterize every SVG in the archive and store by lowercase stem and uppercase stem
        let mut by_rel_path: HashMap<String, TextureHandle> = HashMap::new();
        for (entry_name, svg_bytes) in &svg_entries {
            if let Some(rgba) = rasterize_svg_rgba(svg_bytes, ICON_RASTER_SIZE, ICON_RASTER_SIZE) {
                let image = ColorImage::from_rgba_unmultiplied(
                    [ICON_RASTER_SIZE as usize, ICON_RASTER_SIZE as usize],
                    &rgba,
                );
                let stem = file_stem_normalized(entry_name);
                let tex_name = format!("ts3_svg_{stem}");
                let handle = ctx.load_texture(tex_name, image, TextureOptions::LINEAR);

                let normalized_path = entry_name.replace('\\', "/").to_lowercase();
                by_rel_path.insert(normalized_path.clone(), handle.clone());
                if let Some(file_name) = Path::new(&normalized_path).file_name().and_then(|s| s.to_str()) {
                    by_rel_path.insert(file_name.to_string(), handle.clone());
                }

                self.icons.insert(stem.to_lowercase(), handle.clone());
                self.icons.insert(stem.to_uppercase(), handle);
            }
        }

        // Map all logical keys from settings.ini (`KEY = FILE`)
        for (logical_key, target_file) in &ini_map {
            let lookup = target_file.replace('\\', "/").to_lowercase();
            let file_only = Path::new(&lookup)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or(&lookup);
            let stem = file_stem_normalized(&lookup);

            let resolved = by_rel_path
                .get(&lookup)
                .or_else(|| by_rel_path.get(file_only))
                .or_else(|| self.icons.get(&stem));

            if let Some(handle) = resolved.cloned() {
                self.icons.insert(logical_key.clone(), handle.clone());
                self.icons.insert(logical_key.to_uppercase(), handle.clone());
                self.icons.insert(logical_key.to_lowercase(), handle);
            }
        }

        self.register_common_aliases();
        !self.icons.is_empty()
    }

    fn load_legacy_fallback_zip(&mut self, ctx: &Context, zip_path: &Path) {
        let Ok(file) = File::open(zip_path) else {
            return;
        };
        let Ok(mut archive) = zip::ZipArchive::new(file) else {
            return;
        };

        // Load missing keys mentioned in docs/re/gui/05-gfx-icons.md (SELECTFOLDER, STOP)
        let fallbacks = [
            ("SELECTFOLDER", "24x24_selectfolder.png", "selectfolder"),
            ("STOP", "16x16_stop.png", "stop"),
        ];

        for (logical_key, entry_name, stem) in fallbacks {
            if self.icons.contains_key(logical_key) {
                continue;
            }
            if let Ok(mut entry) = archive.by_name(entry_name) {
                let mut buf = Vec::new();
                if entry.read_to_end(&mut buf).is_ok() {
                    if let Some((w, h, rgba)) = decode_png_rgba(&buf) {
                        let image = ColorImage::from_rgba_unmultiplied([w, h], &rgba);
                        let handle = ctx.load_texture(
                            format!("ts3_legacy_{stem}"),
                            image,
                            TextureOptions::LINEAR,
                        );
                        self.icons.insert(logical_key.to_string(), handle.clone());
                        self.icons.insert(logical_key.to_lowercase(), handle.clone());
                        self.icons.insert(stem.to_string(), handle);
                    }
                }
            }
        }
    }

    fn load_countries_zip(&mut self, ctx: &Context, zip_path: &Path) -> bool {
        let Ok(flag_entries) = read_countries_zip_data(zip_path) else {
            return false;
        };

        for (code_upper, png_bytes) in flag_entries {
            if let Some((w, h, rgba)) = decode_png_rgba(&png_bytes) {
                let image = ColorImage::from_rgba_unmultiplied([w, h], &rgba);
                let handle = ctx.load_texture(
                    format!("ts3_flag_{code_upper}"),
                    image,
                    TextureOptions::LINEAR,
                );
                self.flags.insert(code_upper.to_lowercase(), handle.clone());
                self.flags.insert(code_upper, handle);
            }
        }

        !self.flags.is_empty()
    }

    fn register_common_aliases(&mut self) {
        let aliases = [
            ("BOOKMARKS", "BOOKMARK_MANAGER"),
            ("PLAYER_MUTED", "INPUT_MUTED"),
            ("PLAYER_COMMANDER", "PLAYER_COMMANDER_OFF"),
            ("MIC_MUTED", "INPUT_MUTED"),
            ("SOUND_MUTED", "OUTPUT_MUTED"),
            ("SPEAKER_MUTED", "OUTPUT_MUTED"),
            ("CHANNEL_SUBSCRIBED", "CHANNEL_GREEN_SUBSCRIBED"),
            ("SERVER", "SERVER_GREEN"),
        ];

        for (alias, target) in aliases {
            if !self.icons.contains_key(alias) {
                if let Some(handle) = self
                    .icons
                    .get(target)
                    .or_else(|| self.icons.get(&target.to_lowercase()))
                    .cloned()
                {
                    self.icons.insert(alias.to_string(), handle.clone());
                    self.icons.insert(alias.to_lowercase(), handle);
                }
            }
        }
    }

    fn populate_procedural_fallback_textures(&mut self, ctx: &Context) {
        const CORE_FALLBACK_KEYS: &[&str] = &[
            "CONNECT",
            "DISCONNECT",
            "BOOKMARKS",
            "BOOKMARK_MANAGER",
            "PLAYER_ON",
            "PLAYER_OFF",
            "PLAYER_WHISPER",
            "PLAYER_COMMANDER",
            "PLAYER_COMMANDER_ON",
            "PLAYER_COMMANDER_OFF",
            "CHANNEL_COMMANDER",
            "PLAYER_MUTED",
            "INPUT_MUTED",
            "INPUT_MUTED_LOCAL",
            "HARDWARE_INPUT_MUTED",
            "OUTPUT_MUTED",
            "HARDWARE_OUTPUT_MUTED",
            "AWAY",
            "ACTIVATE_MICROPHONE",
            "CHANNEL_GREEN",
            "CHANNEL_GREEN_SUBSCRIBED",
            "CHANNEL_YELLOW",
            "CHANNEL_YELLOW_SUBSCRIBED",
            "CHANNEL_RED",
            "CHANNEL_RED_SUBSCRIBED",
            "CHANNEL_PRIVATE",
            "SERVER_GREEN",
            "SUBSCRIBE_TO_ALL_CHANNELS",
            "UNSUBSCRIBE_FROM_ALL_CHANNELS",
            "SETTINGS",
            "FILETRANSFER",
            "POKE",
            "INFO",
            "WARNING",
            "ERROR",
            "ADD",
            "DELETE",
            "EDIT",
            "FIND",
            "REFRESH",
            "MODERATED",
            "DEFAULT",
            "MUSIC",
            "REGISTER",
            "SECURITY",
            "GROUP_100",
            "GROUP_200",
            "GROUP_300",
            "GROUP_500",
            "GROUP_600",
        ];

        for &key in CORE_FALLBACK_KEYS {
            if self.get_icon(key).is_none() {
                let rgba = generate_fallback_icon_rgba(key, ICON_RASTER_SIZE, ICON_RASTER_SIZE);
                let image = ColorImage::from_rgba_unmultiplied(
                    [ICON_RASTER_SIZE as usize, ICON_RASTER_SIZE as usize],
                    &rgba,
                );
                let handle = ctx.load_texture(
                    format!("ts3_fallback_{}", key.to_lowercase()),
                    image,
                    TextureOptions::LINEAR,
                );
                self.icons.insert(key.to_string(), handle.clone());
                self.icons.insert(key.to_lowercase(), handle);
            }
        }
        self.register_common_aliases();
    }

    pub fn get_icon(&self, key: &str) -> Option<&TextureHandle> {
        let trimmed = key.trim();
        if let Some(tex) = self.icons.get(trimmed) {
            return Some(tex);
        }
        let upper = trimmed.to_uppercase();
        if let Some(tex) = self.icons.get(&upper) {
            return Some(tex);
        }
        let lower = trimmed.to_lowercase();
        if let Some(tex) = self.icons.get(&lower) {
            return Some(tex);
        }
        let stem = file_stem_normalized(trimmed);
        self.icons
            .get(&stem)
            .or_else(|| self.icons.get(&stem.to_uppercase()))
    }

    pub fn get_flag(&self, country_code: &str) -> Option<&TextureHandle> {
        let trimmed = country_code.trim();
        let upper = trimmed.to_uppercase();
        self.flags
            .get(&upper)
            .or_else(|| self.flags.get(&trimmed.to_lowercase()))
            .or_else(|| self.flags.get(trimmed))
    }

    pub fn paint_icon_or_fallback(&self, ui: &mut Ui, key: &str, size: Vec2) -> Response {
        if let Some(tex) = self.get_icon(key) {
            let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
            if ui.is_rect_visible(rect) {
                ui.painter().image(
                    tex.id(),
                    rect,
                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
            response
        } else {
            paint_fallback_icon(ui, key, size)
        }
    }

    pub fn paint_flag_or_fallback(&self, ui: &mut Ui, country_code: &str, size: Vec2) -> Response {
        if let Some(tex) = self.get_flag(country_code) {
            let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
            if ui.is_rect_visible(rect) {
                ui.painter().image(
                    tex.id(),
                    rect,
                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
            response
        } else {
            let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
            if ui.is_rect_visible(rect) {
                let painter = ui.painter();
                painter.rect_filled(rect, 1.5_f32, Color32::from_rgb(0x3A, 0x6E, 0xA5));
                painter.rect_stroke(
                    rect,
                    1.5_f32,
                    Stroke::new(1.0_f32, Color32::from_rgb(0x90, 0xB4, 0xD4)),
                    egui::StrokeKind::Inside,
                );
            }
            response
        }
    }
}

/// Standalone procedural vector icon painter so UI components always render crisp icons
/// even when no textures are loaded into `Ts3Assets`.
pub fn paint_fallback_icon(ui: &mut Ui, key: &str, size: Vec2) -> Response {
    let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
    if !ui.is_rect_visible(rect) {
        return response;
    }

    let painter = ui.painter();
    let center = rect.center();
    let radius = rect.width().min(rect.height()) * 0.40_f32;
    let norm = key.trim().to_uppercase();

    match norm.as_str() {
        "PLAYER_ON" | "IS_TALKER" => {
            // Bright sky-blue talking indicator sphere with dark rim
            painter.circle_filled(center, radius, Color32::from_rgb(0x38, 0xB0, 0xFF));
            painter.circle_stroke(center, radius, Stroke::new(1.2_f32, Color32::from_rgb(0x10, 0x5A, 0x9E)));
            painter.circle_filled(
                center - Vec2::new(radius * 0.28_f32, radius * 0.28_f32),
                radius * 0.25_f32,
                Color32::from_rgba_unmultiplied(255, 255, 255, 180),
            );
        }
        "PLAYER_OFF" => {
            // Muted dark-blue / slate non-talking indicator sphere
            painter.circle_filled(center, radius, Color32::from_rgb(0x1D, 0x56, 0x8A));
            painter.circle_stroke(center, radius, Stroke::new(1.2_f32, Color32::from_rgb(0x0F, 0x33, 0x54)));
        }
        "PLAYER_WHISPER" | "WHISPER" | "ON_WHISPERLIST" => {
            // Crimson/red whisper sphere
            painter.circle_filled(center, radius, Color32::from_rgb(0xD9, 0x36, 0x36));
            painter.circle_stroke(center, radius, Stroke::new(1.2_f32, Color32::from_rgb(0x7A, 0x16, 0x16)));
        }
        "PLAYER_COMMANDER" | "PLAYER_COMMANDER_ON" | "PLAYER_COMMANDER_OFF" | "CHANNEL_COMMANDER" => {
            let fill = if norm.ends_with("_ON") {
                Color32::from_rgb(0xFF, 0xA8, 0x26)
            } else {
                Color32::from_rgb(0xC9, 0x7A, 0x10)
            };
            painter.circle_filled(center, radius, fill);
            painter.circle_stroke(center, radius, Stroke::new(1.2_f32, Color32::from_rgb(0x6E, 0x3E, 0x00)));
        }
        "CHANNEL_GREEN" | "CHANNEL_GREEN_SUBSCRIBED" | "SUBSCRIBE_TO_CHANNEL" => {
            paint_channel_bubble(
                painter,
                rect,
                Color32::from_rgb(0x4C, 0xB8, 0x48),
                Color32::from_rgb(0x23, 0x70, 0x20),
                norm.contains("SUBSCRIBED"),
            );
        }
        "CHANNEL_YELLOW" | "CHANNEL_YELLOW_SUBSCRIBED" => {
            paint_channel_bubble(
                painter,
                rect,
                Color32::from_rgb(0xE8, 0xB8, 0x28),
                Color32::from_rgb(0x8C, 0x6B, 0x08),
                norm.contains("SUBSCRIBED"),
            );
        }
        "CHANNEL_RED" | "CHANNEL_RED_SUBSCRIBED" | "CHANNEL_PRIVATE" => {
            paint_channel_bubble(
                painter,
                rect,
                Color32::from_rgb(0xD8, 0x42, 0x42),
                Color32::from_rgb(0x7B, 0x1D, 0x1D),
                norm.contains("SUBSCRIBED"),
            );
        }
        "SERVER_GREEN" | "SERVER" | "WEBLIST" => {
            let fill = Color32::from_rgb(0x36, 0xA6, 0x5B);
            let stroke = Stroke::new(1.2_f32, Color32::from_rgb(0x1B, 0x61, 0x33));
            painter.circle_filled(center, radius, fill);
            painter.circle_stroke(center, radius, stroke);
            painter.line_segment(
                [Pos2::new(center.x - radius, center.y), Pos2::new(center.x + radius, center.y)],
                stroke,
            );
            painter.line_segment(
                [Pos2::new(center.x, center.y - radius), Pos2::new(center.x, center.y + radius)],
                stroke,
            );
        }
        "INPUT_MUTED" | "INPUT_MUTED_LOCAL" | "HARDWARE_INPUT_MUTED" | "PLAYER_MUTED" | "MIC_MUTED" => {
            let mic_rect = Rect::from_center_size(
                center - Vec2::new(0.0_f32, radius * 0.15_f32),
                Vec2::new(radius * 0.85_f32, radius * 1.35_f32),
            );
            painter.rect_filled(mic_rect, radius * 0.4_f32, Color32::from_rgb(0x55, 0x65, 0x75));
            painter.line_segment(
                [
                    Pos2::new(center.x - radius * 0.85_f32, center.y - radius * 0.85_f32),
                    Pos2::new(center.x + radius * 0.85_f32, center.y + radius * 0.85_f32),
                ],
                Stroke::new(2.0_f32, Color32::from_rgb(0xE0, 0x30, 0x30)),
            );
        }
        "OUTPUT_MUTED" | "HARDWARE_OUTPUT_MUTED" | "SOUND_MUTED" | "SPEAKER_MUTED" => {
            let spk = Rect::from_center_size(
                center - Vec2::new(radius * 0.25_f32, 0.0_f32),
                Vec2::new(radius * 0.8_f32, radius * 0.9_f32),
            );
            painter.rect_filled(spk, 1.5_f32, Color32::from_rgb(0x55, 0x65, 0x75));
            painter.line_segment(
                [
                    Pos2::new(center.x - radius * 0.85_f32, center.y - radius * 0.85_f32),
                    Pos2::new(center.x + radius * 0.85_f32, center.y + radius * 0.85_f32),
                ],
                Stroke::new(2.0_f32, Color32::from_rgb(0xE0, 0x30, 0x30)),
            );
        }
        "AWAY" => {
            let box_rect = Rect::from_center_size(center, Vec2::splat(radius * 1.65_f32));
            painter.rect_filled(box_rect, 2.5_f32, Color32::from_rgb(0xE6, 0x9A, 0x28));
            let cross = radius * 0.45_f32;
            let stroke = Stroke::new(1.8_f32, Color32::WHITE);
            painter.line_segment(
                [Pos2::new(center.x - cross, center.y - cross), Pos2::new(center.x + cross, center.y + cross)],
                stroke,
            );
            painter.line_segment(
                [Pos2::new(center.x + cross, center.y - cross), Pos2::new(center.x - cross, center.y + cross)],
                stroke,
            );
        }
        "CONNECT" | "CONNECT_NEW_TAB" => {
            painter.circle_filled(center, radius, Color32::from_rgb(0x2F, 0x9E, 0x44));
            let s = Stroke::new(1.8_f32, Color32::WHITE);
            painter.line_segment([Pos2::new(center.x - radius * 0.45_f32, center.y), Pos2::new(center.x + radius * 0.45_f32, center.y)], s);
            painter.line_segment([Pos2::new(center.x, center.y - radius * 0.45_f32), Pos2::new(center.x, center.y + radius * 0.45_f32)], s);
        }
        "DISCONNECT" | "DISCONNECT_ALL" => {
            painter.circle_filled(center, radius, Color32::from_rgb(0xD9, 0x38, 0x38));
            let s = Stroke::new(1.8_f32, Color32::WHITE);
            painter.line_segment([Pos2::new(center.x - radius * 0.45_f32, center.y), Pos2::new(center.x + radius * 0.45_f32, center.y)], s);
        }
        "BOOKMARKS" | "BOOKMARK_MANAGER" | "BOOKMARK_ADD" | "FAVORITE" => {
            let star_rect = Rect::from_center_size(center, Vec2::splat(radius * 1.55_f32));
            painter.rect_filled(star_rect, 2.0_f32, Color32::from_rgb(0xF0, 0xB4, 0x29));
            painter.rect_stroke(
                star_rect,
                2.0_f32,
                Stroke::new(1.0_f32, Color32::from_rgb(0x9C, 0x6E, 0x0C)),
                egui::StrokeKind::Inside,
            );
        }
        _ => {
            let badge = Rect::from_center_size(center, Vec2::splat(radius * 1.55_f32));
            painter.rect_filled(badge, 2.5_f32, Color32::from_rgb(0x2F, 0x71, 0xAF));
            painter.rect_stroke(
                badge,
                2.5_f32,
                Stroke::new(1.0_f32, Color32::from_rgb(0x1C, 0x49, 0x73)),
                egui::StrokeKind::Inside,
            );
        }
    }

    response
}

fn paint_channel_bubble(
    painter: &egui::Painter,
    rect: Rect,
    fill: Color32,
    border: Color32,
    subscribed: bool,
) {
    let center = rect.center();
    let r = rect.width().min(rect.height()) * 0.38_f32;
    let hex = vec![
        Pos2::new(center.x, center.y - r),
        Pos2::new(center.x + r * 0.87_f32, center.y - r * 0.5_f32),
        Pos2::new(center.x + r * 0.87_f32, center.y + r * 0.5_f32),
        Pos2::new(center.x, center.y + r),
        Pos2::new(center.x - r * 0.87_f32, center.y + r * 0.5_f32),
        Pos2::new(center.x - r * 0.87_f32, center.y - r * 0.5_f32),
    ];
    painter.add(egui::Shape::convex_polygon(
        hex,
        fill,
        Stroke::new(1.2_f32, border),
    ));
    if subscribed {
        let s = Stroke::new(1.5_f32, Color32::WHITE);
        painter.line_segment(
            [
                Pos2::new(center.x - r * 0.35_f32, center.y),
                Pos2::new(center.x - r * 0.05_f32, center.y + r * 0.30_f32),
            ],
            s,
        );
        painter.line_segment(
            [
                Pos2::new(center.x - r * 0.05_f32, center.y + r * 0.30_f32),
                Pos2::new(center.x + r * 0.40_f32, center.y - r * 0.25_f32),
            ],
            s,
        );
    }
}

/// Parses `settings.ini` from a TeamSpeak 3 icon pack (`[gfxfiles]` `KEY = FILE` mappings).
pub fn parse_settings_ini(content: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let mut current_section = String::new();

    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            current_section = line[1..line.len() - 1].trim().to_lowercase();
            continue;
        }
        // Accept lines inside `[gfxfiles]` or top-level lines when no section headers are used
        if current_section == "info" || current_section == "options" {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            let key = k.trim();
            let val = v.trim().trim_matches('"');
            if !key.is_empty() && !val.is_empty() {
                map.insert(key.to_string(), val.to_string());
            }
        }
    }

    map
}

/// Rasterizes raw SVG bytes into an unmultiplied `width x height` RGBA8 buffer (`width * height * 4` bytes).
/// Scales the SVG to fit `(width, height)` if the SVG's intrinsic `tree.size()` differs.
pub fn rasterize_svg_rgba(svg_bytes: &[u8], width: u32, height: u32) -> Option<Vec<u8>> {
    if width == 0 || height == 0 {
        return None;
    }
    let options = resvg::usvg::Options::default();
    let tree = resvg::usvg::Tree::from_data(svg_bytes, &options).ok()?;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height)?;

    let svg_size = tree.size();
    let sx = if svg_size.width() > 0.0 {
        width as f32 / svg_size.width()
    } else {
        1.0
    };
    let sy = if svg_size.height() > 0.0 {
        height as f32 / svg_size.height()
    } else {
        1.0
    };
    let transform = resvg::tiny_skia::Transform::from_scale(sx, sy);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    // Convert premultiplied RGBA from tiny_skia into unmultiplied RGBA for egui::ColorImage
    let mut rgba = pixmap.take();
    for px in rgba.chunks_exact_mut(4) {
        let a = px[3] as u32;
        if a > 0 && a < 255 {
            px[0] = ((px[0] as u32 * 255 + a / 2) / a).min(255) as u8;
            px[1] = ((px[1] as u32 * 255 + a / 2) / a).min(255) as u8;
            px[2] = ((px[2] as u32 * 255 + a / 2) / a).min(255) as u8;
        }
    }
    Some(rgba)
}

/// Generates a procedural 24x24 (or `width x height`) RGBA8 buffer for fallback icons when TS3 assets are absent.
pub fn generate_fallback_icon_rgba(key: &str, width: u32, height: u32) -> Vec<u8> {
    let w = width.max(1) as usize;
    let h = height.max(1) as usize;
    let mut buf = vec![0u8; w * h * 4];
    let norm = key.trim().to_uppercase();

    let (r, g, b) = match norm.as_str() {
        "PLAYER_ON" | "IS_TALKER" => (0x38, 0xB0, 0xFF),
        "PLAYER_OFF" => (0x1D, 0x56, 0x8A),
        "PLAYER_WHISPER" | "WHISPER" => (0xD9, 0x36, 0x36),
        "PLAYER_COMMANDER" | "PLAYER_COMMANDER_ON" | "PLAYER_COMMANDER_OFF" | "CHANNEL_COMMANDER" => (0xE8, 0x96, 0x18),
        "CHANNEL_GREEN" | "CHANNEL_GREEN_SUBSCRIBED" | "SERVER_GREEN" | "CONNECT" => (0x4C, 0xB8, 0x48),
        "CHANNEL_YELLOW" | "CHANNEL_YELLOW_SUBSCRIBED" | "AWAY" | "BOOKMARKS" | "BOOKMARK_MANAGER" | "WARNING" => (0xE8, 0xB8, 0x28),
        "CHANNEL_RED" | "CHANNEL_RED_SUBSCRIBED" | "CHANNEL_PRIVATE" | "DISCONNECT" | "INPUT_MUTED" | "OUTPUT_MUTED" | "PLAYER_MUTED" | "ERROR" => (0xD8, 0x42, 0x42),
        _ => (0x2F, 0x71, 0xAF),
    };

    let cx = (w as f32 - 1.0) * 0.5;
    let cy = (h as f32 - 1.0) * 0.5;
    let rad = (w.min(h) as f32) * 0.38;
    let is_channel = norm.starts_with("CHANNEL_");

    for y in 0..h {
        for x in 0..w {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let dist = if is_channel {
                // Hexagonal/diamond distance metric for channels
                dx.abs() * 0.85 + dy.abs() * 0.85
            } else {
                (dx * dx + dy * dy).sqrt()
            };

            let idx = (y * w + x) * 4;
            if dist <= rad {
                let is_border = dist >= rad - 1.4;
                if is_border {
                    buf[idx] = (r as u16 * 3 / 5) as u8;
                    buf[idx + 1] = (g as u16 * 3 / 5) as u8;
                    buf[idx + 2] = (b as u16 * 3 / 5) as u8;
                    buf[idx + 3] = 255;
                } else {
                    buf[idx] = r;
                    buf[idx + 1] = g;
                    buf[idx + 2] = b;
                    buf[idx + 3] = 255;
                }
            }
        }
    }

    buf
}

/// Reads `settings.ini` and all `.svg` entries from an icon pack zip archive.
pub fn read_icon_pack_zip_data(
    zip_path: &Path,
) -> std::io::Result<(HashMap<String, String>, Vec<(String, Vec<u8>)>)> {
    let file = File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file).map_err(std::io::Error::other)?;

    let mut ini_map = HashMap::new();
    let mut svg_entries = Vec::new();

    for i in 0..archive.len() {
        let Ok(mut entry) = archive.by_index(i) else {
            continue;
        };
        if entry.is_dir() {
            continue;
        }
        let name = entry.name().replace('\\', "/");
        let lower = name.to_lowercase();
        if lower == "settings.ini" || lower.ends_with("/settings.ini") {
            let mut content = String::new();
            if entry.read_to_string(&mut content).is_ok() {
                ini_map = parse_settings_ini(&content);
            }
        } else if lower.ends_with(".svg") {
            let mut bytes = Vec::new();
            if entry.read_to_end(&mut bytes).is_ok() {
                svg_entries.push((name, bytes));
            }
        }
    }

    Ok((ini_map, svg_entries))
}

/// Reads all `<cc>.png` flag entries from `countries.zip`, returning `(UPPERCASE_CODE, png_bytes)`.
pub fn read_countries_zip_data(zip_path: &Path) -> std::io::Result<Vec<(String, Vec<u8>)>> {
    let file = File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file).map_err(std::io::Error::other)?;
    let mut flags = Vec::new();

    for i in 0..archive.len() {
        let Ok(mut entry) = archive.by_index(i) else {
            continue;
        };
        if entry.is_dir() {
            continue;
        }
        let name = entry.name().replace('\\', "/");
        if name.to_lowercase().ends_with(".png") {
            let code = file_stem_normalized(&name).to_uppercase();
            if !code.is_empty() {
                let mut bytes = Vec::new();
                if entry.read_to_end(&mut bytes).is_ok() {
                    flags.push((code, bytes));
                }
            }
        }
    }

    Ok(flags)
}

fn decode_png_rgba(png_bytes: &[u8]) -> Option<(usize, usize, Vec<u8>)> {
    let img = image::load_from_memory(png_bytes).ok()?;
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    Some((w as usize, h as usize, rgba.into_raw()))
}

fn load_png_file_texture(ctx: &Context, path: &Path, tex_name: &str) -> Option<TextureHandle> {
    let bytes = std::fs::read(path).ok()?;
    let (w, h, rgba) = decode_png_rgba(&bytes)?;
    let image = ColorImage::from_rgba_unmultiplied([w, h], &rgba);
    Some(ctx.load_texture(tex_name.to_string(), image, TextureOptions::LINEAR))
}

fn file_stem_normalized(path_or_name: &str) -> String {
    let norm = path_or_name.replace('\\', "/");
    Path::new(&norm)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(path_or_name)
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_settings_ini() {
        let sample = r#"
[info]
; Icon pack metadata

[options]
FALLBACK=true
PLACEHOLDER=2

[gfxfiles]
CONNECT = connect.svg
DISCONNECT = disconnect.svg
BOOKMARK_MANAGER = bookmark_manager.svg
PLAYER_ON = player_on.svg
PLAYER_OFF = player_off.svg
CHANNEL_GREEN_SUBSCRIBED = channel_green_subscribed.svg
; SAVE = save.svg
"#;
        let map = parse_settings_ini(sample);
        assert_eq!(map.get("CONNECT").map(String::as_str), Some("connect.svg"));
        assert_eq!(map.get("DISCONNECT").map(String::as_str), Some("disconnect.svg"));
        assert_eq!(
            map.get("BOOKMARK_MANAGER").map(String::as_str),
            Some("bookmark_manager.svg")
        );
        assert_eq!(map.get("PLAYER_ON").map(String::as_str), Some("player_on.svg"));
        assert_eq!(map.get("PLAYER_OFF").map(String::as_str), Some("player_off.svg"));
        assert_eq!(
            map.get("CHANNEL_GREEN_SUBSCRIBED").map(String::as_str),
            Some("channel_green_subscribed.svg")
        );
        assert!(!map.contains_key("FALLBACK"));
        assert!(!map.contains_key("PLACEHOLDER"));
        assert!(!map.contains_key("SAVE"));
    }

    #[test]
    fn test_rasterize_svg_rgba_scales_to_24x24() {
        let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 128 128" width="128" height="128">
            <circle cx="64" cy="64" r="56" fill="#38b0ff"/>
        </svg>"##;
        let rgba = rasterize_svg_rgba(svg, 24, 24).expect("SVG should rasterize to 24x24 RGBA");
        assert_eq!(rgba.len(), 24 * 24 * 4);

        // Center pixel (12, 12) should be non-transparent blue
        let center_idx = (12 * 24 + 12) * 4;
        assert_eq!(rgba[center_idx], 0x38);
        assert_eq!(rgba[center_idx + 1], 0xB0);
        assert_eq!(rgba[center_idx + 2], 0xFF);
        assert_eq!(rgba[center_idx + 3], 0xFF);
    }

    #[test]
    fn test_fallback_procedural_icons_and_case_insensitive_lookup() {
        let ctx = egui::Context::default();
        let mut assets = Ts3Assets::new();
        assets.load_from_dir(&ctx, Path::new(r"C:\nonexistent_ts3_dir_for_test"));

        assert!(!assets.loaded_from_disk);
        assert!(assets.get_icon("PLAYER_ON").is_some());
        assert!(assets.get_icon("player_on").is_some());
        assert!(assets.get_icon("Player_On").is_some());
        assert!(assets.get_icon("CHANNEL_GREEN_SUBSCRIBED").is_some());
        assert!(assets.get_icon("BOOKMARKS").is_some());
        assert!(assets.get_icon("PLAYER_MUTED").is_some());
    }

    #[test]
    fn test_live_ts3_archives_if_present() {
        let base = Path::new(DEFAULT_TS3_INSTALL_DIR);
        let icon_zip = base.join("gfx").join("default_colored_2014.zip");
        if icon_zip.is_file() {
            let (ini_map, svg_entries) =
                read_icon_pack_zip_data(&icon_zip).expect("read default_colored_2014.zip");
            assert!(
                ini_map.len() >= 193,
                "Expected at least 193 logical icon keys in settings.ini, got {}",
                ini_map.len()
            );
            assert!(
                svg_entries.len() >= 190,
                "Expected at least 190 SVG files in default_colored_2014.zip, got {}",
                svg_entries.len()
            );
            assert_eq!(ini_map.get("CONNECT").map(String::as_str), Some("connect.svg"));
            assert_eq!(ini_map.get("PLAYER_ON").map(String::as_str), Some("player_on.svg"));

            // Verify rasterizing a real SVG from the archive
            let (_, first_svg) = &svg_entries[0];
            let rgba = rasterize_svg_rgba(first_svg, 24, 24).expect("rasterize live TS3 SVG");
            assert_eq!(rgba.len(), 24 * 24 * 4);
        }

        let countries_zip = base.join("gfx").join("countries.zip");
        if countries_zip.is_file() {
            let flags = read_countries_zip_data(&countries_zip).expect("read countries.zip");
            assert!(flags.len() >= 240, "Expected >= 240 country flags, got {}", flags.len());
            assert!(flags.iter().any(|(code, _)| code == "US"));
            assert!(flags.iter().any(|(code, _)| code == "DE"));
            assert!(flags.iter().any(|(code, _)| code == "BD"));
        }

        if icon_zip.is_file() && countries_zip.is_file() {
            let ctx = egui::Context::default();
            let mut assets = Ts3Assets::new();
            assets.load_all(&ctx);
            assert!(assets.loaded_from_disk);
            assert!(assets.get_icon("CONNECT").is_some());
            assert!(assets.get_icon("connect").is_some());
            assert!(assets.get_icon("BOOKMARKS").is_some());
            assert!(assets.get_icon("PLAYER_ON").is_some());
            assert!(assets.get_icon("PLAYER_OFF").is_some());
            assert!(assets.get_icon("PLAYER_MUTED").is_some());
            assert!(assets.get_icon("CHANNEL_GREEN_SUBSCRIBED").is_some());
            assert!(assets.get_icon("AWAY").is_some());
            assert!(assets.get_icon("INPUT_MUTED").is_some());
            assert!(assets.get_icon("OUTPUT_MUTED").is_some());
            assert!(assets.get_icon("SERVER_GREEN").is_some());
            assert!(assets.get_flag("US").is_some());
            assert!(assets.get_flag("de").is_some());
            assert!(assets.get_flag("BD").is_some());
            assert!(assets.watermark_logo.is_some());
            assert!(assets.watermark_chatlog.is_some());
        }
    }
}
