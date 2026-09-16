use crate::atspi::AccessibleElement;
use cairo::Context;
use gtk4::prelude::*;
use notmouse_core::Rect;

pub const HINTS: &[char] = &['a', 's', 'd', 'f', 'j', 'k', 'l', 'g', 'h'];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayMode {
    Grid,
    FreeRoam,
    Scroll,
    TopBar,
}

pub struct OverlayDrawState {
    pub mode: OverlayMode,
    pub path: Vec<char>,
    pub rect: Rect,
    pub point: Option<(f64, f64)>,
    pub dragging: bool,
    pub drag_start_point: Option<(f64, f64)>,
    pub top_bar_target: Option<char>,
    pub snapped_element: Option<AccessibleElement>,
    pub snap_candidates: Vec<AccessibleElement>,
    pub snap_index: usize,
    pub last_scroll_dir: Option<&'static str>,
    pub scroll_speed: i32,
    pub last_click_time_ms: u64,
    pub is_turbo: bool,
    pub is_crawl: bool,
}

pub fn child_rect(parent: Rect, index: usize) -> Rect {
    let width = parent.width / 3.0;
    let height = parent.height / 3.0;
    let row = (index / 3) as f64;
    let col = (index % 3) as f64;

    Rect {
        x: parent.x + col * width,
        y: parent.y + row * height,
        width,
        height,
    }
}

pub fn draw_overlay(
    area: &gtk4::DrawingArea,
    cr: &Context,
    width: i32,
    height: i32,
    state: &OverlayDrawState,
) {
    let w = f64::from(width);
    let h = f64::from(height);

    match state.mode {
        OverlayMode::Scroll => draw_scroll_hud(area, cr, w, h, state),
        OverlayMode::FreeRoam => draw_free_roam_hud(area, cr, w, h, state),
        OverlayMode::Grid => draw_grid(area, cr, w, h, state),
        OverlayMode::TopBar => draw_topbar(area, cr, w, h, state),
    }
}

fn draw_scroll_hud(
    area: &gtk4::DrawingArea,
    cr: &Context,
    width: f64,
    height: f64,
    state: &OverlayDrawState,
) {
    let pill_w = width.min(560.0);
    let pill_h = height.min(48.0);
    let pill_x = (width - pill_w) / 2.0;
    let pill_y = (height - pill_h) / 2.0;
    let radius = 12.0;

    // Dark acrylic background
    cr.set_source_rgba(0.06, 0.08, 0.14, 0.94);
    draw_rounded_rect(cr, pill_x, pill_y, pill_w, pill_h, radius);
    let _ = cr.fill();

    // Accent border (cyan)
    cr.set_source_rgba(0.18, 0.80, 0.97, 0.85);
    cr.set_line_width(1.5);
    draw_rounded_rect(cr, pill_x, pill_y, pill_w, pill_h, radius);
    let _ = cr.stroke();

    let dir_icon = match state.last_scroll_dir {
        Some("up") => "▲",
        Some("down") => "▼",
        Some("left") => "◀",
        Some("right") => "▶",
        _ => "⇕",
    };

    let speed = state.scroll_speed;
    let text = format!(
        "{dir_icon} SCROLL ({speed}x)  —  [j/k] Down/Up  [d/u] Page  [Shift] Faster  [Tab] Grid  [Esc] Done"
    );
    let layout = area.create_pango_layout(Some(&text));
    let font_desc = pango::FontDescription::from_string("Sans Bold 11");
    layout.set_font_description(Some(&font_desc));
    let (text_w, text_h) = layout.pixel_size();

    cr.set_source_rgba(0.85, 0.92, 1.0, 0.95);
    cr.move_to(
        pill_x + (pill_w - f64::from(text_w)) / 2.0,
        pill_y + (pill_h - f64::from(text_h)) / 2.0,
    );
    pangocairo::functions::show_layout(cr, &layout);
}

fn draw_free_roam_hud(
    area: &gtk4::DrawingArea,
    cr: &Context,
    width: f64,
    height: f64,
    state: &OverlayDrawState,
) {
    let pill_w = width.min(680.0);
    let pill_h = height.min(48.0);
    let pill_x = (width - pill_w) / 2.0;
    let pill_y = (height - pill_h) / 2.0;
    let radius = 12.0;

    // Dark acrylic glass background
    cr.set_source_rgba(0.06, 0.08, 0.14, 0.94);
    draw_rounded_rect(cr, pill_x, pill_y, pill_w, pill_h, radius);
    let _ = cr.fill();

    // Accent border: electric emerald or amber if dragging
    if state.dragging {
        cr.set_source_rgba(1.0, 0.35, 0.35, 0.95);
    } else if state.is_turbo {
        cr.set_source_rgba(0.97, 0.72, 0.18, 0.95);
    } else if state.is_crawl {
        cr.set_source_rgba(0.35, 0.75, 1.0, 0.95);
    } else {
        cr.set_source_rgba(0.20, 0.95, 0.55, 0.85);
    }
    cr.set_line_width(1.6);
    draw_rounded_rect(cr, pill_x, pill_y, pill_w, pill_h, radius);
    let _ = cr.stroke();

    let speed_label = if state.is_turbo {
        " [TURBO 2.5x]"
    } else if state.is_crawl {
        " [PRECISION 1px]"
    } else {
        ""
    };

    let drag_label = if state.dragging { " [DRAGGING]" } else { "" };

    let text = format!(
        "✦ FREE ROAM{speed_label}{drag_label} — [hjkl] Glide  [Space] Click  [v] Drag  [s] Scroll  [Tab] Grid  [Esc] Done"
    );
    let layout = area.create_pango_layout(Some(&text));
    let font_desc = pango::FontDescription::from_string("Sans Bold 11");
    layout.set_font_description(Some(&font_desc));
    let (text_w, text_h) = layout.pixel_size();

    cr.set_source_rgba(0.90, 0.95, 1.0, 0.95);
    cr.move_to(
        pill_x + (pill_w - f64::from(text_w)) / 2.0,
        pill_y + (pill_h - f64::from(text_h)) / 2.0,
    );
    pangocairo::functions::show_layout(cr, &layout);
}

fn draw_grid(
    area: &gtk4::DrawingArea,
    cr: &Context,
    width: f64,
    height: f64,
    state: &OverlayDrawState,
) {
    let is_macro = state.path.is_empty();
    let is_locked = state.path.len() >= 2;

    // Subtly dim background so desktop underneath remains visible
    cr.set_source_rgba(0.02, 0.03, 0.06, if is_macro { 0.08 } else { 0.16 });
    cr.rectangle(0.0, 0.0, width, height);
    let _ = cr.fill();

    // If dragging active, draw source anchor
    if state.dragging
        && let Some((sx_norm, sy_norm)) = state.drag_start_point
    {
        let start_x = sx_norm * width;
        let start_y = sy_norm * height;
        cr.set_source_rgba(1.0, 0.25, 0.25, 0.90);
        cr.arc(start_x, start_y, 12.0, 0.0, 2.0 * std::f64::consts::PI);
        let _ = cr.fill();

        draw_label(
            area,
            cr,
            "Drag source",
            start_x,
            start_y - 24.0,
            10,
            6.0,
            2.0,
            (0.8, 0.1, 0.1, 0.95),
            (1.0, 1.0, 1.0, 1.0),
            true,
        );
    }

    let selected_px = Rect {
        x: state.rect.x * width,
        y: state.rect.y * height,
        width: state.rect.width * width,
        height: state.rect.height * height,
    };

    if is_macro {
        // Macro view: draw 9 macro cells, faint sub-grids, and center badges
        for (m, &hint_m) in HINTS.iter().enumerate() {
            let macro_zone = child_rect(selected_px, m);
            let macro_cx = macro_zone.x + macro_zone.width / 2.0;
            let macro_cy = macro_zone.y + macro_zone.height / 2.0;

            // Faint sub-grid inside each macro cell
            for (s, &hint_s) in HINTS.iter().enumerate() {
                let sub_zone = child_rect(macro_zone, s);
                cr.set_source_rgba(0.40, 0.45, 0.55, 0.18);
                cr.set_line_width(1.0);
                cr.rectangle(sub_zone.x, sub_zone.y, sub_zone.width, sub_zone.height);
                let _ = cr.stroke();

                let sub_cx = sub_zone.x + sub_zone.width / 2.0;
                let sub_cy = sub_zone.y + sub_zone.height / 2.0;
                let label = format!("{hint_m}{hint_s}");
                draw_label(
                    area,
                    cr,
                    &label,
                    sub_cx,
                    sub_cy,
                    10,
                    5.0,
                    2.0,
                    (0.12, 0.15, 0.22, 0.70),
                    (0.80, 0.85, 0.95, 0.85),
                    false,
                );
            }

            // Outer border of macro cell
            cr.set_source_rgba(0.85, 0.88, 0.95, 0.50);
            cr.set_line_width(if m == 4 { 2.5 } else { 1.6 });
            cr.rectangle(
                macro_zone.x,
                macro_zone.y,
                macro_zone.width,
                macro_zone.height,
            );
            let _ = cr.stroke();

            // Center macro badge
            let label = hint_m.to_string();
            draw_label(
                area,
                cr,
                &label,
                macro_cx,
                macro_cy,
                26,
                18.0,
                10.0,
                (0.97, 0.72, 0.18, 0.88),
                (0.05, 0.06, 0.09, 1.0),
                false,
            );
        }
    } else if !is_locked {
        // Focused region view: highlight selected macro cell, draw 9 subcells with 2-letter hints
        cr.set_source_rgba(0.08, 0.12, 0.20, 0.08);
        cr.rectangle(
            selected_px.x,
            selected_px.y,
            selected_px.width,
            selected_px.height,
        );
        let _ = cr.fill();

        cr.set_source_rgba(0.97, 0.72, 0.18, 0.85);
        cr.set_line_width(3.0);
        cr.rectangle(
            selected_px.x,
            selected_px.y,
            selected_px.width,
            selected_px.height,
        );
        let _ = cr.stroke();

        let stroke1 = state.path[0];
        for (s, &hint_s) in HINTS.iter().enumerate() {
            let sub_zone = child_rect(selected_px, s);
            let cx = sub_zone.x + sub_zone.width / 2.0;
            let cy = sub_zone.y + sub_zone.height / 2.0;

            cr.set_source_rgba(0.72, 0.77, 0.88, 0.65);
            cr.set_line_width(if s == 4 { 2.5 } else { 1.4 });
            cr.rectangle(sub_zone.x, sub_zone.y, sub_zone.width, sub_zone.height);
            let _ = cr.stroke();

            let label = format!("{stroke1}{hint_s}");
            draw_label(
                area,
                cr,
                &label,
                cx,
                cy,
                20,
                14.0,
                8.0,
                (0.97, 0.72, 0.18, 0.90),
                (0.05, 0.06, 0.09, 1.0),
                false,
            );
        }
    } else {
        // Locked target view: highlight locked cell, render reticle and nudge instructions
        cr.set_source_rgba(0.12, 0.18, 0.28, 0.10);
        cr.rectangle(
            selected_px.x,
            selected_px.y,
            selected_px.width,
            selected_px.height,
        );
        let _ = cr.fill();

        cr.set_source_rgba(0.97, 0.72, 0.18, 0.90);
        cr.set_line_width(2.5);
        cr.rectangle(
            selected_px.x,
            selected_px.y,
            selected_px.width,
            selected_px.height,
        );
        let _ = cr.stroke();

        let (reticle_x, reticle_y) = match state.point {
            Some((px, py)) => (px * width, py * height),
            None => (
                (state.rect.x + state.rect.width / 2.0) * width,
                (state.rect.y + state.rect.height / 2.0) * height,
            ),
        };

        let is_snapped = state.snapped_element.is_some();
        draw_reticle(
            cr,
            reticle_x,
            reticle_y,
            is_snapped,
            state.last_click_time_ms,
        );

        // Label above reticle
        let badge_text = if let Some(ref elem) = state.snapped_element {
            let name: String = elem.name.chars().take(18).collect();
            let total = state.snap_candidates.len();
            let cycle_hint = if total > 1 {
                format!(" [{}/{}]", state.snap_index + 1, total)
            } else {
                String::new()
            };
            let chord: String = state.path.iter().collect();
            format!("{}  {}{}", chord.to_uppercase(), name, cycle_hint)
        } else {
            let chord: String = state.path.iter().collect();
            chord.to_uppercase()
        };

        let is_snap_link = state.snapped_element.as_ref().is_some_and(|e| e.is_link);
        let (bg, fg) = if is_snapped {
            if is_snap_link {
                ((0.20, 0.14, 0.05, 0.96), (0.98, 0.75, 0.18, 1.0))
            } else {
                ((0.08, 0.18, 0.28, 0.96), (0.18, 0.85, 0.98, 1.0))
            }
        } else {
            ((0.97, 0.72, 0.18, 0.98), (0.05, 0.06, 0.09, 1.0))
        };

        draw_label(
            area,
            cr,
            &badge_text,
            reticle_x,
            reticle_y - 36.0,
            if is_snapped { 12 } else { 16 },
            10.0,
            5.0,
            bg,
            fg,
            true,
        );
    }

    // Header breadcrumb pill at top center
    let chord_str: String = state.path.iter().collect();
    let breadcrumb = if state.dragging {
        if is_macro {
            "Drag  —  Stroke 1: choose destination  [Esc] Cancel".to_string()
        } else if !is_locked {
            format!(
                "Drag {}  —  Stroke 2: choose drop target  [Space] Drop",
                chord_str.to_uppercase()
            )
        } else {
            format!(
                "Drop target: {}  —  [v/Space/Enter] Drop  [Esc] Cancel",
                chord_str.to_uppercase()
            )
        }
    } else if is_macro {
        "!mouse  —  Stroke 1: choose region  [f] Free Roam  [Esc] Exit".to_string()
    } else if !is_locked {
        format!(
            "Region {}  —  Stroke 2: choose target  [Enter] Region center  [Backspace] Undo",
            chord_str.to_uppercase()
        )
    } else if let Some(ref elem) = state.snapped_element {
        let role = if elem.is_link { "nav link" } else { &elem.role };
        let name_trimmed: String = elem.name.chars().take(28).collect();
        let name_part = if name_trimmed.is_empty() {
            String::new()
        } else {
            format!(" \"{name_trimmed}\"")
        };
        let total = state.snap_candidates.len();
        let tab_hint = if total > 1 {
            format!("  [Tab] {}/{}", state.snap_index + 1, total)
        } else {
            String::new()
        };
        format!(
            "{role}{name_part}{tab_hint}  —  [Enter] Click  [c] Stay  [p] Hover  [f] Free Roam  [hjkl] Nudge  [Esc] Exit"
        )
    } else {
        format!(
            "{}  —  [Enter] Click  [s] Scroll  [v] Drag  [p] Hover  [f] Free Roam  [c] Stay  [Esc] Exit",
            chord_str.to_uppercase()
        )
    };

    let bc_layout = area.create_pango_layout(Some(&breadcrumb));
    let bc_font = pango::FontDescription::from_string("Sans 12");
    bc_layout.set_font_description(Some(&bc_font));
    let (bc_w, bc_h) = bc_layout.pixel_size();

    let bc_pad_x = 16.0;
    let bc_pad_y = 6.0;
    let bc_x = (width - f64::from(bc_w)) / 2.0;
    let bc_y = 16.0;
    let bc_rx = bc_x - bc_pad_x;
    let bc_ry = bc_y - bc_pad_y;
    let bc_rw = f64::from(bc_w) + bc_pad_x * 2.0;
    let bc_rh = f64::from(bc_h) + bc_pad_y * 2.0;

    cr.set_source_rgba(0.04, 0.05, 0.10, 0.82);
    draw_rounded_rect(cr, bc_rx, bc_ry, bc_rw, bc_rh, 6.0);
    let _ = cr.fill();

    cr.set_source_rgba(0.75, 0.82, 0.95, 0.85);
    cr.move_to(bc_x, bc_y);
    pangocairo::functions::show_layout(cr, &bc_layout);
}

fn draw_label(
    area: &gtk4::DrawingArea,
    cr: &Context,
    text: &str,
    cx: f64,
    cy: f64,
    font_size: i32,
    pad_x: f64,
    pad_y: f64,
    bg: (f64, f64, f64, f64),
    fg: (f64, f64, f64, f64),
    preserve_case: bool,
) {
    let display_text = if preserve_case {
        text.to_string()
    } else {
        text.to_uppercase()
    };

    let layout = area.create_pango_layout(Some(&display_text));
    let font_str = format!("Sans Bold {font_size}");
    let font_desc = pango::FontDescription::from_string(&font_str);
    layout.set_font_description(Some(&font_desc));
    let (tw, th) = layout.pixel_size();
    let tw = f64::from(tw);
    let th = f64::from(th);

    let rx = cx - tw / 2.0 - pad_x;
    let ry = cy - th / 2.0 - pad_y;
    let rw = tw + pad_x * 2.0;
    let rh = th + pad_y * 2.0;

    cr.set_source_rgba(bg.0, bg.1, bg.2, bg.3);
    draw_rounded_rect(cr, rx, ry, rw, rh, 4.0);
    let _ = cr.fill();

    cr.set_source_rgba(fg.0, fg.1, fg.2, fg.3);
    cr.move_to(cx - tw / 2.0, cy - th / 2.0);
    pangocairo::functions::show_layout(cr, &layout);
}

fn draw_reticle(cr: &Context, x: f64, y: f64, is_snapped: bool, last_click_time_ms: u64) {
    let radius = 18.0;

    // Visual ripple shockwave on click & stay
    if last_click_time_ms > 0 {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let elapsed_ms = now_ms.saturating_sub(last_click_time_ms);
        if elapsed_ms < 280 {
            let progress = elapsed_ms as f64 / 280.0;
            let ripple_r = radius + progress * 24.0;
            let alpha = (1.0 - progress) * 0.9;
            cr.set_source_rgba(0.20, 0.95, 0.45, alpha); // neon emerald shockwave
            cr.set_line_width(3.0 * (1.0 - progress * 0.5));
            cr.arc(x, y, ripple_r, 0.0, 2.0 * std::f64::consts::PI);
            let _ = cr.stroke();
        }
    }

    if is_snapped {
        // High-contrast electric cyan lock ring
        cr.set_source_rgba(0.18, 0.85, 0.98, 0.40);
        cr.set_line_width(5.0);
        cr.arc(x, y, radius + 3.0, 0.0, 2.0 * std::f64::consts::PI);
        let _ = cr.stroke();

        cr.set_source_rgba(0.18, 0.85, 0.98, 1.0);
        cr.set_line_width(2.2);
        cr.arc(x, y, radius, 0.0, 2.0 * std::f64::consts::PI);
        let _ = cr.stroke();

        // Target corner brackets
        cr.set_line_width(2.0);
        let b_len = 6.0;
        // Top-left
        cr.move_to(x - radius - 2.0, y - radius + b_len);
        cr.line_to(x - radius - 2.0, y - radius - 2.0);
        cr.line_to(x - radius + b_len, y - radius - 2.0);
        // Top-right
        cr.move_to(x + radius + 2.0 - b_len, y - radius - 2.0);
        cr.line_to(x + radius + 2.0, y - radius - 2.0);
        cr.line_to(x + radius + 2.0, y - radius + b_len);
        // Bottom-left
        cr.move_to(x - radius - 2.0, y + radius - b_len);
        cr.line_to(x - radius - 2.0, y + radius + 2.0);
        cr.line_to(x - radius + b_len, y + radius + 2.0);
        // Bottom-right
        cr.move_to(x + radius + 2.0 - b_len, y + radius + 2.0);
        cr.line_to(x + radius + 2.0, y + radius + 2.0);
        cr.line_to(x + radius + 2.0, y + radius - b_len);
        let _ = cr.stroke();

        // Crosshair ticks
        cr.set_line_width(1.8);
        cr.move_to(x - radius - 8.0, y);
        cr.line_to(x - radius + 4.0, y);
        cr.move_to(x + radius - 4.0, y);
        cr.line_to(x + radius + 8.0, y);
        cr.move_to(x, y - radius - 8.0);
        cr.line_to(x, y - radius + 4.0);
        cr.move_to(x, y + radius - 4.0);
        cr.line_to(x, y + radius + 8.0);
        let _ = cr.stroke();

        // Center dot
        cr.set_source_rgba(0.18, 0.85, 0.98, 1.0);
        cr.arc(x, y, 3.5, 0.0, 2.0 * std::f64::consts::PI);
        let _ = cr.fill();
        return;
    }

    // Default golden reticle
    cr.set_source_rgba(0.97, 0.72, 0.18, 0.35);
    cr.set_line_width(4.0);
    cr.arc(x, y, radius + 2.0, 0.0, 2.0 * std::f64::consts::PI);
    let _ = cr.stroke();

    cr.set_source_rgba(0.97, 0.72, 0.18, 0.95);
    cr.set_line_width(2.0);
    cr.arc(x, y, radius, 0.0, 2.0 * std::f64::consts::PI);
    let _ = cr.stroke();

    // Crosshairs
    cr.set_line_width(1.8);
    cr.move_to(x - radius - 8.0, y);
    cr.line_to(x - radius + 4.0, y);
    cr.move_to(x + radius - 4.0, y);
    cr.line_to(x + radius + 8.0, y);
    cr.move_to(x, y - radius - 8.0);
    cr.line_to(x, y - radius + 4.0);
    cr.move_to(x, y + radius - 4.0);
    cr.line_to(x, y + radius + 8.0);
    let _ = cr.stroke();

    // Center dot
    cr.set_source_rgba(0.97, 0.72, 0.18, 1.0);
    cr.arc(x, y, 3.0, 0.0, 2.0 * std::f64::consts::PI);
    let _ = cr.fill();
}

fn draw_rounded_rect(cr: &Context, x: f64, y: f64, w: f64, h: f64, r: f64) {
    let pi = std::f64::consts::PI;
    cr.move_to(x + r, y);
    cr.line_to(x + w - r, y);
    cr.arc(x + w - r, y + r, r, -pi / 2.0, 0.0);
    cr.line_to(x + w, y + h - r);
    cr.arc(x + w - r, y + h - r, r, 0.0, pi / 2.0);
    cr.line_to(x + r, y + h);
    cr.arc(x + r, y + h - r, r, pi / 2.0, pi);
    cr.line_to(x, y + r);
    cr.arc(x + r, y + r, r, pi, -pi / 2.0);
    cr.close_path();
}

fn draw_topbar(
    area: &gtk4::DrawingArea,
    cr: &Context,
    width: f64,
    height: f64,
    state: &OverlayDrawState,
) {
    // Subtle background tint
    cr.set_source_rgba(0.01, 0.02, 0.04, 0.06);
    cr.rectangle(0.0, 0.0, width, height);
    let _ = cr.fill();

    // 3 top bar target zones along top edge
    let zones = [
        ('A', 'a', "ACTIVITIES", 68.0f64.max(0.013 * width)),
        ('S', 's', "CLOCK / DATE", 0.500 * width),
        (
            'D',
            'd',
            "SETTINGS / WIFI",
            (width - 70.0).min(0.973 * width),
        ),
    ];

    // Glowing golden top border
    cr.set_source_rgba(0.97, 0.72, 0.18, 0.90);
    cr.set_line_width(3.0);
    cr.move_to(0.0, 1.5);
    cr.line_to(width, 1.5);
    let _ = cr.stroke();

    for (key, id, name, zx) in zones {
        let is_selected = state.top_bar_target == Some(id);
        let label = format!("{key} • {name}");
        draw_label(
            area,
            cr,
            &label,
            zx,
            26.0,
            12,
            12.0,
            6.0,
            if is_selected {
                (0.97, 0.72, 0.18, 0.95)
            } else {
                (0.10, 0.14, 0.22, 0.95)
            },
            if is_selected {
                (0.05, 0.08, 0.12, 1.0)
            } else {
                (0.97, 0.72, 0.18, 1.0)
            },
            true,
        );
    }

    // Reticle
    let reticle_x = state.point.map_or(0.973 * width, |(px, _)| px * width);
    let reticle_y = 16.0;
    draw_reticle(cr, reticle_x, reticle_y, false, state.last_click_time_ms);

    // Upward arrow pointing into top bar
    cr.set_source_rgba(0.97, 0.72, 0.18, 1.0);
    cr.move_to(reticle_x, 2.0);
    cr.line_to(reticle_x - 8.0, 14.0);
    cr.line_to(reticle_x + 8.0, 14.0);
    cr.close_path();
    let _ = cr.fill();

    // Floating HUD at bottom
    let bar_w = (width - 40.0).min(820.0);
    let bar_h = 44.0;
    let bar_x = (width - bar_w) / 2.0;
    let bar_y = height - bar_h - 24.0;

    cr.set_source_rgba(0.06, 0.08, 0.12, 0.92);
    cr.rectangle(bar_x, bar_y, bar_w, bar_h);
    let _ = cr.fill();

    cr.set_source_rgba(0.97, 0.72, 0.18, 0.85);
    cr.set_line_width(1.0);
    cr.rectangle(bar_x, bar_y, bar_w, bar_h);
    let _ = cr.stroke();

    let text = "Top Bar — [a] Activities [s] Clock [d] Settings [c] Stay [Enter] Click [Tab] Grid [Esc] Exit";
    let layout = area.create_pango_layout(Some(text));
    let font_desc = pango::FontDescription::from_string("Sans 12");
    layout.set_font_description(Some(&font_desc));
    let (text_w, text_h) = layout.pixel_size();

    cr.set_source_rgba(0.80, 0.88, 1.0, 0.90);
    cr.move_to(
        bar_x + (bar_w - f64::from(text_w)) / 2.0,
        bar_y + (bar_h - f64::from(text_h)) / 2.0,
    );
    pangocairo::functions::show_layout(cr, &layout);
}
