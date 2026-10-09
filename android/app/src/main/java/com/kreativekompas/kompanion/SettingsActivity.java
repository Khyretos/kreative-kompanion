package com.kreativekompas.kompanion;

import android.app.Activity;
import android.content.ActivityNotFoundException;
import android.content.Intent;
import android.content.SharedPreferences;
import android.graphics.Typeface;
import android.net.Uri;
import android.os.Build;
import android.os.Bundle;
import android.provider.Settings;
import android.view.View;
import android.widget.Button;
import android.widget.CompoundButton;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.Switch;
import android.widget.TextView;

import java.util.ArrayList;
import java.util.Iterator;
import java.util.List;

/**
 * Settings: the own-connection switch, the battery page of phones that stop
 * background apps, and the server the app uses.
 */
public final class SettingsActivity extends Activity {
    /** Kompas white, headings. */
    private static final int WHITE = 0xFFFFFFFF;
    /** Kompas mist, body text. */
    private static final int MIST = 0xFFF4EEFC;
    /** Kompas night, the page background and text on orange. */
    private static final int NIGHT = 0xFF0C0917;
    /** Kompas orange, the main button. */
    private static final int ORANGE = 0xFFF3941F;
    /** Kompas plum-2, the second button. */
    private static final int PLUM_2 = 0xFF2C1F3F;
    /** Page padding, dp. */
    private static final int PAGE_DP = 24;
    /** Smallest touch target, dp. */
    private static final int TOUCH_DP = 48;
    /** Space above the switch, dp. */
    private static final int SWITCH_GAP_DP = 16;
    /** Space above a section heading, dp. */
    private static final int SECTION_GAP_DP = 32;
    /** Space above a button, dp. */
    private static final int BUTTON_GAP_DP = 12;
    /** Page title size, sp. */
    private static final int TITLE_SP = 24;
    /** Section heading size, sp. */
    private static final int HEADING_SP = 20;
    /** Switch label size, sp. */
    private static final int SWITCH_SP = 18;
    /** Body text size, sp. */
    private static final int BODY_SP = 16;
    /** Hint text size, sp. */
    private static final int HINT_SP = 14;
    /** Preferences file of these settings. */
    private static final String PREFS = "settings";

    /** The page's column of views. */
    private LinearLayout column;

    /** Created by Android. */
    public SettingsActivity() {
        super();
    }

    @Override
    protected void onCreate(final Bundle state) {
        super.onCreate(state);
        final ScrollView scroll = new ScrollView(this);
        scroll.setBackgroundColor(NIGHT);
        column = new LinearLayout(this);
        column.setOrientation(LinearLayout.VERTICAL);
        final int pad = pixels(PAGE_DP);
        column.setPadding(pad, pad, pad, pad);
        scroll.addView(column);
        setContentView(scroll);

        text(getString(R.string.settings_title), WHITE, TITLE_SP, 0);
        add(liveSwitch(), SWITCH_GAP_DP);
        text(getString(R.string.settings_live_hint), MIST, HINT_SP, 0);

        text(getString(R.string.battery_title), WHITE, HEADING_SP,
            SECTION_GAP_DP);
        text(getString(R.string.battery_text, Build.MANUFACTURER), MIST,
            BODY_SP, 0);
        // Dark text on orange: white on orange fails contrast.
        button(R.string.battery_open, ORANGE, NIGHT,
            view -> openBatterySettings());

        // Client mode (REL-01): which Kompanion server the app uses.
        text(getString(R.string.server_heading), WHITE, HEADING_SP,
            SECTION_GAP_DP);
        text(Server.url(this), MIST, BODY_SP, 0);
        button(R.string.server_change, PLUM_2, WHITE, view ->
            startActivity(new Intent(this, ServerActivity.class)));
        button(R.string.settings_done, PLUM_2, WHITE, view -> finish());
    }

    /**
     * The own-connection switch, saved and applied on change.
     *
     * @return the switch view
     */
    private Switch liveSwitch() {
        final SharedPreferences prefs = getSharedPreferences(PREFS,
            MODE_PRIVATE);
        final Switch live = new Switch(this);
        live.setText(R.string.settings_live);
        live.setTextColor(MIST);
        live.setTextSize(SWITCH_SP);
        live.setMinHeight(pixels(TOUCH_DP));
        live.setChecked(prefs.getBoolean("live", true));
        live.setOnCheckedChangeListener(
            (final CompoundButton view, final boolean checked) -> {
                prefs.edit().putBoolean("live", checked).apply();
                if (checked) {
                    LiveService.startIfWanted(this);
                } else {
                    LiveService.stop(this);
                }
            });
        return live;
    }

    /**
     * Density-independent pixels to pixels.
     *
     * @param dips the size in dp
     * @return the size in pixels
     */
    private int pixels(final int dips) {
        final float density = getResources().getDisplayMetrics().density;
        return Math.round(dips * density);
    }

    /**
     * Adds a full-width view to the page.
     *
     * @param view the view
     * @param topDp the space above it, dp
     */
    private void add(final View view, final int topDp) {
        final LinearLayout.LayoutParams params =
            new LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT);
        params.setMargins(0, pixels(topDp), 0, 0);
        column.addView(view, params);
    }

    /**
     * Adds a line of text; headings (white) are bold.
     *
     * @param value the text
     * @param color the text colour
     * @param sizeSp the text size, sp
     * @param topDp the space above it, dp
     */
    private void text(final String value, final int color, final int sizeSp,
            final int topDp) {
        final TextView view = new TextView(this);
        view.setText(value);
        view.setTextColor(color);
        view.setTextSize(sizeSp);
        if (color == WHITE) {
            view.setTypeface(Typeface.DEFAULT_BOLD);
        }
        add(view, topDp);
    }

    /**
     * Adds a full-width button.
     *
     * @param label the label's string resource
     * @param background the button colour
     * @param foreground the label colour
     * @param click what a tap does
     */
    private void button(final int label, final int background,
            final int foreground, final View.OnClickListener click) {
        final Button view = new Button(this);
        view.setText(label);
        view.setAllCaps(false);
        view.setBackgroundColor(background);
        view.setTextColor(foreground);
        view.setMinHeight(pixels(TOUCH_DP));
        view.setOnClickListener(click);
        add(view, BUTTON_GAP_DP);
    }

    /**
     * The maker's autostart page when it has one, else the app's own system
     * settings.
     */
    private void openBatterySettings() {
        final List<Intent> tries = new ArrayList<>();
        final Intent autostart = Makers.autostartPage();
        if (autostart != null) {
            tries.add(autostart);
        }
        tries.add(new Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS,
            Uri.parse("package:" + getPackageName())));
        final Iterator<Intent> next = tries.iterator();
        boolean opened = false;
        while (!opened && next.hasNext()) {
            try {
                startActivity(next.next());
                opened = true;
            } catch (ActivityNotFoundException | SecurityException missing) {
                // Not on this phone or not allowed: try the next page.
            }
        }
    }

    /**
     * First start on a phone that stops background apps: show this screen
     * once.
     *
     * @param activity the starting activity
     */
    public static void showOnceForMaker(final Activity activity) {
        final SharedPreferences prefs = activity.getSharedPreferences(PREFS,
            MODE_PRIVATE);
        if (Makers.restrictive() && !prefs.getBoolean("maker_shown", false)) {
            prefs.edit().putBoolean("maker_shown", true).apply();
            activity.startActivity(new Intent(activity,
                SettingsActivity.class));
        }
    }
}
