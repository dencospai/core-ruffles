use crate::ffi;
use ruffle_core::events::{
    KeyDescriptor, KeyLocation, LogicalKey, MouseButton, MouseWheelDelta, NamedKey, PhysicalKey,
    PlayerEvent,
};

pub const KEY_COUNT: usize = 324;
// libretro joypad IDs -> libretro keyboard IDs.
pub const PAD_KEYS: [(u32, usize, &str); 10] = [
    (4, 273, "Arrow Up\0"),
    (5, 274, "Arrow Down\0"),
    (6, 276, "Arrow Left\0"),
    (7, 275, "Arrow Right\0"),
    (8, 32, "Space\0"),
    (0, 306, "Left Ctrl\0"),
    (9, 120, "X key\0"),
    (1, 122, "Z key\0"),
    (3, 13, "Enter\0"),
    (2, 27, "Escape\0"),
];

pub struct Snapshot {
    pub keys: [bool; KEY_COUNT],
    pub mouse_delta: [f64; 2],
    pub buttons: [bool; 3],
    pub wheel: f64,
    pub text: Vec<char>,
}

impl Default for Snapshot {
    fn default() -> Self {
        Self {
            keys: [false; KEY_COUNT],
            mouse_delta: [0.0; 2],
            buttons: [false; 3],
            wheel: 0.0,
            text: Vec::new(),
        }
    }
}

fn stick_delta(value: i16) -> f64 {
    let value = f64::from(value) / 32768.0;
    if value.abs() <= 0.2 {
        0.0
    } else {
        value.signum() * (value.abs() - 0.2) / 0.8 * 8.0
    }
}

pub fn poll(callback: Option<ffi::InputState>, joypad: bool, text: Vec<char>) -> Snapshot {
    let mut result = Snapshot {
        text,
        ..Snapshot::default()
    };
    let Some(callback) = callback else {
        return result;
    };
    // These IDs and port 0 follow libretro.h; the callback is frontend-owned.
    let read = |device, index, id| unsafe { callback(0, device, index, id) };
    for (id, pressed) in result.keys.iter_mut().enumerate() {
        if key_descriptor(id as u32).is_some() {
            *pressed = read(ffi::KEYBOARD, 0, id as u32) != 0;
        }
    }
    result.mouse_delta = [
        f64::from(read(ffi::MOUSE, 0, 0)),
        f64::from(read(ffi::MOUSE, 0, 1)),
    ];
    result.buttons = [
        read(ffi::MOUSE, 0, 2) != 0,
        read(ffi::MOUSE, 0, 3) != 0,
        read(ffi::MOUSE, 0, 6) != 0,
    ];
    result.wheel = f64::from(read(ffi::MOUSE, 0, 4)) - f64::from(read(ffi::MOUSE, 0, 5));
    if joypad {
        for (button, key, _) in PAD_KEYS {
            result.keys[key] |= read(ffi::JOYPAD, 0, button) != 0;
        }
        result.buttons[0] |= read(ffi::JOYPAD, 0, 10) != 0;
        result.buttons[1] |= read(ffi::JOYPAD, 0, 11) != 0;
        result.mouse_delta[0] += stick_delta(read(ffi::ANALOG, 1, 0));
        result.mouse_delta[1] += stick_delta(read(ffi::ANALOG, 1, 1));
    }
    result
}

pub struct Input {
    keys: [bool; KEY_COUNT],
    buttons: [bool; 3],
    pub x: f64,
    pub y: f64,
}

impl Input {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            keys: [false; KEY_COUNT],
            buttons: [false; 3],
            x: f64::from(width) / 2.0,
            y: f64::from(height) / 2.0,
        }
    }

    pub fn events(&mut self, snapshot: Snapshot, width: u32, height: u32) -> Vec<PlayerEvent> {
        let mut events = Vec::new();
        // Flash AVM1 on(keyPress "<Space>") uses TextInput, not KeyDown.
        // Gamepads and polling-only frontends do not send keyboard text callbacks.
        // Synthesize this character once per press, without duplicating real text.
        let space_text = snapshot.keys[32] && !self.keys[32] && !snapshot.text.contains(&' ');
        for (id, pressed) in snapshot.keys.iter().copied().enumerate() {
            if pressed != self.keys[id] {
                if let Some(key) = key_descriptor(id as u32) {
                    events.push(if pressed {
                        PlayerEvent::KeyDown { key }
                    } else {
                        PlayerEvent::KeyUp { key }
                    });
                }
            }
        }
        self.keys = snapshot.keys;
        if space_text {
            events.push(PlayerEvent::TextInput { codepoint: ' ' });
        }
        events.extend(
            snapshot
                .text
                .into_iter()
                .map(|codepoint| PlayerEvent::TextInput { codepoint }),
        );
        self.x = (self.x + snapshot.mouse_delta[0]).clamp(0.0, f64::from(width - 1));
        self.y = (self.y + snapshot.mouse_delta[1]).clamp(0.0, f64::from(height - 1));
        events.push(PlayerEvent::MouseMove {
            x: self.x,
            y: self.y,
        });
        for (i, button) in [MouseButton::Left, MouseButton::Right, MouseButton::Middle]
            .into_iter()
            .enumerate()
        {
            if snapshot.buttons[i] != self.buttons[i] {
                events.push(if snapshot.buttons[i] {
                    PlayerEvent::MouseDown {
                        x: self.x,
                        y: self.y,
                        button,
                        index: None,
                    }
                } else {
                    PlayerEvent::MouseUp {
                        x: self.x,
                        y: self.y,
                        button,
                    }
                });
            }
        }
        self.buttons = snapshot.buttons;
        if snapshot.wheel != 0.0 {
            events.push(PlayerEvent::MouseWheel {
                delta: MouseWheelDelta::Lines(snapshot.wheel),
            });
        }
        events
    }
}

pub fn key_descriptor(id: u32) -> Option<KeyDescriptor> {
    use LogicalKey::{Character, Named};
    use NamedKey as N;
    use PhysicalKey as P;
    let (physical_key, logical_key) = match id {
        97..=122 => {
            let letters = [
                P::KeyA,
                P::KeyB,
                P::KeyC,
                P::KeyD,
                P::KeyE,
                P::KeyF,
                P::KeyG,
                P::KeyH,
                P::KeyI,
                P::KeyJ,
                P::KeyK,
                P::KeyL,
                P::KeyM,
                P::KeyN,
                P::KeyO,
                P::KeyP,
                P::KeyQ,
                P::KeyR,
                P::KeyS,
                P::KeyT,
                P::KeyU,
                P::KeyV,
                P::KeyW,
                P::KeyX,
                P::KeyY,
                P::KeyZ,
            ];
            (letters[(id - 97) as usize], Character(char::from_u32(id)?))
        }
        48..=57 => {
            let digits = [
                P::Digit0,
                P::Digit1,
                P::Digit2,
                P::Digit3,
                P::Digit4,
                P::Digit5,
                P::Digit6,
                P::Digit7,
                P::Digit8,
                P::Digit9,
            ];
            (digits[(id - 48) as usize], Character(char::from_u32(id)?))
        }
        8 => (P::Backspace, Named(N::Backspace)),
        9 => (P::Tab, Named(N::Tab)),
        13 => (P::Enter, Named(N::Enter)),
        27 => (P::Escape, Named(N::Escape)),
        32 => (P::Space, Character(' ')),
        39 => (P::Quote, Character('\'')),
        44 => (P::Comma, Character(',')),
        45 => (P::Minus, Character('-')),
        46 => (P::Period, Character('.')),
        47 => (P::Slash, Character('/')),
        59 => (P::Semicolon, Character(';')),
        61 => (P::Equal, Character('=')),
        91 => (P::BracketLeft, Character('[')),
        92 => (P::Backslash, Character('\\')),
        93 => (P::BracketRight, Character(']')),
        96 => (P::Backquote, Character('`')),
        127 => (P::Delete, Named(N::Delete)),
        273 => (P::ArrowUp, Named(N::ArrowUp)),
        274 => (P::ArrowDown, Named(N::ArrowDown)),
        275 => (P::ArrowRight, Named(N::ArrowRight)),
        276 => (P::ArrowLeft, Named(N::ArrowLeft)),
        277 => (P::Insert, Named(N::Insert)),
        278 => (P::Home, Named(N::Home)),
        279 => (P::End, Named(N::End)),
        280 => (P::PageUp, Named(N::PageUp)),
        281 => (P::PageDown, Named(N::PageDown)),
        301 => (P::CapsLock, Named(N::CapsLock)),
        303 => (P::ShiftRight, Named(N::Shift)),
        304 => (P::ShiftLeft, Named(N::Shift)),
        305 => (P::ControlRight, Named(N::Control)),
        306 => (P::ControlLeft, Named(N::Control)),
        307 => (P::AltRight, Named(N::Alt)),
        308 => (P::AltLeft, Named(N::Alt)),
        _ => return None,
    };
    let key_location = match id {
        303 | 305 | 307 => KeyLocation::Right,
        304 | 306 | 308 => KeyLocation::Left,
        _ => KeyLocation::Standard,
    };
    Some(KeyDescriptor {
        physical_key,
        logical_key,
        key_location,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn space_supports_flash_button_keypress_without_duplicate_text() {
        for text in [vec![], vec![' ']] {
            let mut input = Input::new(320, 240);
            let mut snapshot = Snapshot {
                text,
                ..Snapshot::default()
            };
            snapshot.keys[32] = true;
            let events = input.events(snapshot, 320, 240);
            assert_eq!(
                events
                    .iter()
                    .filter(|event| matches!(event, PlayerEvent::TextInput { codepoint: ' ' }))
                    .count(),
                1
            );
            let mut held = Snapshot::default();
            held.keys[32] = true;
            assert!(!input
                .events(held, 320, 240)
                .iter()
                .any(|event| matches!(event, PlayerEvent::TextInput { .. })));
        }
    }

    #[test]
    fn key_edges_and_pointer_bounds() {
        let mut input = Input::new(320, 240);
        let mut snapshot = Snapshot::default();
        snapshot.keys[32] = true;
        snapshot.buttons[0] = true;
        snapshot.mouse_delta = [-10000.0, 10000.0];
        let events = input.events(snapshot, 320, 240);
        assert_eq!((input.x, input.y), (0.0, 239.0));
        assert!(events
            .iter()
            .any(|e| matches!(e, PlayerEvent::KeyDown { .. })));
        let mut snapshot = Snapshot::default();
        snapshot.keys[32] = true;
        snapshot.buttons[0] = true;
        let events = input.events(snapshot, 320, 240);
        assert_eq!(events.len(), 1); // Only pointer motion, no repeated presses.
        let events = input.events(Snapshot::default(), 320, 240);
        assert!(events
            .iter()
            .any(|e| matches!(e, PlayerEvent::KeyUp { .. })));
        assert!(events
            .iter()
            .any(|e| matches!(e, PlayerEvent::MouseUp { .. })));
    }

    #[test]
    fn stick_center_has_deadzone() {
        assert_eq!(stick_delta(2000), 0.0);
        assert_eq!(stick_delta(i16::MIN), -8.0);
    }

    #[test]
    fn keyboard_keeps_key_down_when_gamepad_releases_it() {
        unsafe extern "C" fn keyboard_only(_port: u32, device: u32, _index: u32, id: u32) -> i16 {
            i16::from(device == ffi::KEYBOARD && id == 32)
        }
        unsafe extern "C" fn pad_only(_port: u32, device: u32, _index: u32, id: u32) -> i16 {
            i16::from(device == ffi::JOYPAD && id == 8)
        }
        let mut input = Input::new(320, 240);
        input.events(poll(Some(pad_only), true, vec![]), 320, 240);
        let events = input.events(poll(Some(keyboard_only), true, vec![]), 320, 240);
        assert!(!events
            .iter()
            .any(|e| matches!(e, PlayerEvent::KeyUp { .. } | PlayerEvent::KeyDown { .. })));
        assert!(!poll(Some(pad_only), false, vec![]).keys[32]);
    }
}
