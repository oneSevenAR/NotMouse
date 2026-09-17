use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use gtk4::gdk::Key;
use gtk4::glib;
use gtk4::prelude::*;
use notmouse_core::Rect;

use crate::OverlayEvent;
use crate::atspi::{self, AccessibleElement};
use crate::input::{InputDevice, MouseButton};

pub mod draw;
pub mod kinematics;

use draw::{HINTS, OverlayDrawState, OverlayMode, child_rect, draw_overlay};
use kinematics::{Direction, FreeRoamKinematics};

pub struct State {
    pub mode: OverlayMode,
    pub path: Vec<char>,
    pub rect: Rect,
    pub history: Vec<Rect>,
    pub point: Option<(f64, f64)>,
    pub dragging: bool,
    pub drag_start_point: Option<(f64, f64)>,
    pub top_bar_target: Option<char>,
    pub snapped_element: Option<AccessibleElement>,
    pub snap_candidates: Vec<AccessibleElement>,
    pub snap_index: usize,
    pub cached_elements: Vec<AccessibleElement>,
    pub target_pid: Option<u32>,
    pub scroll_speed: i32,
    pub last_scroll_dir: Option<&'static str>,
    pub last_click_time_ms: u64,
    pub kinematics: FreeRoamKinematics,
    pub is_turbo: bool,
    pub is_crawl: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            mode: OverlayMode::Grid,
            path: Vec::new(),
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width: 1.0,
                height: 1.0,
            },
            history: Vec::new(),
            point: None,
            dragging: false,
            drag_start_point: None,
            top_bar_target: None,
            snapped_element: None,
            snap_candidates: Vec::new(),
            snap_index: 0,
            cached_elements: Vec::new(),
            target_pid: None,
            scroll_speed: 5,
            last_scroll_dir: None,
            last_click_time_ms: 0,
            kinematics: FreeRoamKinematics::new(0.5, 0.5),
            is_turbo: false,
            is_crawl: false,
        }
    }
}

pub fn run_overlay(is_resident: bool) -> Result<(), String> {
    let app = gtk4::Application::builder()
        .application_id("io.github.onesevenar.notmouse.overlay")
        .flags(gtk4::gio::ApplicationFlags::NON_UNIQUE)
        .build();

    app.connect_activate(move |application| {
        build_ui(application, is_resident);
    });

    let empty_args: [&str; 0] = [];
    app.run_with_args(&empty_args);
    Ok(())
}

fn build_ui(application: &gtk4::Application, is_resident: bool) {
    let window = gtk4::ApplicationWindow::builder()
        .application(application)
        .title("!mouse")
        .decorated(false)
        .build();

    window.add_css_class("notmouse-overlay");

    let provider = gtk4::CssProvider::new();
    provider.load_from_string(
        "window,
         window.background,
         window.maximized,
         window.fullscreen,
         .background,
         .fullscreen,
         .maximized,
         .notmouse-overlay,
         drawingarea {
             background-color: rgba(0, 0, 0, 0);
             background-image: none;
             box-shadow: none;
             border: none;
         }",
    );

    if let Some(display) = gtk4::gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_USER,
        );
    }

    window.connect_realize(|w| {
        if let Some(surface) = w.surface() {
            surface.set_opaque_region(None);
        }
    });

    let state = Rc::new(RefCell::new(State::default()));
    let input_device = Rc::new(RefCell::new(InputDevice::new().ok()));

    let drawing_area = gtk4::DrawingArea::builder()
        .hexpand(true)
        .vexpand(true)
        .focusable(true)
        .build();

    {
        let state = state.clone();
        drawing_area.set_draw_func(move |area, cr, width, height| {
            cr.set_operator(cairo::Operator::Clear);
            let _ = cr.paint();
            cr.set_operator(cairo::Operator::Over);

            let s = state.borrow();
            let draw_state = OverlayDrawState {
                mode: s.mode,
                path: s.path.clone(),
                rect: s.rect,
                point: s.point,
                dragging: s.dragging,
                drag_start_point: s.drag_start_point,
                top_bar_target: s.top_bar_target,
                snapped_element: s.snapped_element.clone(),
                snap_candidates: s.snap_candidates.clone(),
                snap_index: s.snap_index,
                last_scroll_dir: s.last_scroll_dir,
                scroll_speed: s.scroll_speed,
                last_click_time_ms: s.last_click_time_ms,
                is_turbo: s.is_turbo,
                is_crawl: s.is_crawl,
            };
            draw_overlay(area, cr, width, height, &draw_state);
        });
    }

    // Kinematics ticker (60Hz / 16ms)
    {
        let state = state.clone();
        let input_device = input_device.clone();
        let drawing_area = drawing_area.clone();
        let window = window.clone();

        glib::timeout_add_local(Duration::from_millis(16), move || {
            let mut s = state.borrow_mut();
            if s.mode == OverlayMode::FreeRoam {
                let win_w = f64::from(window.width());
                let win_h = f64::from(window.height());
                if let Some((nx, ny)) = s.kinematics.update(0.016, win_w, win_h) {
                    s.point = Some((nx, ny));
                    emit_cursor_move(&input_device, &window, s.mode, nx, ny);
                    drawing_area.queue_draw();
                }
            }
            glib::ControlFlow::Continue
        });
    }

    let key_controller = gtk4::EventControllerKey::new();

    // Key Pressed Handler
    {
        let state = state.clone();
        let input_device = input_device.clone();
        let drawing_area = drawing_area.clone();
        let window = window.clone();
        let app = application.clone();

        key_controller.connect_key_pressed(move |_controller, keyval, _keycode, modifier| {
            let is_shift = modifier.contains(gtk4::gdk::ModifierType::SHIFT_MASK);
            let is_ctrl = modifier.contains(gtk4::gdk::ModifierType::CONTROL_MASK)
                || modifier.contains(gtk4::gdk::ModifierType::ALT_MASK);

            let mut s = state.borrow_mut();
            s.is_turbo = is_shift;
            s.is_crawl = is_ctrl;
            s.kinematics.set_modifiers(is_shift, is_ctrl);

            // ── SCROLL MODE ──────────────────────────────────────────────────
            if s.mode == OverlayMode::Scroll {
                match keyval {
                    Key::Escape | Key::q => {
                        dismiss_overlay(&window, &app, is_resident);
                        return glib::Propagation::Stop;
                    }
                    Key::Tab => {
                        // Return to Fullscreen Grid mode
                        s.mode = OverlayMode::Grid;
                        show_overlay_window(&window);
                        drawing_area.queue_draw();
                        return glib::Propagation::Stop;
                    }
                    Key::j | Key::s | Key::Down => {
                        s.last_scroll_dir = Some("down");
                        let mult = if is_shift { 3 } else { 1 };
                        emit_scroll(&input_device, -s.scroll_speed * mult, 0);
                        drawing_area.queue_draw();
                        return glib::Propagation::Stop;
                    }
                    Key::k | Key::w | Key::Up => {
                        s.last_scroll_dir = Some("up");
                        let mult = if is_shift { 3 } else { 1 };
                        emit_scroll(&input_device, s.scroll_speed * mult, 0);
                        drawing_area.queue_draw();
                        return glib::Propagation::Stop;
                    }
                    Key::d | Key::Page_Down => {
                        s.last_scroll_dir = Some("down");
                        emit_scroll(&input_device, -s.scroll_speed * 5, 0);
                        drawing_area.queue_draw();
                        return glib::Propagation::Stop;
                    }
                    Key::u | Key::Page_Up => {
                        s.last_scroll_dir = Some("up");
                        emit_scroll(&input_device, s.scroll_speed * 5, 0);
                        drawing_area.queue_draw();
                        return glib::Propagation::Stop;
                    }
                    Key::h | Key::Left => {
                        s.last_scroll_dir = Some("left");
                        emit_scroll(&input_device, 0, -s.scroll_speed * 2);
                        drawing_area.queue_draw();
                        return glib::Propagation::Stop;
                    }
                    Key::l | Key::Right => {
                        s.last_scroll_dir = Some("right");
                        emit_scroll(&input_device, 0, s.scroll_speed * 2);
                        drawing_area.queue_draw();
                        return glib::Propagation::Stop;
                    }
                    _ => {}
                }
                return glib::Propagation::Proceed;
            }

            // ── FREE ROAM MODE ───────────────────────────────────────────────
            if s.mode == OverlayMode::FreeRoam {
                let win_w = f64::from(window.width());
                let win_h = f64::from(window.height());

                match keyval {
                    Key::Escape | Key::q => {
                        if s.dragging {
                            emit_release(&input_device, MouseButton::Left);
                            s.dragging = false;
                        }
                        dismiss_overlay(&window, &app, is_resident);
                        return glib::Propagation::Stop;
                    }
                    Key::Tab => {
                        // Return to Fullscreen Grid mode
                        s.mode = OverlayMode::Grid;
                        show_overlay_window(&window);
                        drawing_area.queue_draw();
                        return glib::Propagation::Stop;
                    }
                    Key::Return | Key::KP_Enter | Key::space => {
                        if s.dragging {
                            // Drop target
                            emit_release(&input_device, MouseButton::Left);
                            s.dragging = false;
                            dismiss_overlay(&window, &app, is_resident);
                            return glib::Propagation::Stop;
                        }
                        emit_selection(
                            &input_device,
                            &window,
                            &app,
                            is_resident,
                            s.mode,
                            &s.path,
                            if is_shift {
                                "right-click"
                            } else {
                                "click"
                            },
                            s.point,
                        );
                        return glib::Propagation::Stop;
                    }
                    Key::r => {
                        emit_selection(
                            &input_device,
                            &window,
                            &app,
                            is_resident,
                            s.mode,
                            &s.path,
                            "right-click",
                            s.point,
                        );
                        return glib::Propagation::Stop;
                    }
                    Key::d => {
                        emit_selection(
                            &input_device,
                            &window,
                            &app,
                            is_resident,
                            s.mode,
                            &s.path,
                            "double-click",
                            s.point,
                        );
                        return glib::Propagation::Stop;
                    }
                    Key::m => {
                        emit_selection(
                            &input_device,
                            &window,
                            &app,
                            is_resident,
                            s.mode,
                            &s.path,
                            "middle-click",
                            s.point,
                        );
                        return glib::Propagation::Stop;
                    }
                    Key::v => {
                        // Toggle drag
                        if s.dragging {
                            emit_release(&input_device, MouseButton::Left);
                            s.dragging = false;
                            dismiss_overlay(&window, &app, is_resident);
                        } else {
                            s.dragging = true;
                            s.drag_start_point = s.point;
                            emit_press(&input_device, MouseButton::Left);
                            drawing_area.queue_draw();
                        }
                        return glib::Propagation::Stop;
                    }
                    Key::p => {
                        // Point & Hover: dismiss immediately leaving cursor positioned
                        dismiss_overlay(&window, &app, is_resident);
                        return glib::Propagation::Stop;
                    }
                    Key::s | Key::w => {
                        // Switch to Scroll HUD
                        s.mode = OverlayMode::Scroll;
                        window.unmaximize();
                        window.set_default_size(560, 48);
                        drawing_area.queue_draw();
                        return glib::Propagation::Stop;
                    }
                    Key::h | Key::Left => {
                        let (nx, ny) = s.kinematics.press_direction(Direction::Left, win_w, win_h);
                        s.point = Some((nx, ny));
                        emit_cursor_move(&input_device, &window, s.mode, nx, ny);
                        drawing_area.queue_draw();
                        return glib::Propagation::Stop;
                    }
                    Key::l | Key::Right => {
                        let (nx, ny) = s.kinematics.press_direction(Direction::Right, win_w, win_h);
                        s.point = Some((nx, ny));
                        emit_cursor_move(&input_device, &window, s.mode, nx, ny);
                        drawing_area.queue_draw();
                        return glib::Propagation::Stop;
                    }
                    Key::k | Key::Up => {
                        let (nx, ny) = s.kinematics.press_direction(Direction::Up, win_w, win_h);
                        s.point = Some((nx, ny));
                        emit_cursor_move(&input_device, &window, s.mode, nx, ny);
                        drawing_area.queue_draw();
                        return glib::Propagation::Stop;
                    }
                    Key::j | Key::Down => {
                        let (nx, ny) = s.kinematics.press_direction(Direction::Down, win_w, win_h);
                        s.point = Some((nx, ny));
                        emit_cursor_move(&input_device, &window, s.mode, nx, ny);
                        drawing_area.queue_draw();
                        return glib::Propagation::Stop;
                    }
                    _ => {}
                }
                return glib::Propagation::Proceed;
            }

            // ── TOP BAR MODE ──────────────────────────────────────────────────
            if s.mode == OverlayMode::TopBar {
                match keyval {
                    Key::Escape | Key::q => {
                        dismiss_overlay(&window, &app, is_resident);
                        return glib::Propagation::Stop;
                    }
                    Key::Tab | Key::BackSpace => {
                        s.mode = OverlayMode::Grid;
                        s.point = None;
                        s.top_bar_target = None;
                        drawing_area.queue_draw();
                        return glib::Propagation::Stop;
                    }
                    Key::Return | Key::KP_Enter | Key::space => {
                        emit_selection(
                            &input_device,
                            &window,
                            &app,
                            is_resident,
                            s.mode,
                            &s.path,
                            if is_shift {
                                "right-click"
                            } else {
                                "click"
                            },
                            s.point,
                        );
                        return glib::Propagation::Stop;
                    }
                    Key::r => {
                        emit_selection(
                            &input_device,
                            &window,
                            &app,
                            is_resident,
                            s.mode,
                            &s.path,
                            "right-click",
                            s.point,
                        );
                        return glib::Propagation::Stop;
                    }
                    Key::c => {
                        let btn = if is_shift {
                            MouseButton::Right
                        } else {
                            MouseButton::Left
                        };
                        emit_click(&input_device, &window, s.mode, btn, s.point);
                        window.set_visible(false);
                        let now_ms = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_millis() as u64)
                            .unwrap_or(0);
                        s.last_click_time_ms = now_ms;

                        let w_clone = window.clone();
                        let da_clone = drawing_area.clone();
                        glib::timeout_add_local(Duration::from_millis(180), move || {
                            w_clone.set_visible(true);
                            w_clone.present();
                            da_clone.grab_focus();
                            da_clone.queue_draw();
                            glib::ControlFlow::Break
                        });
                        return glib::Propagation::Stop;
                    }
                    Key::a => {
                        s.top_bar_target = Some('a');
                        let win_w = f64::from(window.width().max(1));
                        let nx = (68.0f64.max(0.013 * win_w)) / win_w;
                        s.point = Some((nx, 0.0));
                        emit_cursor_move(&input_device, &window, s.mode, nx, 0.0);
                        drawing_area.queue_draw();
                        return glib::Propagation::Stop;
                    }
                    Key::s => {
                        s.top_bar_target = Some('s');
                        s.point = Some((0.500, 0.0));
                        emit_cursor_move(&input_device, &window, s.mode, 0.500, 0.0);
                        drawing_area.queue_draw();
                        return glib::Propagation::Stop;
                    }
                    Key::d => {
                        s.top_bar_target = Some('d');
                        let win_w = f64::from(window.width().max(1));
                        let nx = ((win_w - 70.0).min(0.973 * win_w)) / win_w;
                        s.point = Some((nx, 0.0));
                        emit_cursor_move(&input_device, &window, s.mode, nx, 0.0);
                        drawing_area.queue_draw();
                        return glib::Propagation::Stop;
                    }
                    _ => {}
                }
                return glib::Propagation::Proceed;
            }

            // ── GRID MODE ────────────────────────────────────────────────────
            if keyval == Key::Escape {
                if s.dragging {
                    emit_release(&input_device, MouseButton::Left);
                    s.dragging = false;
                }
                dismiss_overlay(&window, &app, is_resident);
                return glib::Propagation::Stop;
            }

            if keyval == Key::BackSpace {
                if let Some(prev) = s.history.pop() {
                    s.rect = prev;
                    s.path.pop();
                    s.point = None;
                    s.snapped_element = None;
                    s.snap_candidates.clear();
                    s.snap_index = 0;
                    drawing_area.queue_draw();
                }
                return glib::Propagation::Stop;
            }

            let is_locked = s.path.len() >= 2;

            if is_locked {
                // Actions in locked mode
                match keyval {
                    Key::Return | Key::KP_Enter | Key::space => {
                        if s.dragging {
                            emit_release(&input_device, MouseButton::Left);
                            s.dragging = false;
                            dismiss_overlay(&window, &app, is_resident);
                            return glib::Propagation::Stop;
                        }
                        emit_selection(
                            &input_device,
                            &window,
                            &app,
                            is_resident,
                            s.mode,
                            &s.path,
                            if is_shift {
                                "right-click"
                            } else {
                                "click"
                            },
                            s.point,
                        );
                        return glib::Propagation::Stop;
                    }
                    Key::r => {
                        emit_selection(
                            &input_device,
                            &window,
                            &app,
                            is_resident,
                            s.mode,
                            &s.path,
                            "right-click",
                            s.point,
                        );
                        return glib::Propagation::Stop;
                    }
                    Key::d => {
                        emit_selection(
                            &input_device,
                            &window,
                            &app,
                            is_resident,
                            s.mode,
                            &s.path,
                            "double-click",
                            s.point,
                        );
                        return glib::Propagation::Stop;
                    }
                    Key::m => {
                        emit_selection(
                            &input_device,
                            &window,
                            &app,
                            is_resident,
                            s.mode,
                            &s.path,
                            "middle-click",
                            s.point,
                        );
                        return glib::Propagation::Stop;
                    }
                    // Hover primitives
                    Key::p => {
                        // Point & Hover: cursor is already coupled! Just dismiss window.
                        dismiss_overlay(&window, &app, is_resident);
                        return glib::Propagation::Stop;
                    }
                    Key::P => {
                        // Hover & Stay: temporarily hide window so underlying app opens flyout
                        window.set_visible(false);
                        let w_clone = window.clone();
                        let da_clone = drawing_area.clone();
                        let s_clone = state.clone();
                        glib::timeout_add_local(Duration::from_millis(120), move || {
                            w_clone.set_visible(true);
                            w_clone.present();
                            da_clone.grab_focus();
                            da_clone.queue_draw();
                            trigger_background_scan(&s_clone, &da_clone, &w_clone);
                            glib::ControlFlow::Break
                        });
                        return glib::Propagation::Stop;
                    }
                    // Free roam from locked target
                    Key::f => {
                        s.mode = OverlayMode::FreeRoam;
                        let initial = s.point.unwrap_or((0.5, 0.5));
                        s.kinematics = FreeRoamKinematics::new(initial.0, initial.1);
                        window.unmaximize();
                        window.set_default_size(680, 48);
                        drawing_area.queue_draw();
                        return glib::Propagation::Stop;
                    }
                    // Kinetic Scroll HUD
                    Key::s | Key::w => {
                        s.mode = OverlayMode::Scroll;
                        s.last_scroll_dir = Some(if keyval == Key::w { "up" } else { "down" });
                        window.unmaximize();
                        window.set_default_size(560, 48);
                        let mult = if is_shift { 3 } else { 1 };
                        emit_scroll(
                            &input_device,
                            if keyval == Key::w {
                                s.scroll_speed * mult
                            } else {
                                -s.scroll_speed * mult
                            },
                            0,
                        );
                        drawing_area.queue_draw();
                        return glib::Propagation::Stop;
                    }
                    // Click & Stay
                    Key::c => {
                        let btn = if is_shift {
                            MouseButton::Right
                        } else {
                            MouseButton::Left
                        };
                        let was_link = s
                            .snapped_element
                            .as_ref()
                            .is_some_and(|e| e.is_link);
                        let now_ms = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_millis() as u64)
                            .unwrap_or(0);
                        s.last_click_time_ms = now_ms;

                        emit_click(&input_device, &window, s.mode, btn, s.point);
                        window.set_visible(false);

                        let w_clone = window.clone();
                        let da_clone = drawing_area.clone();
                        let s_clone = state.clone();

                        let restore_delay_ms = if was_link { 600 } else { 180 };
                        glib::timeout_add_local(Duration::from_millis(restore_delay_ms), move || {
                            w_clone.set_visible(true);
                            w_clone.present();
                            da_clone.grab_focus();
                            da_clone.queue_draw();
                            trigger_background_scan(&s_clone, &da_clone, &w_clone);
                            glib::ControlFlow::Break
                        });
                        return glib::Propagation::Stop;
                    }
                    // Click & Drag (v)
                    Key::v => {
                        if s.dragging {
                            emit_release(&input_device, MouseButton::Left);
                            s.dragging = false;
                            dismiss_overlay(&window, &app, is_resident);
                        } else {
                            s.dragging = true;
                            s.drag_start_point = s.point;
                            emit_press(&input_device, MouseButton::Left);

                            // Switch to Free Roam HUD so dragging has live Wayland pointer focus
                            s.mode = OverlayMode::FreeRoam;
                            let initial = s.point.unwrap_or((0.5, 0.5));
                            s.kinematics = FreeRoamKinematics::new(initial.0, initial.1);
                            window.unmaximize();
                            window.set_default_size(680, 48);
                            drawing_area.queue_draw();
                        }
                        return glib::Propagation::Stop;
                    }
                    // Tab / Shift+Tab: cycle snap candidates
                    Key::Tab | Key::ISO_Left_Tab => {
                        if !s.snap_candidates.is_empty() {
                            let total = s.snap_candidates.len();
                            let next_idx = if is_shift {
                                (s.snap_index + total - 1) % total
                            } else {
                                (s.snap_index + 1) % total
                            };
                            s.snap_index = next_idx;
                            let candidate = s.snap_candidates[next_idx].clone();
                            s.snapped_element = Some(candidate.clone());

                            let win_w = f64::from(window.width());
                            let win_h = f64::from(window.height());
                            let nx = (candidate.cx / win_w).clamp(0.0, 1.0);
                            let ny = (candidate.cy / win_h).clamp(0.0, 1.0);
                            s.point = Some((nx, ny));

                            // Live pointer coupling on cycle!
                            emit_cursor_move(&input_device, &window, s.mode, nx, ny);
                            drawing_area.queue_draw();
                        }
                        return glib::Propagation::Stop;
                    }
                    // Nudge
                    Key::h | Key::Left => {
                        nudge_reticle(&mut s, &input_device, &window, -0.005, 0.0);
                        drawing_area.queue_draw();
                        return glib::Propagation::Stop;
                    }
                    Key::l | Key::Right => {
                        nudge_reticle(&mut s, &input_device, &window, 0.005, 0.0);
                        drawing_area.queue_draw();
                        return glib::Propagation::Stop;
                    }
                    Key::k | Key::Up => {
                        nudge_reticle(&mut s, &input_device, &window, 0.0, -0.005);
                        drawing_area.queue_draw();
                        return glib::Propagation::Stop;
                    }
                    Key::j | Key::Down => {
                        nudge_reticle(&mut s, &input_device, &window, 0.0, 0.005);
                        drawing_area.queue_draw();
                        return glib::Propagation::Stop;
                    }
                    _ => {}
                }
                return glib::Propagation::Proceed;
            }

            // Stroke 1 center confirmation with Enter/Space
            if s.path.len() == 1
                && (keyval == Key::Return || keyval == Key::KP_Enter || keyval == Key::space)
            {
                if s.dragging {
                    emit_release(&input_device, MouseButton::Left);
                    s.dragging = false;
                    dismiss_overlay(&window, &app, is_resident);
                    return glib::Propagation::Stop;
                }
                emit_selection(
                    &input_device,
                    &window,
                    &app,
                    is_resident,
                    s.mode,
                    &s.path,
                    if is_shift {
                        "right-click"
                    } else {
                        "click"
                    },
                    s.point,
                );
                return glib::Propagation::Stop;
            }

            // Stroke 1 Free Roam mode shortcut (f)
            if s.path.is_empty() && keyval == Key::f {
                s.mode = OverlayMode::FreeRoam;
                let initial = s.point.unwrap_or((0.5, 0.5));
                s.kinematics = FreeRoamKinematics::new(initial.0, initial.1);
                window.unmaximize();
                window.set_default_size(680, 48);
                drawing_area.queue_draw();
                return glib::Propagation::Stop;
            }

            // Stroke 1 Top Bar mode shortcut (t)
            if s.path.is_empty() && keyval == Key::t {
                s.mode = OverlayMode::TopBar;
                s.top_bar_target = Some('d');
                let win_w = f64::from(window.width().max(1));
                let nx = ((win_w - 70.0).min(0.973 * win_w)) / win_w;
                s.point = Some((nx, 0.0));
                emit_cursor_move(&input_device, &window, s.mode, nx, 0.0);
                drawing_area.queue_draw();
                return glib::Propagation::Stop;
            }

            // Hint chord character check
            let key_char = match keyval {
                Key::a => 'a',
                Key::s => 's',
                Key::d => 'd',
                Key::f => 'f',
                Key::j => 'j',
                Key::k => 'k',
                Key::l => 'l',
                Key::g => 'g',
                Key::h => 'h',
                _ => return glib::Propagation::Proceed,
            };

            let Some(index) = HINTS.iter().position(|&c| c == key_char) else {
                return glib::Propagation::Proceed;
            };

            let cur_rect = s.rect;
            s.history.push(cur_rect);
            s.rect = child_rect(cur_rect, index);
            s.path.push(key_char);

            let center_x = s.rect.x + s.rect.width / 2.0;
            let center_y = s.rect.y + s.rect.height / 2.0;
            s.point = Some((center_x, center_y));

            // ── LIVE POINTER COUPLING ("WAKE ON AIM") ────────────────────────
            // Instantly dispatch OS cursor movement to the center of the chosen cell!
            // Autohiding elements (YouTube player controls, video overlays, dropdowns)
            // receive real Wayland mousemove events immediately!
            emit_cursor_move(&input_device, &window, s.mode, center_x, center_y);

            // Stroke 2 entered: attempt magnetic snap to element!
            if s.path.len() >= 2 {
                let win_w = f64::from(window.width().max(1));
                let win_h = f64::from(window.height().max(1));

                let target_px_x = center_x * win_w;
                let target_px_y = center_y * win_h;
                let cell_min_x = s.rect.x * win_w;
                let cell_max_x = (s.rect.x + s.rect.width) * win_w;
                let cell_min_y = s.rect.y * win_h;
                let cell_max_y = (s.rect.y + s.rect.height) * win_h;

                let snap_res = atspi::snap_to_nearest(
                    &s.cached_elements,
                    target_px_x,
                    target_px_y,
                    cell_min_x,
                    cell_max_x,
                    cell_min_y,
                    cell_max_y,
                );

                if let Some(candidate) = snap_res.candidate {
                    let nx = (candidate.cx / win_w).clamp(0.0, 1.0);
                    let ny = (candidate.cy / win_h).clamp(0.0, 1.0);
                    s.point = Some((nx, ny));
                    s.snapped_element = Some(candidate);
                    s.snap_candidates = snap_res.all_candidates;
                    s.snap_index = snap_res.index;

                    // Magnetic cursor snap dispatch
                    emit_cursor_move(&input_device, &window, s.mode, nx, ny);
                }
            }

            drawing_area.queue_draw();
            glib::Propagation::Stop
        });
    }

    // Key Released Handler
    {
        let state = state.clone();
        key_controller.connect_key_released(move |_controller, keyval, _keycode, _modifier| {
            let mut s = state.borrow_mut();
            if s.mode == OverlayMode::FreeRoam {
                match keyval {
                    Key::h | Key::Left => s.kinematics.release_direction(Direction::Left),
                    Key::l | Key::Right => s.kinematics.release_direction(Direction::Right),
                    Key::k | Key::Up => s.kinematics.release_direction(Direction::Up),
                    Key::j | Key::Down => s.kinematics.release_direction(Direction::Down),
                    _ => {}
                }
            }
        });
    }

    window.add_controller(key_controller);
    window.set_child(Some(&drawing_area));

    if is_resident {
        setup_overlay_socket(state.clone(), window.clone(), drawing_area.clone());
        // Pre-warm Wayland surface and GTK pipeline invisibly
        window.set_opacity(0.0);
        show_overlay_window(&window);

        let w_clone = window.clone();
        glib::idle_add_local(move || {
            w_clone.set_visible(false);
            w_clone.set_opacity(1.0);
            glib::ControlFlow::Break
        });
    } else {
        show_overlay_window(&window);
        drawing_area.grab_focus();

        // Trigger initial background AT-SPI scan
        trigger_background_scan(&state, &drawing_area, &window);
    }
}

fn show_overlay_window(window: &gtk4::ApplicationWindow) {
    if let Some(monitor) = get_active_monitor(Some(window)) {
        let geom = monitor.geometry();
        window.set_default_size(geom.width(), geom.height());
    }
    window.maximize();
    window.set_visible(true);
    window.present();
    if let Some(surface) = window.surface() {
        surface.set_opaque_region(None);
    }
}

fn setup_overlay_socket(
    state: Rc<RefCell<State>>,
    window: gtk4::ApplicationWindow,
    drawing_area: gtk4::DrawingArea,
) {
    let sock_path = crate::session::overlay_socket_path();
    let _ = std::fs::remove_file(&sock_path);

    let listener = match std::os::unix::net::UnixListener::bind(&sock_path) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("!mouse: failed to bind overlay socket: {e}");
            return;
        }
    };

    #[derive(serde::Deserialize)]
    struct OverlayShowMessage {
        #[serde(default)]
        target_pid: Option<u32>,
    }

    let (tx, rx) = std::sync::mpsc::channel::<Option<u32>>();

    std::thread::spawn(move || {
        for mut s in listener.incoming().flatten() {
            use std::io::Read;
            let mut buf = [0u8; 512];
            if let Ok(n) = s.read(&mut buf) {
                let msg = String::from_utf8_lossy(&buf[..n]);
                if msg.contains("\"show\"") {
                    let req: Option<OverlayShowMessage> = serde_json::from_str(&msg).ok();
                    let pid = req.and_then(|r| r.target_pid);
                    let _ = tx.send(pid);
                }
            }
        }
    });

    glib::timeout_add_local(Duration::from_millis(20), move || {
        if let Ok(target_pid) = rx.try_recv() {
            {
                let mut s = state.borrow_mut();
                s.target_pid = target_pid;
                s.mode = OverlayMode::Grid;
                s.path.clear();
                s.rect = Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 1.0,
                    height: 1.0,
                };
                s.history.clear();
                s.point = None;
                s.top_bar_target = None;
                s.snapped_element = None;
                s.snap_candidates.clear();
                s.snap_index = 0;
            }
            show_overlay_window(&window);
            drawing_area.grab_focus();
            drawing_area.queue_draw();
            trigger_background_scan(&state, &drawing_area, &window);
        }
        glib::ControlFlow::Continue
    });
}

fn trigger_background_scan(
    state: &Rc<RefCell<State>>,
    drawing_area: &gtk4::DrawingArea,
    window: &gtk4::ApplicationWindow,
) {
    let (tx, rx) = std::sync::mpsc::channel::<Vec<AccessibleElement>>();
    let target_pid = state.borrow().target_pid;

    std::thread::spawn(move || {
        let elements = zbus::block_on(async {
            let scanner = atspi::Scanner::new().await.ok()?;
            Some(scanner.scan(target_pid, None).await)
        });
        if let Some(elems) = elements {
            let _ = tx.send(elems);
        }
    });

    let state = state.clone();
    let da = drawing_area.clone();
    let win = window.clone();

    glib::timeout_add_local(Duration::from_millis(40), move || {
        if let Ok(elements) = rx.try_recv() {
            let mut s = state.borrow_mut();
            s.cached_elements = elements;

            // If already on Stroke 2, perform snap immediately
            if s.path.len() >= 2 && s.snapped_element.is_none() {
                let win_w = f64::from(win.width().max(1));
                let win_h = f64::from(win.height().max(1));
                let (center_x, center_y) = s.point.unwrap_or((
                    s.rect.x + s.rect.width / 2.0,
                    s.rect.y + s.rect.height / 2.0,
                ));

                let target_px_x = center_x * win_w;
                let target_px_y = center_y * win_h;
                let cell_min_x = s.rect.x * win_w;
                let cell_max_x = (s.rect.x + s.rect.width) * win_w;
                let cell_min_y = s.rect.y * win_h;
                let cell_max_y = (s.rect.y + s.rect.height) * win_h;

                let snap_res = atspi::snap_to_nearest(
                    &s.cached_elements,
                    target_px_x,
                    target_px_y,
                    cell_min_x,
                    cell_max_x,
                    cell_min_y,
                    cell_max_y,
                );
                if let Some(candidate) = snap_res.candidate {
                    let nx = (candidate.cx / win_w).clamp(0.0, 1.0);
                    let ny = (candidate.cy / win_h).clamp(0.0, 1.0);
                    s.point = Some((nx, ny));
                    s.snapped_element = Some(candidate);
                    s.snap_candidates = snap_res.all_candidates;
                    s.snap_index = snap_res.index;
                }
            }
            da.queue_draw();
            return glib::ControlFlow::Break;
        }
        glib::ControlFlow::Continue
    });
}

fn get_active_monitor(window: Option<&gtk4::ApplicationWindow>) -> Option<gtk4::gdk::Monitor> {
    let display = gtk4::gdk::Display::default()?;
    if let Some(w) = window
        && let Some(surface) = w.surface()
        && let Some(m) = display.monitor_at_surface(&surface)
    {
        return Some(m);
    }
    let monitors = display.monitors();
    if monitors.n_items() > 0 {
        monitors
            .item(0)
            .and_then(|obj| obj.downcast::<gtk4::gdk::Monitor>().ok())
    } else {
        None
    }
}

fn get_desktop_bounds() -> (f64, f64, f64, f64) {
    let Some(display) = gtk4::gdk::Display::default() else {
        return (0.0, 0.0, 1920.0, 1080.0);
    };
    let monitors = display.monitors();
    let count = monitors.n_items();
    if count == 0 {
        return (0.0, 0.0, 1920.0, 1080.0);
    }

    let mut min_x = 0i32;
    let mut min_y = 0i32;
    let mut max_x = 0i32;
    let mut max_y = 0i32;

    for i in 0..count {
        if let Some(m) = monitors
            .item(i)
            .and_then(|obj| obj.downcast::<gtk4::gdk::Monitor>().ok())
        {
            let g = m.geometry();
            if i == 0 {
                min_x = g.x();
                min_y = g.y();
                max_x = g.x() + g.width();
                max_y = g.y() + g.height();
            } else {
                min_x = min_x.min(g.x());
                min_y = min_y.min(g.y());
                max_x = max_x.max(g.x() + g.width());
                max_y = max_y.max(g.y() + g.height());
            }
        }
    }

    (
        f64::from(min_x),
        f64::from(min_y),
        f64::from((max_x - min_x).max(1)),
        f64::from((max_y - min_y).max(1)),
    )
}

fn map_window_to_screen(
    window: &gtk4::ApplicationWindow,
    mode: OverlayMode,
    win_norm_x: f64,
    win_norm_y: f64,
) -> (f64, f64) {
    let Some(monitor) = get_active_monitor(Some(window)) else {
        return (win_norm_x, win_norm_y);
    };

    let geom = monitor.geometry();
    let raw_w = window.width();
    let raw_h = window.height();

    let (win_w, offset_x) = if raw_w > 0 {
        (
            f64::from(raw_w),
            f64::from((geom.width() - raw_w).max(0)),
        )
    } else {
        (f64::from(geom.width()), 0.0)
    };

    let (win_h, offset_y) = if raw_h > 0 {
        (
            f64::from(raw_h),
            f64::from((geom.height() - raw_h).max(0)),
        )
    } else {
        // When unmapped or hidden, GTK 4 reports 0 for window dimensions.
        // Fall back to standard workarea (monitor height minus top panel e.g. 29px)
        (f64::from(geom.height() - 29), 29.0)
    };

    let pixel_x = win_norm_x * win_w;
    let pixel_y = win_norm_y * win_h;

    let screen_x = f64::from(geom.x()) + offset_x + pixel_x;
    let screen_y = if mode == OverlayMode::TopBar {
        f64::from(geom.y()) + (offset_y / 2.0).min(16.0)
    } else {
        f64::from(geom.y()) + offset_y + pixel_y
    };

    let (min_x, min_y, desk_w, desk_h) = get_desktop_bounds();
    (
        ((screen_x - min_x) / desk_w).clamp(0.0, 1.0),
        ((screen_y - min_y) / desk_h).clamp(0.0, 1.0),
    )
}

fn emit_cursor_move(
    input_device: &Rc<RefCell<Option<InputDevice>>>,
    window: &gtk4::ApplicationWindow,
    mode: OverlayMode,
    norm_x: f64,
    norm_y: f64,
) {
    let (screen_x, screen_y) = map_window_to_screen(window, mode, norm_x, norm_y);
    let evt = OverlayEvent::Move {
        x: screen_x,
        y: screen_y,
    };
    if crate::session::send_event(&evt).unwrap_or(false) {
        return;
    }
    if let Some(ref mut dev) = *input_device.borrow_mut() {
        let _ = dev.move_to_normalized(screen_x, screen_y);
    }
}

fn nudge_reticle(
    s: &mut State,
    input_device: &Rc<RefCell<Option<InputDevice>>>,
    window: &gtk4::ApplicationWindow,
    dx: f64,
    dy: f64,
) {
    let (cur_x, cur_y) = s.point.unwrap_or((
        s.rect.x + s.rect.width / 2.0,
        s.rect.y + s.rect.height / 2.0,
    ));
    let nx = (cur_x + dx).clamp(0.0, 1.0);
    let ny = (cur_y + dy).clamp(0.0, 1.0);
    s.point = Some((nx, ny));

    emit_cursor_move(input_device, window, s.mode, nx, ny);
}

fn emit_selection(
    input_device: &Rc<RefCell<Option<InputDevice>>>,
    window: &gtk4::ApplicationWindow,
    app: &gtk4::Application,
    is_resident: bool,
    mode: OverlayMode,
    path: &[char],
    action: &str,
    point: Option<(f64, f64)>,
) {
    let (screen_x, screen_y) = point.map_or_else(
        || map_window_to_screen(window, mode, 0.5, 0.5),
        |(nx, ny)| map_window_to_screen(window, mode, nx, ny),
    );
    dismiss_overlay(window, app, is_resident);

    let evt = OverlayEvent::Selected {
        strokes: path.iter().collect(),
        action: action.to_string(),
        normalized: crate::NormalizedPoint {
            x: screen_x,
            y: screen_y,
        },
    };
    if crate::session::send_event(&evt).unwrap_or(false) {
        return;
    }
    if let Some(ref mut dev) = *input_device.borrow_mut() {
        std::thread::sleep(Duration::from_millis(120));
        let _ = dev.move_to_normalized(screen_x, screen_y);
        std::thread::sleep(Duration::from_millis(50));
        let _ = crate::session::execute_action(dev, action);
        std::thread::sleep(Duration::from_millis(200));
    }
}

fn emit_click(
    input_device: &Rc<RefCell<Option<InputDevice>>>,
    window: &gtk4::ApplicationWindow,
    mode: OverlayMode,
    button: MouseButton,
    point: Option<(f64, f64)>,
) {
    let screen_coords = point.map(|(nx, ny)| map_window_to_screen(window, mode, nx, ny));
    let evt = OverlayEvent::Click {
        button: match button {
            MouseButton::Left => "left".to_string(),
            MouseButton::Right => "right".to_string(),
            MouseButton::Middle => "middle".to_string(),
        },
        normalized: screen_coords.map(|(x, y)| crate::NormalizedPoint { x, y }),
    };
    if crate::session::send_event(&evt).unwrap_or(false) {
        return;
    }
    if let Some(ref mut dev) = *input_device.borrow_mut() {
        std::thread::sleep(Duration::from_millis(70));
        if let Some((sx, sy)) = screen_coords {
            let _ = dev.move_to_normalized(sx, sy);
            std::thread::sleep(Duration::from_millis(40));
        }
        let _ = dev.click(button);
    }
}

fn emit_press(input_device: &Rc<RefCell<Option<InputDevice>>>, button: MouseButton) {
    let btn_str = match button {
        MouseButton::Left => "left".to_string(),
        MouseButton::Right => "right".to_string(),
        MouseButton::Middle => "middle".to_string(),
    };
    let evt = OverlayEvent::Press { button: btn_str };
    if crate::session::send_event(&evt).unwrap_or(false) {
        return;
    }
    if let Some(ref mut dev) = *input_device.borrow_mut() {
        let _ = dev.press(button);
    }
}

fn emit_release(input_device: &Rc<RefCell<Option<InputDevice>>>, button: MouseButton) {
    let btn_str = match button {
        MouseButton::Left => "left".to_string(),
        MouseButton::Right => "right".to_string(),
        MouseButton::Middle => "middle".to_string(),
    };
    let evt = OverlayEvent::Release { button: btn_str };
    if crate::session::send_event(&evt).unwrap_or(false) {
        return;
    }
    if let Some(ref mut dev) = *input_device.borrow_mut() {
        let _ = dev.release(button);
    }
}

fn emit_scroll(input_device: &Rc<RefCell<Option<InputDevice>>>, dy: i32, dx: i32) {
    let evt = OverlayEvent::Scroll { dx, dy };
    if crate::session::send_event(&evt).unwrap_or(false) {
        return;
    }
    if let Some(ref mut dev) = *input_device.borrow_mut() {
        let _ = dev.scroll(dy, dx);
    }
}

fn dismiss_overlay(window: &gtk4::ApplicationWindow, app: &gtk4::Application, is_resident: bool) {
    if is_resident {
        window.set_visible(false);
    } else {
        window.close();
        app.quit();
    }
}
