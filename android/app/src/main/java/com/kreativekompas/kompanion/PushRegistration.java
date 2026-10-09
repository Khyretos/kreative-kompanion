package com.kreativekompas.kompanion;

import android.content.Context;
import android.content.SharedPreferences;
import android.os.Build;
import android.util.Log;
import android.webkit.CookieManager;

import org.json.JSONException;
import org.json.JSONObject;

import java.io.IOException;
import java.io.OutputStream;
import java.net.HttpURLConnection;
import java.net.URL;
import java.nio.charset.StandardCharsets;

/**
 * Stores the UnifiedPush endpoint and registers it once with the server.
 */
public final class PushRegistration {
    /** Shared preferences file of the push state. */
    private static final String PREFS = "push";
    /** Key of the endpoint the distributor gave us. */
    private static final String KEY_ENDPOINT = "endpoint";
    /** Key of the endpoint the server already knows. */
    private static final String KEY_SENT = "sent";
    /** Connect and read timeout of the register call, in milliseconds. */
    private static final int TIMEOUT_MS = 10_000;
    /** The server answers 204 No Content when it stored the endpoint. */
    private static final int HTTP_NO_CONTENT = 204;
    /** Log tag. */
    private static final String TAG = "Kompanion";

    private PushRegistration() {
        // Only static helpers.
    }

    /**
     * Remembers the endpoint the distributor gave us.
     *
     * @param context any context of the app
     * @param endpoint the UnifiedPush endpoint URL
     */
    public static void save(final Context context, final String endpoint) {
        prefs(context).edit().putString(KEY_ENDPOINT, endpoint).apply();
    }

    /**
     * Forgets the endpoint and that the server knew it.
     *
     * @param context any context of the app
     */
    public static void clear(final Context context) {
        prefs(context).edit().remove(KEY_ENDPOINT).remove(KEY_SENT).apply();
    }

    /**
     * Forgets only that the server knew the endpoint, so it is sent again
     * (after switching servers).
     *
     * @param context any context of the app
     */
    public static void resend(final Context context) {
        prefs(context).edit().remove(KEY_SENT).apply();
    }

    /**
     * Sends the endpoint to the server in the background, once per endpoint.
     *
     * @param context any context of the app
     */
    public static void sendIfNeeded(final Context context) {
        new Thread(() -> {
            final String endpoint = pendingEndpoint(context);
            final String server = Server.url(context);
            final String cookie = sessionCookie(server);
            if (endpoint != null && cookie != null) {
                try {
                    if (register(server, cookie, endpoint)) {
                        prefs(context).edit()
                            .putString(KEY_SENT, endpoint).apply();
                    }
                } catch (IOException | JSONException error) {
                    Log.w(TAG, "push register failed: "
                        + error.getClass().getSimpleName());
                }
            }
        }).start();
    }

    /**
     * The push state of the app.
     *
     * @param context any context of the app
     * @return the push preferences
     */
    private static SharedPreferences prefs(final Context context) {
        return context.getSharedPreferences(PREFS, Context.MODE_PRIVATE);
    }

    /**
     * The endpoint the server does not know yet.
     *
     * @param context any context of the app
     * @return the endpoint, or null when there is nothing to send
     */
    private static String pendingEndpoint(final Context context) {
        final SharedPreferences prefs = prefs(context);
        final String endpoint = prefs.getString(KEY_ENDPOINT, "");
        String pending = null;
        if (!endpoint.isEmpty()
            && !endpoint.equals(prefs.getString(KEY_SENT, null))) {
            pending = endpoint;
        }
        return pending;
    }

    /**
     * The web view's session cookie for the server.
     *
     * @param server the chosen server's URL
     * @return the cookie header value, or null when not signed in
     */
    private static String sessionCookie(final String server) {
        String cookie = null;
        if (server != null && !server.isEmpty()) {
            final CookieManager cookies = CookieManager.getInstance();
            cookies.setAcceptCookie(true);
            cookie = cookies.getCookie(server);
        }
        if (cookie != null && cookie.isEmpty()) {
            cookie = null;
        }
        return cookie;
    }

    /**
     * POSTs the endpoint to the server.
     *
     * @param server the server URL
     * @param cookie the session cookie header value
     * @param endpoint the UnifiedPush endpoint URL
     * @return true when the server stored it
     * @throws IOException when the request fails
     * @throws JSONException when the body cannot be built
     */
    private static boolean register(final String server, final String cookie,
            final String endpoint) throws IOException, JSONException {
        final URL url = new URL(server + "/api/push/register");
        final HttpURLConnection connection =
            (HttpURLConnection) url.openConnection();
        try {
            connection.setConnectTimeout(TIMEOUT_MS);
            connection.setReadTimeout(TIMEOUT_MS);
            connection.setRequestMethod("POST");
            connection.setRequestProperty("Content-Type", "application/json");
            connection.setRequestProperty("X-Kompanion", "1");
            connection.setRequestProperty("Origin", server);
            connection.setRequestProperty("Cookie", cookie);
            connection.setDoOutput(true);
            final JSONObject body = new JSONObject();
            body.put("endpoint", endpoint);
            body.put("device", Build.MANUFACTURER + " " + Build.MODEL);
            send(connection.getOutputStream(),
                body.toString().getBytes(StandardCharsets.UTF_8));
            return connection.getResponseCode() == HTTP_NO_CONTENT;
        } finally {
            connection.disconnect();
        }
    }

    /**
     * Writes the request body and closes the stream.
     *
     * @param out the request's output stream
     * @param body the bytes to send
     * @throws IOException when writing fails
     */
    private static void send(final OutputStream out, final byte[] body)
            throws IOException {
        try (OutputStream stream = out) {
            stream.write(body);
        }
    }
}
