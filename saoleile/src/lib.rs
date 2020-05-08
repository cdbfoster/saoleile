#[macro_use]
pub mod log;

// XXX Fix visibility
pub mod component;
pub mod context;
pub mod event;
pub mod layer;
pub mod network;
pub mod scene;
pub mod util;

#[cfg(target_os = "android")]
pub fn initialize_app() -> u64 {
    0
}