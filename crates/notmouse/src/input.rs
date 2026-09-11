//! Linux `uinput` virtual device input injection backend.

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

/// Virtual mouse device managing mouse movement, clicks, drags, and scrolling.
pub struct InputDevice {
    device: VirtualDevice,
}

impl InputDevice {
    /// Creates and registers a new virtual mouse with `/dev/uinput`.
    ///
    /// # Errors
    ///
    /// Returns [`InputError::DeviceCreation`] if `/dev/uinput` cannot be opened or registered.
    pub fn new() -> Result<Self, InputError> {
        let mut keys = AttributeSet::<KeyCode>::new();
        keys.insert(KeyCode::BTN_LEFT);
        keys.insert(KeyCode::BTN_RIGHT);
        keys.insert(KeyCode::BTN_MIDDLE);
        keys.insert(KeyCode::BTN_TOUCH);

        let mut rel_axes = AttributeSet::<RelativeAxisCode>::new();
        rel_axes.insert(RelativeAxisCode::REL_X);
        rel_axes.insert(RelativeAxisCode::REL_Y);
        rel_axes.insert(RelativeAxisCode::REL_WHEEL);
        rel_axes.insert(RelativeAxisCode::REL_HWHEEL);

        let x_axis_info = AbsInfo::new(0, 0, ABS_MAX_RANGE, 0, 0, 1);
        let y_axis_info = AbsInfo::new(0, 0, ABS_MAX_RANGE, 0, 0, 1);

        let abs_x = UinputAbsSetup::new(AbsoluteAxisCode::ABS_X, x_axis_info);
        let abs_y = UinputAbsSetup::new(AbsoluteAxisCode::ABS_Y, y_axis_info);

        let device = VirtualDevice::builder()
            .map_err(InputError::DeviceCreation)?
            .name("notmouse-virtual-pointer")
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

        // Give the kernel / compositor a short pause to bind the new virtual device.
        thread::sleep(Duration::from_millis(100));

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
    #[allow(dead_code)]
    pub fn move_relative(&mut self, dx: i32, dy: i32) -> Result<(), InputError> {
        let events = [
            InputEvent::new(EventType::RELATIVE.0, RelativeAxisCode::REL_X.0, dx),
            InputEvent::new(EventType::RELATIVE.0, RelativeAxisCode::REL_Y.0, dy),
            InputEvent::new(EventType::SYNCHRONIZATION.0, 0, 0),
        ];

        self.device.emit(&events).map_err(InputError::Emit)?;
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
        thread::sleep(Duration::from_millis(25));
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
        events.push(InputEvent::new(EventType::SYNCHRONIZATION.0, 0, 0));

        self.device.emit(&events).map_err(InputError::Emit)
    }
}
