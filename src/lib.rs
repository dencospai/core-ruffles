//! Ruffle/libretro bridge. RetroArch calls lifecycle and frame functions on its
//! core thread. Keep the non-Send Ruffle player on that thread; frontend
//! callbacks and the keyboard text queue are synchronized separately.
//!
//! # FFI safety
//! All pointer arguments follow libretro.h: non-null pointers must refer to
//! valid, correctly aligned storage of the declared size for the entire call.
//! Callback pointers must remain callable until replaced or core unload.
#![allow(clippy::missing_safety_doc)]

mod audio;
mod compatibility;
mod cursor;
mod engine;
pub mod ffi;
mod input;
mod navigator;
mod options;
mod storage;
mod ui;

use engine::Game;
use std::{
    cell::RefCell,
    ffi::{c_void, CStr, CString},
    panic::{catch_unwind, AssertUnwindSafe},
    path::PathBuf,
    ptr,
    sync::Mutex,
};

#[derive(Clone, Copy, Default)]
struct Callbacks {
    environment: Option<ffi::Environment>,
    video: Option<ffi::Video>,
    audio: Option<ffi::Audio>,
    audio_batch: Option<ffi::AudioBatch>,
    input_poll: Option<ffi::InputPoll>,
    input_state: Option<ffi::InputState>,
}

static CALLBACKS: Mutex<Callbacks> = Mutex::new(Callbacks {
    environment: None,
    video: None,
    audio: None,
    audio_batch: None,
    input_poll: None,
    input_state: None,
});
static TEXT: Mutex<Vec<char>> = Mutex::new(Vec::new());

struct State {
    game: Option<Game>,
    info: ffi::AvInfo,
    joypad: bool,
    // RetroArch replays this pointer while its menu is open, after retro_run.
    // Keep it alive across pause, reset and unload until a replacement is sent.
    last_video: Vec<u32>,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State {
        game: None, info: ffi::av_info(640, 480), joypad: true, last_video: Vec::new(),
    });
}

fn callbacks() -> Callbacks {
    *CALLBACKS.lock().unwrap_or_else(|e| e.into_inner())
}

fn report(message: &str) {
    eprintln!("[ruffle_libretro] {message}");
    if let (Some(environment), Ok(text)) = (callbacks().environment, CString::new(message)) {
        let mut message = ffi::Message {
            msg: text.as_ptr(),
            frames: 300,
        };
        unsafe {
            environment(ffi::SET_MESSAGE, (&mut message as *mut ffi::Message).cast());
        }
    }
}

fn guarded<T>(fallback: T, action: impl FnOnce() -> Result<T, String>) -> T {
    match catch_unwind(AssertUnwindSafe(action)) {
        Ok(Ok(value)) => value,
        Ok(Err(error)) => {
            report(&error);
            fallback
        }
        Err(_) => {
            // Never unwind through the C ABI or continue using a panicked player.
            STATE.with(|state| {
                if let Ok(mut state) = state.try_borrow_mut() {
                    state.game = None;
                }
            });
            report("Ruffle encountered a panic. Close and reload the content.");
            fallback
        }
    }
}

unsafe extern "C" fn keyboard(down: bool, _key: u32, character: u32, _modifiers: u16) {
    if down {
        if let Some(character) = char::from_u32(character).filter(|c| !c.is_control()) {
            let mut queue = TEXT.lock().unwrap_or_else(|e| e.into_inner());
            if queue.len() < 1024 {
                queue.push(character);
            }
        }
    }
}

fn save_directory(environment: ffi::Environment) -> Option<PathBuf> {
    let mut path = ptr::null::<std::ffi::c_char>();
    let supported = unsafe {
        environment(
            ffi::GET_SAVE_DIRECTORY,
            (&mut path as *mut *const std::ffi::c_char).cast(),
        )
    };
    if !supported || path.is_null() {
        return None;
    }
    let path = unsafe { CStr::from_ptr(path) }.to_str().ok()?;
    if path.is_empty() {
        None
    } else {
        Some(PathBuf::from(path))
    }
}

#[no_mangle]
pub extern "C" fn retro_api_version() -> u32 {
    1
}

#[no_mangle]
pub extern "C" fn retro_set_environment(callback: Option<ffi::Environment>) {
    CALLBACKS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .environment = callback;
    if let Some(callback) = callback {
        options::register(callback);
    }
}

macro_rules! setter {
    ($name:ident, $field:ident, $ty:ty) => {
        #[no_mangle]
        pub extern "C" fn $name(callback: Option<$ty>) {
            CALLBACKS.lock().unwrap_or_else(|e| e.into_inner()).$field = callback;
        }
    };
}
setter!(retro_set_video_refresh, video, ffi::Video);
setter!(retro_set_audio_sample, audio, ffi::Audio);
setter!(retro_set_audio_sample_batch, audio_batch, ffi::AudioBatch);
setter!(retro_set_input_poll, input_poll, ffi::InputPoll);
setter!(retro_set_input_state, input_state, ffi::InputState);

#[no_mangle]
pub extern "C" fn retro_init() {
    guarded((), || {
        if let Some(environment) = callbacks().environment {
            let mut keyboard = ffi::KeyboardCallback {
                callback: Some(keyboard),
            };
            unsafe {
                environment(
                    ffi::SET_KEYBOARD_CALLBACK,
                    (&mut keyboard as *mut ffi::KeyboardCallback).cast(),
                );
            }
            let mut descriptors: Vec<ffi::InputDescriptor> = input::PAD_KEYS
                .iter()
                .map(|(id, _, description)| ffi::InputDescriptor {
                    port: 0,
                    device: ffi::JOYPAD,
                    index: 0,
                    id: *id,
                    description: description.as_ptr().cast(),
                })
                .collect();
            for (id, label) in [(10, c"Left mouse button"), (11, c"Right mouse button")] {
                descriptors.push(ffi::InputDescriptor {
                    port: 0,
                    device: ffi::JOYPAD,
                    index: 0,
                    id,
                    description: label.as_ptr(),
                });
            }
            descriptors.push(ffi::InputDescriptor {
                port: 0,
                device: 0,
                index: 0,
                id: 0,
                description: ptr::null(),
            });
            unsafe {
                environment(ffi::SET_INPUT_DESCRIPTORS, descriptors.as_mut_ptr().cast());
            }
        }
        Ok(())
    });
}

#[no_mangle]
pub extern "C" fn retro_deinit() {
    retro_unload_game();
    STATE.with(|state| state.borrow_mut().last_video.clear());
    TEXT.lock().unwrap_or_else(|e| e.into_inner()).clear();
}

#[no_mangle]
pub unsafe extern "C" fn retro_get_system_info(info: *mut ffi::SystemInfo) {
    if !info.is_null() {
        info.write(ffi::SystemInfo {
            library_name: c"Ruffle".as_ptr(),
            library_version: c"0.2.2".as_ptr(),
            valid_extensions: c"swf".as_ptr(),
            need_fullpath: true,
            block_extract: false,
        });
    }
}

#[no_mangle]
pub unsafe extern "C" fn retro_get_system_av_info(info: *mut ffi::AvInfo) {
    if !info.is_null() {
        guarded((), || {
            STATE.with(|state| info.write(state.borrow().info));
            Ok(())
        });
    }
}

#[no_mangle]
pub unsafe extern "C" fn retro_load_game(info: *const ffi::GameInfo) -> bool {
    guarded(false, || {
        retro_unload_game();
        let info = info.as_ref().ok_or("A SWF file is required")?;
        if info.path.is_null() {
            return Err("Missing SWF path".into());
        }
        let path = PathBuf::from(
            CStr::from_ptr(info.path)
                .to_str()
                .map_err(|e| e.to_string())?,
        );
        let environment = callbacks()
            .environment
            .ok_or("Missing libretro environment callback")?;
        let mut format = ffi::XRGB8888;
        if !environment(ffi::SET_PIXEL_FORMAT, (&mut format as *mut u32).cast()) {
            return Err("Frontend does not support XRGB8888 pixels".into());
        }
        let save_directory = save_directory(environment);
        if save_directory.is_none() {
            report("No RetroArch save directory: Flash saves will only last for this session.");
        }
        let game = Game::load(
            &path,
            save_directory,
            options::Options::read(Some(environment)),
        )?;
        STATE.with(|state| {
            let mut state = state.borrow_mut();
            state.info = game.av_info();
            state.game = Some(game);
        });
        Ok(true)
    })
}

#[no_mangle]
pub extern "C" fn retro_unload_game() {
    guarded((), || {
        STATE.with(|state| {
            let mut state = state.borrow_mut();
            state.game = None;
            state.info = ffi::av_info(640, 480);
        });
        TEXT.lock().unwrap_or_else(|e| e.into_inner()).clear();
        Ok(())
    });
}

#[no_mangle]
pub extern "C" fn retro_reset() {
    guarded((), || {
        let old = STATE.with(|state| state.borrow_mut().game.take());
        if let Some(old) = old {
            let path = old.path.clone();
            let directory = old.save_directory.clone();
            drop(old);
            let game = Game::load(
                &path,
                directory,
                options::Options::read(callbacks().environment),
            )?;
            STATE.with(|state| state.borrow_mut().game = Some(game));
            TEXT.lock().unwrap_or_else(|e| e.into_inner()).clear();
        }
        Ok(())
    });
}

#[no_mangle]
pub extern "C" fn retro_run() {
    guarded((), || {
        let callbacks = callbacks();
        let updated_options = options::updated(callbacks.environment)
            .then(|| options::Options::read(callbacks.environment));
        // Poll before borrowing STATE: the frontend may invoke keyboard here.
        if let Some(poll) = callbacks.input_poll {
            unsafe {
                poll();
            }
        }
        let text = std::mem::take(&mut *TEXT.lock().unwrap_or_else(|e| e.into_inner()));
        let joypad = STATE.with(|state| state.borrow().joypad);
        let snapshot = input::poll(callbacks.input_state, joypad, text);
        let frame = STATE.with(|state| {
            let mut state = state.borrow_mut();
            state
                .game
                .as_mut()
                .map(|game| game.run(snapshot, updated_options))
                .transpose()
        })?;
        if let Some(frame) = frame {
            let geometry = STATE.with(|state| {
                let mut state = state.borrow_mut();
                let old = state.info.geometry;
                if (old.base_width, old.base_height) != (frame.width, frame.height) {
                    state.info = ffi::av_info(frame.width, frame.height);
                    Some(state.info.geometry)
                } else {
                    None
                }
            });
            if let (Some(environment), Some(mut geometry)) = (callbacks.environment, geometry) {
                unsafe {
                    environment(
                        ffi::SET_GEOMETRY,
                        (&mut geometry as *mut ffi::Geometry).cast(),
                    );
                }
            }
            if let Some(video) = callbacks.video {
                unsafe {
                    video(
                        frame.video.as_ptr().cast(),
                        frame.width,
                        frame.height,
                        frame.width as usize * 4,
                    );
                }
            }
            // Replace only AFTER video returns: both the previous cached frame
            // and its replacement remain valid throughout the frontend call.
            STATE.with(|state| state.borrow_mut().last_video = frame.video);
            if let Some(batch) = callbacks.audio_batch {
                // Honor partial consumption without spinning on a full frontend buffer.
                let mut offset = 0;
                while offset < frame.audio.len() / 2 {
                    let remaining = frame.audio.len() / 2 - offset;
                    let written = unsafe { batch(frame.audio.as_ptr().add(offset * 2), remaining) }
                        .min(remaining);
                    if written == 0 {
                        break;
                    }
                    offset += written;
                }
            } else if let Some(audio) = callbacks.audio {
                for sample in frame.audio.as_chunks::<2>().0 {
                    unsafe {
                        audio(sample[0], sample[1]);
                    }
                }
            }
        }
        Ok(())
    });
}

#[no_mangle]
pub extern "C" fn retro_set_controller_port_device(port: u32, device: u32) {
    if port == 0 {
        guarded((), || {
            STATE.with(|state| {
                state.borrow_mut().joypad = matches!(device & 0xff, ffi::JOYPAD | ffi::ANALOG)
            });
            Ok(())
        });
    }
}

#[no_mangle]
pub extern "C" fn retro_get_region() -> u32 {
    0
}
#[no_mangle]
pub extern "C" fn retro_serialize_size() -> usize {
    0
}
#[no_mangle]
pub extern "C" fn retro_serialize(_data: *mut c_void, _size: usize) -> bool {
    false
}
#[no_mangle]
pub extern "C" fn retro_unserialize(_data: *const c_void, _size: usize) -> bool {
    false
}
#[no_mangle]
pub extern "C" fn retro_get_memory_data(_id: u32) -> *mut c_void {
    ptr::null_mut()
}
#[no_mangle]
pub extern "C" fn retro_get_memory_size(_id: u32) -> usize {
    0
}
#[no_mangle]
pub extern "C" fn retro_cheat_reset() {}
#[no_mangle]
pub extern "C" fn retro_cheat_set(_index: u32, _enabled: bool, _code: *const std::ffi::c_char) {}
#[no_mangle]
pub extern "C" fn retro_load_game_special(
    _kind: u32,
    _info: *const ffi::GameInfo,
    _count: usize,
) -> bool {
    false
}
