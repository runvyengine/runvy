use std::sync::Mutex;

/// Timer component
pub struct Timer {
    /// Time to wait
    pub duration: f32,
    /// Automatically start after component awake
    pub autostart: bool,
    /// Automatically restart after timeout
    pub cyclical: bool,
    /// Event
    pub on_timeout: Option<Mutex<Box<dyn FnMut() + Send>>>,

    pub remaining: f32,
    pub running: bool,
    /// Timeout counter
    pub times_fired: u32,
}

impl Timer {
    pub fn new(
        duration: f32,
        autostart: bool,
        cyclical: bool,
        on_timeout: Option<Mutex<Box<dyn FnMut() + Send>>>,
    ) -> Self {
        Self {
            duration,
            autostart,
            cyclical,
            on_timeout,
            remaining: 1.0,
            running: false,
            times_fired: 0,
        }
    }

    pub fn play(&mut self) {
        self.running = true;
    }

    pub fn stop(&mut self) {
        self.running = false;
    }

    pub fn restart(&mut self) {
        self.remaining = self.duration;
        self.play();
    }

    pub fn set_on_timeout(&mut self, on_timeout: Option<Mutex<Box<dyn FnMut() + Send>>>) {
        self.on_timeout = on_timeout;
    }

    /// Return f32 in range from 0.0 to 1.0
    pub fn remaining_fraction(&self) -> f32 {
        (1.0 - self.remaining) / self.duration
    }

    pub fn on_timeout_mut(&mut self) -> Option<&mut Mutex<Box<dyn FnMut() + Send>>> {
        self.on_timeout.as_mut()
    }
}

impl Default for Timer {
    fn default() -> Self {
        Self {
            duration: 1.0,
            autostart: false,
            cyclical: false,
            on_timeout: None,
            remaining: 1.0,
            running: false,
            times_fired: 0,
        }
    }
}
