package org.nvdarust.screenreader;

import android.app.Activity;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.text.InputType;
import android.view.accessibility.AccessibilityEvent;
import android.widget.Button;
import android.widget.EditText;
import android.widget.LinearLayout;
import android.widget.TextView;

/** Debug-only UI used by CI to generate deterministic framework accessibility events. */
public final class AccessibilityFixtureActivity extends Activity {
    public static final String PASSWORD_SENTINEL = "android-runtime-secret";

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);

        LinearLayout root = new LinearLayout(this);
        root.setOrientation(LinearLayout.VERTICAL);
        root.setPadding(32, 32, 32, 32);

        TextView heading = new TextView(this);
        heading.setText("Accessibility runtime fixture");
        heading.setContentDescription("Accessibility runtime fixture heading");
        root.addView(heading);

        Button button = new Button(this);
        button.setText("Runtime accessibility action");
        button.setContentDescription("Runtime accessibility action button");
        root.addView(button);

        EditText password = new EditText(this);
        password.setHint("Password fixture");
        password.setInputType(
                InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_VARIATION_PASSWORD);
        password.setText(PASSWORD_SENTINEL);
        root.addView(password);

        setContentView(root);

        new Handler(Looper.getMainLooper()).postDelayed(() -> {
            button.requestFocus();
            button.sendAccessibilityEvent(AccessibilityEvent.TYPE_VIEW_FOCUSED);
            button.performClick();

            password.requestFocus();
            password.sendAccessibilityEvent(AccessibilityEvent.TYPE_VIEW_FOCUSED);
            password.sendAccessibilityEvent(AccessibilityEvent.TYPE_VIEW_TEXT_CHANGED);
        }, 1000);
    }
}
