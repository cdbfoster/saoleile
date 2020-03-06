use std::sync::Mutex;

use lazy_static::lazy_static;

#[cfg(target_os = "android")]
use log_sys;

#[macro_export]
macro_rules! log {
    (ERROR, $($arg:tt)*) => { $crate::log::log_error(&format!($($arg)*)) };
    (INFO, $($arg:tt)*) => { $crate::log::log_info(&format!($($arg)*)) };
    (VERBOSE, $($arg:tt)*) => { $crate::log::log_verbose(&format!($($arg)*)) };
    ($($arg:tt)*) => { $crate::log::log_verbose(&format!($($arg)*)) };
}

#[macro_export]
macro_rules! log_level {
    ($l:ident) => { *$crate::log::LOG_LEVEL_EXPLICIT.lock().unwrap() = $crate::log::LogLevel::$l };
}

#[derive(Clone, Copy)]
pub enum LogLevel {
    NONE,
    ERROR,
    INFO,
    VERBOSE,
}

impl LogLevel {
    pub fn value(&self) -> u8 {
        match *self {
            LogLevel::NONE => 3,
            LogLevel::ERROR => 2,
            LogLevel::INFO => 1,
            LogLevel::VERBOSE => 0,
        }
    }
}

lazy_static! {
    pub static ref LOG_LEVEL_EXPLICIT: Mutex<LogLevel> = Mutex::new(LogLevel::VERBOSE);
    static ref LOG_LEVEL: LogLevel = *LOG_LEVEL_EXPLICIT.lock().unwrap();
}

const TAG: &'static str = "saoleile\0";

#[cfg(not(target_os = "android"))]
pub fn log_error(string: &str) {
    if LogLevel::ERROR.value() >= LOG_LEVEL.value() {
        eprintln!("E {}: {}", TAG, string);
    }
}

#[cfg(not(target_os = "android"))]
pub fn log_info(string: &str) {
    if LogLevel::INFO.value() >= LOG_LEVEL.value() {
        println!("I {}: {}", TAG, string);
    }
}

#[cfg(not(target_os = "android"))]
pub fn log_verbose(string: &str) {
    if LogLevel::VERBOSE.value() >= LOG_LEVEL.value() {
        println!("V {}: {}", TAG, string);
    }
}

#[cfg(target_os = "android")]
pub fn log_error(string: &str) {
    unsafe {
        log_sys::__android_log_print(
            log_sys::android_LogPriority_ANDROID_LOG_ERROR as i32,
            TAG.as_ptr(),
            std::ffi::CString::new(string).unwrap().as_ptr(),
        );
    }
}

#[cfg(target_os = "android")]
pub fn log_info(string: &str) {
    unsafe {
        log_sys::__android_log_print(
            log_sys::android_LogPriority_ANDROID_LOG_INFO as i32,
            TAG.as_ptr(),
            std::ffi::CString::new(string).unwrap().as_ptr(),
        );
    }
}

#[cfg(target_os = "android")]
pub fn log_verbose(string: &str) {
    unsafe {
        log_sys::__android_log_print(
            log_sys::android_LogPriority_ANDROID_LOG_VERBOSE as i32,
            TAG.as_ptr(),
            std::ffi::CString::new(string).unwrap().as_ptr(),
        );
    }
}