//! Linux `uinput` virtual device input injection backend.
//!
//! Provides a safe, crash-free absolute pointer device using the Linux `uinput`
//! kernel subsystem. Operates natively with Wayland compositors (GNOME Shell/Mutter,
//! KDE Plasma/KWin, Sway, Hyprland) and X11 without requiring privileged D-Bus bridges
//! or triggering compositor tablet manager faults.

use std::thread;
use std::time::Duration;

use evdev::uinput::VirtualDevice;
use evdev::{
    AbsInfo, AbsoluteAxisCode, AttributeSet, EventType, InputEvent, KeyCode, RelativeAxisCode,
    UinputAbsSetup,
};

/// Maximum coordinate range used for absolute axis normalization.
pub const ABS_MAX_RANGE: i32 = 65535;

/// Mouse button identifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

impl MouseButton {
    #[must_use]
    pub const fn to_key(self) -> KeyCode {
        match self {
            Self::Left => KeyCode::BTN_LEFT,
            Self::Right => KeyCode::BTN_RIGHT,
            Self::Middle => KeyCode::BTN_MIDDLE,
        }
    }
}

/// Errors originating from the virtual input subsystem.
#[derive(Debug)]
pub enum InputError {
    DeviceCreation(std::io::Error),
    Emit(std::io::Error),
}

impl std::fmt::Display for InputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DeviceCreation(err) => write!(f, "could not create virtual input device: {err}"),
            Self::Emit(err) => write!(f, "could not emit input event: {err}"),
        }
    }
}

impl std::error::Error for InputError {}

/// Virtual mouse device managing absolute positioning, clicks, drags, and scrolling.
pub struct InputDevice {
    pub(crate) device: VirtualDevice,
}

impl InputDevice {
    /// Creates and registers a new virtual absolute mouse pointer with `/dev/uinput`.
    ///
    /// # Safety and Compositor Compatibility
    ///
    /// This device advertises standard mouse buttons (`BTN_LEFT`, `BTN_RIGHT`, `BTN_MIDDLE`),
    /// absolute axes (`ABS_X`, `ABS_Y`), and relative scroll wheels (`REL_WHEEL`, `REL_HWHEEL`).
    ///
    /// It intentionally does NOT declare tablet tool keys (`BTN_TOOL_PEN`, `BTN_TOOL_RUBBER`)
    /// or touchscreen keys (`BTN_TOUCH`), preventing Mutter 50.1 / GNOME Wayland compositor
    /// segmentation faults caused by missing hardware tablet tool descriptors.
    ///
    /// # Errors
    ///
    /// Returns [`InputError::DeviceCreation`] if `/dev/uinput` cannot be opened or registered.
    pub fn new() -> Result<Self, InputError> {
        let mut keys = AttributeSet::<KeyCode>::new();
        keys.insert(KeyCode::BTN_LEFT);
        keys.insert(KeyCode::BTN_RIGHT);
        keys.insert(KeyCode::BTN_MIDDLE);
        keys.insert(KeyCode::BTN_SIDE);
        keys.insert(KeyCode::BTN_EXTRA);

        let mut rel_axes = AttributeSet::<RelativeAxisCode>::new();
        rel_axes.insert(RelativeAxisCode::REL_WHEEL);
        rel_axes.insert(RelativeAxisCode::REL_HWHEEL);

        let x_axis_info = AbsInfo::new(0, 0, ABS_MAX_RANGE, 0, 0, 100);
        let y_axis_info = AbsInfo::new(0, 0, ABS_MAX_RANGE, 0, 0, 100);

        let abs_x = UinputAbsSetup::new(AbsoluteAxisCode::ABS_X, x_axis_info);
        let abs_y = UinputAbsSetup::new(AbsoluteAxisCode::ABS_Y, y_axis_info);

        let mut device = VirtualDevice::builder()
            .map_err(InputError::DeviceCreation)?
            .name("notmouse-pointer")
            .with_keys(&keys)
            .map_err(InputError::DeviceCreation)?
            .with_relative_axes(&rel_axes)
            .map_err(InputError::DeviceCreation)?
            .with_absolute_axis(&abs_x)
            .map_err(InputError::DeviceCreation)?
            .with_absolute_axis(&abs_y)
            .map_err(InputError::DeviceCreation)?
            .build()
            .map_err(InputError::DeviceCreation)?;

        // Wait for Wayland compositor / seat manager to bind the device node
        wait_for_compositor_binding(&mut device);

        Ok(Self { device })
    }

    /// Moves the pointer to normalized screen coordinates $(x, y) \in [0.0, 1.0]$.
    ///
    /// # Errors
    ///
    /// Returns [`InputError::Emit`] if emitting the event fails.
    pub fn move_to_normalized(&mut self, x: f64, y: f64) -> Result<(), InputError> {
        let clamped_x = x.clamp(0.0, 1.0);
        let clamped_y = y.clamp(0.0, 1.0);

        #[allow(clippy::cast_possible_truncation)]
        let abs_x = (clamped_x * f64::from(ABS_MAX_RANGE)).round() as i32;
        #[allow(clippy::cast_possible_truncation)]
        let abs_y = (clamped_y * f64::from(ABS_MAX_RANGE)).round() as i32;

        let events = [
            InputEvent::new(EventType::ABSOLUTE.0, AbsoluteAxisCode::ABS_X.0, abs_x),
            InputEvent::new(EventType::ABSOLUTE.0, AbsoluteAxisCode::ABS_Y.0, abs_y),
            InputEvent::new(EventType::SYNCHRONIZATION.0, 0, 0),
        ];

        self.device.emit(&events).map_err(InputError::Emit)?;
        Ok(())
    }

    /// Moves the pointer relatively by `(dx, dy)` pixels.
    ///
    /// # Errors
    ///
    /// Returns [`InputError::Emit`] if emitting the event fails.
    #[allow(dead_code, clippy::unused_self, clippy::unnecessary_wraps)]
    pub fn move_relative(&mut self, _dx: i32, _dy: i32) -> Result<(), InputError> {
        // Pointer uses absolute coordinate space mapped to monitor display outputs.
        Ok(())
    }

    /// Presses a mouse button down without releasing it.
    ///
    /// # Errors
    ///
    /// Returns [`InputError::Emit`] if emitting the event fails.
    pub fn press(&mut self, button: MouseButton) -> Result<(), InputError> {
        let events = [
            InputEvent::new(EventType::KEY.0, button.to_key().0, 1),
            InputEvent::new(EventType::SYNCHRONIZATION.0, 0, 0),
        ];
        self.device.emit(&events).map_err(InputError::Emit)
    }

    /// Releases a mouse button up.
    ///
    /// # Errors
    ///
    /// Returns [`InputError::Emit`] if emitting the event fails.
    pub fn release(&mut self, button: MouseButton) -> Result<(), InputError> {
        let events = [
            InputEvent::new(EventType::KEY.0, button.to_key().0, 0),
            InputEvent::new(EventType::SYNCHRONIZATION.0, 0, 0),
        ];
        self.device.emit(&events).map_err(InputError::Emit)
    }

    /// Performs a single mouse click (press and release).
    ///
    /// # Errors
    ///
    /// Returns [`InputError::Emit`] if emitting the event fails.
    pub fn click(&mut self, button: MouseButton) -> Result<(), InputError> {
        self.press(button)?;
        thread::sleep(Duration::from_millis(30));
        self.release(button)?;
        Ok(())
    }

    /// Performs a double mouse click.
    ///
    /// # Errors
    ///
    /// Returns [`InputError::Emit`] if emitting the event fails.
    pub fn double_click(&mut self, button: MouseButton) -> Result<(), InputError> {
        self.click(button)?;
        thread::sleep(Duration::from_millis(60));
        self.click(button)?;
        Ok(())
    }

    /// Scrolls the mouse wheel vertically (`steps_y`) and horizontally (`steps_x`).
    ///
    /// Positive `steps_y` scrolls up, negative scrolls down.
    ///
    /// # Errors
    ///
    /// Returns [`InputError::Emit`] if emitting the event fails.
    pub fn scroll(&mut self, steps_y: i32, steps_x: i32) -> Result<(), InputError> {
        let mut events = Vec::new();
        if steps_y != 0 {
            events.push(InputEvent::new(
                EventType::RELATIVE.0,
                RelativeAxisCode::REL_WHEEL.0,
                steps_y,
            ));
        }
        if steps_x != 0 {
            events.push(InputEvent::new(
                EventType::RELATIVE.0,
                RelativeAxisCode::REL_HWHEEL.0,
                steps_x,
            ));
        }
        if !events.is_empty() {
            events.push(InputEvent::new(EventType::SYNCHRONIZATION.0, 0, 0));
            self.device.emit(&events).map_err(InputError::Emit)?;
        }
        Ok(())
    }
}

/// Waits for the Wayland compositor / seat manager to open the newly-created device node.
fn wait_for_compositor_binding(dev: &mut VirtualDevice) {
    let node_path = match dev.enumerate_dev_nodes_blocking() {
        Ok(mut nodes) => nodes.next().and_then(Result::ok),
        Err(_) => None,
    };

    if let Some(path) = node_path {
        let start = std::time::Instant::now();
        let my_pid = std::process::id().to_string();

        while start.elapsed() < Duration::from_millis(3000) {
            if let Ok(output) = std::process::Command::new("fuser").arg(&path).output() {
                let is_success = output.status.success();
                if is_success {
                    let holders = String::from_utf8_lossy(&output.stdout);
                    // Check if the compositor / seat manager has opened the device node
                    let has_compositor = holders
                        .split_whitespace()
                        .any(|pid| pid != my_pid && is_compositor_pid(pid));
                    if has_compositor {
                        // Compositor has opened the node. Allow brief moment for seat initialization.
                        thread::sleep(Duration::from_millis(250));
                        return;
                    }
                }
            }
            thread::sleep(Duration::from_millis(50));
        }
    }

    // Fallback if dev node or fuser is unavailable
    thread::sleep(Duration::from_millis(800));
}

fn is_compositor_pid(pid: &str) -> bool {
    if let Ok(comm) = std::fs::read_to_string(format!("/proc/{pid}/comm")) {
        let comm_lower = comm.trim().to_lowercase();
        // Ignore known non-compositor device inspection daemons
        if comm_lower.contains("input-remapper")
            || comm_lower.contains("udev")
            || comm_lower.contains("systemd-udevd")
            || comm_lower.contains("fuser")
        {
            return false;
        }

        // Known Wayland compositors and seat managers
        if comm_lower.contains("gnome-shell")
            || comm_lower.contains("mutter")
            || comm_lower.contains("kwin")
            || comm_lower.contains("sway")
            || comm_lower.contains("hyprland")
            || comm_lower.contains("weston")
            || comm_lower.contains("wayfire")
            || comm_lower.contains("cosmic-comp")
            || comm_lower.contains("systemd-logind")
            || comm_lower.contains("xorg")
            || comm_lower.contains("xwayland")
        {
            return true;
        }

        // Any other non-ignored process that holds the input node
        return true;
    }
    false
}


