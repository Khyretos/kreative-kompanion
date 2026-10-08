package com.kreativekompas.kompanion;

import android.content.Context;
import android.util.Log;

import org.json.JSONException;
import org.json.JSONObject;

import org.unifiedpush.android.connector.PushService;
import org.unifiedpush.android.connector.FailedReason;
import org.unifiedpush.android.connector.data.PushEndpoint;
import org.unifiedpush.android.connector.data.PushMessage;

public class PushServiceImpl extends PushService {

    @Override
    public void onNewEndpoint(PushEndpoint endpoint, String instance) {
        PushRegistration.save(this, endpoint.getUrl());
        PushRegistration.sendIfNeeded(this);
    }

    @Override
    public void onMessage(PushMessage message, String instance) {
        String text = null;
        try {
            text = new String(message.getContent(), java.nio.charset.StandardCharsets.UTF_8);
            JSONObject json = new JSONObject(text);
            
            String title = json.optString("title", "Kompanion");
            String state = json.optString("state", "");
            String url = json.optString("url", "");

            Notifier.show(this, title, state, url);
        } catch (JSONException e) {
            Notifier.show(this, "Kompanion", text, "");
        }
    }

    @Override
    public void onRegistrationFailed(FailedReason reason, String instance) {
        Log.w("Kompanion", "push registration failed: " + reason);
        getSharedPreferences("push", MODE_PRIVATE).edit().remove("mode").apply();
        LiveService.startIfWanted(this);
    }

    @Override
    public void onUnregistered(String instance) {
        PushRegistration.clear(this);
        getSharedPreferences("push", MODE_PRIVATE).edit().remove("mode").apply();
        LiveService.startIfWanted(this);
    }
}
