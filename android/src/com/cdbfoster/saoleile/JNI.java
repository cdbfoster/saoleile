package com.cdbfoster.saoleile;

import android.app.Activity;
import android.view.Surface;

public class JNI {
    public static native long onCreate(Activity activity);
    public static native void onStart(long appData);
    public static native void onResume(long appData);
    public static native void onPause(long appData);
    public static native void onStop(long appData);
    public static native void onDestroy(long appData);
}