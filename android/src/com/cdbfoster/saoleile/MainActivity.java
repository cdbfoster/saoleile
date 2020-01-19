package com.cdbfoster.saoleile;

import android.app.Activity;
import android.os.Bundle;
import android.util.Log;

public class MainActivity extends Activity {
    static {
        System.loadLibrary("jni_wrapper");
    }

    private static final String TAG = "saoleile";

    @Override
    public void onCreate(Bundle savedInstanceState) {
        Log.v(TAG, "MainActivity::onCreate() ==========");
        super.onCreate(savedInstanceState);
        appData = JNI.onCreate(this);
    }

    @Override
    public void onStart() {
        Log.v(TAG, "MainActivity::onStart()");
        super.onStart();
        JNI.onStart(appData);
    }

    @Override
    public void onResume() {
        Log.v(TAG, "MainActivity::onResume()");
        super.onResume();
        JNI.onResume(appData);
    }

    @Override
    public void onPause() {
        Log.v(TAG, "MainActivity::onPause()");
        super.onPause();
        JNI.onPause(appData);
    }

    @Override
    public void onStop() {
        Log.v(TAG, "MainActivity::onStop()");
        super.onStop();
        JNI.onStop(appData);
        this.finish();
    }

    @Override
    public void onDestroy() {
        Log.v(TAG, "MainActivity::onDestroy()");
        JNI.onDestroy(appData);
        super.onDestroy();
        appData = 0;
    }

    private long appData;
}
