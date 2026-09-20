// #![windows_subsystem = "windows"]

mod content_browser;
mod editor_app;
mod editor_camera;
mod editor_settings;
mod editor_textures;
mod inspector;
mod style;

use std::path::PathBuf;

fn main() -> Result<(), winit::error::EventLoopError> {
    editor_app::run(parse_project_arg())
}

fn parse_project_arg() -> Option<PathBuf> {
    let mut args = std::env::args_os().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--project" {
            return args.next().map(PathBuf::from);
        }
    }

    None
}
