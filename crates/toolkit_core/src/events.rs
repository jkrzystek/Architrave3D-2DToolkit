use glam::Vec2;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PointerButton {
    Primary,
    Secondary,
    Middle,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct StylusState {
    pub pressure: f32,
    pub tilt: Vec2,
    pub rotation: f32,
}

impl Default for StylusState {
    fn default() -> Self {
        Self {
            pressure: 1.0,
            tilt: Vec2::ZERO,
            rotation: 0.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ViewportInputEvent {
    PointerMoved {
        position: Vec2,
        stylus: StylusState,
    },
    PointerPressed {
        position: Vec2,
        button: PointerButton,
        stylus: StylusState,
    },
    PointerReleased {
        position: Vec2,
        button: PointerButton,
    },
    Scroll {
        delta: Vec2,
        position: Vec2,
    },
    PinchZoom {
        delta: f32,
        center: Vec2,
    },
    KeyPressed {
        key: KeyCode,
        modifiers: Modifiers,
    },
    KeyReleased {
        key: KeyCode,
        modifiers: Modifiers,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KeyCode {
    Key1, Key2, Key3, Key4, Key5, Key6, Key7, Key8, Key9, Key0,
    A, B, C, D, E, F, G, H, I, J, K, L, M,
    N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
    Escape, Tab, Space, Enter, Backspace, Delete,
    Left, Right, Up, Down,
    Home, End, PageUp, PageDown,
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
    LShift, RShift, LCtrl, RCtrl, LAlt, RAlt,
    BracketLeft, BracketRight,
    Minus, Equal, Comma, Period, Slash, Backslash, Semicolon, Quote,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_stylus_state() {
        let s = StylusState::default();
        assert_eq!(s.pressure, 1.0);
        assert_eq!(s.tilt, Vec2::ZERO);
    }

    #[test]
    fn default_modifiers() {
        let m = Modifiers::default();
        assert!(!m.shift && !m.ctrl && !m.alt && !m.meta);
    }

    #[test]
    fn event_serialization_roundtrip() {
        let event = ViewportInputEvent::PointerMoved {
            position: Vec2::new(100.0, 200.0),
            stylus: StylusState {
                pressure: 0.5,
                tilt: Vec2::new(0.1, 0.2),
                rotation: 0.0,
            },
        };
        let json = serde_json::to_string(&event).unwrap();
        let deserialized: ViewportInputEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, deserialized);
    }
}
