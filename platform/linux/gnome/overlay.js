imports.gi.versions.Gdk = '4.0';
imports.gi.versions.Gtk = '4.0';
imports.gi.versions.Pango = '1.0';
imports.gi.versions.PangoCairo = '1.0';

const { Gdk, Gio, GLib, Gtk, Pango, PangoCairo } = imports.gi;

const HINTS = ['a', 's', 'd', 'f', 'j', 'k', 'l', 'g', 'h'];
const GRID_SIZE = 3;
const MAX_DEPTH = 6;

function childRect(parent, index) {
    const width = parent.width / GRID_SIZE;
    const height = parent.height / GRID_SIZE;
    const row = Math.floor(index / GRID_SIZE);
    const column = index % GRID_SIZE;

    return {
        x: parent.x + column * width,
        y: parent.y + row * height,
        width,
        height,
    };
}

function drawLabel(area, context, label, x, y) {
    const layout = area.create_pango_layout(label.toUpperCase());
    const font = Pango.FontDescription.from_string('Sans Bold 22');
    layout.set_font_description(font);
    const [textWidth, textHeight] = layout.get_pixel_size();
    const paddingX = 14;
    const paddingY = 8;

    context.setSourceRGBA(0.97, 0.72, 0.18, 0.96);
    context.rectangle(
        x - textWidth / 2 - paddingX,
        y - textHeight / 2 - paddingY,
        textWidth + paddingX * 2,
        textHeight + paddingY * 2,
    );
    context.fill();

    context.setSourceRGBA(0.05, 0.06, 0.09, 1);
    context.moveTo(x - textWidth / 2, y - textHeight / 2);
    PangoCairo.show_layout(context, layout);
}

function drawOverlay(area, context, width, height, state) {
    context.setSourceRGBA(0.02, 0.03, 0.06, 0.50);
    context.rectangle(0, 0, width, height);
    context.fill();

    const selected = {
        x: state.rect.x * width,
        y: state.rect.y * height,
        width: state.rect.width * width,
        height: state.rect.height * height,
    };

    context.setSourceRGBA(0.10, 0.13, 0.20, 0.24);
    context.rectangle(selected.x, selected.y, selected.width, selected.height);
    context.fill();

    for (let index = 0; index < HINTS.length; index += 1) {
        const zone = childRect(selected, index);
        const centerX = zone.x + zone.width / 2;
        const centerY = zone.y + zone.height / 2;

        context.setSourceRGBA(0.72, 0.77, 0.88, 0.55);
        context.setLineWidth(index === 4 ? 2.5 : 1.4);
        context.rectangle(zone.x, zone.y, zone.width, zone.height);
        context.stroke();
        drawLabel(area, context, HINTS[index], centerX, centerY);
    }

    const breadcrumb = state.path.length === 0
        ? 'Choose a zone'
        : `Zone ${state.path.join(' ')}  •  Enter to select  •  Backspace to go back`;
    const layout = area.create_pango_layout(`${breadcrumb}  •  Esc to close`);
    layout.set_font_description(Pango.FontDescription.from_string('Sans 14'));
    const [textWidth] = layout.get_pixel_size();
    context.setSourceRGBA(0.96, 0.97, 1, 0.96);
    context.moveTo((width - textWidth) / 2, 20);
    PangoCairo.show_layout(context, layout);
}

function keyCharacter(keyval) {
    const codePoint = Gdk.keyval_to_unicode(keyval);
    return codePoint === 0 ? '' : String.fromCodePoint(codePoint).toLowerCase();
}

function runOverlay() {
    const application = new Gtk.Application({
        application_id: 'io.github.onesevenar.notmouse.overlay',
        flags: Gio.ApplicationFlags.NON_UNIQUE,
    });

    application.connect('activate', () => {
        const state = {
            path: [],
            rect: { x: 0, y: 0, width: 1, height: 1 },
            history: [],
        };
        const window = new Gtk.ApplicationWindow({
            application,
            title: '!mouse',
            decorated: false,
        });
        window.add_css_class('notmouse-overlay');

        const provider = new Gtk.CssProvider();
        provider.load_from_data(
            'window.notmouse-overlay { background-color: transparent; }',
            -1,
        );
        Gtk.StyleContext.add_provider_for_display(
            Gdk.Display.get_default(),
            provider,
            Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        const drawingArea = new Gtk.DrawingArea({
            hexpand: true,
            vexpand: true,
            focusable: true,
        });
        drawingArea.set_draw_func((area, context, width, height) => {
            drawOverlay(area, context, width, height, state);
        });

        const keyboard = new Gtk.EventControllerKey();
        keyboard.connect('key-pressed', (_controller, keyval) => {
            if (keyval === Gdk.KEY_Escape) {
                application.quit();
                return true;
            }

            if (keyval === Gdk.KEY_BackSpace) {
                const previous = state.history.pop();
                if (previous !== undefined) {
                    state.rect = previous;
                    state.path.pop();
                    drawingArea.queue_draw();
                }
                return true;
            }

            if (keyval === Gdk.KEY_Return || keyval === Gdk.KEY_KP_Enter) {
                const centerX = state.rect.x + state.rect.width / 2;
                const centerY = state.rect.y + state.rect.height / 2;
                print(JSON.stringify({
                    event: 'selected',
                    path: state.path.join(''),
                    normalized: { x: centerX, y: centerY },
                }));
                application.quit();
                return true;
            }

            const index = HINTS.indexOf(keyCharacter(keyval));
            if (index === -1 || state.path.length >= MAX_DEPTH) {
                return true;
            }

            state.history.push({ ...state.rect });
            state.rect = childRect(state.rect, index);
            state.path.push(HINTS[index]);
            drawingArea.queue_draw();
            return true;
        });
        window.add_controller(keyboard);
        window.set_child(drawingArea);
        window.fullscreen();
        window.present();
        drawingArea.grab_focus();

        if (GLib.getenv('NOTMOUSE_SMOKE_TEST') === '1') {
            GLib.timeout_add(GLib.PRIORITY_DEFAULT, 300, () => {
                print('overlay smoke test passed');
                application.quit();
                return GLib.SOURCE_REMOVE;
            });
        }
    });

    application.run([]);
}

function selfTest() {
    const center = childRect({ x: 0, y: 0, width: 1, height: 1 }, 4);
    if (
        Math.abs(center.x - 1 / 3) > Number.EPSILON
        || Math.abs(center.y - 1 / 3) > Number.EPSILON
        || Math.abs(center.width - 1 / 3) > Number.EPSILON
        || Math.abs(center.height - 1 / 3) > Number.EPSILON
    ) {
        throw new Error('zone geometry self-test failed');
    }
    print('overlay self-test passed');
}

if (ARGV.includes('--self-test')) {
    selfTest();
} else {
    runOverlay();
}
