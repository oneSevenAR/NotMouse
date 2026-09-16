use gtk4::gdk::Key;
use gtk4::glib;
use gtk4::prelude::*;

pub fn run_test_bench() -> Result<(), String> {
    let app = gtk4::Application::builder()
        .application_id("io.github.onesevenar.notmouse.testbench")
        .flags(gtk4::gio::ApplicationFlags::NON_UNIQUE)
        .build();

    app.connect_activate(|application| {
        let window = gtk4::ApplicationWindow::builder()
            .application(application)
            .title("!mouse Test Bench — Event Playground")
            .default_width(960)
            .default_height(720)
            .build();

        window.add_css_class("testbench");

        let provider = gtk4::CssProvider::new();
        provider.load_from_string(
            "window.testbench {
                background-color: #0d1117;
                color: #e6edf3;
                font-family: monospace, sans-serif;
            }
            .header-bar {
                background-color: #161b22;
                border-bottom: 2px solid #30363d;
                padding: 12px 18px;
            }
            .title-label {
                font-size: 16px;
                font-weight: bold;
                color: #f0883e;
            }
            .target-card {
                background-color: #161b22;
                border: 2px solid #30363d;
                border-radius: 8px;
                padding: 16px;
                min-height: 80px;
            }
            .btn-target {
                padding: 12px 24px;
                font-size: 14px;
                font-weight: bold;
                border-radius: 6px;
            }"
        );

        if let Some(display) = gtk4::gdk::Display::default() {
            gtk4::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk4::STYLE_PROVIDER_PRIORITY_USER,
            );
        }

        let main_box = gtk4::Box::new(gtk4::Orientation::Vertical, 16);
        main_box.set_margin_top(20);
        main_box.set_margin_bottom(20);
        main_box.set_margin_start(24);
        main_box.set_margin_end(24);

        let header = gtk4::Label::builder()
            .label("!mouse Interactive Event Verification Bench")
            .css_classes(["title-label"])
            .build();
        main_box.append(&header);

        let sub = gtk4::Label::builder()
            .label("Press [Super+M] (or notmouse overlay) to aim, hover, snap, free-glide, drag, and click. [Esc] exits this bench.")
            .css_classes(["esc-hint"])
            .build();
        main_box.append(&sub);

        // Target buttons row
        let btn_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 16);

        let log_view = gtk4::TextView::builder()
            .editable(false)
            .monospace(true)
            .build();
        let log_buffer = log_view.buffer();
        log_buffer.set_text("System ready. Hover over controls or click targets below.\n");

        let log_scroll = gtk4::ScrolledWindow::builder()
            .hexpand(true)
            .vexpand(true)
            .child(&log_view)
            .min_content_height(250)
            .build();

        // Left Click Target
        let btn_left = gtk4::Button::with_label("Single Left Click");
        btn_left.add_css_class("btn-target");
        {
            let buffer = log_buffer.clone();
            btn_left.connect_clicked(move |_| {
                let mut iter = buffer.end_iter();
                buffer.insert(&mut iter, "[SUCCESS] Left Click received!\n");
            });
        }
        btn_row.append(&btn_left);

        // Double Click Target
        let btn_double = gtk4::Button::with_label("Double Click Here");
        btn_double.add_css_class("btn-target");
        {
            let gesture = gtk4::GestureClick::new();
            let buffer = log_buffer.clone();
            gesture.connect_pressed(move |g, n_press, _x, _y| {
                if n_press == 2 {
                    let mut iter = buffer.end_iter();
                    buffer.insert(&mut iter, "[SUCCESS] Double Click verified!\n");
                    g.set_state(gtk4::EventSequenceState::Claimed);
                }
            });
            btn_double.add_controller(gesture);
        }
        btn_row.append(&btn_double);

        // Hover Target with tooltip
        let btn_hover = gtk4::Button::with_label("Hover Over Me (Point & Hover 'p')");
        btn_hover.add_css_class("btn-target");
        btn_hover.set_tooltip_text(Some("✨ Hover verified! Autohiding popup triggered!"));
        {
            let buffer = log_buffer.clone();
            let motion = gtk4::EventControllerMotion::new();
            motion.connect_enter(move |_controller, _x, _y| {
                let mut iter = buffer.end_iter();
                buffer.insert(&mut iter, "[HOVER] Cursor entered target (Wake-on-Aim verified)!\n");
            });
            btn_hover.add_controller(motion);
        }
        btn_row.append(&btn_hover);

        main_box.append(&btn_row);

        // Scroll container target
        let scroll_title = gtk4::Label::new(Some("Scroll Test Container ([s] / [w]):"));
        scroll_title.set_halign(gtk4::Align::Start);
        main_box.append(&scroll_title);

        let scroll_content = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
        for i in 1..=40 {
            let item = gtk4::Label::new(Some(&format!("Scroll Row Item #{i} — kinetic scroll test")));
            item.set_halign(gtk4::Align::Start);
            scroll_content.append(&item);
        }
        let test_scroller = gtk4::ScrolledWindow::builder()
            .hexpand(true)
            .child(&scroll_content)
            .min_content_height(140)
            .build();
        main_box.append(&test_scroller);

        // Log section
        let log_title = gtk4::Label::new(Some("Live Event Log:"));
        log_title.set_halign(gtk4::Align::Start);
        main_box.append(&log_title);
        main_box.append(&log_scroll);

        // Key controller for Esc
        let key_ctl = gtk4::EventControllerKey::new();
        {
            let win = window.clone();
            let app = application.clone();
            key_ctl.connect_key_pressed(move |_ctl, keyval, _keycode, _modifier| {
                if keyval == Key::Escape {
                    win.close();
                    app.quit();
                    return glib::Propagation::Stop;
                }
                glib::Propagation::Proceed
            });
        }
        window.add_controller(key_ctl);

        window.set_child(Some(&main_box));
        window.present();
    });

    let empty_args: [&str; 0] = [];
    app.run_with_args(&empty_args);
    Ok(())
}
