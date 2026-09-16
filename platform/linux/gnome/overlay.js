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

// ── AT-SPI Accessibility Semantic Target Scanner & Cache ───────────────────
let _cachedElements = [];
let _activeScanId = 0;
let _scanInProgress = false;

function getScannerScriptPath() {
    const devPath = GLib.build_filenamev([GLib.path_get_dirname(new Error().fileName || ''), 'atspi_scanner.py']);
    if (GLib.file_test(devPath, GLib.FileTest.IS_REGULAR)) {
        return devPath;
    }
    const userPath = GLib.build_filenamev([GLib.get_user_data_dir(), 'notmouse', 'atspi_scanner.py']);
    if (GLib.file_test(userPath, GLib.FileTest.IS_REGULAR)) {
        return userPath;
    }
    return '/usr/local/share/notmouse/atspi_scanner.py';
}

function readActivePidFromFile() {
    try {
        const runtimeDir = GLib.getenv('XDG_RUNTIME_DIR') || GLib.get_tmp_dir();
        const pidPath = `${runtimeDir}/notmouse-active-pid`;
        if (GLib.file_test(pidPath, GLib.FileTest.IS_REGULAR)) {
            const [ok, contents] = GLib.file_get_contents(pidPath);
            if (ok) {
                const text = (typeof contents === 'string')
                    ? contents.trim()
                    : new TextDecoder('utf-8').decode(contents).trim();
                const pid = parseInt(text, 10);
                if (pid > 0) {
                    return pid;
                }
            }
        }
    } catch (_e) {}
    return null;
}

// Asynchronously fetch elements in background without blocking the UI or delaying window presentation.
function fetchElementsAsync(callback, targetPid = null) {
    if (_scanInProgress) {
        return;
    }
    _scanInProgress = true;
    const scanId = ++_activeScanId;
    const script = getScannerScriptPath();
    if (!GLib.file_test(script, GLib.FileTest.IS_REGULAR)) {
        _scanInProgress = false;
        if (callback) callback([]);
        return;
    }

    const argv = ['python3', script];
    const pidToScan = targetPid || readActivePidFromFile();
    if (pidToScan) {
        argv.push('--pid', pidToScan.toString());
    }

    try {
        const [success, pid, stdin, stdout, stderr] = GLib.spawn_async_with_pipes(
            null,
            argv,
            null,
            GLib.SpawnFlags.SEARCH_PATH | GLib.SpawnFlags.DO_NOT_REAP_CHILD,
            null
        );
        if (!success) {
            _scanInProgress = false;
            if (callback) callback([]);
            return;
        }

        const stdoutChannel = GLib.IOChannel.unix_new(stdout);
        let output = '';
        GLib.io_add_watch(stdoutChannel, GLib.PRIORITY_DEFAULT, GLib.IOCondition.IN | GLib.IOCondition.HUP, (_ch, cond) => {
            if (cond & GLib.IOCondition.IN) {
                const [status, str] = stdoutChannel.read_to_end();
                if (status === GLib.IOStatus.NORMAL && str) {
                    output += (typeof str === 'string') ? str : new TextDecoder('utf-8').decode(str);
                }
            }
            if (cond & GLib.IOCondition.HUP) {
                GLib.spawn_close_pid(pid);
                _scanInProgress = false;
                if (scanId !== _activeScanId) {
                    // Outdated scan! Discard silently.
                    return GLib.SOURCE_REMOVE;
                }
                try {
                    const parsed = JSON.parse(output);
                    if (Array.isArray(parsed)) {
                        _cachedElements = parsed;
                    }
                    if (callback) callback(parsed);
                } catch (_err) {
                    if (callback) callback([]);
                }
                return GLib.SOURCE_REMOVE;
            }
            return GLib.SOURCE_CONTINUE;
        });
    } catch (_err) {
        _scanInProgress = false;
        if (callback) callback([]);
    }
}

function snapToNearestElement(state, window, drawingArea) {
    if (!state.point || !window) {
        return;
    }
    const winW = window.get_width() || 2560;
    const winH = window.get_height() || 1411;

    // Current reticle target in window pixel coordinates
    const targetPxX = state.point.x * winW;
    const targetPxY = state.point.y * winH;

    const source = _cachedElements || [];

    if (!source || source.length === 0) {
        state.snapCandidates = [];
        state.snapIndex = -1;
        state.snappedElement = null;
        if (drawingArea) {
            drawingArea.queue_draw();
        }
        return;
    }

    // Strictly confine candidates inside the selected micro cell boundary.
    // A small tolerance (4px) is applied so elements whose centroids sit at the
    // very edge of a cell (e.g. GitHub "Code" tab flush with a cell boundary)
    // are not silently excluded.
    const CELL_TOLERANCE_PX = 4;
    const cellMinX = state.rect.x * winW - CELL_TOLERANCE_PX;
    const cellMaxX = (state.rect.x + state.rect.width) * winW + CELL_TOLERANCE_PX;
    const cellMinY = state.rect.y * winH - CELL_TOLERANCE_PX;
    const cellMaxY = (state.rect.y + state.rect.height) * winH + CELL_TOLERANCE_PX;

    let inCell = [];
    for (const e of source) {
        const centerInCell = (
            e.cx >= cellMinX && e.cx <= cellMaxX &&
            e.cy >= cellMinY && e.cy <= cellMaxY
        );
        const intersectsCell = (
            e.x <= cellMaxX && (e.x + e.w) >= cellMinX &&
            e.y <= cellMaxY && (e.y + e.h) >= cellMinY
        );

        if (centerInCell) {
            inCell.push({ ...e });
        } else if (intersectsCell) {
            // For wide elements spanning across cells (e.g. tabs), compute the center
            // of the portion visible within this micro-cell so the snap coordinate stays
            // strictly inside the cell boundary without teleporting across the screen.
            const interLeft = Math.max(e.x, cellMinX);
            const interRight = Math.min(e.x + e.w, cellMaxX);
            const interTop = Math.max(e.y, cellMinY);
            const interBottom = Math.min(e.y + e.h, cellMaxY);
            const cellCx = (interLeft + interRight) / 2.0;
            const cellCy = (interTop + interBottom) / 2.0;
            inCell.push({
                ...e,
                cx: cellCx,
                cy: cellCy,
            });
        }
    }

    if (inCell.length === 0) {
        state.snapCandidates = [];
        state.snapIndex = -1;
        state.snappedElement = null;
        if (drawingArea) {
            drawingArea.queue_draw();
        }
        return;
    }

    // Ensure all cycled candidates strictly belong to the primary application in the cell
    const primaryApp = inCell[0].app_name;
    if (primaryApp) {
        inCell = inCell.filter(e => e.app_name === primaryApp);
    }

    // ── Reading-order sort ────────────────────────────────────────────────────
    // Bucket elements into horizontal rows using adaptive row banding:
    //   - Sort by Y first, then within each cluster of elements whose Y centers
    //     are within ROW_BAND_PX of each other, treat them as the same row and
    //     sort by X left-to-right.
    // Controls (buttons, inputs, tabs) are always listed before links within the
    // same row, so Tab lands on functional buttons first in dense mixed cells.
    const ROW_BAND_PX = 14; // px — elements within this vertical distance share a row

    // Sort: primary by cy, secondary by cx (reading order seed)
    const sorted = inCell.slice().sort((a, b) => {
        if (Math.abs(a.cy - b.cy) <= ROW_BAND_PX) {
            // Same row: controls before links, then left-to-right
            const aIsLink = a.is_link ? 1 : 0;
            const bIsLink = b.is_link ? 1 : 0;
            if (aIsLink !== bIsLink) return aIsLink - bIsLink;
            return a.cx - b.cx;
        }
        return a.cy - b.cy;
    });

    state.snapCandidates = sorted;

    // Auto-snap to the element nearest the reticle in reading-order list
    // (closest by Euclidean distance to give a natural starting anchor)
    let bestIdx = 0;
    let bestDist = Infinity;
    for (let i = 0; i < sorted.length; i++) {
        const dx = sorted[i].cx - targetPxX;
        const dy = sorted[i].cy - targetPxY;
        const d = dx * dx + dy * dy;
        if (d < bestDist) { bestDist = d; bestIdx = i; }
    }
    state.snapIndex = bestIdx;
    state.snappedElement = sorted[bestIdx];
    state.point.x = sorted[bestIdx].cx / winW;
    state.point.y = sorted[bestIdx].cy / winH;


    if (drawingArea) {
        drawingArea.queue_draw();
    }
}

// Cycle to the next/previous snappable element in state.snapCandidates.
// direction: +1 for forward (Tab), -1 for backward (Shift+Tab)
function cycleSnap(state, window, drawingArea, direction) {
    if (!state.snapCandidates || state.snapCandidates.length === 0) {
        return;
    }
    const winW = window.get_width() || 2560;
    const winH = window.get_height() || 1411;

    const n = state.snapCandidates.length;
    // If nothing snapped yet, start at 0; otherwise advance
    const next = state.snapIndex < 0
        ? 0
        : ((state.snapIndex + direction) % n + n) % n;

    state.snapIndex = next;
    state.snappedElement = state.snapCandidates[next];
    state.point = {
        x: state.snappedElement.cx / winW,
        y: state.snappedElement.cy / winH,
    };
    if (drawingArea) {
        drawingArea.queue_draw();
    }
}

function drawRoundedRect(context, x, y, w, h, radius) {
    context.moveTo(x + radius, y);
    context.lineTo(x + w - radius, y);
    context.arc(x + w - radius, y + radius, radius, -Math.PI / 2, 0);
    context.lineTo(x + w, y + h - radius);
    context.arc(x + w - radius, y + h - radius, radius, 0, Math.PI / 2);
    context.lineTo(x + radius, y + h);
    context.arc(x + radius, y + h - radius, radius, Math.PI / 2, Math.PI);
    context.lineTo(x, y + radius);
    context.arc(x + radius, y + radius, radius, Math.PI, -Math.PI / 2);
    context.closePath();
}

function drawLabel(area, context, label, x, y, options = {}) {
    const fontSize = options.fontSize || 22;
    const fontDesc = options.font || `Sans Bold ${fontSize}`;
    const text = options.preserveCase ? label : label.toUpperCase();
    const layout = area.create_pango_layout(text);
    const font = Pango.FontDescription.from_string(fontDesc);
    layout.set_font_description(font);
    const [textWidth, textHeight] = layout.get_pixel_size();
    const paddingX = options.paddingX !== undefined ? options.paddingX : 12;
    const paddingY = options.paddingY !== undefined ? options.paddingY : 6;

    const bg = options.bg || [0.97, 0.72, 0.18, 0.96];
    const fg = options.fg || [0.05, 0.06, 0.09, 1.0];

    const radius = options.radius !== undefined ? options.radius : 4;
    if (radius > 0) {
        // Rounded rectangle
        const rx = x - textWidth / 2 - paddingX;
        const ry = y - textHeight / 2 - paddingY;
        const rw = textWidth + paddingX * 2;
        const rh = textHeight + paddingY * 2;
        context.setSourceRGBA(bg[0], bg[1], bg[2], bg[3]);
        context.moveTo(rx + radius, ry);
        context.lineTo(rx + rw - radius, ry);
        context.arc(rx + rw - radius, ry + radius, radius, -Math.PI / 2, 0);
        context.lineTo(rx + rw, ry + rh - radius);
        context.arc(rx + rw - radius, ry + rh - radius, radius, 0, Math.PI / 2);
        context.lineTo(rx + radius, ry + rh);
        context.arc(rx + radius, ry + rh - radius, radius, Math.PI / 2, Math.PI);
        context.lineTo(rx, ry + radius);
        context.arc(rx + radius, ry + radius, radius, Math.PI, -Math.PI / 2);
        context.closePath();
        context.fill();
    } else {
        context.setSourceRGBA(bg[0], bg[1], bg[2], bg[3]);
        context.rectangle(
            x - textWidth / 2 - paddingX,
            y - textHeight / 2 - paddingY,
            textWidth + paddingX * 2,
            textHeight + paddingY * 2,
        );
        context.fill();
    }

    context.setSourceRGBA(fg[0], fg[1], fg[2], fg[3]);
    context.moveTo(x - textWidth / 2, y - textHeight / 2);
    PangoCairo.show_layout(context, layout);
}

function startRippleAnimation(drawingArea, state) {
    if (!drawingArea || !state) return;
    GLib.timeout_add(GLib.PRIORITY_DEFAULT, 16, () => {
        if (!state.lastClickTime) return GLib.SOURCE_REMOVE;
        const elapsedUs = GLib.get_monotonic_time() - state.lastClickTime;
        if (elapsedUs >= 280 * 1000) {
            drawingArea.queue_draw();
            return GLib.SOURCE_REMOVE;
        }
        drawingArea.queue_draw();
        return GLib.SOURCE_CONTINUE;
    });
}

function drawReticle(context, x, y, isSnapped = false, lastClickTime = 0) {
    const radius = 18;

    // Visual ripple shockwave on "Click & Stay" (c)
    if (lastClickTime) {
        const elapsedUs = GLib.get_monotonic_time() - lastClickTime;
        const elapsedMs = elapsedUs / 1000.0;
        if (elapsedMs < 280) {
            const progress = elapsedMs / 280.0;
            const rippleR = radius + progress * 24.0;
            const alpha = (1.0 - progress) * 0.9;
            context.setSourceRGBA(0.20, 0.95, 0.45, alpha); // neon emerald shockwave
            context.setLineWidth(3.0 * (1.0 - progress * 0.5));
            context.arc(x, y, rippleR, 0, 2 * Math.PI);
            context.stroke();
        }
    }

    if (isSnapped) {
        // High-contrast electric cyan lock ring indicating snapped UI element
        context.setSourceRGBA(0.18, 0.85, 0.98, 0.40);
        context.setLineWidth(5.0);
        context.arc(x, y, radius + 3, 0, 2 * Math.PI);
        context.stroke();

        context.setSourceRGBA(0.18, 0.85, 0.98, 1.0);
        context.setLineWidth(2.2);
        context.arc(x, y, radius, 0, 2 * Math.PI);
        context.stroke();

        // Target corner brackets
        context.setLineWidth(2.0);
        const bLen = 6;
        // Top-left bracket
        context.moveTo(x - radius - 2, y - radius + bLen);
        context.lineTo(x - radius - 2, y - radius - 2);
        context.lineTo(x - radius + bLen, y - radius - 2);
        // Top-right bracket
        context.moveTo(x + radius + 2 - bLen, y - radius - 2);
        context.lineTo(x + radius + 2, y - radius - 2);
        context.lineTo(x + radius + 2, y - radius + bLen);
        // Bottom-left bracket
        context.moveTo(x - radius - 2, y + radius - bLen);
        context.lineTo(x - radius - 2, y + radius + 2);
        context.lineTo(x - radius + bLen, y + radius + 2);
        // Bottom-right bracket
        context.moveTo(x + radius + 2 - bLen, y + radius + 2);
        context.lineTo(x + radius + 2, y + radius + 2);
        context.lineTo(x + radius + 2, y + radius - bLen);
        context.stroke();

        // Crosshair ticks
        context.setLineWidth(1.8);
        context.moveTo(x - radius - 8, y);
        context.lineTo(x - radius + 4, y);
        context.moveTo(x + radius - 4, y);
        context.lineTo(x + radius + 8, y);
        context.moveTo(x, y - radius - 8);
        context.lineTo(x, y - radius + 4);
        context.moveTo(x, y + radius - 4);
        context.lineTo(x, y + radius + 8);
        context.stroke();

        // Center dot
        context.setSourceRGBA(0.18, 0.85, 0.98, 1.0);
        context.arc(x, y, 3.5, 0, 2 * Math.PI);
        context.fill();
        return;
    }

    // Default golden reticle
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

let _lastTriggerStartUs = null;

function drawOverlay(area, context, width, height, state) {
    if (_lastTriggerStartUs) {
        const tNowUs = GLib.get_real_time();
        const totalLatency = ((tNowUs - _lastTriggerStartUs) / 1000.0).toFixed(1);
        _lastTriggerStartUs = null;
        const logLine = `[LIVE MEASUREMENT] Overlay displayed in: ${totalLatency} ms\n`;
        print(logLine.trim());
        try {
            const file = Gio.File.new_for_path('/tmp/notmouse_latency.log');
            const outStream = file.append_to(Gio.FileCreateFlags.NONE, null);
            outStream.write(logLine, null);
            outStream.close(null);
        } catch (_e) {}
    }

    if (state.mode === 'scroll') {
        // Compact acrylic HUD pill fitting inside (width, height)
        const pillWidth = Math.min(width, 560);
        const pillHeight = Math.min(height, 48);
        const pillX = (width - pillWidth) / 2;
        const pillY = (height - pillHeight) / 2;
        const radius = 12;

        // Dark acrylic glass background
        context.setSourceRGBA(0.06, 0.08, 0.14, 0.94);
        drawRoundedRect(context, pillX, pillY, pillWidth, pillHeight, radius);
        context.fill();

        // Accent border (cyan)
        context.setSourceRGBA(0.18, 0.80, 0.97, 0.85);
        context.setLineWidth(1.5);
        drawRoundedRect(context, pillX, pillY, pillWidth, pillHeight, radius);
        context.stroke();

        // Direction indicators & status
        const isUp = state.lastScrollDir === 'up';
        const isDown = state.lastScrollDir === 'down';
        const isLeft = state.lastScrollDir === 'left';
        const isRight = state.lastScrollDir === 'right';

        let dirIcon = '⇕';
        if (isUp) dirIcon = '▲';
        else if (isDown) dirIcon = '▼';
        else if (isLeft) dirIcon = '◀';
        else if (isRight) dirIcon = '▶';

        const hudText = `${dirIcon} SCROLL  —  [j/k] Down/Up  [d/u] Page  [Shift] Faster  [Tab] Grid  [Esc] Done`;
        const hudLayout = area.create_pango_layout(hudText);
        hudLayout.set_font_description(Pango.FontDescription.from_string('Sans Bold 11'));
        const [textW, textH] = hudLayout.get_pixel_size();

        context.setSourceRGBA(0.85, 0.92, 1.0, 0.95);
        context.moveTo(pillX + (pillWidth - textW) / 2, pillY + (pillHeight - textH) / 2);
        PangoCairo.show_layout(context, hudLayout);
        return;
    }

    if (state.mode === 'topbar') {
        // Subtle background tint so desktop underneath remains completely clear
        context.setSourceRGBA(0.01, 0.02, 0.04, 0.06);
        context.rectangle(0, 0, width, height);
        context.fill();

        // 3 top bar target zones along top edge (calibrated to GNOME Shell panel widgets)
        const zones = [
            { key: 'A', id: 'a', name: 'ACTIVITIES', x: Math.max(68, 0.013 * width) },
            { key: 'S', id: 's', name: 'CLOCK / DATE', x: 0.500 * width },
            { key: 'D', id: 'd', name: 'SETTINGS / WIFI', x: Math.min(width - 70, 0.973 * width) },
        ];

        // Glowing golden top border indicating active top bar mode
        context.setSourceRGBA(0.97, 0.72, 0.18, 0.90);
        context.setLineWidth(3.0);
        context.moveTo(0, 1.5);
        context.lineTo(width, 1.5);
        context.stroke();

        for (const z of zones) {
            const isSelected = state.topBarTarget === z.id;
            drawLabel(area, context, `${z.key} • ${z.name}`, z.x, 26, {
                fontSize: 12,
                paddingX: 12,
                paddingY: 6,
                bg: isSelected ? [0.97, 0.72, 0.18, 0.95] : [0.10, 0.14, 0.22, 0.95],
                fg: isSelected ? [0.05, 0.08, 0.12, 1.0] : [0.97, 0.72, 0.18, 1.0],
            });
        }

        // Draw reticle at current position pointing up into the bar
        const reticleX = (state.point ? state.point.x : 0.973) * width;
        const reticleY = 16;
        drawReticle(context, reticleX, reticleY, false, state.lastClickTime || 0);

        // Upward arrow on reticle pointing into the top bar
        context.setSourceRGBA(0.97, 0.72, 0.18, 1.0);
        context.moveTo(reticleX, 2);
        context.lineTo(reticleX - 8, 14);
        context.lineTo(reticleX + 8, 14);
        context.closePath();
        context.fill();

        // Floating HUD at bottom
        const barWidth = Math.min(width - 40, 820);
        const barHeight = 44;
        const barX = (width - barWidth) / 2;
        const barY = height - barHeight - 24;

        context.setSourceRGBA(0.06, 0.08, 0.12, 0.92);
        context.rectangle(barX, barY, barWidth, barHeight);
        context.fill();

        context.setSourceRGBA(0.97, 0.72, 0.18, 0.85);
        context.setLineWidth(1.0);
        context.rectangle(barX, barY, barWidth, barHeight);
        context.stroke();

        const hudText = 'Top Bar  —  [a] Activities  [s] Clock  [d] Settings  [c] Click & Stay  [Enter] Click  [Tab] Grid  [Esc] Exit';
        const hudLayout = area.create_pango_layout(hudText);
        hudLayout.set_font_description(Pango.FontDescription.from_string('Sans 12'));
        const [textW, textH] = hudLayout.get_pixel_size();
        context.setSourceRGBA(0.80, 0.88, 1.0, 0.90);
        context.moveTo(barX + (barWidth - textW) / 2, barY + (barHeight - textH) / 2);
        PangoCairo.show_layout(context, hudLayout);
        return;
    }

    const isMacroView = state.path.length === 0;
    const isLocked = state.path.length >= MAX_DEPTH;

    // Subtly dim the screen background so elements underneath remain clearly visible
    context.setSourceRGBA(0.02, 0.03, 0.06, isMacroView ? 0.08 : 0.16);
    context.rectangle(0, 0, width, height);
    context.fill();

    // If dragging active, draw source anchor
    if (state.dragging && state.dragStartPoint) {
        const startX = state.dragStartPoint.x * width;
        const startY = state.dragStartPoint.y * height;
        context.setSourceRGBA(1.0, 0.25, 0.25, 0.90);
        context.arc(startX, startY, 12, 0, 2 * Math.PI);
        context.fill();
        drawLabel(area, context, 'Drag source', startX, startY - 24, {
            fontSize: 10,
            paddingX: 6,
            paddingY: 2,
            preserveCase: true,
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
        const isSnapped = Boolean(state.snappedElement);
        drawReticle(context, reticleX, reticleY, isSnapped, state.lastClickTime || 0);

        // Label above reticle (shows chord + snapped element name + cycle position)
        let badgeText;
        if (isSnapped && state.snappedElement.name) {
            const name = state.snappedElement.name.slice(0, 18);
            const total = state.snapCandidates ? state.snapCandidates.length : 0;
            const idx = state.snapIndex >= 0 ? state.snapIndex + 1 : 1;
            const cycleHint = total > 1 ? ` [${idx}/${total}]` : '';
            badgeText = `${state.path.join('').toUpperCase()}  ${name}${cycleHint}`;
        } else {
            badgeText = state.path.join('').toUpperCase();
        }

        // Links get a warm amber badge; controls keep the electric cyan badge
        const isSnapLink = isSnapped && state.snappedElement.is_link;
        drawLabel(area, context, badgeText, reticleX, reticleY - 36, {
            fontSize: isSnapped ? 12 : 16,
            paddingX: isSnapped ? 10 : 10,
            paddingY: 5,
            preserveCase: true,
            bg: isSnapped
                ? (isSnapLink ? [0.20, 0.14, 0.05, 0.96] : [0.08, 0.18, 0.28, 0.96])
                : [0.97, 0.72, 0.18, 0.98],
            fg: isSnapped
                ? (isSnapLink ? [0.98, 0.75, 0.18, 1.0] : [0.18, 0.85, 0.98, 1.0])
                : [0.05, 0.06, 0.09, 1.0],
        });
    }

    // Header breadcrumb — shown at top center with a background pill
    let breadcrumb;
    if (state.dragging) {
        if (isMacroView) {
            breadcrumb = 'Drag  —  Stroke 1: choose destination  [Esc] Cancel';
        } else if (!isLocked) {
            breadcrumb = `Drag  ${state.path[0].toUpperCase()}  —  Stroke 2: choose drop target  [Space] Drop`;
        } else {
            breadcrumb = `Drop target: ${state.path.join('').toUpperCase()}  —  [v/Space/Enter] Drop  [Esc] Cancel`;
        }
    } else if (isMacroView) {
        breadcrumb = '!mouse  —  Stroke 1: choose region  [t] Top Bar  [Esc] Exit';
    } else if (!isLocked) {
        breadcrumb = `Region ${state.path[0].toUpperCase()}  —  Stroke 2: choose target  [Enter] Region center  [Backspace] Undo`;
    } else {
        if (state.snappedElement) {
            const rawRole = state.snappedElement.role || 'element';
            const role = state.snappedElement.is_link ? 'nav link' : rawRole;
            const name = state.snappedElement.name ? `"${state.snappedElement.name.slice(0, 28)}"` : '';
            const total = state.snapCandidates ? state.snapCandidates.length : 0;
            const tabHint = total > 1 ? `  [Tab] ${state.snapIndex + 1}/${total}` : '';
            breadcrumb = `${role}${name ? '  ' + name : ''}${tabHint}  —  [Enter] Click  [c] Click & Stay  [h/j/k/l] Nudge  [Esc] Exit`;
        } else {
            breadcrumb = `${state.path.join('').toUpperCase()}  —  [Enter] Click  [s] Scroll  [v] Drag  [c] Click & Stay  [r/d/m] Other  [Esc] Exit`;
        }
    }

    // Pill background behind breadcrumb
    const bcLayout = area.create_pango_layout(breadcrumb);
    bcLayout.set_font_description(Pango.FontDescription.from_string('Sans 12'));
    const [bcW, bcH] = bcLayout.get_pixel_size();
    const bcPadX = 16;
    const bcPadY = 6;
    const bcX = (width - bcW) / 2;
    const bcY = 16;
    const bcRx = bcX - bcPadX;
    const bcRy = bcY - bcPadY;
    const bcRw = bcW + bcPadX * 2;
    const bcRh = bcH + bcPadY * 2;
    const bcR = 6;
    context.setSourceRGBA(0.04, 0.05, 0.10, 0.82);
    context.moveTo(bcRx + bcR, bcRy);
    context.lineTo(bcRx + bcRw - bcR, bcRy);
    context.arc(bcRx + bcRw - bcR, bcRy + bcR, bcR, -Math.PI / 2, 0);
    context.lineTo(bcRx + bcRw, bcRy + bcRh - bcR);
    context.arc(bcRx + bcRw - bcR, bcRy + bcRh - bcR, bcR, 0, Math.PI / 2);
    context.lineTo(bcRx + bcR, bcRy + bcRh);
    context.arc(bcRx + bcR, bcRy + bcRh - bcR, bcR, Math.PI / 2, Math.PI);
    context.lineTo(bcRx, bcRy + bcR);
    context.arc(bcRx + bcR, bcRy + bcR, bcR, Math.PI, -Math.PI / 2);
    context.closePath();
    context.fill();

    context.setSourceRGBA(0.75, 0.82, 0.95, 0.85);
    context.moveTo(bcX, bcY);
    PangoCairo.show_layout(context, bcLayout);
}

function keyCharacter(keyval) {
    const codePoint = Gdk.keyval_to_unicode(keyval);
    return codePoint === 0 ? '' : String.fromCodePoint(codePoint).toLowerCase();
}

function getActiveMonitor(window) {
    const display = Gdk.Display.get_default();
    if (!display) return null;
    const monitors = display.get_monitors();
    if (!monitors || monitors.get_n_items() === 0) return null;

    if (window) {
        const surface = window.get_surface();
        if (surface && display.get_monitor_at_surface) {
            const m = display.get_monitor_at_surface(surface);
            if (m) return m;
        }
    }
    return monitors.get_item(0);
}

function getDesktopBounds() {
    const display = Gdk.Display.get_default();
    const monitors = display ? display.get_monitors() : null;
    if (!monitors || monitors.get_n_items() === 0) {
        return { minX: 0, minY: 0, width: 1920, height: 1080 };
    }
    let minX = 0, minY = 0, maxX = 0, maxY = 0;
    const count = monitors.get_n_items();
    for (let i = 0; i < count; i++) {
        const g = monitors.get_item(i).get_geometry();
        if (i === 0) {
            minX = g.x;
            minY = g.y;
            maxX = g.x + g.width;
            maxY = g.y + g.height;
        } else {
            minX = Math.min(minX, g.x);
            minY = Math.min(minY, g.y);
            maxX = Math.max(maxX, g.x + g.width);
            maxY = Math.max(maxY, g.y + g.height);
        }
    }
    return {
        minX,
        minY,
        width: Math.max(1, maxX - minX),
        height: Math.max(1, maxY - minY),
    };
}

function getScreenCoordinates(state, window) {
    const target = state.point || {
        x: state.rect.x + state.rect.width / 2,
        y: state.rect.y + state.rect.height / 2,
    };
    if (!window) {
        return target;
    }
    const monitor = getActiveMonitor(window);
    if (!monitor) {
        return target;
    }
    const geom = monitor.get_geometry();
    const winW = window.get_width();
    const winH = window.get_height();

    // Workarea vertical offset (e.g. GNOME top status bar) and horizontal offset
    const offsetY = Math.max(0, geom.height - winH);
    const offsetX = Math.max(0, geom.width - winW);

    const pixelX = target.x * winW;
    const pixelY = target.y * winH;

    const desktop = getDesktopBounds();
    const globalX = geom.x + offsetX + pixelX;
    const globalY = geom.y + offsetY + pixelY;

    return {
        x: (globalX - desktop.minX) / desktop.width,
        y: (globalY - desktop.minY) / desktop.height,
    };
}

let _isResident = false;
let _daemonStream = null;

function sendEvent(eventObj) {
    const line = JSON.stringify(eventObj) + '\n';
    for (let attempt = 0; attempt < 2; attempt++) {
        if (!_daemonStream) {
            try {
                const runtimeDir = GLib.getenv('XDG_RUNTIME_DIR') || GLib.get_tmp_dir();
                const sockPath = `${runtimeDir}/notmouse.sock`;
                const sockFile = Gio.File.new_for_path(sockPath);
                if (sockFile.query_exists(null)) {
                    const client = new Gio.SocketClient();
                    const addr = Gio.UnixSocketAddress.new(sockPath);
                    const conn = client.connect(addr, null);
                    _daemonStream = conn.get_output_stream();
                }
            } catch (_e) {
                _daemonStream = null;
            }
        }
        if (_daemonStream) {
            try {
                _daemonStream.write(line, null);
                _daemonStream.flush(null);
                return;
            } catch (_e) {
                try {
                    _daemonStream.close(null);
                } catch (_) {}
                _daemonStream = null;
                // Reconnect and retry on next iteration if socket connection was stale/broken
            }
        }
    }
    print(line.trim());
}

function dismissOverlay(window, application) {
    if (_isResident) {
        window.set_visible(false);
    } else {
        application.quit();
    }
}

function emitSelection(state, action, window) {
    const target = getScreenCoordinates(state, window);
    sendEvent({
        event: 'selected',
        strokes: state.path.join(''),
        action,
        normalized: { x: target.x, y: target.y },
    });
}

function ensureScrollFocused(state, window) {
    if (!state.scrollFocused) {
        const target = state.scrollTarget || getScreenCoordinates(state, window);
        sendEvent({ event: 'move', x: target.x, y: target.y });
        state.scrollFocused = true;
    }
}

function resetOverlayState(state) {
    state.mode = 'grid';
    state.path = [];
    state.rect = { x: 0, y: 0, width: 1, height: 1 };
    state.history = [];
    state.point = null;
    state.scrollSpeed = 5;
    state.lastScrollDir = null;
    state.scrollFocused = false;
    state.scrollTarget = null;
    state.dragging = false;
    state.dragStartPoint = null;
    state.snappedElement = null;
    state.nearbyElements = [];
    state.snapCandidates = [];
    state.snapIndex = -1;
    state.topBarTarget = null;
    state.targetPid = null;
}

function showOverlay(state, window, drawingArea, startUs = null, targetPid = null) {
    _lastTriggerStartUs = startUs || GLib.get_real_time();
    resetOverlayState(state);
    if (targetPid) {
        state.targetPid = targetPid;
    }

    const monitor = getActiveMonitor(window);
    if (monitor) {
        const geometry = monitor.get_geometry();
        window.set_default_size(geometry.width, geometry.height);
    }
    window.maximize();

    window.set_visible(true);
    window.present();
    drawingArea.grab_focus();
    drawingArea.queue_draw();

    if (startUs) {
        const nowUs = GLib.get_real_time();
        const deltaMs = (nowUs - startUs) / 1000.0;
        try {
            const logLine = `[WARM_LATENCY] from trigger to window.present(): ${deltaMs.toFixed(2)} ms (start: ${startUs}, present: ${nowUs})\n`;
            const file = Gio.File.new_for_path('/tmp/notmouse_latency.log');
            const stream = file.append_to(Gio.FileCreateFlags.NONE, null);
            stream.write(logLine, null);
            stream.close(null);
        } catch (_e) {}
    }

    // Always flush element cache on every summon — the user may have navigated
    // to a new page since the last overlay use, so stale cached elements from
    // the previous URL should never be served.
    _activeScanId++;
    _cachedElements = [];

    // Scan the focused window asynchronously in the background with zero UI freeze
    fetchElementsAsync((elements) => {
        if (elements && elements.length > 0) {
            if (state.path.length >= MAX_DEPTH && !state.snappedElement && state.point) {
                snapToNearestElement(state, window, drawingArea);
            }
        }
    }, state.targetPid);
}

function setupOverlaySocket(state, window, drawingArea, application) {
    const runtimeDir = GLib.getenv('XDG_RUNTIME_DIR') || GLib.get_tmp_dir();
    const sockPath = `${runtimeDir}/notmouse-overlay.sock`;
    const sockFile = Gio.File.new_for_path(sockPath);

    try {
        sockFile.delete(null);
    } catch (_e) {}

    const listener = new Gio.SocketListener();
    const addr = Gio.UnixSocketAddress.new(sockPath);
    try {
        listener.add_address(addr, Gio.SocketType.STREAM, Gio.SocketProtocol.DEFAULT, null);
    } catch (e) {
        printerr(`!mouse: could not bind overlay socket at ${sockPath}: ${e}`);
        return;
    }

    function acceptConnection() {
        listener.accept_async(null, (source, res) => {
            try {
                const [conn] = source.accept_finish(res);
                const dis = new Gio.DataInputStream({
                    base_stream: conn.get_input_stream(),
                });
                readCommand(dis);
            } catch (_e) {
                return;
            }
            acceptConnection();
        });
    }

    function readCommand(dis) {
        dis.read_line_async(GLib.PRIORITY_DEFAULT, null, (stream, res) => {
            try {
                const [line] = stream.read_line_finish_utf8(res);
                if (line) {
                    const data = JSON.parse(line);
                    if (data.action === 'show') {
                        showOverlay(state, window, drawingArea, data.start_us || null, data.target_pid || null);
                    } else if (data.action === 'hide') {
                        dismissOverlay(window, application);
                    } else if (data.action === 'quit') {
                        application.quit();
                    }
                }
            } catch (_e) {}
        });
    }

    acceptConnection();

    application.connect('shutdown', () => {
        try {
            sockFile.delete(null);
        } catch (_e) {}
    });
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
            scrollFocused: false,
            scrollTarget: null,
            dragging: false,
            dragStartPoint: null,
            snappedElement: null,
            nearbyElements: [],
            snapCandidates: [],  // all elements within cycling radius, sorted by distance
            snapIndex: -1,       // index into snapCandidates of current snap target
            targetPid: null,
        };
        const window = new Gtk.ApplicationWindow({
            application,
            title: '!mouse',
            decorated: false,
        });
        window.add_css_class('notmouse-overlay');

        const provider = new Gtk.CssProvider();
        provider.load_from_data(
            `window,
             window.background,
             window.fullscreen,
             .background,
             .fullscreen,
             .notmouse-overlay,
             drawingarea {
                 background-color: rgba(0, 0, 0, 0);
                 background-image: none;
                 box-shadow: none;
                 border: none;
             }`,
            -1,
        );
        Gtk.StyleContext.add_provider_for_display(
            Gdk.Display.get_default(),
            provider,
            Gtk.STYLE_PROVIDER_PRIORITY_USER,
        );

        const drawingArea = new Gtk.DrawingArea({
            hexpand: true,
            vexpand: true,
            focusable: true,
        });
        drawingArea.set_draw_func((area, context, width, height) => {
            context.setOperator(Cairo.Operator.CLEAR);
            context.paint();
            context.setOperator(Cairo.Operator.OVER);
            drawOverlay(area, context, width, height, state);
        });

        const keyboard = new Gtk.EventControllerKey();
        keyboard.connect('key-pressed', (_controller, keyval, _keycode, modifierState) => {
            const char = keyCharacter(keyval);
            const isShift = (modifierState & Gdk.ModifierType.SHIFT_MASK) !== 0;

            // Handle Top Bar Mode
            if (state.mode === 'topbar') {
                if (keyval === Gdk.KEY_Escape || char === 'q') {
                    dismissOverlay(window, application);
                    return true;
                }
                if (keyval === Gdk.KEY_Tab || keyval === Gdk.KEY_BackSpace) {
                    state.mode = 'grid';
                    state.point = null;
                    state.topBarTarget = null;
                    drawingArea.queue_draw();
                    return true;
                }
                if (keyval === Gdk.KEY_Return || keyval === Gdk.KEY_KP_Enter || keyval === Gdk.KEY_space) {
                    emitSelection(state, isShift ? 'right-click' : 'click', window);
                    dismissOverlay(window, application);
                    return true;
                }
                if (char === 'r') {
                    emitSelection(state, 'right-click', window);
                    dismissOverlay(window, application);
                    return true;
                }
                // Click and stay: click and STAY locked on target!
                if (char === 'c') {
                    const target = getScreenCoordinates(state, window);
                    // Capture link flag BEFORE clearing state — nav links (GitHub tabs etc.)
                    // trigger SPA page transitions that take ~500–800ms to settle in the DOM.
                    const wasNavLink = Boolean(state.snappedElement && state.snappedElement.is_link);

                    window.set_visible(false);
                    sendEvent({
                        event: 'click',
                        button: isShift ? 'right' : 'left',
                        normalized: { x: target.x, y: target.y },
                    });

                    state.lastClickTime = GLib.get_monotonic_time();

                    // Invalidate stale element cache since the click may mutate/destroy UI elements
                    _activeScanId++;
                    _cachedElements = [];
                    _cacheTimestamp = 0;
                    state.snappedElement = null;
                    state.snapCandidates = [];
                    state.snapIndex = -1;

                    // Re-present overlay immediately after click completes (~60ms),
                    // KEEPING all state completely locked in place!
                    GLib.timeout_add(GLib.PRIORITY_DEFAULT, 60, () => {
                        window.set_visible(true);
                        window.present();
                        drawingArea.grab_focus();
                        drawingArea.queue_draw();
                        startRippleAnimation(drawingArea, state);

                        // For nav-link clicks (SPA page transitions), delay the rescan by an
                        // additional 550ms so the browser has time to render the new page before
                        // AT-SPI walks the DOM. Button clicks keep the original fast path.
                        const rescanDelay = wasNavLink ? 650 : 150;
                        GLib.timeout_add(GLib.PRIORITY_DEFAULT, rescanDelay, () => {
                            fetchElementsAsync((elements) => {
                                if (elements && elements.length > 0) {
                                    if (state.path.length >= MAX_DEPTH && state.point) {
                                        snapToNearestElement(state, window, drawingArea);
                                    }
                                }
                            }, state.targetPid);
                            return GLib.SOURCE_REMOVE;
                        });

                        return GLib.SOURCE_REMOVE;
                    });
                    return true;
                }
                if (char === 'a') {
                    if (state.topBarTarget === 'a') {
                        emitSelection(state, isShift ? 'right-click' : 'click', window);
                        dismissOverlay(window, application);
                        return true;
                    }
                    state.topBarTarget = 'a';
                    state.point = { x: 0.013, y: -0.010 };
                    drawingArea.queue_draw();
                    return true;
                }
                if (char === 's') {
                    if (state.topBarTarget === 's') {
                        emitSelection(state, isShift ? 'right-click' : 'click', window);
                        dismissOverlay(window, application);
                        return true;
                    }
                    state.topBarTarget = 's';
                    state.point = { x: 0.500, y: -0.010 };
                    drawingArea.queue_draw();
                    return true;
                }
                if (char === 'd') {
                    if (state.topBarTarget === 'd') {
                        emitSelection(state, isShift ? 'right-click' : 'click', window);
                        dismissOverlay(window, application);
                        return true;
                    }
                    state.topBarTarget = 'd';
                    state.point = { x: 0.973, y: -0.010 };
                    drawingArea.queue_draw();
                    return true;
                }

                // Nudge inside top bar
                const step = isShift ? 0.02 : 0.005;
                if (!state.point) {
                    state.point = { x: 0.973, y: -0.010 };
                }
                if (char === 'h' || keyval === Gdk.KEY_Left) {
                    state.point.x = Math.max(0, state.point.x - step);
                    drawingArea.queue_draw();
                    return true;
                }
                if (char === 'l' || keyval === Gdk.KEY_Right) {
                    state.point.x = Math.min(1, state.point.x + step);
                    drawingArea.queue_draw();
                    return true;
                }
                return true;
            }

            // Handle Scroll Mode
            if (state.mode === 'scroll') {
                const mult = isShift ? 3 : 1;

                if (keyval === Gdk.KEY_Escape || char === 'q') {
                    dismissOverlay(window, application);
                    return true;
                }

                if (keyval === Gdk.KEY_space || keyval === Gdk.KEY_Return || keyval === Gdk.KEY_KP_Enter) {
                    const target = state.scrollTarget || getScreenCoordinates(state, window);
                    sendEvent({
                        event: 'click',
                        button: isShift ? 'right' : 'left',
                        normalized: { x: target.x, y: target.y },
                    });
                    dismissOverlay(window, application);
                    return true;
                }

                if (keyval === Gdk.KEY_Tab) {
                    state.mode = 'grid';
                    const monitor = getActiveMonitor(window);
                    if (monitor) {
                        const geometry = monitor.get_geometry();
                        window.set_default_size(geometry.width, geometry.height);
                    }
                    window.maximize();
                    drawingArea.queue_draw();
                    return true;
                }

                if (char === 'j' || char === 's' || keyval === Gdk.KEY_Down) {
                    ensureScrollFocused(state, window);
                    state.lastScrollDir = 'down';
                    sendEvent({ event: 'scroll', dx: 0, dy: -state.scrollSpeed * mult });
                    drawingArea.queue_draw();
                    return true;
                }

                if (char === 'k' || char === 'w' || keyval === Gdk.KEY_Up) {
                    ensureScrollFocused(state, window);
                    state.lastScrollDir = 'up';
                    sendEvent({ event: 'scroll', dx: 0, dy: state.scrollSpeed * mult });
                    drawingArea.queue_draw();
                    return true;
                }

                if (char === 'd' || keyval === Gdk.KEY_Page_Down) {
                    ensureScrollFocused(state, window);
                    state.lastScrollDir = 'down';
                    sendEvent({ event: 'scroll', dx: 0, dy: -state.scrollSpeed * 3 * mult });
                    drawingArea.queue_draw();
                    return true;
                }

                if (char === 'u' || keyval === Gdk.KEY_Page_Up) {
                    ensureScrollFocused(state, window);
                    state.lastScrollDir = 'up';
                    sendEvent({ event: 'scroll', dx: 0, dy: state.scrollSpeed * 3 * mult });
                    drawingArea.queue_draw();
                    return true;
                }

                if (char === 'h' || keyval === Gdk.KEY_Left) {
                    ensureScrollFocused(state, window);
                    state.lastScrollDir = 'left';
                    sendEvent({ event: 'scroll', dx: -state.scrollSpeed * mult, dy: 0 });
                    drawingArea.queue_draw();
                    return true;
                }

                if (char === 'l' || keyval === Gdk.KEY_Right) {
                    ensureScrollFocused(state, window);
                    state.lastScrollDir = 'right';
                    sendEvent({ event: 'scroll', dx: state.scrollSpeed * mult, dy: 0 });
                    drawingArea.queue_draw();
                    return true;
                }

                return true;
            }

            // Handle Grid Mode
            if (keyval === Gdk.KEY_Escape) {
                if (state.dragging) {
                    sendEvent({ event: 'release', button: 'left' });
                }
                sendEvent({ event: 'cancelled' });
                dismissOverlay(window, application);
                return true;
            }

            if (keyval === Gdk.KEY_BackSpace) {
                const previous = state.history.pop();
                if (previous !== undefined) {
                    state.rect = previous;
                    state.path.pop();
                    state.point = null;
                    state.snappedElement = null;
                    state.snapCandidates = [];
                    state.snapIndex = -1;

                    // If returning to root after a click occurred (e.g. user clicked a tab/link
                    // and backed out to aim at a new target on the new page), flush stale cache
                    // and start a fresh scan immediately.
                    if (state.path.length === 0 && state.lastClickTime > 0) {
                        _activeScanId++;
                        _cachedElements = [];
                        fetchElementsAsync(null, state.targetPid);
                    }

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
                        const dropTarget = getScreenCoordinates(state, window);
                        sendEvent({ event: 'move', x: dropTarget.x, y: dropTarget.y });
                        sendEvent({ event: 'release', button: 'left' });
                        state.dragging = false;
                        dismissOverlay(window, application);
                        return true;
                    }
                    emitSelection(state, isShift ? 'right-click' : 'click', window);
                    dismissOverlay(window, application);
                    return true;
                }

                // Click and stay: click and STAY on the targeted element!
                if (char === 'c') {
                    const target = getScreenCoordinates(state, window);
                    // Capture link flag BEFORE clearing state — nav links (GitHub tabs etc.)
                    // trigger SPA page transitions that take ~500–800ms to settle in the DOM.
                    const wasNavLink = Boolean(state.snappedElement && state.snappedElement.is_link);

                    window.set_visible(false);
                    sendEvent({
                        event: 'click',
                        button: isShift ? 'right' : 'left',
                        normalized: { x: target.x, y: target.y },
                    });

                    state.lastClickTime = GLib.get_monotonic_time();

                    // Invalidate stale element cache since the click may mutate/destroy UI elements
                    _activeScanId++;
                    _cachedElements = [];
                    _cacheTimestamp = 0;
                    state.snappedElement = null;
                    state.snapCandidates = [];
                    state.snapIndex = -1;

                    // Re-present overlay immediately after click completes (~60ms),
                    // KEEPING all state (coordinates, micro-cell, reticle lock) completely locked!
                    GLib.timeout_add(GLib.PRIORITY_DEFAULT, 60, () => {
                        window.set_visible(true);
                        window.present();
                        drawingArea.grab_focus();
                        drawingArea.queue_draw();
                        startRippleAnimation(drawingArea, state);

                        // For nav-link clicks (SPA page transitions), delay the rescan by an
                        // additional 650ms so the browser has time to render the new page before
                        // AT-SPI walks the DOM. Button clicks keep the fast path.
                        const rescanDelay = wasNavLink ? 650 : 150;
                        GLib.timeout_add(GLib.PRIORITY_DEFAULT, rescanDelay, () => {
                            fetchElementsAsync((elements) => {
                                if (elements && elements.length > 0) {
                                    if (state.path.length >= MAX_DEPTH && state.point) {
                                        snapToNearestElement(state, window, drawingArea);
                                    }
                                }
                            }, state.targetPid);
                            return GLib.SOURCE_REMOVE;
                        });

                        return GLib.SOURCE_REMOVE;
                    });
                    return true;
                }

                if (char === 'r') {
                    emitSelection(state, 'right-click', window);
                    dismissOverlay(window, application);
                    return true;
                }
                if (char === 'd') {
                    emitSelection(state, 'double-click', window);
                    dismissOverlay(window, application);
                    return true;
                }
                if (char === 'm') {
                    emitSelection(state, 'middle-click', window);
                    dismissOverlay(window, application);
                    return true;
                }

                // Tab / Shift+Tab: cycle through snappable elements near current point
                if (keyval === Gdk.KEY_Tab || keyval === Gdk.KEY_ISO_Left_Tab) {
                    if (!state.snapCandidates || state.snapCandidates.length === 0) {
                        snapToNearestElement(state, window, drawingArea);
                    }
                    if (state.snapCandidates && state.snapCandidates.length > 0) {
                        cycleSnap(state, window, drawingArea, isShift ? -1 : 1);
                    }
                    return true;
                }

                // Drag Mode (Two-Phase Drag and Drop)
                if (char === 'v') {
                    if (!state.dragging) {
                        state.dragging = true;
                        const target = getScreenCoordinates(state, window);
                        state.dragStartPoint = target;
                        sendEvent({ event: 'move', x: target.x, y: target.y });
                        sendEvent({ event: 'press', button: 'left' });

                        state.path = [];
                        state.history = [];
                        state.rect = { x: 0, y: 0, width: 1, height: 1 };
                        state.point = null;
                        drawingArea.queue_draw();
                        return true;
                    } else {
                        const dropTarget = getScreenCoordinates(state, window);
                        sendEvent({ event: 'move', x: dropTarget.x, y: dropTarget.y });
                        sendEvent({ event: 'release', button: 'left' });
                        state.dragging = false;
                        dismissOverlay(window, application);
                        return true;
                    }
                }

                // Scroll Mode (Continuous Interactive Kinetic Scroll)
                if (char === 's' || char === 'w') {
                    const isUp = char === 'w';
                    state.mode = 'scroll';
                    state.lastScrollDir = isUp ? 'up' : 'down';

                    // 1. Capture screen target before unmaximizing
                    const target = getScreenCoordinates(state, window);

                    // 2. Collision avoidance with centered HUD (560x48)
                    let scrollX = target.x;
                    let scrollY = target.y;
                    if (Math.abs(scrollX - 0.5) < 0.16 && Math.abs(scrollY - 0.5) < 0.05) {
                        scrollY = scrollY >= 0.5 ? 0.56 : 0.44;
                    }
                    state.scrollTarget = { x: scrollX, y: scrollY };

                    // 3. Unmaximize window to compact HUD dimensions so underlying window gets pointer focus
                    window.unmaximize();
                    window.set_default_size(560, 48);

                    // 4. Prime pointer position immediately
                    sendEvent({ event: 'move', x: scrollX, y: scrollY });

                    // 5. Emit initial scroll step immediately
                    const mult = isShift ? 3 : 1;
                    sendEvent({
                        event: 'scroll',
                        dx: 0,
                        dy: isUp ? state.scrollSpeed * mult : -state.scrollSpeed * mult,
                    });

                    drawingArea.queue_draw();

                    // Re-dispatch move after Mutter commits unmaximize configure to guarantee wl_pointer.enter
                    // reaches the target window before user begins scrolling via j/k.
                    GLib.timeout_add(GLib.PRIORITY_DEFAULT, 40, () => {
                        if (state.mode === 'scroll' && state.scrollTarget) {
                            sendEvent({ event: 'move', x: state.scrollTarget.x, y: state.scrollTarget.y });
                            state.scrollFocused = true;
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

                let minY = 0;
                const monitor = getActiveMonitor(window);
                if (monitor) {
                    const geom = monitor.get_geometry();
                    const winH = window.get_height();
                    minY = -(Math.max(0, geom.height - winH) / winH);
                }

                let moved = false;
                if (char === 'h' || keyval === Gdk.KEY_Left) {
                    state.point.x = Math.max(0, state.point.x - step);
                    moved = true;
                } else if (char === 'l' || keyval === Gdk.KEY_Right) {
                    state.point.x = Math.min(1, state.point.x + step);
                    moved = true;
                } else if (char === 'k' || keyval === Gdk.KEY_Up) {
                    state.point.y = Math.max(minY, state.point.y - step);
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
                    const dropTarget = getScreenCoordinates(state, window);
                    sendEvent({ event: 'move', x: dropTarget.x, y: dropTarget.y });
                    sendEvent({ event: 'release', button: 'left' });
                    state.dragging = false;
                    dismissOverlay(window, application);
                    return true;
                }
                emitSelection(state, isShift ? 'right-click' : 'click', window);
                dismissOverlay(window, application);
                return true;
            }

            // Stroke 1 Top Bar Mode shortcut
            if (state.path.length === 0 && (char === 't' || keyval === Gdk.KEY_grave)) {
                state.mode = 'topbar';
                state.topBarTarget = null;
                state.point = { x: 0.973, y: -0.010 };
                drawingArea.queue_draw();
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

            // If Stroke 1 was just entered, reset snap state
            if (state.path.length === 1) {
                state.snappedElement = null;
                state.snapCandidates = [];
                state.snapIndex = -1;
            }

            // If 2nd stroke was just entered, initialize target lock point and attempt magnetic snap!
            if (state.path.length >= MAX_DEPTH) {
                const centerNormX = state.rect.x + state.rect.width / 2;
                const centerNormY = state.rect.y + state.rect.height / 2;
                state.point = { x: centerNormX, y: centerNormY };
                state.snappedElement = null;
                state.snapCandidates = [];
                state.snapIndex = -1;

                snapToNearestElement(state, window, drawingArea);
            }

            drawingArea.queue_draw();
            return true;
        });

        window.add_controller(keyboard);
        window.set_child(drawingArea);

        const isBackground = ARGV.includes('--background') || ARGV.includes('--daemon');
        _isResident = isBackground || ARGV.includes('--resident') || GLib.getenv('NOTMOUSE_RESIDENT') === '1';

        if (_isResident) {
            setupOverlaySocket(state, window, drawingArea, application);
        }

        if (isBackground) {
            // Pre-warm Wayland surface & GTK pipeline invisibly
            window.set_opacity(0);
            window.present();
            GLib.idle_add(GLib.PRIORITY_HIGH, () => {
                window.set_visible(false);
                window.set_opacity(1);
                return GLib.SOURCE_REMOVE;
            });
        } else {
            const startEnv = GLib.getenv('NOTMOUSE_START_US');
            const startUs = startEnv ? parseInt(startEnv, 10) : null;
            showOverlay(state, window, drawingArea, startUs);
        }

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
