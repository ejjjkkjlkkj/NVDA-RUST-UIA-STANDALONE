package org.nvdarust.screenreader;

import android.accessibilityservice.AccessibilityService;
import android.util.Log;
import android.view.accessibility.AccessibilityEvent;
import android.view.accessibility.AccessibilityNodeInfo;

public final class ScreenReaderAccessibilityService extends AccessibilityService {
    public static final String TAG = "RustScreenReader";

    @Override
    protected void onServiceConnected() {
        super.onServiceConnected();
        Log.i(TAG, "SERVICE_CONNECTED");
    }

    @Override
    public void onAccessibilityEvent(AccessibilityEvent event) {
        if (event == null) {
            return;
        }

        AccessibilityNodeInfo source = event.getSource();
        CharSequence packageName = event.getPackageName();
        CharSequence className = event.getClassName();
        CharSequence text = event.getText() == null ? "" : event.getText().toString();
        CharSequence description = event.getContentDescription();

        String viewId = source == null ? "" : String.valueOf(source.getViewIdResourceName());
        String nodeClass = source == null ? "" : String.valueOf(source.getClassName());

        Log.i(
                TAG,
                "EVENT type=" + event.getEventType()
                        + " package=" + packageName
                        + " class=" + className
                        + " nodeClass=" + nodeClass
                        + " viewId=" + viewId
                        + " text=" + text
                        + " description=" + description);
    }

    @Override
    public void onInterrupt() {
        Log.i(TAG, "SERVICE_INTERRUPTED");
    }
}
