package org.nvdarust.screenreader;

import android.accessibilityservice.AccessibilityService;
import android.accessibilityservice.AccessibilityServiceInfo;
import android.graphics.Rect;
import android.speech.tts.TextToSpeech;
import android.util.Log;
import android.view.KeyEvent;
import android.view.accessibility.AccessibilityEvent;
import android.view.accessibility.AccessibilityNodeInfo;
import java.util.List;
import java.util.Locale;

public final class ScreenReaderAccessibilityService extends AccessibilityService {
    public static final String TAG = "RustScreenReader";

    private TextToSpeech textToSpeech;
    private boolean speechReady;

    @Override
    protected void onServiceConnected() {
        super.onServiceConnected();

        AccessibilityServiceInfo info = getServiceInfo();
        if (info != null) {
            info.eventTypes = AccessibilityEvent.TYPES_ALL_MASK;
            info.feedbackType = AccessibilityServiceInfo.FEEDBACK_SPOKEN
                    | AccessibilityServiceInfo.FEEDBACK_HAPTIC
                    | AccessibilityServiceInfo.FEEDBACK_BRAILLE;
            info.flags |= AccessibilityServiceInfo.FLAG_REPORT_VIEW_IDS
                    | AccessibilityServiceInfo.FLAG_RETRIEVE_INTERACTIVE_WINDOWS
                    | AccessibilityServiceInfo.FLAG_REQUEST_FILTER_KEY_EVENTS
                    | AccessibilityServiceInfo.FLAG_REQUEST_TOUCH_EXPLORATION_MODE;
            info.notificationTimeout = 20;
            setServiceInfo(info);
        }

        textToSpeech = new TextToSpeech(this, status -> {
            speechReady = status == TextToSpeech.SUCCESS;
            if (speechReady) {
                int languageResult = textToSpeech.setLanguage(Locale.getDefault());
                Log.i(TAG, "TTS_READY languageResult=" + languageResult);
            } else {
                Log.w(TAG, "TTS_INIT_FAILED status=" + status);
            }
        });

        Log.i(TAG, "SERVICE_CONNECTED");
        Log.i(TAG, "WINDOW_COUNT=" + getWindows().size());
    }

    @Override
    public void onAccessibilityEvent(AccessibilityEvent event) {
        if (event == null) {
            return;
        }

        AccessibilityNodeInfo source = event.getSource();
        logEvent(event, source);

        int type = event.getEventType();
        if (type == AccessibilityEvent.TYPE_VIEW_ACCESSIBILITY_FOCUSED
                || type == AccessibilityEvent.TYPE_VIEW_FOCUSED) {
            speakFocusedNode(event, source);
        }
    }

    private void logEvent(AccessibilityEvent event, AccessibilityNodeInfo source) {
        CharSequence packageName = event.getPackageName();
        CharSequence className = event.getClassName();
        boolean password = event.isPassword();

        String viewId = source == null ? "" : valueOf(source.getViewIdResourceName());
        String nodeClass = source == null ? "" : valueOf(source.getClassName());
        String nodeText = source == null ? "" : valueOf(source.getText());
        String nodeDescription = source == null ? "" : valueOf(source.getContentDescription());
        String bounds = "";
        boolean editable = false;
        boolean checkable = false;
        boolean checked = false;
        boolean selected = false;
        boolean enabled = false;

        if (source != null) {
            Rect rect = new Rect();
            source.getBoundsInScreen(rect);
            bounds = rect.flattenToString();
            editable = source.isEditable();
            password |= source.isPassword();
            checkable = source.isCheckable();
            checked = source.isChecked();
            selected = source.isSelected();
            enabled = source.isEnabled();
        }

        String eventText = password ? "<password>" : valueOf(event.getText());
        String eventDescription = password ? "<password>" : valueOf(event.getContentDescription());
        String safeNodeText = password ? "<password>" : nodeText;
        String safeNodeDescription = password ? "<password>" : nodeDescription;

        Log.i(
                TAG,
                "EVENT type=" + event.getEventType()
                        + " eventName=" + AccessibilityEvent.eventTypeToString(event.getEventType())
                        + " package=" + packageName
                        + " class=" + className
                        + " nodeClass=" + nodeClass
                        + " viewId=" + viewId
                        + " text=" + eventText
                        + " nodeText=" + safeNodeText
                        + " description=" + eventDescription
                        + " nodeDescription=" + safeNodeDescription
                        + " editable=" + editable
                        + " password=" + password
                        + " checkable=" + checkable
                        + " checked=" + checked
                        + " selected=" + selected
                        + " enabled=" + enabled
                        + " bounds=" + bounds
                        + " windows=" + getWindows().size());
    }

    private void speakFocusedNode(AccessibilityEvent event, AccessibilityNodeInfo source) {
        if (!speechReady || textToSpeech == null) {
            return;
        }

        String text = presentationText(event, source);
        if (text.isEmpty()) {
            return;
        }

        String utteranceId = "focus-" + event.getEventTime();
        int result = textToSpeech.speak(text, TextToSpeech.QUEUE_FLUSH, null, utteranceId);
        Log.i(TAG, "SPEAK result=" + result + " text=" + text);
    }

    static String presentationText(AccessibilityEvent event, AccessibilityNodeInfo source) {
        if (event.isPassword() || (source != null && source.isPassword())) {
            return "password field";
        }

        if (source != null) {
            String description = valueOf(source.getContentDescription()).trim();
            if (!description.isEmpty() && !"null".equals(description)) {
                return description;
            }

            String nodeText = valueOf(source.getText()).trim();
            if (!nodeText.isEmpty() && !"null".equals(nodeText)) {
                return nodeText;
            }
        }

        CharSequence eventDescription = event.getContentDescription();
        if (eventDescription != null && eventDescription.length() > 0) {
            return eventDescription.toString();
        }

        List<CharSequence> eventText = event.getText();
        if (eventText != null && !eventText.isEmpty()) {
            StringBuilder builder = new StringBuilder();
            for (CharSequence item : eventText) {
                if (item == null || item.length() == 0) {
                    continue;
                }
                if (builder.length() > 0) {
                    builder.append(' ');
                }
                builder.append(item);
            }
            return builder.toString();
        }

        return "";
    }

    private static String valueOf(CharSequence value) {
        return value == null ? "" : value.toString();
    }

    private static String valueOf(Object value) {
        return value == null ? "" : value.toString();
    }

    @Override
    protected boolean onKeyEvent(KeyEvent event) {
        if (event != null) {
            Log.i(
                    TAG,
                    "KEY action=" + event.getAction()
                            + " code=" + event.getKeyCode()
                            + " meta=" + event.getMetaState());
        }
        return false;
    }

    @Override
    public void onInterrupt() {
        if (textToSpeech != null) {
            textToSpeech.stop();
        }
        Log.i(TAG, "SERVICE_INTERRUPTED");
    }

    @Override
    public void onDestroy() {
        if (textToSpeech != null) {
            textToSpeech.stop();
            textToSpeech.shutdown();
            textToSpeech = null;
        }
        speechReady = false;
        Log.i(TAG, "SERVICE_DESTROYED");
        super.onDestroy();
    }
}
