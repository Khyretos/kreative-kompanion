package com.kreativekompas.kompanion;

import android.util.Log;

import org.json.JSONException;
import org.json.JSONObject;
import org.unifiedpush.android.connector.FailedReason;
import org.unifiedpush.android.connector.PushService;
import org.unifiedpush.android.connector.data.PushEndpoint;
import org.unifiedpush.android.connector.data.PushMessage;

import java.nio.charset.StandardCharsets;

/**
 * Receives UnifiedPush events: new endpoints, messages and failures.
 */
public final class PushServiceImpl extends PushService {
    /** Title of a notification whose message has none. */
    private static final String APP = "Kompanion";

    /** Created by the UnifiedPush connector. */
    public PushServiceImpl() {
        super();
    }

    @Override
    public void onNewEndpoint(final PushEndpoint endpoint,
            final String instance) {
        PushRegistration.save(this, endpoint.getUrl());
        PushRegistration.sendIfNeeded(this);
    }

    @Override
    public void onMessage(final PushMessage message, final String instance) {
        final String text =
            new String(message.getContent(), StandardCharsets.UTF_8);
        try {
            final JSONObject json = new JSONObject(text);
            Notifier.show(this, json.optString("title", APP),
                json.optString("state", ""), json.optString("url", ""));
        } catch (JSONException error) {
            Notifier.show(this, APP, text, "");
        }
    }

    @Override
    public void onRegistrationFailed(final FailedReason reason,
            final String instance) {
        Log.w(APP, "push registration failed: " + reason);
        forgetMode();
    }

    @Override
    public void onUnregistered(final String instance) {
        PushRegistration.clear(this);
        forgetMode();
    }

    /** Falls back to the own live connection when push is gone. */
    private void forgetMode() {
        getSharedPreferences("push", MODE_PRIVATE).edit().remove("mode")
            .apply();
        LiveService.startIfWanted(this);
    }
}
