#!/usr/bin/env python3
"""
atspi_scanner.py — fast background accessibility scanner for !mouse

Scans the ACTIVE (focused) desktop window for clickable/actionable elements
(buttons, tabs, entries, links, menus) and outputs JSON.
Can filter by a bounding box (min_x, max_x, min_y, max_y).
"""
import sys
import json


def scan(min_x=None, max_x=None, min_y=None, max_y=None, target_pid=None):
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
    # Roles that are unambiguously interactive controls.
    CONTROL_ROLES = {
        'push button', 'toggle button', 'check box', 'radio button',
        'page tab', 'menu item', 'check menu item', 'radio menu item',
        'entry', 'password text', 'combo box', 'button',
    }
    # 'link' is included separately because navigation links (GitHub tabs, breadcrumbs,
    # sidebar menus) behave like buttons but are exposed as anchor elements.
    # We apply stricter heuristics to prevent inline article text links from flooding
    # the candidate list.
    LINK_ROLE = 'link'
    # Minimum bounding-box size to accept a link as a snap candidate.
    # Inline word-level hyperlinks are typically narrow (< 16 px tall); navigation
    # tabs and buttons tend to be at least 16 px tall and 20 px wide.
    LINK_MIN_W = 20
    LINK_MIN_H = 14


    active_frames = []
    active_app_name = ''
    active_app_pid = 0
    count = desktop.get_child_count()

    IGNORED_APPS = {
        'gjs', 'ibus-extension-gtk3', 'evolution-alarm-notify',
        'update-notifier', 'gpaste-daemon', 'xdg-desktop-portal-gtk',
        'gnome-shell',
    }

    # Fast path: if target_pid is specified, match the exact process directly
    if target_pid is not None:
        for i in range(count):
            try:
                app = desktop.get_child_at_index(i)
                if not app:
                    continue
                if app.get_process_id() == target_pid:
                    active_app_name = app.get_name() or ''
                    active_app_pid = target_pid
                    for j in range(app.get_child_count()):
                        frame = app.get_child_at_index(j)
                        if frame:
                            active_frames.append(frame)
                    break
            except Exception:
                continue

    # If no target_pid or PID match yielded no frames, rank apps to pick strictly
    # ONE active foreground application. Never merge frames from multiple apps.
    if not active_frames:
        candidate_apps = []
        for i in range(count):
            try:
                app = desktop.get_child_at_index(i)
                if not app:
                    continue
                name = app.get_name()
                if not name or name in IGNORED_APPS:
                    continue
                pid = app.get_process_id()

                app_frames = []
                has_active_frame = False
                has_focused_node = False
                contains_target = False

                for j in range(app.get_child_count()):
                    try:
                        frame = app.get_child_at_index(j)
                        if not frame:
                            continue
                        ss = frame.get_state_set()
                        if not ss:
                            continue
                        if ss.contains(Atspi.StateType.ICONIFIED):
                            continue
                        if not (ss.contains(Atspi.StateType.SHOWING) or ss.contains(Atspi.StateType.VISIBLE)):
                            continue

                        comp = frame.get_component_iface()
                        if comp:
                            b = comp.get_extents(Atspi.CoordType.SCREEN)
                            if b.width < 50 or b.height < 50:
                                continue
                            if min_x is not None and min_y is not None:
                                if b.x <= min_x <= b.x + b.width and b.y <= min_y <= b.y + b.height:
                                    contains_target = True

                        if ss.contains(Atspi.StateType.ACTIVE):
                            has_active_frame = True

                        # Shallow search for focused descendant (< 6 levels, max 20 children)
                        focused_found = []
                        def check_focus(node, depth=0):
                            if focused_found or depth > 5:
                                return
                            nss = node.get_state_set()
                            if nss and nss.contains(Atspi.StateType.FOCUSED):
                                focused_found.append(True)
                                return
                            for c in range(min(node.get_child_count(), 20)):
                                check_focus(node.get_child_at_index(c), depth + 1)

                        check_focus(frame)
                        if focused_found:
                            has_focused_node = True

                        app_frames.append(frame)
                    except Exception:
                        continue

                if app_frames:
                    score = 0
                    if has_focused_node:
                        score += 150
                    if has_active_frame:
                        score += 100
                    if contains_target:
                        score += 50
                    score += i

                    candidate_apps.append({
                        'name': name,
                        'pid': pid,
                        'frames': app_frames,
                        'score': score,
                    })
            except Exception:
                continue

        if candidate_apps:
            candidate_apps.sort(key=lambda c: c['score'], reverse=True)
            chosen = candidate_apps[0]
            active_app_name = chosen['name']
            active_app_pid = chosen['pid']
            active_frames = chosen['frames']

    # Get frame dimensions and global screen coordinates so we can accurately
    # place elements across multi-monitor or offset windows.
    frame_bounds = {}  # frame id → (x, y, w, h)
    for frame in active_frames:
        try:
            comp = frame.get_component_iface()
            if comp:
                b = comp.get_extents(Atspi.CoordType.SCREEN)
                frame_bounds[id(frame)] = (b.x, b.y, b.width, b.height)
        except Exception:
            pass

    def walk(node, depth=0, frame_x=0, frame_y=0, frame_w=99999, frame_h=99999):
        if depth > 25:
            return
        try:
            comp = node.get_component_iface()
            if comp:
                b = comp.get_extents(Atspi.CoordType.WINDOW)
                if b.width > 0 and b.height > 0:
                    # Prune subtrees completely outside the window's visible area
                    if b.x >= frame_w or b.y >= frame_h:
                        return
                    if b.x + b.width <= 0 or b.y + b.height <= 0:
                        return

                    # Compute global screen coordinates
                    sx = frame_x + b.x
                    sy = frame_y + b.y
                    cx = sx + b.width / 2.0
                    cy = sy + b.height / 2.0

                    role = node.get_role_name()
                    is_control = role in CONTROL_ROLES and b.width >= 10 and b.height >= 10
                    is_link = (role == LINK_ROLE
                                and b.width >= LINK_MIN_W
                                and b.height >= LINK_MIN_H)

                    if is_control or is_link:
                        # Must be SHOWING — element and all ancestors are actually rendered
                        ss = node.get_state_set()
                        if ss and ss.contains(Atspi.StateType.SHOWING):
                            # Center must fall inside the visible frame and requested bounds
                            if 0 <= b.x < frame_w and 0 <= b.y < frame_h:
                                if min_x is not None and not (min_x <= cx <= max_x):
                                    pass
                                elif min_y is not None and not (min_y <= cy <= max_y):
                                    pass
                                else:
                                    elements.append({
                                        'name': node.get_name(),
                                        'role': role,
                                        'is_link': is_link and not is_control,
                                        'app_name': active_app_name,
                                        'app_pid': active_app_pid,
                                        'x': sx,
                                        'y': sy,
                                        'w': b.width,
                                        'h': b.height,
                                        'cx': cx,
                                        'cy': cy,
                                    })

            c_count = node.get_child_count()
            for c in range(c_count):
                walk(node.get_child_at_index(c), depth + 1, frame_x, frame_y, frame_w, frame_h)
        except Exception:
            pass

    for frame in active_frames:
        fx, fy, fw, fh = frame_bounds.get(id(frame), (0, 0, 99999, 99999))
        walk(frame, depth=0, frame_x=fx, frame_y=fy, frame_w=fw, frame_h=fh)

    # Spatial deduplication: if two elements have centers within 6px of each other
    # (e.g. AT-SPI reporting nested button/panel or label wrappers at the same spot),
    # keep only one.
    deduped = []
    for e in elements:
        duplicate = False
        for kept in deduped:
            if abs(e['cx'] - kept['cx']) <= 6 and abs(e['cy'] - kept['cy']) <= 6:
                duplicate = True
                break
        if not duplicate:
            deduped.append(e)
    elements = deduped

    return elements


if __name__ == '__main__':
    target_pid = None
    min_x = None
    max_x = None
    min_y = None
    max_y = None

    args = sys.argv[1:]
    i = 0
    pos_args = []
    while i < len(args):
        arg = args[i]
        if arg in ('--pid', '-p') and i + 1 < len(args):
            try:
                target_pid = int(args[i + 1])
            except ValueError:
                pass
            i += 2
        else:
            pos_args.append(arg)
            i += 1

    if len(pos_args) > 0:
        try:
            min_x = float(pos_args[0])
        except ValueError:
            pass
    if len(pos_args) > 1:
        try:
            max_x = float(pos_args[1])
        except ValueError:
            pass
    if len(pos_args) > 2:
        try:
            min_y = float(pos_args[2])
        except ValueError:
            pass
    if len(pos_args) > 3:
        try:
            max_y = float(pos_args[3])
        except ValueError:
            pass

    res = scan(min_x, max_x, min_y, max_y, target_pid=target_pid)
    json.dump(res, sys.stdout)
