//! Legacy libretro variables work in both current and older RetroArch versions.
use crate::ffi;
use ruffle_core::{config::Letterbox, StageScaleMode};
use ruffle_render::quality::StageQuality;
use std::{ffi::CStr, ptr};

const DEFINITIONS: &[(&CStr, &CStr)] = &[
    (
        c"ruffle_resolution",
        c"Screen size; Native|640x480|800x600|1024x768|1280x720|1920x1080",
    ),
    (
        c"ruffle_scale_mode",
        c"Scale mode; Zoom to Fit|Unscaled (100%)|Stretch to Fit|Crop to Fit",
    ),
    (
        c"ruffle_force_scale",
        c"Force scale mode (override game); Off|On",
    ),
    (c"ruffle_letterbox", c"Letterbox; On|Off"),
    (
        c"ruffle_quality",
        c"Quality; High|Low|Medium|Best|High (8x8)|High (8x8) Linear|High (16x16)|High (16x16) Linear",
    ),
    (
        c"ruffle_compatibility",
        c"Compatibility fixes (restart content); Auto|Off",
    ),
    (c"ruffle_gamepad_mouse", c"Gamepad mouse; Off|On"),
    (
        c"ruffle_mouse_stick",
        c"Gamepad mouse - movement stick; Left|Right",
    ),
    (
        c"ruffle_mouse_click",
        c"Gamepad mouse - left click; R3|L3",
    ),
];

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum MouseStick {
    #[default]
    Left,
    Right,
}

impl MouseStick {
    pub fn index(self) -> u32 {
        match self {
            Self::Left => 0,
            Self::Right => 1,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum MouseClick {
    L3,
    #[default]
    R3,
}

impl MouseClick {
    pub fn button(self) -> u32 {
        match self {
            Self::L3 => 14,
            Self::R3 => 15,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GamepadMouse {
    pub enabled: bool,
    pub stick: MouseStick,
    pub click: MouseClick,
}

pub fn register(environment: ffi::Environment) {
    let mut variables: Vec<_> = DEFINITIONS
        .iter()
        .map(|(key, value)| ffi::Variable {
            key: key.as_ptr(),
            value: value.as_ptr(),
        })
        .collect();
    variables.push(ffi::Variable {
        key: ptr::null(),
        value: ptr::null(),
    });
    unsafe {
        environment(ffi::SET_VARIABLES, variables.as_mut_ptr().cast());
    }
}

pub fn updated(environment: Option<ffi::Environment>) -> bool {
    let mut changed = false;
    environment.is_some_and(|env| unsafe {
        env(ffi::GET_VARIABLE_UPDATE, (&mut changed as *mut bool).cast()) && changed
    })
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Options {
    pub resolution: Option<(u32, u32)>,
    pub scale: StageScaleMode,
    pub force_scale: bool,
    pub letterbox: Letterbox,
    pub quality: StageQuality,
    pub compatibility: bool,
    pub gamepad_mouse: GamepadMouse,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            resolution: None,
            scale: StageScaleMode::ShowAll,
            force_scale: false,
            letterbox: Letterbox::On,
            quality: StageQuality::High,
            compatibility: true,
            gamepad_mouse: GamepadMouse::default(),
        }
    }
}

impl Options {
    pub fn read(environment: Option<ffi::Environment>) -> Self {
        let mut options = Self::default();
        if let Some(env) = environment {
            for (key, _) in DEFINITIONS {
                let mut variable = ffi::Variable {
                    key: key.as_ptr(),
                    value: ptr::null(),
                };
                if unsafe {
                    env(
                        ffi::GET_VARIABLE,
                        (&mut variable as *mut ffi::Variable).cast(),
                    )
                } && !variable.value.is_null()
                {
                    if let Ok(value) = unsafe { CStr::from_ptr(variable.value) }.to_str() {
                        options.set(key.to_str().unwrap(), value);
                    }
                }
            }
        }
        options
    }

    fn set(&mut self, key: &str, value: &str) {
        let normalized = value.to_ascii_lowercase();
        let value = normalized.as_str();
        match key {
            "ruffle_resolution" => {
                self.resolution = match value {
                    "640x480" => Some((640, 480)),
                    "800x600" => Some((800, 600)),
                    "1024x768" => Some((1024, 768)),
                    "1280x720" => Some((1280, 720)),
                    "1920x1080" => Some((1920, 1080)),
                    _ => None,
                }
            }
            "ruffle_scale_mode" => {
                self.scale = match value {
                    "no_scale" | "unscaled (100%)" => StageScaleMode::NoScale,
                    "exact_fit" | "stretch to fit" => StageScaleMode::ExactFit,
                    "no_border" | "crop to fit" => StageScaleMode::NoBorder,
                    _ => StageScaleMode::ShowAll,
                }
            }
            "ruffle_force_scale" => self.force_scale = value == "on",
            "ruffle_letterbox" => {
                self.letterbox = if value == "off" {
                    Letterbox::Off
                } else {
                    Letterbox::On
                }
            }
            "ruffle_quality" => {
                self.quality = match value {
                    "high (8x8)" => StageQuality::High8x8,
                    "high (8x8) linear" => StageQuality::High8x8Linear,
                    "high (16x16)" => StageQuality::High16x16,
                    "high (16x16) linear" => StageQuality::High16x16Linear,
                    _ => value.parse().unwrap_or(StageQuality::High),
                }
            }
            "ruffle_compatibility" => self.compatibility = value != "off",
            "ruffle_gamepad_mouse" => self.gamepad_mouse.enabled = value == "on",
            "ruffle_mouse_stick" => {
                self.gamepad_mouse.stick = if value == "right" {
                    MouseStick::Right
                } else {
                    MouseStick::Left
                };
            }
            "ruffle_mouse_click" => {
                self.gamepad_mouse.click = if value == "l3" {
                    MouseClick::L3
                } else {
                    MouseClick::R3
                };
            }
            _ => (),
        }
    }

    pub fn dimensions(self, native: (u32, u32)) -> (u32, u32) {
        self.resolution.unwrap_or(native)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gamepad_mouse_defaults_and_selections() {
        let mut options = Options::default();
        assert!(!options.gamepad_mouse.enabled);
        assert_eq!(options.gamepad_mouse.stick, MouseStick::Left);
        assert_eq!(options.gamepad_mouse.click, MouseClick::R3);
        options.set("ruffle_gamepad_mouse", "On");
        options.set("ruffle_mouse_stick", "Right");
        options.set("ruffle_mouse_click", "L3");
        assert!(options.gamepad_mouse.enabled);
        assert_eq!(options.gamepad_mouse.stick, MouseStick::Right);
        assert_eq!(options.gamepad_mouse.click, MouseClick::L3);
        options.set("ruffle_gamepad_mouse", "Off");
        assert!(!options.gamepad_mouse.enabled);
        // Missing or corrupt frontend values have the same fallback as defaults.
        options.set("ruffle_mouse_stick", "invalid");
        options.set("ruffle_mouse_click", "invalid");
        assert_eq!(options.gamepad_mouse, GamepadMouse::default());
    }

    #[test]
    fn native_portrait_and_explicit_canvas() {
        let mut options = Options::default();
        assert_eq!(options.dimensions((300, 500)), (300, 500));
        options.set("ruffle_resolution", "1280x720");
        assert_eq!(options.dimensions((300, 500)), (1280, 720));
        options.set("ruffle_resolution", "99999x99999");
        assert_eq!(options.dimensions((300, 500)), (300, 500));
        options.set("ruffle_quality", "invalid");
        assert_eq!(options.quality, StageQuality::High);
        for name in [
            "low",
            "medium",
            "high",
            "best",
            "8x8",
            "8x8linear",
            "16x16",
            "16x16linear",
        ] {
            assert!(name.parse::<StageQuality>().is_ok(), "{name}");
        }
    }
}
