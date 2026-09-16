use std::collections::HashSet;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct FreeRoamKinematics {
    pub norm_x: f64,
    pub norm_y: f64,
    pub active_directions: HashSet<Direction>,
    pub press_start_time: Option<Instant>,
    pub is_turbo: bool,
    pub is_crawl: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

impl FreeRoamKinematics {
    pub fn new(initial_norm_x: f64, initial_norm_y: f64) -> Self {
        Self {
            norm_x: initial_norm_x.clamp(0.0, 1.0),
            norm_y: initial_norm_y.clamp(0.0, 1.0),
            active_directions: HashSet::new(),
            press_start_time: None,
            is_turbo: false,
            is_crawl: false,
        }
    }

    pub fn press_direction(&mut self, dir: Direction, screen_w: f64, screen_h: f64) -> (f64, f64) {
        let is_new = self.active_directions.insert(dir);
        if is_new && self.active_directions.len() == 1 {
            self.press_start_time = Some(Instant::now());
            // Tap micro-step: instantly shift 2.5px for crisp single-tap response
            let tap_px = if self.is_crawl {
                1.0
            } else if self.is_turbo {
                5.0
            } else {
                2.5
            };
            match dir {
                Direction::Left => self.norm_x -= tap_px / screen_w,
                Direction::Right => self.norm_x += tap_px / screen_w,
                Direction::Up => self.norm_y -= tap_px / screen_h,
                Direction::Down => self.norm_y += tap_px / screen_h,
            }
            self.norm_x = self.norm_x.clamp(0.0, 1.0);
            self.norm_y = self.norm_y.clamp(0.0, 1.0);
        }
        (self.norm_x, self.norm_y)
    }

    pub fn release_direction(&mut self, dir: Direction) {
        self.active_directions.remove(&dir);
        if self.active_directions.is_empty() {
            // Instant freeze on release!
            self.press_start_time = None;
        } else {
            // If other directions are still held, reset timer to current moment so direction changes start smooth
            self.press_start_time = Some(Instant::now());
        }
    }

    pub fn set_modifiers(&mut self, turbo: bool, crawl: bool) {
        self.is_turbo = turbo;
        self.is_crawl = crawl;
    }

    /// Advance physics by dt seconds (typically 0.016s at 60Hz).
    /// Returns `Some((norm_x`, `norm_y`)) if moved, or None if stationary.
    pub fn update(&mut self, dt: f64, screen_w: f64, screen_h: f64) -> Option<(f64, f64)> {
        if self.active_directions.is_empty() {
            return None;
        }

        let mut dir_x: f64 = 0.0;
        let mut dir_y: f64 = 0.0;

        for dir in &self.active_directions {
            match dir {
                Direction::Left => dir_x -= 1.0,
                Direction::Right => dir_x += 1.0,
                Direction::Up => dir_y -= 1.0,
                Direction::Down => dir_y += 1.0,
            }
        }

        if dir_x == 0.0 && dir_y == 0.0 {
            return None;
        }

        // Normalize vector for diagonal movement
        let mag = (dir_x * dir_x + dir_y * dir_y).sqrt();
        let norm_dx = dir_x / mag;
        let norm_dy = dir_y / mag;

        // Kinematic acceleration curve:
        // v(t) = v_min + (v_max - v_min) * (t / ramp_time)^2
        let speed_px_s = if self.is_crawl {
            45.0 // ~0.75 px/frame: pixel-perfect crawl
        } else {
            let elapsed = self
                .press_start_time
                .map_or(0.0, |t| t.elapsed().as_secs_f64());
            const V_MIN: f64 = 150.0;
            const V_MAX: f64 = 2400.0;
            const RAMP_TIME: f64 = 0.55;

            let tau = (elapsed / RAMP_TIME).min(1.0);
            let curve = tau * tau; // Quadratic acceleration
            let base = V_MIN + (V_MAX - V_MIN) * curve;

            if self.is_turbo { base * 2.5 } else { base }
        };

        let move_px_x = norm_dx * speed_px_s * dt;
        let move_px_y = norm_dy * speed_px_s * dt;

        self.norm_x = (self.norm_x + move_px_x / screen_w).clamp(0.0, 1.0);
        self.norm_y = (self.norm_y + move_px_y / screen_h).clamp(0.0, 1.0);

        Some((self.norm_x, self.norm_y))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_free_roam_tap_micro_step() {
        let mut kin = FreeRoamKinematics::new(0.5, 0.5);
        let (nx, _ny) = kin.press_direction(Direction::Left, 1000.0, 1000.0);
        // Tap micro step shifts by 2.5px on 1000px screen = 0.0025
        assert!((nx - 0.4975).abs() < 1e-6);
    }

    #[test]
    fn test_free_roam_instant_freeze_on_release() {
        let mut kin = FreeRoamKinematics::new(0.5, 0.5);
        kin.press_direction(Direction::Right, 1000.0, 1000.0);
        let _ = kin.update(0.1, 1000.0, 1000.0);
        assert!(!kin.active_directions.is_empty());

        kin.release_direction(Direction::Right);
        assert!(kin.active_directions.is_empty());
        assert!(kin.press_start_time.is_none());

        // Update after release returns None (instant freeze!)
        let res = kin.update(0.1, 1000.0, 1000.0);
        assert!(res.is_none());
    }

    #[test]
    fn test_free_roam_turbo_crawl_modifiers() {
        let mut kin = FreeRoamKinematics::new(0.5, 0.5);
        kin.set_modifiers(true, false);
        assert!(kin.is_turbo);
        assert!(!kin.is_crawl);

        kin.set_modifiers(false, true);
        assert!(!kin.is_turbo);
        assert!(kin.is_crawl);
    }
}
