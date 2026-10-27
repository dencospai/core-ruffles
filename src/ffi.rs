//! Minimal libretro ABI, checked against libretro-common/include/libretro.h.
//! Pointers passed by a frontend must remain valid for the documented call duration.
use std::ffi::{c_char, c_void};

pub type Environment = unsafe extern "C" fn(u32, *mut c_void) -> bool;
pub type Video = unsafe extern "C" fn(*const c_void, u32, u32, usize);
pub type Audio = unsafe extern "C" fn(i16, i16);
pub type AudioBatch = unsafe extern "C" fn(*const i16, usize) -> usize;
pub type InputPoll = unsafe extern "C" fn();
pub type InputState = unsafe extern "C" fn(u32, u32, u32, u32) -> i16;

pub const SET_MESSAGE: u32 = 6;
pub const SET_PIXEL_FORMAT: u32 = 10;
pub const SET_INPUT_DESCRIPTORS: u32 = 11;
pub const SET_KEYBOARD_CALLBACK: u32 = 12;
pub const GET_VARIABLE: u32 = 15;
pub const SET_VARIABLES: u32 = 16;
pub const GET_VARIABLE_UPDATE: u32 = 17;
pub const SET_GEOMETRY: u32 = 37;
pub const GET_SAVE_DIRECTORY: u32 = 31;
pub const JOYPAD: u32 = 1;
pub const MOUSE: u32 = 2;
pub const KEYBOARD: u32 = 3;
pub const ANALOG: u32 = 5;
pub const XRGB8888: u32 = 1;

#[repr(C)]
pub struct SystemInfo {
    pub library_name: *const c_char,
    pub library_version: *const c_char,
    pub valid_extensions: *const c_char,
    pub need_fullpath: bool,
    pub block_extract: bool,
}

#[repr(C)]
pub struct GameInfo {
    pub path: *const c_char,
    pub data: *const c_void,
    pub size: usize,
    pub meta: *const c_char,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Geometry {
    pub base_width: u32,
    pub base_height: u32,
    pub max_width: u32,
    pub max_height: u32,
    pub aspect_ratio: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Timing {
    pub fps: f64,
    pub sample_rate: f64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct AvInfo {
    pub geometry: Geometry,
    pub timing: Timing,
}

pub fn av_info(width: u32, height: u32) -> AvInfo {
    AvInfo {
        geometry: Geometry {
            base_width: width,
            base_height: height,
            max_width: 4096,
            max_height: 4096,
            aspect_ratio: width as f32 / height as f32,
        },
        // Ruffle advances the movie at its own frame rate within this host clock.
        timing: Timing {
            fps: 60.0,
            sample_rate: 48_000.0,
        },
    }
}

#[repr(C)]
pub struct Message {
    pub msg: *const c_char,
    pub frames: u32,
}

#[repr(C)]
pub struct Variable {
    pub key: *const c_char,
    pub value: *const c_char,
}

#[repr(C)]
pub struct KeyboardCallback {
    pub callback: Option<unsafe extern "C" fn(bool, u32, u32, u16)>,
}

#[repr(C)]
pub struct InputDescriptor {
    pub port: u32,
    pub device: u32,
    pub index: u32,
    pub id: u32,
    pub description: *const c_char,
}
