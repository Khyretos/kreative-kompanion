package com.kreativekompas.kompanion;

import android.app.Activity;
import android.content.ActivityNotFoundException;
import android.content.ComponentName;
import android.content.Intent;
import android.content.SharedPreferences;
import android.graphics.Typeface;
import android.net.Uri;
import android.os.Build;
import android.os.Bundle;
import android.provider.Settings;
import android.view.View;
import android.widget.Button;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.Switch;
import android.widget.TextView;
import java.util.ArrayList;
import java.util.List;
import java.util.Locale;

/** Notification settings: the own-connection switch and the battery page of phones that stop background apps. */
public class SettingsActivity extends Activity {
    private static final int WHITE = 0xFFFFFFFF;
    private static final int MIST = 0xFFF4EEFC;

    private LinearLayout col;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        ScrollView scroll = new ScrollView(this);
        scroll.setBackgroundColor(0xFF0C0917);
        col = new LinearLayout(this);
        col.setOrientation(LinearLayout.VERTICAL);
        col.setPadding(dp(24), dp(24), dp(24), dp(24));
        scroll.addView(col);
        setContentView(scroll);

        text(getString(R.string.settings_title), WHITE, 24, true, 0);

        SharedPreferences prefs = getSharedPreferences("settings", MODE_PRIVATE);
        Switch live = new Switch(this);
        live.setText(R.string.settings_live);
        live.setTextColor(MIST);
        live.setTextSize(18);
        live.setMinHeight(dp(48));
        live.setChecked(prefs.getBoolean("live", true));
        live.setOnCheckedChangeListener((v, on) -> {
            prefs.edit().putBoolean("live", on).apply();
            if (on) {
                LiveService.startIfWanted(this);
            } else {
                LiveService.stop(this);
            }
        });
        add(live, 16);
        text(getString(R.string.settings_live_hint), MIST, 14, false, 0);

        text(getString(R.string.battery_title), WHITE, 20, true, 32);
        text(getString(R.string.battery_text, Build.MANUFACTURER), MIST, 16, false, 0);
        // Dark text on orange: white on orange fails contrast.
        button(R.string.battery_open, 0xFFF3941F, 0xFF0C0917, v -> openBatterySettings());
        button(R.string.settings_done, 0xFF2C1F3F, WHITE, v -> finish());
    }

    private int dp(int v) {
        return Math.round(v * getResources().getDisplayMetrics().density);
    }

    private void add(View v, int topDp) {
        LinearLayout.LayoutParams lp = new LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT, LinearLayout.LayoutParams.WRAP_CONTENT);
        lp.topMargin = dp(topDp);
        col.addView(v, lp);
    }

    private void text(String s, int color, int sp, boolean bold, int topDp) {
        TextView t = new TextView(this);
        t.setText(s);
        t.setTextColor(color);
        t.setTextSize(sp);
        if (bold) t.setTypeface(Typeface.DEFAULT_BOLD);
        add(t, topDp);
    }

    private void button(int label, int bg, int fg, View.OnClickListener click) {
        Button b = new Button(this);
        b.setText(label);
        b.setAllCaps(false);
        b.setBackgroundColor(bg);
        b.setTextColor(fg);
        b.setMinHeight(dp(48));
        b.setOnClickListener(click);
        add(b, 12);
    }

    /** The maker's autostart page when it has one, else the app's own system settings. */
    private void openBatterySettings() {
        String m = Build.MANUFACTURER.toLowerCase(Locale.ROOT);
        List<Intent> tries = new ArrayList<>();
        if (m.equals("xiaomi") || m.equals("redmi") || m.equals("poco")) {
            tries.add(component("com.miui.securitycenter", "com.miui.permcenter.autostart.AutoStartManagementActivity"));
        } else if (m.equals("huawei") || m.equals("honor")) {
            tries.add(component("com.huawei.systemmanager", "com.huawei.systemmanager.startupmgr.ui.StartupNormalAppListActivity"));
        } else if (m.equals("oppo") || m.equals("realme") || m.equals("oneplus")) {
            tries.add(component("com.coloros.safecenter", "com.coloros.safecenter.permission.startup.StartupAppListActivity"));
        } else if (m.equals("vivo")) {
            tries.add(component("com.vivo.permissionmanager", "com.vivo.permissionmanager.activity.BgStartUpManagerActivity"));
        }
        tries.add(new Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS, Uri.parse("package:" + getPackageName())));
        for (Intent i : tries) {
            try {
                startActivity(i);
                return;
            } catch (ActivityNotFoundException | SecurityException e) {
                // next
            }
        }
    }

    private static Intent component(String pkg, String cls) {
        return new Intent().setComponent(new ComponentName(pkg, cls));
    }

    public static boolean restrictiveMaker() {
        switch (Build.MANUFACTURER.toLowerCase(Locale.ROOT)) {
            case "xiaomi": case "redmi": case "poco": case "huawei": case "honor":
            case "oppo": case "realme": case "oneplus": case "vivo":
                return true;
            default:
                return false;
        }
    }

    /** First start on a phone that stops background apps: show this screen once. */
    public static void showOnceForMaker(Activity a) {
        SharedPreferences prefs = a.getSharedPreferences("settings", MODE_PRIVATE);
        if (restrictiveMaker() && !prefs.getBoolean("maker_shown", false)) {
            prefs.edit().putBoolean("maker_shown", true).apply();
            a.startActivity(new Intent(a, SettingsActivity.class));
        }
    }
}
