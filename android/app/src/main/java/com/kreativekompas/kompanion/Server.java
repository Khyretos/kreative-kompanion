package com.kreativekompas.kompanion;

import android.content.Context;
import android.content.SharedPreferences;

import java.io.IOException;
import java.net.HttpURLConnection;
import java.net.URL;
import java.util.Locale;

/**
 * Client mode (REL-01): the Kompanion server this app talks to, chosen on
 * the first start and changeable in the settings.
 */
public final class Server {
    /** Preferences file of the chosen server. */
    private static final String PREFS = "server";
    /** Key of the chosen server's base URL. */
    private static final String KEY_URL = "url";
    /** Scheme separator. */
    private static final String SEPARATOR = "://";
    /** Timeout of the status check, ms. */
    private static final int TIMEOUT_MS = 10_000;
    /** The status page answers 200 OK. */
    private static final int HTTP_OK = 200;

    private Server() {
        // Only static helpers.
    }

    /**
     * The chosen server, else the one suggested in the app's strings.
     *
     * @param context any context of the app
     * @return the server's base URL, without a trailing slash
     */
    public static String url(final Context context) {
        final String suggested = context.getString(R.string.server_url);
        return prefs(context).getString(KEY_URL, suggested);
    }

    /**
     * Whether the user picked a server yet.
     *
     * @param context any context of the app
     * @return true after the first successful connect
     */
    public static boolean chosen(final Context context) {
        return prefs(context).contains(KEY_URL);
    }

    /**
     * Remembers the server.
     *
     * @param context any context of the app
     * @param server a base URL from {@link #normalize(String)}
     */
    public static void save(final Context context, final String server) {
        prefs(context).edit().putString(KEY_URL, server).apply();
    }

    /**
     * Cleans a typed address: https when no scheme is given, http or https
     * only, no user name, no query, no trailing slash.
     *
     * @param typed what the user typed
     * @return the base URL, or null when it is not a server address
     */
    public static String normalize(final String typed) {
        String address = typed.trim();
        if (!address.contains(SEPARATOR)) {
            address = "https" + SEPARATOR + address;
        }
        final int split = address.indexOf(SEPARATOR);
        final String scheme = address.substring(0, split)
            .toLowerCase(Locale.ROOT);
        final String rest = address.substring(split + SEPARATOR.length());
        final int end = firstOf(rest, "/?#");
        final String host = rest.substring(0, end).toLowerCase(Locale.ROOT);
        String path = rest.substring(end);
        path = path.substring(0, firstOf(path, "?#"));
        while (path.endsWith("/")) {
            path = path.substring(0, path.length() - 1);
        }
        String result = null;
        final boolean web = "http".equals(scheme) || "https".equals(scheme);
        if (web && !host.isEmpty() && host.indexOf('@') < 0
            && host.indexOf(' ') < 0) {
            result = scheme + SEPARATOR + host + path;
        }
        return result;
    }

    /**
     * Whether a Kompanion server answers at the URL. Blocks: call it off the
     * main thread.
     *
     * @param server a base URL
     * @return true when its status page answers 200
     */
    public static boolean answers(final String server) {
        boolean answered;
        try {
            final HttpURLConnection request = (HttpURLConnection)
                new URL(server + "/api/status").openConnection();
            request.setConnectTimeout(TIMEOUT_MS);
            request.setReadTimeout(TIMEOUT_MS);
            try {
                answered = request.getResponseCode() == HTTP_OK;
            } finally {
                request.disconnect();
            }
        } catch (IOException | IllegalArgumentException failed) {
            answered = false;
        }
        return answered;
    }

    /**
     * Index of the first of some characters, else the text's length.
     *
     * @param text the text
     * @param chars the characters to look for
     * @return the index
     */
    private static int firstOf(final String text, final String chars) {
        int index = text.length();
        for (int i = 0; i < text.length(); i++) {
            if (chars.indexOf(text.charAt(i)) >= 0 && i < index) {
                index = i;
            }
        }
        return index;
    }

    /**
     * The preferences holding the chosen server.
     *
     * @param context any context of the app
     * @return the preferences
     */
    private static SharedPreferences prefs(final Context context) {
        return context.getSharedPreferences(PREFS, Context.MODE_PRIVATE);
    }
}
