package org.nvdarust.screenreader;

import android.Manifest;
import android.content.ComponentName;
import android.content.Context;
import android.content.pm.PackageManager;
import android.content.pm.ServiceInfo;
import android.provider.Settings;
import android.test.InstrumentationTestCase;

public final class AccessibilityServiceSmokeTest extends InstrumentationTestCase {
    public void testServiceIsDeclaredWithAccessibilityPermission() throws Exception {
        Context context = getInstrumentation().getTargetContext();
        ComponentName component = new ComponentName(context, ScreenReaderAccessibilityService.class);
        ServiceInfo info = context.getPackageManager().getServiceInfo(component, PackageManager.GET_META_DATA);

        assertNotNull(info);
        assertEquals(Manifest.permission.BIND_ACCESSIBILITY_SERVICE, info.permission);
        assertNotNull(info.metaData);
    }

    public void testServiceIsEnabledByEmulatorHarness() {
        Context context = getInstrumentation().getTargetContext();
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
}
