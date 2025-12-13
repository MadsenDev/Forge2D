use std::{
    fs::File,
    io::BufReader,
    path::Path,
    sync::{Arc, Mutex},
};

use anyhow::{anyhow, Result};
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};

/// Manages audio playback for sound effects and music.
pub struct AudioSystem {
    _stream: Option<OutputStream>,
    stream_handle: Option<OutputStreamHandle>,
    music_sink: Arc<Mutex<Option<Sink>>>,
    available: bool,
}

impl AudioSystem {
    /// Create a new audio system.
    ///
    /// This initializes the default audio output device.
    /// Returns an error if audio initialization fails.
    pub fn new() -> Result<Self> {
        match OutputStream::try_default() {
            Ok((stream, stream_handle)) => Ok(Self {
                _stream: Some(stream),
                stream_handle: Some(stream_handle),
                music_sink: Arc::new(Mutex::new(None)),
                available: true,
            }),
            Err(e) => {
                log::warn!("Failed to initialize audio: {}. Audio will be unavailable.", e);
                Ok(Self {
                    _stream: None,
                    stream_handle: None,
                    music_sink: Arc::new(Mutex::new(None)),
                    available: false,
                })
            }
        }
    }

    /// Check if audio is available and working.
    pub fn is_available(&self) -> bool {
        self.available
    }

    /// Play a sound effect from a file path.
    ///
    /// The sound will play once and stop automatically.
    /// Multiple sound effects can play simultaneously.
    pub fn play_sound<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let stream_handle = self
            .stream_handle
            .as_ref()
            .ok_or_else(|| anyhow!("Audio system is not available"))?;

        let file = File::open(path.as_ref())
            .map_err(|e| anyhow!("Failed to open sound file {:?}: {}", path.as_ref(), e))?;
        let source = Decoder::new(BufReader::new(file))
            .map_err(|e| anyhow!("Failed to decode sound file {:?}: {}", path.as_ref(), e))?;

        let sink = Sink::try_new(stream_handle)
            .map_err(|e| anyhow!("Failed to create audio sink: {}", e))?;
        sink.append(source);
        sink.detach(); // Let it play and clean up automatically

        Ok(())
    }

    /// Play a sound effect from bytes (useful for embedded assets).
    pub fn play_sound_from_bytes(&self, bytes: &[u8]) -> Result<()> {
        let stream_handle = self
            .stream_handle
            .as_ref()
            .ok_or_else(|| anyhow!("Audio system is not available"))?;

        // Clone bytes to ensure 'static lifetime
        let bytes = bytes.to_vec();
        let cursor = std::io::Cursor::new(bytes);
        let source = Decoder::new(cursor)
            .map_err(|e| anyhow!("Failed to decode sound from bytes: {}", e))?;

        let sink = Sink::try_new(stream_handle)
            .map_err(|e| anyhow!("Failed to create audio sink: {}", e))?;
        sink.append(source);
        sink.detach();

        Ok(())
    }

    /// Play background music from a file path, looping continuously.
    ///
    /// If music is already playing, it will be stopped and replaced.
    pub fn play_music_loop<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let stream_handle = self
            .stream_handle
            .as_ref()
            .ok_or_else(|| anyhow!("Audio system is not available"))?;

        self.stop_music();

        let file = File::open(path.as_ref())
            .map_err(|e| anyhow!("Failed to open music file {:?}: {}", path.as_ref(), e))?;
        let source = Decoder::new(BufReader::new(file))
            .map_err(|e| anyhow!("Failed to decode music file {:?}: {}", path.as_ref(), e))?
            .repeat_infinite();

        let sink = Sink::try_new(stream_handle)
            .map_err(|e| anyhow!("Failed to create audio sink: {}", e))?;
        sink.append(source);
        sink.set_volume(0.5); // Default music volume

        *self.music_sink.lock().unwrap() = Some(sink);
        Ok(())
    }

    /// Play background music from bytes, looping continuously.
    pub fn play_music_loop_from_bytes(&self, bytes: &[u8]) -> Result<()> {
        let stream_handle = self
            .stream_handle
            .as_ref()
            .ok_or_else(|| anyhow!("Audio system is not available"))?;

        self.stop_music();

        // Clone bytes to ensure 'static lifetime
        let bytes = bytes.to_vec();
        let cursor = std::io::Cursor::new(bytes);
        let source = Decoder::new(cursor)
            .map_err(|e| anyhow!("Failed to decode music from bytes: {}", e))?
            .repeat_infinite();

        let sink = Sink::try_new(stream_handle)
            .map_err(|e| anyhow!("Failed to create audio sink: {}", e))?;
        sink.append(source);
        sink.set_volume(0.5);

        *self.music_sink.lock().unwrap() = Some(sink);
        Ok(())
    }

    /// Stop the currently playing background music.
    pub fn stop_music(&self) {
        if let Some(sink) = self.music_sink.lock().unwrap().take() {
            sink.stop();
        }
    }

    /// Set the volume of background music (0.0 to 1.0).
    pub fn set_music_volume(&self, volume: f32) {
        if let Some(sink) = self.music_sink.lock().unwrap().as_ref() {
            sink.set_volume(volume.clamp(0.0, 1.0));
        }
    }

    /// Check if background music is currently playing.
    pub fn is_music_playing(&self) -> bool {
        self.music_sink.lock().unwrap().is_some()
    }
}

// Note: Default implementation is intentionally omitted because AudioSystem::new()
// can fail. Use AudioSystem::new() directly or handle errors appropriately.

