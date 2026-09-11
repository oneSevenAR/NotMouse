imports.gi.versions.Gdk = '4.0';
imports.gi.versions.Gtk = '4.0';
imports.gi.versions.Pango = '1.0';
imports.gi.versions.PangoCairo = '1.0';

const { Gdk, Gio, GLib, Gtk, Pango } = imports.gi;

function formatTime() {
    const now = GLib.DateTime.new_now_local();
    return now.format('%H:%M:%S.%f').slice(0, 12);
}

function runTestBench() {
    const application = new Gtk.Application({
        application_id: 'io.github.onesevenar.notmouse.testbench',
        flags: Gio.ApplicationFlags.NON_UNIQUE,
    });

    application.connect('activate', () => {
        const window = new Gtk.ApplicationWindow({
            application,
            title: '!mouse Test Bench — Mouse Event Playground',
            default_width: 860,
            default_height: 640,
        });
        window.maximize();

        // CSS Styling
        const provider = new Gtk.CssProvider();
        provider.load_from_data(`
            window.testbench {
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
            .esc-hint {
                font-size: 12px;
                color: #8b949e;
            }
            .target-card {
                background-color: #161b22;
                border: 2px solid #30363d;
                border-radius: 8px;
                padding: 16px;
                min-height: 80px;
            }
            .target-card:hover {
                border-color: #58a6ff;
            }
            .target-single {
                border-color: #238636;
            }
            .target-double {
                border-color: #a371f7;
            }
            .target-right {
                border-color: #d29922;
            }
            .target-middle {
                border-color: #388bfd;
            }
            .drag-area {
                background-color: #161b22;
                border: 2px dashed #8b949e;
                border-radius: 8px;
                padding: 20px;
                min-height: 120px;
            }
            .drag-token {
                background-color: #f0883e;
                color: #0d1117;
                font-weight: bold;
                border-radius: 6px;
                padding: 10px 16px;
            }
            .scroll-box {
                background-color: #161b22;
                border: 2px solid #30363d;
                border-radius: 8px;
                min-height: 140px;
            }
            .log-view {
                background-color: #090d13;
                border: 1px solid #30363d;
                border-radius: 6px;
                padding: 10px;
                color: #7ee787;
                font-family: monospace;
                font-size: 12px;
            }
            .counter-badge {
                background-color: #21262d;
                border-radius: 12px;
                padding: 4px 10px;
                font-size: 12px;
                font-weight: bold;
                color: #c9d1d9;
            }
            .summon-btn {
                background-color: #238636;
                color: #ffffff;
                font-weight: bold;
                font-size: 12px;
                border-radius: 6px;
                padding: 6px 14px;
            }
            .summon-btn:hover {
                background-color: #2ea043;
            }
        `, -1);
        Gtk.StyleContext.add_provider_for_display(
            Gdk.Display.get_default(),
            provider,
            Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
        window.add_css_class('testbench');

        function summonOverlay() {
            appendLog('🚀 Summoning !mouse overlay...');
            const bin = GLib.getenv('NOTMOUSE_BIN');
            let argv;
            if (bin && bin.length > 0) {
                argv = [bin, 'overlay'];
            } else {
                argv = ['cargo', 'run', '-p', 'notmouse', '--', 'overlay'];
            }

            try {
                const proc = new Gio.Subprocess({
                    argv,
                    flags: Gio.SubprocessFlags.NONE,
                });
                proc.init(null);
            } catch (e) {
                appendLog(`[ERROR] Failed to summon overlay: ${e.message}`);
            }
        }

        // Main Layout Container
        const rootBox = new Gtk.Box({
            orientation: Gtk.Orientation.VERTICAL,
            spacing: 0,
        });
        window.set_child(rootBox);

        // Header
        const headerBox = new Gtk.Box({
            orientation: Gtk.Orientation.HORIZONTAL,
            spacing: 12,
        });
        headerBox.add_css_class('header-bar');
        rootBox.append(headerBox);

        const titleLabel = new Gtk.Label({
            label: '!mouse Interactive Test Bench',
            halign: Gtk.Align.START,
            hexpand: true,
        });
        titleLabel.add_css_class('title-label');
        headerBox.append(titleLabel);

        const summonBtn = new Gtk.Button({
            label: '🎯 Summon !mouse (Tab)',
        });
        summonBtn.add_css_class('summon-btn');
        summonBtn.connect('clicked', () => {
            summonOverlay();
        });
        headerBox.append(summonBtn);

        const escHintLabel = new Gtk.Label({
            label: 'Press Esc to exit',
            halign: Gtk.Align.END,
        });
        escHintLabel.add_css_class('esc-hint');
        headerBox.append(escHintLabel);

        // Content Horizontal Split
        const contentBox = new Gtk.Box({
            orientation: Gtk.Orientation.HORIZONTAL,
            spacing: 16,
            margin_start: 16,
            margin_end: 16,
            margin_top: 14,
            margin_bottom: 14,
            hexpand: true,
            vexpand: true,
        });
        rootBox.append(contentBox);

        // Left Column: Interactive Targets
        const targetsColumn = new Gtk.Box({
            orientation: Gtk.Orientation.VERTICAL,
            spacing: 12,
            hexpand: true,
            vexpand: true,
        });
        contentBox.append(targetsColumn);

        // Right Column: Log & Counters
        const logColumn = new Gtk.Box({
            orientation: Gtk.Orientation.VERTICAL,
            spacing: 10,
            width_request: 360,
            vexpand: true,
        });
        contentBox.append(logColumn);

        // Log HUD text area
        const logLabel = new Gtk.Label({
            label: 'Event Log:',
            halign: Gtk.Align.START,
        });
        logLabel.add_css_class('esc-hint');
        logColumn.append(logLabel);

        const logScrolled = new Gtk.ScrolledWindow({
            hexpand: true,
            vexpand: true,
        });
        logColumn.append(logScrolled);

        const logBuffer = new Gtk.TextBuffer();
        logBuffer.set_text(
            "--- !mouse Event Listener Ready ---\n[INFO] Aim and click anywhere.\n[INFO] Window stays open until Esc is pressed.\n\n",
            -1,
        );
        const logTextView = new Gtk.TextView({
            buffer: logBuffer,
            editable: false,
            cursor_visible: false,
            wrap_mode: Gtk.WrapMode.WORD_CHAR,
        });
        logTextView.add_css_class('log-view');
        logScrolled.set_child(logTextView);

        // Status / Stats Counters
        const stats = {
            totalClicks: 0,
            singleClicks: 0,
            doubleClicks: 0,
            rightClicks: 0,
            middleClicks: 0,
            drags: 0,
            scrolls: 0,
        };

        const counterLabel = new Gtk.Label({
            label: 'Clicks: 0 | Right: 0 | Double: 0 | Middle: 0 | Scrolls: 0',
            halign: Gtk.Align.START,
        });
        counterLabel.add_css_class('counter-badge');
        logColumn.append(counterLabel);

        function updateCounterDisplay() {
            counterLabel.set_text(
                `Clicks: ${stats.singleClicks} | Right: ${stats.rightClicks} | Double: ${stats.doubleClicks} | Middle: ${stats.middleClicks} | Scrolls: ${stats.scrolls}`
            );
        }

        function appendLog(message) {
            const time = formatTime();
            const fullLine = `[${time}] ${message}\n`;
            print(fullLine.trim());
            try {
                const file = Gio.File.new_for_path('/tmp/testbench_events.log');
                const outStream = file.append_to(Gio.FileCreateFlags.NONE, null);
                outStream.write_all(fullLine, null);
                outStream.close(null);
            } catch (_) {}
            const endIter = logBuffer.get_end_iter();
            logBuffer.insert(endIter, fullLine, -1);

            // Auto-scroll to bottom
            GLib.idle_add(GLib.PRIORITY_DEFAULT_IDLE, () => {
                const end = logBuffer.get_end_iter();
                const mark = logBuffer.create_mark(null, end, false);
                logTextView.scroll_to_mark(mark, 0, false, 0, 0);
                return GLib.SOURCE_REMOVE;
            });
        }

        // --- Target 1: Click Targets Row ---
        const buttonsRow = new Gtk.Box({
            orientation: Gtk.Orientation.HORIZONTAL,
            spacing: 10,
            homogeneous: true,
        });
        targetsColumn.append(buttonsRow);

        // Single Click Button
        const singleBtn = new Gtk.Button({
            label: '🎯 Single Click',
        });
        singleBtn.add_css_class('target-card');
        singleBtn.add_css_class('target-single');
        singleBtn.connect('clicked', () => {
            stats.singleClicks += 1;
            stats.totalClicks += 1;
            appendLog(`🎯 Single Click registered on target button!`);
            updateCounterDisplay();
        });
        buttonsRow.append(singleBtn);

        // Double Click Button
        const doubleBtn = new Gtk.Button({
            label: '⚡ Double Click',
        });
        doubleBtn.add_css_class('target-card');
        doubleBtn.add_css_class('target-double');
        const doubleGesture = new Gtk.GestureClick();
        doubleGesture.connect('pressed', (_g, nPress, _x, _y) => {
            if (nPress === 2) {
                stats.doubleClicks += 1;
                appendLog(`⚡ Double Click confirmed on target button! (n_press=2)`);
                updateCounterDisplay();
            }
        });
        doubleBtn.add_controller(doubleGesture);
        buttonsRow.append(doubleBtn);

        // Right & Middle Click Targets Row
        const buttonsRow2 = new Gtk.Box({
            orientation: Gtk.Orientation.HORIZONTAL,
            spacing: 10,
            homogeneous: true,
        });
        targetsColumn.append(buttonsRow2);

        // Right Click Target
        const rightCard = new Gtk.Label({
            label: '🖱️ Right Click Me',
        });
        rightCard.add_css_class('target-card');
        rightCard.add_css_class('target-right');
        const rightGesture = new Gtk.GestureClick();
        rightGesture.set_button(Gdk.BUTTON_SECONDARY);
        rightGesture.connect('pressed', (_g, _nPress, x, y) => {
            stats.rightClicks += 1;
            appendLog(`🖱️ Right Click registered on target at (${Math.round(x)}, ${Math.round(y)})`);
            updateCounterDisplay();
        });
        rightCard.add_controller(rightGesture);
        buttonsRow2.append(rightCard);

        // Middle Click Target
        const middleCard = new Gtk.Label({
            label: '🔘 Middle Click Me',
        });
        middleCard.add_css_class('target-card');
        middleCard.add_css_class('target-middle');
        const middleGesture = new Gtk.GestureClick();
        middleGesture.set_button(Gdk.BUTTON_MIDDLE);
        middleGesture.connect('pressed', (_g, _nPress, x, y) => {
            stats.middleClicks += 1;
            appendLog(`🔘 Middle Click registered on target at (${Math.round(x)}, ${Math.round(y)})`);
            updateCounterDisplay();
        });
        middleCard.add_controller(middleGesture);
        buttonsRow2.append(middleCard);

        // --- Target 2: Drag & Drop Sandbox ---
        const dragArea = new Gtk.Box({
            orientation: Gtk.Orientation.VERTICAL,
            spacing: 8,
        });
        dragArea.add_css_class('drag-area');
        targetsColumn.append(dragArea);

        const dragTitle = new Gtk.Label({
            label: '🧲 Drag & Drop Zone (Drag token below):',
            halign: Gtk.Align.START,
        });
        dragTitle.add_css_class('esc-hint');
        dragArea.append(dragTitle);

        const dragTrack = new Gtk.Box({
            orientation: Gtk.Orientation.HORIZONTAL,
            spacing: 12,
            hexpand: true,
        });
        dragArea.append(dragTrack);

        const dragToken = new Gtk.Label({
            label: '⠿ DRAG TOKEN',
            halign: Gtk.Align.START,
        });
        dragToken.add_css_class('drag-token');
        dragTrack.append(dragToken);

        const dragStatusLabel = new Gtk.Label({
            label: 'Drop Target Area',
            hexpand: true,
            halign: Gtk.Align.CENTER,
        });
        dragStatusLabel.add_css_class('esc-hint');
        dragTrack.append(dragStatusLabel);

        const dragGesture = new Gtk.GestureDrag();
        dragGesture.connect('drag-begin', (_g, startX, startY) => {
            appendLog(`🧲 Drag STARTED at (${Math.round(startX)}, ${Math.round(startY)})`);
            dragStatusLabel.set_text('Dragging in progress...');
        });
        dragGesture.connect('drag-update', (_g, offsetX, offsetY) => {
            // Feedback during drag
        });
        dragGesture.connect('drag-end', (_g, offsetX, offsetY) => {
            stats.drags += 1;
            appendLog(`🧲 Drag ENDED offset: (${Math.round(offsetX)}, ${Math.round(offsetY)})`);
            dragStatusLabel.set_text(`✓ Last drag: dx=${Math.round(offsetX)}px, dy=${Math.round(offsetY)}px`);
            updateCounterDisplay();
        });
        dragArea.add_controller(dragGesture);

        // --- Target 3: Scroll Testing Area ---
        const scrollContainer = new Gtk.Box({
            orientation: Gtk.Orientation.VERTICAL,
            spacing: 4,
        });
        targetsColumn.append(scrollContainer);

        const scrollTitle = new Gtk.Label({
            label: '📜 Scroll Area (Wheel scroll up/down here):',
            halign: Gtk.Align.START,
        });
        scrollTitle.add_css_class('esc-hint');
        scrollContainer.append(scrollTitle);

        const scrollWindow = new Gtk.ScrolledWindow({
            min_content_height: 140,
            hexpand: true,
            vexpand: true,
        });
        scrollWindow.add_css_class('scroll-box');
        scrollContainer.append(scrollWindow);

        const scrollItemsBox = new Gtk.Box({
            orientation: Gtk.Orientation.VERTICAL,
            spacing: 4,
            margin_start: 12,
            margin_end: 12,
            margin_top: 8,
            margin_bottom: 8,
        });
        scrollWindow.set_child(scrollItemsBox);

        for (let i = 1; i <= 30; i += 1) {
            const itemLabel = new Gtk.Label({
                label: `Item #${i} — Scroll item for testing wheel navigation`,
                halign: Gtk.Align.START,
            });
            scrollItemsBox.append(itemLabel);
        }

        const scrollController = new Gtk.EventControllerScroll({
            flags: Gtk.EventControllerScrollFlags.VERTICAL | Gtk.EventControllerScrollFlags.HORIZONTAL,
        });
        scrollController.connect('scroll', (_c, dx, dy) => {
            stats.scrolls += 1;
            const direction = dy > 0 ? 'DOWN' : (dy < 0 ? 'UP' : (dx > 0 ? 'RIGHT' : 'LEFT'));
            appendLog(`📜 Scroll detected: ${direction} (dx=${dx.toFixed(1)}, dy=${dy.toFixed(1)})`);
            updateCounterDisplay();
            return false;
        });
        scrollWindow.add_controller(scrollController);

        // --- Window-wide Click Catcher (Tracks any click anywhere in the window) ---
        const globalClick = new Gtk.GestureClick();
        globalClick.set_button(0); // Listen to ALL buttons (1=Left, 2=Middle, 3=Right)
        globalClick.set_propagation_phase(Gtk.PropagationPhase.CAPTURE);
        globalClick.connect('pressed', (_g, nPress, x, y) => {
            const button = globalClick.get_current_button();
            let buttonName = 'BUTTON_' + button;
            if (button === Gdk.BUTTON_PRIMARY) buttonName = 'LEFT';
            else if (button === Gdk.BUTTON_SECONDARY) buttonName = 'RIGHT';
            else if (button === Gdk.BUTTON_MIDDLE) buttonName = 'MIDDLE';

            const pressDesc = nPress === 1 ? 'click' : (nPress === 2 ? 'DOUBLE click' : `${nPress}x click`);
            appendLog(`[WINDOW] ${buttonName} ${pressDesc} at (${Math.round(x)}, ${Math.round(y)})`);
        });
        window.add_controller(globalClick);

        // --- Window-wide Scroll Catcher ---
        const globalScroll = new Gtk.EventControllerScroll({
            flags: Gtk.EventControllerScrollFlags.BOTH_AXES,
        });
        globalScroll.set_propagation_phase(Gtk.PropagationPhase.CAPTURE);
        globalScroll.connect('scroll', (_c, dx, dy) => {
            stats.scrolls += 1;
            const direction = dy > 0 ? 'DOWN' : (dy < 0 ? 'UP' : (dx > 0 ? 'RIGHT' : 'LEFT'));
            appendLog(`[WINDOW] 📜 Scroll detected: ${direction} (dx=${dx.toFixed(1)}, dy=${dy.toFixed(1)})`);
            updateCounterDisplay();
            return false;
        });
        window.add_controller(globalScroll);

        // --- Window-wide Key Controller: ONLY exits on Escape, Tab summons overlay ---
        const keyController = new Gtk.EventControllerKey();
        keyController.connect('key-pressed', (_c, keyval) => {
            if (keyval === Gdk.KEY_Escape) {
                appendLog(`[EXIT] Escape pressed, closing test bench...`);
                application.quit();
                return true;
            }
            if (keyval === Gdk.KEY_Tab || keyval === Gdk.KEY_F1) {
                summonOverlay();
                return true;
            }
            // Do NOT quit on any other key!
            return false;
        });
        window.add_controller(keyController);

        if (GLib.getenv('NOTMOUSE_SMOKE_TEST') === '1') {
            GLib.timeout_add(GLib.PRIORITY_DEFAULT, 300, () => {
                print('test bench smoke test passed');
                application.quit();
                return GLib.SOURCE_REMOVE;
            });
        }

        appendLog('[INIT] Test bench started and ready for input events');
        window.present();
    });

    application.run([]);
}

if (ARGV.includes('--self-test')) {
    print('test bench self-test passed');
} else {
    runTestBench();
}
