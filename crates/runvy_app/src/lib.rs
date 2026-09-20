//! Runvy Application Framework
//! Provides ready-to-use game loop and window management

pub use winit;

mod app;
mod runvy_app;
pub use app::RunvyWindowConfig;
pub use runvy_app::RunvyApp;
