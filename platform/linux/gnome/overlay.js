imports.gi.versions.Gdk = '4.0';
imports.gi.versions.Gtk = '4.0';
imports.gi.versions.Pango = '1.0';
imports.gi.versions.PangoCairo = '1.0';

const { Gdk, Gio, GLib, Gtk, Pango, PangoCairo } = imports.gi;
const Cairo = imports.cairo;

const HINTS = ['a', 's', 'd', 'f', 'j', 'k', 'l', 'g', 'h'];
const GRID_SIZE = 3;
const MAX_DEPTH = 2;

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

function drawLabel(area, context, label, x, y, options = {}) {
    const fontSize = options.fontSize || 22;
    const fontDesc = options.font || `Sans Bold ${fontSize}`;
    const layout = area.create_pango_layout(label.toUpperCase());
    const font = Pango.FontDescription.from_string(fontDesc);
    layout.set_font_description(font);
    const [textWidth, textHeight] = layout.get_pixel_size();
    const paddingX = options.paddingX !== undefined ? options.paddingX : 12;
    const paddingY = options.paddingY !== undefined ? options.paddingY : 6;

    const bg = options.bg || [0.97, 0.72, 0.18, 0.96];
    const fg = options.fg || [0.05, 0.06, 0.09, 1.0];

    context.setSourceRGBA(bg[0], bg[1], bg[2], bg[3]);
    context.rectangle(
        x - textWidth / 2 - paddingX,
        y - textHeight / 2 - paddingY,
        textWidth + paddingX * 2,
        textHeight + paddingY * 2,
    );
    context.fill();

    context.setSourceRGBA(fg[0], fg[1], fg[2], fg[3]);
    context.moveTo(x - textWidth / 2, y - textHeight / 2);
    PangoCairo.show_layout(context, layout);
}

function drawReticle(context, x, y) {
    const radius = 18;

    // Outer glow ring
    context.setSourceRGBA(0.97, 0.72, 0.18, 0.35);
    context.setLineWidth(4.0);
    context.arc(x, y, radius + 2, 0, 2 * Math.PI);
    context.stroke();

    // Main ring
    context.setSourceRGBA(0.97, 0.72, 0.18, 0.95);
    context.setLineWidth(2.0);
    context.arc(x, y, radius, 0, 2 * Math.PI);
    context.stroke();

    // Crosshair ticks
    context.setLineWidth(1.8);
    // Left
    context.moveTo(x - radius - 8, y);
    context.lineTo(x - radius + 4, y);
    // Right
    context.moveTo(x + radius - 4, y);
    context.lineTo(x + radius + 8, y);
    // Top
    context.moveTo(x, y - radius - 8);
    context.lineTo(x, y - radius + 4);
    // Bottom
    context.moveTo(x, y + radius - 4);
    context.lineTo(x, y + radius + 8);
    context.stroke();

    // Center dot
    context.setSourceRGBA(0.97, 0.72, 0.18, 1.0);
    context.arc(x, y, 3.0, 0, 2 * Math.PI);
    context.fill();
}

function drawOverlay(area, context, width, height, state) {
    if (state.mode === 'scroll') {
        // Minimal background tint so underlying content is 100% legible
        context.setSourceRGBA(0.01, 0.02, 0.04, 0.04);
        context.rectangle(0, 0, width, height);
        context.fill();

        const reticleX = (state.point ? state.point.x : 0.5) * width;
        const reticleY = (state.point ? state.point.y : 0.5) * height;

        // Animated / directional scroll anchor ring at reticle position
        const radius = 22;
        context.setSourceRGBA(0.18, 0.80, 0.97, 0.35); // cyan glow
        context.setLineWidth(4.0);
        context.arc(reticleX, reticleY, radius + 2, 0, 2 * Math.PI);
        context.stroke();

        context.setSourceRGBA(0.18, 0.80, 0.97, 0.95);
        context.setLineWidth(2.0);
        context.arc(reticleX, reticleY, radius, 0, 2 * Math.PI);
        context.stroke();

        // Center dot
        context.setSourceRGBA(0.18, 0.80, 0.97, 1.0);
        context.arc(reticleX, reticleY, 3.0, 0, 2 * Math.PI);
        context.fill();

        // Direction indicators around anchor
        const isUp = state.lastScrollDir === 'up';
        const isDown = state.lastScrollDir === 'down';

        // Up arrow
        context.setSourceRGBA(0.18, 0.80, 0.97, isUp ? 1.0 : 0.4);
        context.moveTo(reticleX, reticleY - radius - 14);
        context.lineTo(reticleX - 7, reticleY - radius - 3);
        context.lineTo(reticleX + 7, reticleY - radius - 3);
        context.closePath();
        context.fill();

        // Down arrow
        context.setSourceRGBA(0.18, 0.80, 0.97, isDown ? 1.0 : 0.4);
        context.moveTo(reticleX, reticleY + radius + 14);
        context.lineTo(reticleX - 7, reticleY + radius + 3);
        context.lineTo(reticleX + 7, reticleY + radius + 3);
        context.closePath();
        context.fill();

        // Small badge at scroll anchor
        drawLabel(area, context, 'SCROLL', reticleX, reticleY - 36, {
            fontSize: 11,
            paddingX: 8,
            paddingY: 3,
            bg: [0.10, 0.15, 0.25, 0.95],
            fg: [0.30, 0.85, 1.0, 1.0],
        });

        // Floating bottom HUD bar
        const barWidth = Math.min(width - 40, 780);
        const barHeight = 44;
        const barX = (width - barWidth) / 2;
        const barY = height - barHeight - 24;

        context.setSourceRGBA(0.06, 0.08, 0.12, 0.92);
        context.rectangle(barX, barY, barWidth, barHeight);
        context.fill();

        context.setSourceRGBA(0.18, 0.80, 0.97, 0.80);
        context.setLineWidth(1.8);
        context.rectangle(barX, barY, barWidth, barHeight);
        context.stroke();

        const hudText = '📜 SCROLL MODE  •  [j / s / ↓] Down  •  [k / w / ↑] Up  •  [d / u] Page  •  [Shift] Faster  •  [Tab] Grid  •  [Enter] Click  •  [Esc] Exit';
        const hudLayout = area.create_pango_layout(hudText);
        hudLayout.set_font_description(Pango.FontDescription.from_string('Sans Bold 12'));
        const [textW, textH] = hudLayout.get_pixel_size();
        context.setSourceRGBA(0.95, 0.97, 1.0, 1.0);
        context.moveTo(barX + (barWidth - textW) / 2, barY + (barHeight - textH) / 2);
        PangoCairo.show_layout(context, hudLayout);
        return;
    }

    const isMacroView = state.path.length === 0;
    const isLocked = state.path.length >= MAX_DEPTH;

    // Subtly dim the screen background so elements underneath remain clearly visible
    context.setSourceRGBA(0.02, 0.03, 0.06, isMacroView ? 0.12 : 0.24);
    context.rectangle(0, 0, width, height);
    context.fill();

    // If dragging active, draw source anchor
    if (state.dragging && state.dragStartPoint) {
        const startX = state.dragStartPoint.x * width;
        const startY = state.dragStartPoint.y * height;
        context.setSourceRGBA(1.0, 0.25, 0.25, 0.90);
        context.arc(startX, startY, 12, 0, 2 * Math.PI);
        context.fill();
        drawLabel(area, context, 'DRAG SOURCE', startX, startY - 24, {
            fontSize: 10,
            paddingX: 6,
            paddingY: 2,
            bg: [0.8, 0.1, 0.1, 0.95],
            fg: [1, 1, 1, 1],
        });
    }

    const selected = {
        x: state.rect.x * width,
        y: state.rect.y * height,
        width: state.rect.width * width,
        height: state.rect.height * height,
    };

    if (isMacroView) {
        // Macro view: draw 9 macro cells, each with center badge and faint sub-cell hints
        for (let m = 0; m < HINTS.length; m += 1) {
            const macroZone = childRect(selected, m);
            const macroCenterX = macroZone.x + macroZone.width / 2;
            const macroCenterY = macroZone.y + macroZone.height / 2;

            // Faint sub-grid inside each macro cell
            for (let s = 0; s < HINTS.length; s += 1) {
                const subZone = childRect(macroZone, s);
                context.setSourceRGBA(0.40, 0.45, 0.55, 0.18);
                context.setLineWidth(1.0);
                context.rectangle(subZone.x, subZone.y, subZone.width, subZone.height);
                context.stroke();

                // Small 2-letter hint in each subcell
                const subCenterX = subZone.x + subZone.width / 2;
                const subCenterY = subZone.y + subZone.height / 2;
                drawLabel(area, context, HINTS[m] + HINTS[s], subCenterX, subCenterY, {
                    fontSize: 10,
                    paddingX: 5,
                    paddingY: 2,
                    bg: [0.12, 0.15, 0.22, 0.70],
                    fg: [0.80, 0.85, 0.95, 0.85],
                });
            }

            // Outer border of macro cell
            context.setSourceRGBA(0.85, 0.88, 0.95, 0.50);
            context.setLineWidth(m === 4 ? 2.5 : 1.6);
            context.rectangle(macroZone.x, macroZone.y, macroZone.width, macroZone.height);
            context.stroke();

            // Prominent center macro badge
            drawLabel(area, context, HINTS[m], macroCenterX, macroCenterY, {
                fontSize: 26,
                paddingX: 18,
                paddingY: 10,
                bg: [0.97, 0.72, 0.18, 0.88],
                fg: [0.05, 0.06, 0.09, 1.0],
            });
        }
    } else if (!isLocked) {
        // Focused region view: highlight selected macro cell, draw 9 subcells
        context.setSourceRGBA(0.08, 0.12, 0.20, 0.08);
        context.rectangle(selected.x, selected.y, selected.width, selected.height);
        context.fill();

        // Prominent border around focused region
        context.setSourceRGBA(0.97, 0.72, 0.18, 0.85);
        context.setLineWidth(3.0);
        context.rectangle(selected.x, selected.y, selected.width, selected.height);
        context.stroke();

        // Draw the 9 subcells inside the selected region with clear 2-stroke hints
        for (let s = 0; s < HINTS.length; s += 1) {
            const subZone = childRect(selected, s);
            const centerX = subZone.x + subZone.width / 2;
            const centerY = subZone.y + subZone.height / 2;

            context.setSourceRGBA(0.72, 0.77, 0.88, 0.65);
            context.setLineWidth(s === 4 ? 2.5 : 1.4);
            context.rectangle(subZone.x, subZone.y, subZone.width, subZone.height);
            context.stroke();

            const fullHint = state.path[0] + HINTS[s];
            drawLabel(area, context, fullHint, centerX, centerY, {
                fontSize: 20,
                paddingX: 14,
                paddingY: 8,
                bg: [0.97, 0.72, 0.18, 0.90],
                fg: [0.05, 0.06, 0.09, 1.0],
            });
        }
    } else {
        // Locked target view: highlight locked cell, render reticle and nudge instructions
        context.setSourceRGBA(0.12, 0.18, 0.28, 0.10);
        context.rectangle(selected.x, selected.y, selected.width, selected.height);
        context.fill();

        context.setSourceRGBA(0.97, 0.72, 0.18, 0.90);
        context.setLineWidth(2.5);
        context.rectangle(selected.x, selected.y, selected.width, selected.height);
        context.stroke();

        // Target reticle
        const reticleX = (state.point ? state.point.x : state.rect.x + state.rect.width / 2) * width;
        const reticleY = (state.point ? state.point.y : state.rect.y + state.rect.height / 2) * height;
        drawReticle(context, reticleX, reticleY);

        // Label above reticle
        drawLabel(area, context, state.path.join(''), reticleX, reticleY - 32, {
            fontSize: 16,
            paddingX: 10,
            paddingY: 5,
            bg: [0.97, 0.72, 0.18, 0.98],
            fg: [0.05, 0.06, 0.09, 1.0],
        });
    }

    // Header breadcrumb
    let breadcrumb;
    if (state.dragging) {
        if (isMacroView) {
            breadcrumb = '🎯 DRAG ENGAGED  •  Stroke 1: Choose destination region  •  [Esc] Cancel drag';
        } else if (!isLocked) {
            breadcrumb = `🎯 DRAG ENGAGED  •  Region ${state.path[0].toUpperCase()}  •  Stroke 2: Choose drop target  •  [Space] Drop here`;
        } else {
            breadcrumb = `🎯 DROP TARGET LOCKED: ${state.path.join('').toUpperCase()}  •  [v / Space / Enter] DROP  •  [Esc] Cancel`;
        }
    } else if (isMacroView) {
        breadcrumb = '!mouse  •  Stroke 1: Choose region (A S D F J K L G H)';
    } else if (!isLocked) {
        breadcrumb = `Region ${state.path[0].toUpperCase()}  •  Stroke 2: Choose target  •  Enter for region center  •  Backspace to undo`;
    } else {
        breadcrumb = `Target: ${state.path.join('').toUpperCase()}  •  Space/Enter: Click  •  s: Scroll Mode  •  v: Drag Mode  •  c: Click & Stay  •  r/d/m: Other Clicks`;
    }

    const layout = area.create_pango_layout(`${breadcrumb}  •  Esc to cancel`);
    layout.set_font_description(Pango.FontDescription.from_string('Sans 13'));
    const [textWidth] = layout.get_pixel_size();
    context.setSourceRGBA(0.96, 0.97, 1, 0.96);
    context.moveTo((width - textWidth) / 2, 20);
    PangoCairo.show_layout(context, layout);
}

function keyCharacter(keyval) {
    const codePoint = Gdk.keyval_to_unicode(keyval);
    return codePoint === 0 ? '' : String.fromCodePoint(codePoint).toLowerCase();
}

function emitSelection(state, action) {
    const target = state.point || {
        x: state.rect.x + state.rect.width / 2,
        y: state.rect.y + state.rect.height / 2,
    };
    print(JSON.stringify({
        event: 'selected',
        strokes: state.path.join(''),
        action,
        normalized: { x: target.x, y: target.y },
    }));
}

function runOverlay() {
    const application = new Gtk.Application({
        application_id: 'io.github.onesevenar.notmouse.overlay',
        flags: Gio.ApplicationFlags.NON_UNIQUE,
    });

    application.connect('activate', () => {
        const state = {
            mode: 'grid', // 'grid' | 'scroll'
            path: [],
            rect: { x: 0, y: 0, width: 1, height: 1 },
            history: [],
            point: null,
            scrollSpeed: 5,
            lastScrollDir: null,
            dragging: false,
            dragStartPoint: null,
        };
        const window = new Gtk.ApplicationWindow({
            application,
            title: '!mouse',
            decorated: false,
        });
        window.add_css_class('notmouse-overlay');

        const provider = new Gtk.CssProvider();
        provider.load_from_data(
            `window {
                 background: transparent;
                 background-color: transparent;
             }`,
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
        keyboard.connect('key-pressed', (_controller, keyval, _keycode, modifierState) => {
            const char = keyCharacter(keyval);
            const isShift = (modifierState & Gdk.ModifierType.SHIFT_MASK) !== 0;

            // Handle Scroll Mode
            if (state.mode === 'scroll') {
                const mult = isShift ? 3 : 1;

                if (keyval === Gdk.KEY_Escape || char === 'q') {
                    application.quit();
                    return true;
                }

                if (keyval === Gdk.KEY_space || keyval === Gdk.KEY_Return || keyval === Gdk.KEY_KP_Enter) {
                    const target = state.point || {
                        x: state.rect.x + state.rect.width / 2,
                        y: state.rect.y + state.rect.height / 2,
                    };
                    print(JSON.stringify({
                        event: 'click',
                        button: isShift ? 'right' : 'left',
                        normalized: { x: target.x, y: target.y },
                    }));
                    application.quit();
                    return true;
                }

                if (keyval === Gdk.KEY_Tab) {
                    state.mode = 'grid';
                    const surface = window.get_surface();
                    if (surface) {
                        surface.set_input_region(null);
                    }
                    drawingArea.queue_draw();
                    return true;
                }

                if (char === 'j' || char === 's' || keyval === Gdk.KEY_Down) {
                    state.lastScrollDir = 'down';
                    print(JSON.stringify({ event: 'scroll', dx: 0, dy: -state.scrollSpeed * mult }));
                    drawingArea.queue_draw();
                    return true;
                }

                if (char === 'k' || char === 'w' || keyval === Gdk.KEY_Up) {
                    state.lastScrollDir = 'up';
                    print(JSON.stringify({ event: 'scroll', dx: 0, dy: state.scrollSpeed * mult }));
                    drawingArea.queue_draw();
                    return true;
                }

                if (char === 'd' || keyval === Gdk.KEY_Page_Down) {
                    state.lastScrollDir = 'down';
                    print(JSON.stringify({ event: 'scroll', dx: 0, dy: -state.scrollSpeed * 3 * mult }));
                    drawingArea.queue_draw();
                    return true;
                }

                if (char === 'u' || keyval === Gdk.KEY_Page_Up) {
                    state.lastScrollDir = 'up';
                    print(JSON.stringify({ event: 'scroll', dx: 0, dy: state.scrollSpeed * 3 * mult }));
                    drawingArea.queue_draw();
                    return true;
                }

                if (char === 'h' || keyval === Gdk.KEY_Left) {
                    state.lastScrollDir = 'left';
                    print(JSON.stringify({ event: 'scroll', dx: -state.scrollSpeed * mult, dy: 0 }));
                    drawingArea.queue_draw();
                    return true;
                }

                if (char === 'l' || keyval === Gdk.KEY_Right) {
                    state.lastScrollDir = 'right';
                    print(JSON.stringify({ event: 'scroll', dx: state.scrollSpeed * mult, dy: 0 }));
                    drawingArea.queue_draw();
                    return true;
                }

                return true;
            }

            // Handle Grid Mode
            if (keyval === Gdk.KEY_Escape) {
                if (state.dragging) {
                    print(JSON.stringify({ event: 'release', button: 'left' }));
                }
                print(JSON.stringify({ event: 'cancelled' }));
                application.quit();
                return true;
            }

            if (keyval === Gdk.KEY_BackSpace) {
                const previous = state.history.pop();
                if (previous !== undefined) {
                    state.rect = previous;
                    state.path.pop();
                    state.point = null;
                    drawingArea.queue_draw();
                }
                return true;
            }

            const isLocked = state.path.length >= MAX_DEPTH;

            // When in locked target mode: handle actions and nudging
            if (isLocked) {
                // Actions
                if (keyval === Gdk.KEY_Return || keyval === Gdk.KEY_KP_Enter || keyval === Gdk.KEY_space) {
                    if (state.dragging) {
                        const dropTarget = state.point || {
                            x: state.rect.x + state.rect.width / 2,
                            y: state.rect.y + state.rect.height / 2,
                        };
                        print(JSON.stringify({ event: 'move', x: dropTarget.x, y: dropTarget.y }));
                        print(JSON.stringify({ event: 'release', button: 'left' }));
                        state.dragging = false;
                        application.quit();
                        return true;
                    }
                    emitSelection(state, isShift ? 'right-click' : 'click');
                    application.quit();
                    return true;
                }

                // Click and stay (chained click)
                if (char === 'c') {
                    const target = state.point || {
                        x: state.rect.x + state.rect.width / 2,
                        y: state.rect.y + state.rect.height / 2,
                    };
                    print(JSON.stringify({
                        event: 'click',
                        button: isShift ? 'right' : 'left',
                        normalized: { x: target.x, y: target.y },
                    }));
                    state.path = [];
                    state.history = [];
                    state.rect = { x: 0, y: 0, width: 1, height: 1 };
                    state.point = null;
                    drawingArea.queue_draw();
                    return true;
                }

                if (char === 'r') {
                    emitSelection(state, 'right-click');
                    application.quit();
                    return true;
                }
                if (char === 'd') {
                    emitSelection(state, 'double-click');
                    application.quit();
                    return true;
                }
                if (char === 'm') {
                    emitSelection(state, 'middle-click');
                    application.quit();
                    return true;
                }

                // Drag Mode (Two-Phase Drag and Drop)
                if (char === 'v') {
                    if (!state.dragging) {
                        state.dragging = true;
                        const target = state.point || {
                            x: state.rect.x + state.rect.width / 2,
                            y: state.rect.y + state.rect.height / 2,
                        };
                        state.dragStartPoint = target;
                        print(JSON.stringify({ event: 'move', x: target.x, y: target.y }));
                        print(JSON.stringify({ event: 'press', button: 'left' }));

                        state.path = [];
                        state.history = [];
                        state.rect = { x: 0, y: 0, width: 1, height: 1 };
                        state.point = null;
                        drawingArea.queue_draw();
                        return true;
                    } else {
                        const dropTarget = state.point || {
                            x: state.rect.x + state.rect.width / 2,
                            y: state.rect.y + state.rect.height / 2,
                        };
                        print(JSON.stringify({ event: 'move', x: dropTarget.x, y: dropTarget.y }));
                        print(JSON.stringify({ event: 'release', button: 'left' }));
                        state.dragging = false;
                        application.quit();
                        return true;
                    }
                }

                // Scroll Mode (Continuous Interactive Kinetic Scroll)
                if (char === 's' || char === 'w') {
                    state.mode = 'scroll';
                    const target = state.point || {
                        x: state.rect.x + state.rect.width / 2,
                        y: state.rect.y + state.rect.height / 2,
                    };
                    const surface = window.get_surface();
                    if (surface) {
                        surface.set_input_region(new Cairo.Region());
                    }

                    const isUp = char === 'w' || isShift;
                    state.lastScrollDir = isUp ? 'up' : 'down';
                    drawingArea.queue_draw();

                    // Delay slightly so Mutter commits empty input region, then emit pointer motion
                    // so Mutter transfers pointer focus to the underlying window, followed by initial scroll!
                    GLib.timeout_add(GLib.PRIORITY_DEFAULT, 50, () => {
                        if (state.mode === 'scroll') {
                            print(JSON.stringify({ event: 'move', x: target.x, y: target.y }));
                            print(JSON.stringify({ event: 'scroll', dx: 0, dy: isUp ? 5 : -5 }));
                        }
                        return GLib.SOURCE_REMOVE;
                    });
                    return true;
                }

                // Nudge with hjkl or arrow keys
                const step = isShift ? 0.02 : 0.005;
                if (!state.point) {
                    state.point = {
                        x: state.rect.x + state.rect.width / 2,
                        y: state.rect.y + state.rect.height / 2,
                    };
                }

                let moved = false;
                if (char === 'h' || keyval === Gdk.KEY_Left) {
                    state.point.x = Math.max(0, state.point.x - step);
                    moved = true;
                } else if (char === 'l' || keyval === Gdk.KEY_Right) {
                    state.point.x = Math.min(1, state.point.x + step);
                    moved = true;
                } else if (char === 'k' || keyval === Gdk.KEY_Up) {
                    state.point.y = Math.max(0, state.point.y - step);
                    moved = true;
                } else if (char === 'j' || keyval === Gdk.KEY_Down) {
                    state.point.y = Math.min(1, state.point.y + step);
                    moved = true;
                }

                if (moved) {
                    drawingArea.queue_draw();
                    return true;
                }

                return true;
            }

            // Stroke 1 confirmation with Enter/Space for macro region center
            if (state.path.length === 1 && (keyval === Gdk.KEY_Return || keyval === Gdk.KEY_KP_Enter || keyval === Gdk.KEY_space)) {
                if (state.dragging) {
                    const dropTarget = {
                        x: state.rect.x + state.rect.width / 2,
                        y: state.rect.y + state.rect.height / 2,
                    };
                    print(JSON.stringify({ event: 'move', x: dropTarget.x, y: dropTarget.y }));
                    print(JSON.stringify({ event: 'release', button: 'left' }));
                    state.dragging = false;
                    application.quit();
                    return true;
                }
                emitSelection(state, isShift ? 'right-click' : 'click');
                application.quit();
                return true;
            }

            // Hint navigation (Stroke 1 or Stroke 2)
            const index = HINTS.indexOf(char);
            if (index === -1) {
                return true;
            }

            state.history.push({ ...state.rect });
            state.rect = childRect(state.rect, index);
            state.path.push(HINTS[index]);

            // If 2nd stroke was just entered, initialize target lock point (do NOT close!)
            if (state.path.length >= MAX_DEPTH) {
                state.point = {
                    x: state.rect.x + state.rect.width / 2,
                    y: state.rect.y + state.rect.height / 2,
                };
            }

            drawingArea.queue_draw();
            return true;
        });
        window.add_controller(keyboard);
        window.set_child(drawingArea);
        const display = Gdk.Display.get_default();
        const monitors = display.get_monitors();
        if (monitors.get_n_items() > 0) {
            const monitor = monitors.get_item(0);
            const geometry = monitor.get_geometry();
            window.set_default_size(geometry.width, geometry.height);
        }
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
    const macroCenter = childRect({ x: 0, y: 0, width: 1, height: 1 }, 4);
    if (
        Math.abs(macroCenter.x - 1 / 3) > Number.EPSILON
        || Math.abs(macroCenter.y - 1 / 3) > Number.EPSILON
        || Math.abs(macroCenter.width - 1 / 3) > Number.EPSILON
        || Math.abs(macroCenter.height - 1 / 3) > Number.EPSILON
    ) {
        throw new Error('macro zone geometry self-test failed');
    }

    // 2-stroke test: macro index 2 ('d'), micro index 5 ('k')
    const macroD = childRect({ x: 0, y: 0, width: 1, height: 1 }, 2);
    const microK = childRect(macroD, 5);
    const expectedX = 2 / 3 + 2 / 9;
    const expectedY = 0 / 3 + 1 / 9;
    if (
        Math.abs(microK.x - expectedX) > 1e-6
        || Math.abs(microK.y - expectedY) > 1e-6
        || Math.abs(microK.width - 1 / 9) > 1e-6
        || Math.abs(microK.height - 1 / 9) > 1e-6
    ) {
        throw new Error('2-stroke micro geometry self-test failed');
    }
    print('overlay self-test passed');
}

if (ARGV.includes('--self-test')) {
    selfTest();
} else {
    runOverlay();
}
