package com.kreativekompas.kompanion;

import android.content.ComponentName;
import android.content.Intent;
import android.os.Build;

import java.util.Arrays;
import java.util.List;
import java.util.Locale;

/**
 * Phone makers that stop background apps, and their autostart pages.
 */
public final class Makers {
    /** Makers that stop background apps unless the user allows autostart. */
    private static final List<String> RESTRICTIVE = Arrays.asList("xiaomi",
        "redmi", "poco", "huawei", "honor", "oppo", "realme", "oneplus",
        "vivo");

    private Makers() {
        // Only static helpers.
    }

    /**
     * The phone maker in lower case.
     *
     * @return for example "xiaomi"
     */
    private static String maker() {
        return Build.MANUFACTURER.toLowerCase(Locale.ROOT);
    }

    /**
     * The autostart settings page of this phone's maker, if it has one.
     *
     * @return the page's intent, or null
     */
    public static Intent autostartPage() {
        final String maker = maker();
        final Intent page;
        if ("xiaomi".equals(maker) || "redmi".equals(maker)
            || "poco".equals(maker)) {
            page = component("com.miui.securitycenter",
                "com.miui.permcenter.autostart.AutoStartManagementActivity");
        } else if ("huawei".equals(maker) || "honor".equals(maker)) {
            page = component("com.huawei.systemmanager",
                "com.huawei.systemmanager.startupmgr.ui."
                    + "StartupNormalAppListActivity");
        } else if ("oppo".equals(maker) || "realme".equals(maker)
            || "oneplus".equals(maker)) {
            page = component("com.coloros.safecenter",
                "com.coloros.safecenter.permission.startup."
                    + "StartupAppListActivity");
        } else if ("vivo".equals(maker)) {
            page = component("com.vivo.permissionmanager",
                "com.vivo.permissionmanager.activity."
                    + "BgStartUpManagerActivity");
        } else {
            page = null;
        }
        return page;
    }

    /**
     * An intent for one activity of another app.
     *
     * @param pkg the app's package
     * @param cls the activity's class name
     * @return the intent
     */
    private static Intent component(final String pkg, final String cls) {
        return new Intent().setComponent(new ComponentName(pkg, cls));
    }

    /**
     * Whether this phone's maker stops background apps.
     *
     * @return true for Xiaomi, Huawei, Oppo, Vivo and their brands
     */
    public static boolean restrictive() {
        return RESTRICTIVE.contains(maker());
    }
}
