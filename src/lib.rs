#[macro_use]
pub mod log_functions;

pub mod context;
pub mod event;
pub mod layer;
mod util;

#[cfg(target_os = "android")]
pub fn initialize_app() -> u64 {
    0
}