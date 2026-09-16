use atspi::connection::AccessibilityConnection;
use atspi::proxy::accessible::AccessibleProxy;
use atspi::proxy::component::ComponentProxy;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub async fn test_scan() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let scanner = Scanner::new().await?;
    let elements = scanner.scan(None, None).await;
    println!("Scan found {} interactive elements:", elements.len());
    for (i, elem) in elements.iter().take(20).enumerate() {
        println!(
            "  #{i}: [{}] '{}' at ({:.0}, {:.0}) size {:.0}x{:.0} (app: {})",
            elem.role, elem.name, elem.cx, elem.cy, elem.w, elem.h, elem.app_name
        );
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessibleElement {
    pub name: String,
    pub role: String,
    pub is_link: bool,
    pub app_name: String,
    pub app_pid: u32,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub cx: f64,
    pub cy: f64,
}

impl rstar::RTreeObject for AccessibleElement {
    type Envelope = rstar::AABB<[f64; 2]>;

    fn envelope(&self) -> Self::Envelope {
        rstar::AABB::from_corners([self.x, self.y], [self.x + self.w, self.y + self.h])
    }
}

impl rstar::PointDistance for AccessibleElement {
    fn distance_2(&self, point: &[f64; 2]) -> f64 {
        let dx = self.cx - point[0];
        let dy = self.cy - point[1];
        dx * dx + dy * dy
    }
}

#[derive(Debug, Clone)]
pub struct SnapResult {
    pub candidate: Option<AccessibleElement>,
    pub all_candidates: Vec<AccessibleElement>,
    pub index: usize,
}

const CONTROL_ROLES: &[&str] = &[
    "push button",
    "toggle button",
    "check box",
    "radio button",
    "page tab",
    "menu item",
    "check menu item",
    "radio menu item",
    "entry",
    "password text",
    "combo box",
    "button",
    "table cell",
    "list item",
    "tree item",
];

const IGNORED_APPS: &[&str] = &[
    "gjs",
    "ibus-extension-gtk3",
    "evolution-alarm-notify",
    "update-notifier",
    "gpaste-daemon",
    "xdg-desktop-portal-gtk",
    "gnome-shell",
    "!mouse",
];

pub struct Scanner {
    conn: Arc<AccessibilityConnection>,
}

impl Scanner {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let conn = AccessibilityConnection::new().await?;
        Ok(Self {
            conn: Arc::new(conn),
        })
    }

    pub async fn scan(
        &self,
        target_pid: Option<u32>,
        bounds: Option<(f64, f64, f64, f64)>,
    ) -> Vec<AccessibleElement> {
        let root = match AccessibleProxy::builder(self.conn.connection())
            .destination("org.a11y.atspi.Registry")
            .ok()
            .and_then(|b| b.path("/org/a11y/atspi/accessible/root").ok())
        {
            Some(b) => match b.build().await {
                Ok(p) => p,
                Err(_) => return Vec::new(),
            },
            None => return Vec::new(),
        };

        let Ok(children) = root.get_children().await else {
            return Vec::new();
        };

        // Find primary app
        let mut target_app = None;
        for child in children {
            let Some(name) = child.name() else {
                continue;
            };
            let bus_name = name.as_str();
            let path = child.path().as_str();

            let Some(app_builder) = AccessibleProxy::builder(self.conn.connection())
                .destination(bus_name)
                .ok()
                .and_then(|b| b.path(path).ok())
            else {
                continue;
            };

            let Ok(app_proxy) = app_builder.build().await else {
                continue;
            };
            let app_name = app_proxy.name().await.unwrap_or_default();
            if IGNORED_APPS
                .iter()
                .any(|&ig| app_name.eq_ignore_ascii_case(ig))
            {
                continue;
            }

            // Check PID if target_pid was provided
            if let Some(pid) = target_pid {
                // If we match PID, this is strictly our target app
                let Ok(app_children) = app_proxy.get_children().await else {
                    continue;
                };
                if !app_children.is_empty() {
                    target_app = Some((app_name, bus_name.to_string(), app_children, pid));
                    break;
                }
            } else {
                // Fallback: choose first visible app with windows
                let Ok(app_children) = app_proxy.get_children().await else {
                    continue;
                };
                if !app_children.is_empty() {
                    target_app = Some((app_name, bus_name.to_string(), app_children, 0));
                    break;
                }
            }
        }

        let Some((app_name, bus_name, window_refs, app_pid)) = target_app else {
            return Vec::new();
        };

        let mut all_elements = Vec::new();

        for win_ref in window_refs {
            let win_path = win_ref.path().as_str();
            let mut elements = Vec::new();
            self.walk_node(
                &bus_name,
                win_path,
                &app_name,
                app_pid,
                0,
                bounds,
                &mut elements,
            )
            .await;
            all_elements.extend(elements);
        }

        // Deduplicate elements whose centers are within 6px
        let mut deduped: Vec<AccessibleElement> = Vec::with_capacity(all_elements.len());
        for elem in all_elements {
            let mut duplicate = false;
            for existing in &deduped {
                if (elem.cx - existing.cx).abs() <= 6.0 && (elem.cy - existing.cy).abs() <= 6.0 {
                    duplicate = true;
                    break;
                }
            }
            if !duplicate {
                deduped.push(elem);
            }
        }

        deduped
    }

    #[async_recursion::async_recursion]
    async fn walk_node(
        &self,
        bus_name: &str,
        path: &str,
        app_name: &str,
        app_pid: u32,
        depth: usize,
        bounds: Option<(f64, f64, f64, f64)>,
        results: &mut Vec<AccessibleElement>,
    ) {
        if depth > 30 || results.len() > 1500 {
            return;
        }

        let Some(node_builder) = AccessibleProxy::builder(self.conn.connection())
            .destination(bus_name)
            .ok()
            .and_then(|b| b.path(path).ok())
        else {
            return;
        };

        let Ok(node) = node_builder.build().await else {
            return;
        };

        // Check extents
        let mut node_bounds = None;
        if let Some(comp_builder) = ComponentProxy::builder(self.conn.connection())
            .destination(bus_name)
            .ok()
            .and_then(|b| b.path(path).ok())
            && let Ok(comp) = comp_builder.build().await
            && let Ok((x, y, w, h)) = comp.get_extents(atspi::CoordType::Screen).await
            && w > 0
            && h > 0
        {
            node_bounds = Some((f64::from(x), f64::from(y), f64::from(w), f64::from(h)));
        }

        // Subtree spatial pruning: if bounds are specified and node is disjoint from target region at depth >= 2
        if let (Some((nx, ny, nw, nh)), Some((bx1, bx2, by1, by2))) = (node_bounds, bounds)
            && depth >= 2
        {
            let disjoint = nx > bx2 || (nx + nw) < bx1 || ny > by2 || (ny + nh) < by1;
            if disjoint {
                return;
            }
        }

        // Check role and actionable attributes
        let role = node.get_role_name().await.unwrap_or_default();
        let is_control = CONTROL_ROLES.iter().any(|&r| role.eq_ignore_ascii_case(r));
        let is_link = role.eq_ignore_ascii_case("link");

        if let Some((x, y, w, h)) = node_bounds
            && ((is_control && w >= 10.0 && h >= 10.0) || (is_link && w >= 18.0 && h >= 12.0))
        {
            let cx = x + w / 2.0;
            let cy = y + h / 2.0;

            // Check bounds filter
            let in_bounds = if let Some((bx1, bx2, by1, by2)) = bounds {
                cx >= bx1 && cx <= bx2 && cy >= by1 && cy <= by2
            } else {
                true
            };

            if in_bounds {
                let name = node.name().await.unwrap_or_default();
                results.push(AccessibleElement {
                    name,
                    role: role.clone(),
                    is_link,
                    app_name: app_name.to_string(),
                    app_pid,
                    x,
                    y,
                    w,
                    h,
                    cx,
                    cy,
                });
            }
        }

        // Recurse into children
        if let Ok(children) = node.get_children().await {
            for child in children {
                let child_path = child.path().as_str();
                self.walk_node(
                    bus_name,
                    child_path,
                    app_name,
                    app_pid,
                    depth + 1,
                    bounds,
                    results,
                )
                .await;
            }
        }
    }
}

/// Snap to nearest element inside micro cell with reading-order sorting
pub fn snap_to_nearest(
    candidates: &[AccessibleElement],
    target_px_x: f64,
    target_px_y: f64,
    cell_min_x: f64,
    cell_max_x: f64,
    cell_min_y: f64,
    cell_max_y: f64,
) -> SnapResult {
    let tolerance = 4.0;
    let min_x = cell_min_x - tolerance;
    let max_x = cell_max_x + tolerance;
    let min_y = cell_min_y - tolerance;
    let max_y = cell_max_y + tolerance;

    let mut in_cell: Vec<AccessibleElement> = Vec::new();
    for e in candidates {
        let center_in_cell = e.cx >= min_x && e.cx <= max_x && e.cy >= min_y && e.cy <= max_y;
        let intersects_cell =
            e.x <= max_x && (e.x + e.w) >= min_x && e.y <= max_y && (e.y + e.h) >= min_y;

        if center_in_cell {
            in_cell.push(e.clone());
        } else if intersects_cell {
            // For wide elements spanning cells, clamp center to cell portion
            let inter_left = e.x.max(min_x);
            let inter_right = (e.x + e.w).min(max_x);
            let inter_top = e.y.max(min_y);
            let inter_bottom = (e.y + e.h).min(max_y);
            let mut clamped = e.clone();
            clamped.cx = f64::midpoint(inter_left, inter_right);
            clamped.cy = f64::midpoint(inter_top, inter_bottom);
            in_cell.push(clamped);
        }
    }

    if in_cell.is_empty() {
        return SnapResult {
            candidate: None,
            all_candidates: Vec::new(),
            index: 0,
        };
    }

    // Filter to primary app in the cell
    let primary_app = in_cell[0].app_name.clone();
    in_cell.retain(|e| e.app_name == primary_app);

    // Reading-order sort with adaptive row banding (14px)
    const ROW_BAND_PX: f64 = 14.0;
    in_cell.sort_by(|a, b| {
        if (a.cy - b.cy).abs() <= ROW_BAND_PX {
            let a_link = i32::from(a.is_link);
            let b_link = i32::from(b.is_link);
            if a_link != b_link {
                return a_link.cmp(&b_link);
            }
            a.cx.partial_cmp(&b.cx).unwrap_or(std::cmp::Ordering::Equal)
        } else {
            a.cy.partial_cmp(&b.cy).unwrap_or(std::cmp::Ordering::Equal)
        }
    });

    // Auto-snap to candidate nearest to the initial reticle
    let mut best_idx = 0;
    let mut best_dist = f64::INFINITY;
    for (i, elem) in in_cell.iter().enumerate() {
        let dx = elem.cx - target_px_x;
        let dy = elem.cy - target_px_y;
        let d = dx * dx + dy * dy;
        if d < best_dist {
            best_dist = d;
            best_idx = i;
        }
    }

    let candidate = in_cell.get(best_idx).cloned();
    SnapResult {
        candidate,
        all_candidates: in_cell,
        index: best_idx,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snap_to_nearest_reading_order() {
        let e1 = AccessibleElement {
            name: "Button B".into(),
            role: "push button".into(),
            is_link: false,
            app_name: "App".into(),
            app_pid: 100,
            x: 200.0,
            y: 100.0,
            w: 50.0,
            h: 30.0,
            cx: 225.0,
            cy: 115.0,
        };
        let e2 = AccessibleElement {
            name: "Button A".into(),
            role: "push button".into(),
            is_link: false,
            app_name: "App".into(),
            app_pid: 100,
            x: 50.0,
            y: 100.0,
            w: 50.0,
            h: 30.0,
            cx: 75.0,
            cy: 115.0,
        };
        let e_link = AccessibleElement {
            name: "Link".into(),
            role: "link".into(),
            is_link: true,
            app_name: "App".into(),
            app_pid: 100,
            x: 30.0,
            y: 100.0,
            w: 40.0,
            h: 30.0,
            cx: 50.0,
            cy: 115.0,
        };

        let candidates = vec![e1, e2, e_link];
        let res = snap_to_nearest(&candidates, 220.0, 115.0, 0.0, 400.0, 0.0, 300.0);
        assert_eq!(res.all_candidates.len(), 3);
        // Controls come before links within same row!
        assert!(!res.all_candidates[0].is_link);
        assert!(!res.all_candidates[1].is_link);
        assert!(res.all_candidates[2].is_link);
        // Within controls, sorted left-to-right (cx 75 before cx 225)
        assert_eq!(res.all_candidates[0].name, "Button A");
        assert_eq!(res.all_candidates[1].name, "Button B");
    }

    #[test]
    fn test_snap_to_nearest_empty() {
        let res = snap_to_nearest(&[], 100.0, 100.0, 0.0, 200.0, 0.0, 200.0);
        assert!(res.candidate.is_none());
        assert!(res.all_candidates.is_empty());
    }
}
