#[macro_use]
pub mod log_functions;

#[cfg(target_os = "android")]
pub fn initialize_app() -> u64 {
    0
}