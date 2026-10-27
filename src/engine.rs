use crate::{
    audio::RetroAudio,
    ffi,
    input::{Input, Snapshot},
    options::Options,
    storage::FileStorage,
};
use ruffle_core::{
    backend::audio::AudioMixerProxy,
    backend::navigator::{NullExecutor, NullNavigatorBackend},
    events::PlayerEvent,
    limits::ExecutionLimit,
    FloatDuration, Player, PlayerBuilder, ViewportDimensions,
};
use ruffle_render_wgpu::{backend::WgpuRenderBackend, target::TextureTarget, wgpu};
use std::{
    any::Any,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};

pub struct Frame {
    pub video: Vec<u32>,
    pub audio: Vec<i16>,
    pub width: u32,
    pub height: u32,
}

pub struct Game {
    player: Arc<Mutex<Player>>,
    executor: NullExecutor,
    mixer: AudioMixerProxy,
    input: Input,
    width: u32,
    height: u32,
    options: Options,
    pub path: PathBuf,
    pub save_directory: Option<PathBuf>,
}

impl Game {
    pub fn load(
        path: &Path,
        save_directory: Option<PathBuf>,
        options: Options,
    ) -> Result<Self, String> {
        let movie = crate::compatibility::load(path, options.compatibility)?;
        let width = movie.width().to_pixels().ceil();
        let height = movie.height().to_pixels().ceil();
        if !(1.0..=4096.0).contains(&width) || !(1.0..=4096.0).contains(&height) {
            return Err(format!(
                "Unsupported SWF dimensions: {width} x {height} (limit: 4096)"
            ));
        }
        if movie.frame_rate().to_f64() <= 0.0 {
            return Err("SWF has an invalid frame rate".into());
        }
        let (width, height) = options.dimensions((width as u32, height as u32));
        let renderer = WgpuRenderBackend::<TextureTarget>::for_offscreen(
            (width, height),
            wgpu::Backends::PRIMARY,
            wgpu::PowerPreference::HighPerformance,
        )
        .map_err(|e| format!("Failed to initialize GPU: {e}"))?;
        let (audio, mixer) = RetroAudio::new();
        let executor = NullExecutor::new();
        let absolute_path = path.canonicalize().map_err(|e| e.to_string())?;
        let navigator = NullNavigatorBackend::with_base_path(
            absolute_path
                .parent()
                .ok_or("SWF path has no parent directory")?,
            &executor,
        )
        .map_err(|e| e.to_string())?;
        let mut builder = PlayerBuilder::new()
            .with_movie(movie)
            .with_renderer(renderer)
            .with_audio(audio)
            .with_ui(crate::ui::RetroUi::default())
            .with_navigator(crate::navigator::OfflineNavigator::new(
                navigator,
                absolute_path.parent().unwrap(),
            )?)
            .with_autoplay(true)
            .with_scale_mode(options.scale, options.force_scale)
            .with_letterbox(options.letterbox)
            .with_quality(options.quality)
            .with_viewport_dimensions(width, height, 1.0)
            .with_max_execution_duration(Duration::from_secs(5));
        if let Some(directory) = &save_directory {
            builder = builder.with_storage(Box::new(
                FileStorage::new(directory)
                    .map_err(|e| format!("Failed to open save directory: {e}"))?,
            ));
        }
        let player = builder.build();
        player
            .lock()
            .map_err(|e| e.to_string())?
            .handle_event(PlayerEvent::FocusGained);
        Ok(Self {
            player,
            executor,
            mixer,
            input: Input::new(width, height),
            width,
            height,
            options,
            path: path.to_owned(),
            save_directory,
        })
    }

    pub fn av_info(&self) -> ffi::AvInfo {
        ffi::av_info(self.width, self.height)
    }

    pub fn options(&self) -> Options {
        self.options
    }

    pub fn run(&mut self, snapshot: Snapshot, updated: Option<Options>) -> Result<Frame, String> {
        self.executor.run();
        let mut player = self.player.lock().map_err(|e| e.to_string())?;
        if let Some(options) = updated {
            player.set_forced_scale_mode(options.force_scale);
            player.set_scale_mode(options.scale);
            player.set_letterbox(options.letterbox);
            player.set_quality(options.quality);
            self.options = options;
        }
        let native = (player.movie_width(), player.movie_height());
        let (width, height) = self.options.dimensions(native);
        if (width, height) != (self.width, self.height) {
            if width == 0 || height == 0 || width > 4096 || height > 4096 {
                return Err("Unsupported viewport dimensions".into());
            }
            self.input.x *= f64::from(width) / f64::from(self.width);
            self.input.y *= f64::from(height) / f64::from(self.height);
            player.set_viewport_dimensions(ViewportDimensions {
                width,
                height,
                scale_factor: 1.0,
            });
            self.width = width;
            self.height = height;
        }
        // Budget loading work so large SWFs do not monopolize retro_run.
        player.preload(&mut ExecutionLimit::with_max_ops_and_time(
            10_000,
            Duration::from_millis(8),
        ));
        for event in self.input.events(snapshot, self.width, self.height) {
            player.handle_event(event);
        }
        player.tick(FloatDuration::from_secs(1.0 / 60.0));
        player.render();
        let renderer =
            <dyn Any>::downcast_mut::<WgpuRenderBackend<TextureTarget>>(player.renderer_mut())
                .ok_or("Unexpected renderer type")?;
        let rgba = renderer.capture_frame().ok_or("GPU frame capture failed")?;
        let mut video: Vec<u32> = rgba.pixels().map(|p| xrgb(p.0)).collect();
        if player.ui().mouse_visible() {
            crate::cursor::draw(
                &mut video,
                self.width,
                self.height,
                self.input.x as u32,
                self.input.y as u32,
                player.mouse_cursor(),
            );
        }
        drop(player);
        self.executor.run();
        let mut audio = vec![0i16; 800 * 2]; // 48,000 Hz / 60 Hz, stereo.
        self.mixer.mix(&mut audio);
        Ok(Frame {
            video,
            audio,
            width: self.width,
            height: self.height,
        })
    }
}

fn xrgb(rgba: [u8; 4]) -> u32 {
    (u32::from(rgba[0]) << 16) | (u32::from(rgba[1]) << 8) | u32::from(rgba[2])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_fixture_has_valid_shapes_sound_and_timing() {
        let bytes = include_bytes!("../tests/fixtures/smoke.swf");
        let buffer = swf::decompress_swf(&bytes[..]).unwrap();
        let movie = swf::parse_swf(&buffer).unwrap();
        assert_eq!(movie.header.frame_rate().to_f64(), 30.0);
        assert_eq!(movie.header.num_frames(), 3);
        assert_eq!(
            movie
                .tags
                .iter()
                .filter(|tag| matches!(tag, swf::Tag::DefineShape(_)))
                .count(),
            3
        );
        assert!(movie
            .tags
            .iter()
            .any(|tag| matches!(tag, swf::Tag::DefineSound(_))));
    }
    #[test]
    fn libretro_pixels_use_native_endian_xrgb() {
        assert_eq!(xrgb([0x12, 0x34, 0x56, 0xff]), 0x00123456);
    }
}
