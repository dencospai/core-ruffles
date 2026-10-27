use ruffle_core::backend::audio::{
    AudioBackend, AudioMixer, AudioMixerProxy, DecodeError, RegisterError, SoundHandle,
    SoundInstanceHandle, SoundStreamInfo, SoundTransform,
};

pub struct RetroAudio {
    mixer: AudioMixer,
}

impl RetroAudio {
    pub fn new() -> (Self, AudioMixerProxy) {
        let mixer = AudioMixer::new(2, 48_000);
        let proxy = mixer.proxy();
        (Self { mixer }, proxy)
    }
}

impl AudioBackend for RetroAudio {
    ruffle_core::impl_audio_mixer_backend!(mixer);

    // The frontend drives the clock. No OS audio device or worker is started.
    fn play(&mut self) {}
    fn pause(&mut self) {}

    fn position_resolution(&self) -> Option<std::time::Duration> {
        Some(std::time::Duration::from_secs_f64(1.0 / 60.0))
    }
}
