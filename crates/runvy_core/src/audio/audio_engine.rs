use crate::components::AudioSource;
use glam::{Quat, Vec3};
use rodio::source::Source;
use rodio::{DeviceSinkBuilder, MixerDeviceSink, Player};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SoundId(pub usize);

/// Stored information about a playing sound
pub struct PlayingSound {
    player: Arc<Player>,
    position: Option<Vec3>, // None for 2D sounds
    is_spatial: bool,
    base_volume: f32,
}

/// Audio engine resource — manages all audio playback
pub struct AudioEngine {
    stream: Option<MixerDeviceSink>,
    sounds: HashMap<SoundId, PlayingSound>,
    next_id: usize,
    master_volume: f32,
    /// Position of the active listener (for 3D audio)
    listener_position: Vec3,
    /// Right direction of the listener (for stereo panning)
    listener_right: Vec3,
    /// Volume of the active listener
    listener_volume: f32,
    /// Stereo separation factor (0.0 = mono, 1.0 = full stereo panning)
    stereo_separation: f32,
}

impl AudioEngine {
    pub fn new() -> Self {
        Self {
            stream: None,
            sounds: HashMap::new(),
            next_id: 0,
            master_volume: 1.0,
            listener_position: Vec3::ZERO,
            listener_right: Vec3::X, // Right is +X
            listener_volume: 1.0,
            stereo_separation: 1.0, // Full stereo by default
        }
    }

    /// Initialize audio output (call once at startup)
    pub fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut stream = DeviceSinkBuilder::open_default_sink()?;
        stream.log_on_drop(false); // Suppress the drop warning
        self.stream = Some(stream);
        Ok(())
    }

    /// Set the listener position and rotation for 3D audio
    ///
    /// Note: Only one listener is supported at a time. If multiple AudioListener
    /// components exist in the world, only the first active one is used.
    pub fn set_listener(&mut self, position: Vec3, rotation: Quat, volume: f32) {
        self.listener_position = position;
        self.listener_volume = volume.clamp(0.0, 1.0);

        // Calculate right vector from rotation (for stereo panning)
        self.listener_right = rotation * Vec3::X;
    }

    /// Set stereo separation factor (0.0 = mono, 1.0 = full stereo panning)
    pub fn set_stereo_separation(&mut self, separation: f32) {
        self.stereo_separation = separation.clamp(0.0, 1.0);
    }

    /// Get current stereo separation factor
    pub fn stereo_separation(&self) -> f32 {
        self.stereo_separation
    }

    /// Calculate volume and stereo pan based on position
    fn calculate_spatial_audio(&self, base_volume: f32, sound_position: Vec3) -> (f32, f32) {
        // Returns (left_volume, right_volume)

        let distance = (sound_position - self.listener_position).length();

        // Distance attenuation (inverse square law with clamping)
        let distance_factor = if distance <= 1.0 {
            1.0
        } else {
            1.0 / (distance * distance)
        };
        let distance_volume = distance_factor.clamp(0.0, 1.0);

        // Stereo panning
        let to_sound = (sound_position - self.listener_position).normalize_or_zero();
        let pan = to_sound.dot(self.listener_right);

        // pan = -1.0 (full left), pan = 0.0 (center), pan = 1.0 (full right)
        // Apply stereo separation factor
        let pan_scaled = pan * self.stereo_separation;

        // Calculate left/right volumes
        // When pan_scaled = -1: left=1.0, right=0.0
        // When pan_scaled = 0: left=1.0, right=1.0
        // When pan_scaled = 1: left=0.0, right=1.0
        let left_pan = (1.0 - pan_scaled).clamp(0.0, 1.0);
        let right_pan = (1.0 + pan_scaled).clamp(0.0, 1.0);

        let final_volume =
            base_volume * distance_volume * self.listener_volume * self.master_volume;

        (left_pan * final_volume, right_pan * final_volume)
    }

    /// Play audio source (2D sound, no spatial positioning)
    pub fn play(&mut self, audio_source: &AudioSource) -> Option<SoundId> {
        // For 2D sounds, use center position
        self.play_spatial(audio_source, Some(Vec3::ZERO))
    }

    /// Play audio source with 3D spatial positioning
    ///
    /// Parameters:
    /// - `audio_source`: The audio source to play
    /// - `sound_position`: Position of the sound in world space
    pub fn play_spatial(
        &mut self,
        audio_source: &AudioSource,
        sound_position: Option<Vec3>,
    ) -> Option<SoundId> {
        let stream = self.stream.as_ref()?;
        let asset = audio_source.audio_asset.as_ref()?;

        let position = sound_position.unwrap_or(Vec3::ZERO);

        // Calculate initial stereo volumes
        let (left_vol, right_vol) = self.calculate_spatial_audio(audio_source.volume, position);

        // Create player - use average volume
        // Note: rodio Player doesn't support true stereo panning
        // We use average volume as a compromise
        let player = Player::connect_new(stream.mixer());
        player.set_volume((left_vol + right_vol) / 2.0);

        // Create source from cached PCM samples
        let source = asset.create_source();

        // Loop if needed
        if audio_source.looped {
            player.append(source.repeat_infinite());
        } else {
            player.append(source);
        }

        // Store player with position info
        let id = SoundId(self.next_id);
        self.next_id += 1;
        self.sounds.insert(
            id,
            PlayingSound {
                player: Arc::new(player),
                position: sound_position,
                is_spatial: audio_source.spatial,
                base_volume: audio_source.volume,
            },
        );

        Some(id)
    }

    /// Update spatial sounds - update volume based on listener position
    pub fn update_spatial_volumes(&mut self) {
        let listener_pos = self.listener_position;
        let listener_volume = self.listener_volume;
        let master_volume = self.master_volume;
        let listener_right = self.listener_right;
        let stereo_separation = self.stereo_separation;

        // Collect volume updates first to avoid borrow conflicts
        let mut volume_updates: Vec<(SoundId, f32)> = Vec::new();

        for (&id, sound) in &self.sounds {
            if sound.is_spatial {
                if let Some(pos) = sound.position {
                    let distance = (pos - listener_pos).length();

                    // Distance attenuation
                    let distance_factor = if distance <= 1.0 {
                        1.0
                    } else {
                        1.0 / (distance * distance)
                    };
                    let distance_volume = distance_factor.clamp(0.0, 1.0);

                    // Stereo panning
                    let to_sound = (pos - listener_pos).normalize_or_zero();
                    let pan = to_sound.dot(listener_right);

                    // Apply stereo separation factor
                    let pan_scaled = pan * stereo_separation;

                    // Calculate left/right volumes
                    let left_pan = (1.0 - pan_scaled).clamp(0.0, 1.0);
                    let right_pan = (1.0 + pan_scaled).clamp(0.0, 1.0);

                    let final_volume =
                        sound.base_volume * distance_volume * listener_volume * master_volume;

                    // Use average volume (rodio Player limitation)
                    let avg_volume = ((left_pan + right_pan) / 2.0) * final_volume;

                    volume_updates.push((id, avg_volume));
                }
            }
        }

        // Apply volume updates
        for (id, new_volume) in volume_updates {
            if let Some(sound) = self.sounds.get(&id) {
                sound.player.set_volume(new_volume);
            }
        }
    }

    /// Stop sound by ID
    pub fn stop(&mut self, id: SoundId) {
        if let Some(sound) = self.sounds.remove(&id) {
            sound.player.stop();
        }
    }

    /// Set master volume
    pub fn set_master_volume(&mut self, volume: f32) {
        self.master_volume = volume.clamp(0.0, 1.0);
    }

    /// Cleanup finished sounds
    pub fn cleanup(&mut self) {
        self.sounds.retain(|_, sound| !sound.player.empty());
    }

    /// Get number of active sounds
    pub fn active_sounds(&self) -> usize {
        self.sounds.len()
    }

    /// Get current listener position
    pub fn listener_position(&self) -> Vec3 {
        self.listener_position
    }
}

impl Default for AudioEngine {
    fn default() -> Self {
        Self::new()
    }
}
