use crate::audio::audio_engine::SoundId as EngineSoundId;

/// Unique identifier for a playing sound (public handle)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SoundId(pub usize);

/// Handle to a playing sound that allows control over it
#[derive(Clone, Debug)]
pub struct SoundHandle {
    pub id: SoundId,
    pub is_playing: bool,
}

impl SoundHandle {
    pub fn new(id: usize) -> Self {
        Self {
            id: SoundId(id),
            is_playing: true,
        }
    }

    /// Create from engine SoundId
    pub fn from_engine(id: EngineSoundId) -> Self {
        Self {
            id: SoundId(id.0),
            is_playing: true,
        }
    }

    /// Mark the sound as stopped
    pub fn stop(&mut self) {
        self.is_playing = false;
    }
}
