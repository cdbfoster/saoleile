use jni::sys::{jlong, jobject, JNIEnv};

use saoleile::{initialize_app, log};

#[no_mangle]
pub unsafe extern "C" fn Java_com_cdbfoster_saoleile_JNI_onCreate(
    env: *mut JNIEnv,
    _: jobject,
    activity: jobject,
) -> jlong {
    // Make sure we panic nicely
    std::panic::set_hook(Box::new(|panic_info| {
        log!(ERROR, "{}", panic_info.to_string());
    }));

    initialize_app() as jlong
}

#[no_mangle]
pub unsafe extern "C" fn Java_com_cdbfoster_saoleile_JNI_onStart(
    _: *mut JNIEnv,
    _: jobject,
    app_data: jlong,
) {

}

#[no_mangle]
pub unsafe extern "C" fn Java_com_cdbfoster_saoleile_JNI_onResume(
    _: *mut JNIEnv,
    _: jobject,
    app_data: jlong,
) {

}

#[no_mangle]
pub unsafe extern "C" fn Java_com_cdbfoster_saoleile_JNI_onPause(
    _: *mut JNIEnv,
    _: jobject,
    app_data: jlong,
) {

}

#[no_mangle]
pub unsafe extern "C" fn Java_com_cdbfoster_saoleile_JNI_onStop(
    _: *mut JNIEnv,
    _: jobject,
    app_data: jlong,
) {

}

#[no_mangle]
pub unsafe extern "C" fn Java_com_cdbfoster_saoleile_JNI_onDestroy(
    _: *mut JNIEnv,
    _: jobject,
    app_data: jlong,
) {

}