package com.kreativekompas.kompanion;

import android.app.Activity;
import android.content.Intent;
import android.graphics.Typeface;
import android.os.Bundle;
import android.text.InputType;
import android.view.View;
import android.widget.Button;
import android.widget.EditText;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.TextView;

/**
 * Client mode (REL-01): asks which Kompanion server to use, checks that it
 * answers and opens it. Shown on the first start and from the settings.
 */
public final class ServerActivity extends Activity {
    /** Kompas white, the title. */
    private static final int WHITE = 0xFFFFFFFF;
    /** Kompas mist, body text and the field's text. */
    private static final int MIST = 0xFFF4EEFC;
    /** Kompas lilac, hints. */
    private static final int LILAC = 0xFFCCA9FF;
    /** Kompas night, the page background and text on orange. */
    private static final int NIGHT = 0xFF0C0917;
    /** Kompas plum-2, the field's background. */
    private static final int PLUM_2 = 0xFF2C1F3F;
    /** Kompas orange, the button. */
    private static final int ORANGE = 0xFFF3941F;
    /** Error text, readable on night. */
    private static final int ERROR = 0xFFFFB4A8;
    /** Page padding, dp. */
    private static final int PAGE_DP = 24;
    /** Space between the blocks, dp. */
    private static final int GAP_DP = 16;
    /** Smallest touch target, dp. */
    private static final int TOUCH_DP = 48;
    /** Title size, sp. */
    private static final int TITLE_SP = 24;
    /** Body text size, sp. */
    private static final int BODY_SP = 16;

    /** The page's column of views. */
    private LinearLayout column;
    /** The address field. */
    private EditText address;
    /** The connect button. */
    private Button go;
    /** The error line. */
    private TextView error;

    /** Created by Android. */
    public ServerActivity() {
        super();
    }

    @Override
    protected void onCreate(final Bundle state) {
        super.onCreate(state);
        final ScrollView scroll = new ScrollView(this);
        scroll.setBackgroundColor(NIGHT);
        scroll.setFitsSystemWindows(true);
        column = new LinearLayout(this);
        column.setOrientation(LinearLayout.VERTICAL);
        final int pad = pixels(PAGE_DP);
        column.setPadding(pad, pad, pad, pad);
        scroll.addView(column);
        setContentView(scroll);

        final TextView title = text(R.string.server_title, WHITE, TITLE_SP);
        title.setTypeface(Typeface.DEFAULT_BOLD);
        text(R.string.server_text, MIST, BODY_SP);

        address = new EditText(this);
        address.setInputType(InputType.TYPE_CLASS_TEXT
            | InputType.TYPE_TEXT_VARIATION_URI);
        address.setHint(R.string.server_hint);
        address.setText(Server.url(this));
        address.setTextColor(MIST);
        address.setHintTextColor(LILAC);
        address.setBackgroundColor(PLUM_2);
        address.setMinHeight(pixels(TOUCH_DP));
        address.setPadding(pad / 2, 0, pad / 2, 0);
        add(address);

        go = new Button(this);
        go.setText(R.string.server_connect);
        go.setAllCaps(false);
        // Dark text on orange: white on orange fails contrast.
        go.setBackgroundColor(ORANGE);
        go.setTextColor(NIGHT);
        go.setMinHeight(pixels(TOUCH_DP));
        go.setOnClickListener(view -> connect());
        add(go);

        error = text(R.string.server_plain_http, ERROR, BODY_SP);
        error.setText("");
        text(R.string.server_plain_http, LILAC, BODY_SP);
    }

    /** Checks the typed server in the background, then opens it. */
    private void connect() {
        final String server = Server.normalize(address.getText().toString());
        error.setText("");
        if (server == null) {
            error.setText(R.string.server_invalid);
        } else {
            go.setEnabled(false);
            go.setText(R.string.server_connecting);
            new Thread(() -> {
                final boolean answered = Server.answers(server);
                runOnUiThread(() -> connected(server, answered));
            }).start();
        }
    }

    /**
     * Saves and opens a server that answered, else says it did not.
     *
     * @param server the base URL
     * @param answered whether its status page answered
     */
    private void connected(final String server, final boolean answered) {
        go.setEnabled(true);
        go.setText(R.string.server_connect);
        if (answered) {
            final boolean changed = !server.equals(Server.url(this));
            Server.save(this, server);
            if (changed) {
                // The new server needs the push endpoint and a fresh stream.
                PushRegistration.resend(this);
                LiveService.stop(this);
            }
            final Intent open = new Intent(this, MainActivity.class);
            open.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK
                | Intent.FLAG_ACTIVITY_CLEAR_TASK);
            startActivity(open);
            finish();
        } else {
            error.setText(R.string.server_no_answer);
        }
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
     * Adds a full-width view below the last one.
     *
     * @param view the view
     */
    private void add(final View view) {
        final LinearLayout.LayoutParams params =
            new LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT);
        params.setMargins(0, pixels(GAP_DP), 0, 0);
        column.addView(view, params);
    }

    /**
     * Adds a line of text.
     *
     * @param value the text's string resource
     * @param color the text colour
     * @param sizeSp the text size, sp
     * @return the view
     */
    private TextView text(final int value, final int color,
            final int sizeSp) {
        final TextView view = new TextView(this);
        view.setText(value);
        view.setTextColor(color);
        view.setTextSize(sizeSp);
        add(view);
        return view;
    }
}
