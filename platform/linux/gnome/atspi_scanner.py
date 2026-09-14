#!/usr/bin/env python3
"""
atspi_scanner.py — fast background accessibility scanner for !mouse

Scans the ACTIVE (focused) desktop window for clickable/actionable elements
(buttons, tabs, entries, links, menus) and outputs JSON.
Can filter by a bounding box (min_x, max_x, min_y, max_y).
"""
import sys
import json


def scan(min_x=None, max_x=None, min_y=None, max_y=None):
    try:
        import gi
        gi.require_version('Atspi', '2.0')
        from gi.repository import Atspi
    except Exception:
        return []

    try:
        Atspi.init()
        desktop = Atspi.get_desktop(0)
    except Exception:
        return []

    elements = []
    ACTIONABLE = {
        'push button', 'toggle button', 'check box', 'radio button',
        'page tab', 'menu item', 'check menu item', 'radio menu item',
        'entry', 'password text', 'combo box', 'link', 'button', 'menu',
    }

    # Collect top-level frames that have ACTIVE state (= focused window).
    # We walk ONLY those frames so we never snap to buttons from background windows.
    active_frames = []
    count = desktop.get_child_count()
    for i in range(count):
        try:
            app = desktop.get_child_at_index(i)
            if not app:
                continue
            name = app.get_name()
            if name in ('gjs', 'ibus-extension-gtk3', 'evolution-alarm-notify'):
                continue
            for j in range(app.get_child_count()):
                try:
                    frame = app.get_child_at_index(j)
                    if not frame:
                        continue
                    state_set = frame.get_state_set()
                    if state_set and state_set.contains(Atspi.StateType.ACTIVE):
                        active_frames.append(frame)
                except Exception:
                    continue
        except Exception:
            continue

    # Fallback: if no ACTIVE frame found, scan everything visible so the
    # overlay degrades gracefully instead of returning nothing.
    if not active_frames:
        for i in range(count):
            try:
                app = desktop.get_child_at_index(i)
                if not app:
                    continue
                name = app.get_name()
                if name in ('gjs', 'ibus-extension-gtk3', 'evolution-alarm-notify'):
                    continue
                for j in range(app.get_child_count()):
                    try:
                        frame = app.get_child_at_index(j)
                        if frame:
                            active_frames.append(frame)
                    except Exception:
                        continue
            except Exception:
                continue

    def walk(node, depth=0):
        if depth > 25:
            return
        try:
            comp = node.get_component_iface()
            if comp:
                b = comp.get_extents(Atspi.CoordType.WINDOW)
                if b.width > 0 and b.height > 0:
                    # Spatial bounding filter: prune subtrees completely outside
                    if min_x is not None and b.x + b.width < min_x:
                        return
                    if max_x is not None and b.x > max_x:
                        return
                    if min_y is not None and b.y + b.height < min_y:
                        return
                    if max_y is not None and b.y > max_y:
                        return

                    role = node.get_role_name()
                    if role in ACTIONABLE and b.width >= 10 and b.height >= 10:
                        cx = b.x + b.width / 2.0
                        cy = b.y + b.height / 2.0
                        elements.append({
                            'name': node.get_name(),
                            'role': role,
                            'x': b.x,
                            'y': b.y,
                            'w': b.width,
                            'h': b.height,
                            'cx': cx,
                            'cy': cy,
                        })

            c_count = node.get_child_count()
            for c in range(c_count):
                walk(node.get_child_at_index(c), depth + 1)
        except Exception:
            pass

    for frame in active_frames:
        walk(frame)

    return elements


if __name__ == '__main__':
    min_x = float(sys.argv[1]) if len(sys.argv) > 1 else None
    max_x = float(sys.argv[2]) if len(sys.argv) > 2 else None
    min_y = float(sys.argv[3]) if len(sys.argv) > 3 else None
    max_y = float(sys.argv[4]) if len(sys.argv) > 4 else None

    res = scan(min_x, max_x, min_y, max_y)
    json.dump(res, sys.stdout)
