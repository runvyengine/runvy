//! Audio asset loading and caching
use rodio::Source;
use std::io::Cursor;
use std::num::NonZero;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AudioLoadError {
    #[error("File not found: {0}")]
    NotFound(String),
    #[error("Invalid audio format: {0}")]
    InvalidFormat(String),
    #[error("Decoding failed: {0}")]
    DecodeFailed(String),
}

/// Audio asset — decoded PCM samples ready for playback
#[derive(Clone)]
pub struct AudioAsset {
    /// Decoded PCM samples (i16 stereo)
    pub samples: Arc<Vec<i16>>,
    /// Sample rate (Hz)
    pub sample_rate: u32,
    /// Number of channels (1 = mono, 2 = stereo)
    pub channels: u16,
    /// Original file path (for debugging)
    pub path: String,
}

/// Source for playing back an AudioAsset
pub struct AudioSource {
    samples: Arc<Vec<i16>>,
    sample_rate: NonZero<u32>,
    channels: NonZero<u16>,
    index: usize,
}

impl AudioSource {
    pub fn new(asset: &AudioAsset) -> Self {
        Self {
            samples: asset.samples.clone(),
            sample_rate: NonZero::new(asset.sample_rate).unwrap_or(NonZero::new(44100).unwrap()),
            channels: NonZero::new(asset.channels).unwrap_or(NonZero::new(2).unwrap()),
            index: 0,
        }
    }
}

impl Iterator for AudioSource {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.samples.len() {
            return None;
        }
        let sample = self.samples[self.index] as f32 / i16::MAX as f32;
        self.index += 1;
        Some(sample)
    }
}

impl Source for AudioSource {
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> NonZero<u16> {
        self.channels
    }

    fn sample_rate(&self) -> NonZero<u32> {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        Some(Duration::from_secs_f64(
            self.samples.len() as f64
                / (self.sample_rate.get() as f64 * self.channels.get() as f64),
        ))
    }
}

impl AudioAsset {
    /// Load and decode audio file (OGG/WAV)
    pub fn from_file(base_dir: &str, path: &str) -> Result<Self, AudioLoadError> {
        let full_path = std::path::PathBuf::from(base_dir).join(path);
        let data = std::fs::read(&full_path)
            .map_err(|e| AudioLoadError::NotFound(format!("{:?}: {}", full_path, e)))?;

        // Decode using rodio
        let cursor = Cursor::new(data);
        let decoder =
            rodio::Decoder::new(cursor).map_err(|e| AudioLoadError::DecodeFailed(e.to_string()))?;

        let sample_rate = decoder.sample_rate().get();
        let channels = decoder.channels().get();

        // Decoder implements Iterator<Item = f32> in rodio 0.22
        // Convert f32 samples to i16
        let samples: Vec<i16> = decoder
            .into_iter()
            .map(|s| (s * i16::MAX as f32) as i16)
            .collect();

        Ok(Self {
            samples: Arc::new(samples),
            sample_rate,
            channels,
            path: path.to_string(),
        })
    }

    /// Create source for playback (cheap clone)
    pub fn create_source(&self) -> impl Source<Item = f32> + Send + 'static {
        AudioSource::new(self)
    }
}
