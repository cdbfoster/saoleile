#[macro_use]
pub mod log_functions;

pub mod component;
pub mod context;
pub mod entity;
pub mod event;
pub mod layer;
pub mod network;
pub mod util;

#[cfg(target_os = "android")]
pub fn initialize_app() -> u64 {
    0
}