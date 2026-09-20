//! Regenerates `crates/runvy_script_api/scripts/runvy_base.luau` — the engine's
//! committed built-in Luau type layer.
//!
//! The binary links the whole engine (`runvy_engine` → `runvy_core` →
//! `runvy_render_api`), so `inventory` picks up every `#[script(builtin)]` type and
//! the aux types registered by the engine crates. Run it whenever a built-in
//! component / aux type changes:
//!
//! ```text
//! cargo run -p runvy_engine --bin gen_runvy_base
//! ```
//!
//! The generated file is committed so user projects only need `include_str!`
//! (via `runvy_script_api`) and never the engine on disk.

use std::path::PathBuf;

use runvy_engine::scripting_api;

fn main() {
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../runvy_script_api/scripts/runvy_base.luau");
    let base = out.parent().unwrap_or(&out);
    if !base.exists() {
        std::fs::create_dir_all(base).expect("create scripts/ dir");
    }
    scripting_api::write_runvy_base(&out);
    eprintln!("wrote {}", out.display());
}
