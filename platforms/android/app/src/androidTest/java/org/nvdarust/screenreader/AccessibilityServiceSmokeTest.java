package org.nvdarust.screenreader;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertNotNull;
import static org.junit.Assert.assertTrue;

import android.Manifest;
import android.content.ComponentName;
import android.content.Context;
import android.content.pm.PackageManager;
import android.content.pm.ServiceInfo;
import android.os.Build;
import android.provider.Settings;
import android.view.accessibility.AccessibilityEvent;
import androidx.test.ext.junit.runners.AndroidJUnit4;
import androidx.test.platform.app.InstrumentationRegistry;
import org.junit.Test;
import org.junit.runner.RunWith;

@RunWith(AndroidJUnit4.class)
public final class AccessibilityServiceSmokeTest {
    @Test
    public void serviceIsDeclaredWithAccessibilityPermission() throws Exception {
        Context context = InstrumentationRegistry.getInstrumentation().getTargetContext();
        ComponentName component = new ComponentName(context, ScreenReaderAccessibilityService.class);
        ServiceInfo info = context.getPackageManager().getServiceInfo(component, PackageManager.GET_META_DATA);

        assertNotNull(info);
        assertEquals(Manifest.permission.BIND_ACCESSIBILITY_SERVICE, info.permission);
        assertNotNull(info.metaData);
    }

    @Test
    public void serviceIsEnabledByEmulatorHarness() {
        Context context = InstrumentationRegistry.getInstrumentation().getTargetContext();
        ComponentName component = new ComponentName(context, ScreenReaderAccessibilityService.class);
        String enabled = Settings.Secure.getString(
                context.getContentResolver(),
                Settings.Secure.ENABLED_ACCESSIBILITY_SERVICES);

        assertNotNull("enabled_accessibility_services is null", enabled);
        assertTrue(
                "screen reader service was not enabled by the test harness: " + enabled,
                enabled.contains(component.flattenToString())
                        || enabled.contains(component.flattenToShortString()));
    }

    @Test
    public void passwordEventPresentationNeverLeaksEventText() {
        AccessibilityEvent event = createTestAccessibilityEvent();
        try {
            event.setEventType(AccessibilityEvent.TYPE_VIEW_TEXT_CHANGED);
            event.setPassword(true);
            event.getText().add("super-secret-value");
            event.setContentDescription("super-secret-description");

            String presentation = ScreenReaderAccessibilityService.presentationText(event, null);

            assertEquals("password field", presentation);
            assertTrue(!presentation.contains("super-secret"));
        } finally {
            recycleLegacyTestAccessibilityEvent(event);
        }
    }

    private static AccessibilityEvent createTestAccessibilityEvent() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            return new AccessibilityEvent();
        }
        return obtainLegacyTestAccessibilityEvent();
    }

    @SuppressWarnings("deprecation")
    private static AccessibilityEvent obtainLegacyTestAccessibilityEvent() {
        return AccessibilityEvent.obtain();
    }

    private static void recycleLegacyTestAccessibilityEvent(AccessibilityEvent event) {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.R) {
            recycleLegacyAccessibilityEvent(event);
        }
    }

    @SuppressWarnings("deprecation")
    private static void recycleLegacyAccessibilityEvent(AccessibilityEvent event) {
        event.recycle();
    }
}
