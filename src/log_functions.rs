#[cfg(target_os = "android")]
use log_sys;

#[macro_export]
macro_rules! log {
    (ERROR, $($arg:tt)*) => { $crate::log_functions::log_error(&format!($($arg)*)) };
    (INFO, $($arg:tt)*) => { $crate::log_functions::log_info(&format!($($arg)*)) };
    (VERBOSE, $($arg:tt)*) => { $crate::log_functions::log_verbose(&format!($($arg)*)) };
    ($($arg:tt)*) => { $crate::log_functions::log_verbose(&format!($($arg)*)) };
}

const TAG: &'static str = "saoleile\0";

#[cfg(not(target_os = "android"))]
pub fn log_error(string: &str) {
    eprintln!("E {}: {}", TAG, string);
}

#[cfg(not(target_os = "android"))]
pub fn log_info(string: &str) {
    println!("I {}: {}", TAG, string);
}

#[cfg(not(target_os = "android"))]
pub fn log_verbose(string: &str) {
    println!("V {}: {}", TAG, string);
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