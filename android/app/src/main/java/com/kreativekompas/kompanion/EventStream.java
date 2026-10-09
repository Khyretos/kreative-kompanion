package com.kreativekompas.kompanion;

import android.content.Context;
import android.webkit.CookieManager;

import org.json.JSONException;
import org.json.JSONObject;

import java.io.BufferedReader;
import java.io.IOException;
import java.io.InputStreamReader;
import java.net.HttpURLConnection;
import java.net.URL;
import java.nio.charset.StandardCharsets;

/**
 * Reads the server's event stream (/api/events) and shows its task
 * notifications.
 */
public final class EventStream {
    /** HTTP 200: the stream is open. */
    private static final int HTTP_OK = 200;
    /** HTTP 401: not signed in. */
    private static final int HTTP_UNAUTHORIZED = 401;
    /** Connect timeout, ms. */
    private static final int CONNECT_MS = 15_000;
    /** Read timeout, ms; the server sends a keep-alive well within it. */
    private static final int READ_MS = 60_000;
    /** Start of a data line. */
    private static final String DATA = "data:";

    /** The live service. */
    private final Context context;
    /** Its ongoing notification. */
    private final LiveNotification status;
    /** The open request, so close() can cut it. */
    private volatile HttpURLConnection connection;
    /** False once the service stops. */
    private volatile boolean open = true;
    /** Whether the last connection delivered at least one line. */
    private volatile boolean delivered;
    /** The data lines of the event being read. */
    private final StringBuilder event = new StringBuilder();

    /**
     * A stream for the live service.
     *
     * @param service the live service
     * @param notification its ongoing notification
     */
    public EventStream(final Context service,
            final LiveNotification notification) {
        context = service;
        status = notification;
    }

    /**
     * Whether the last connection delivered anything.
     *
     * @return true when at least one line arrived
     */
    public boolean deliveredAny() {
        return delivered;
    }

    /** Stops reading and cuts the open request. */
    public void close() {
        open = false;
        final HttpURLConnection current = connection;
        if (current != null) {
            current.disconnect();
        }
    }

    /**
     * Reads the stream until it ends.
     *
     * @return the HTTP status (200 after a stream ended, 401 when signed
     *     out), or 0 when the connection failed
     */
    public int connect() {
        delivered = false;
        final String server = Server.url(context);
        final String cookie = CookieManager.getInstance().getCookie(server);
        int code = HTTP_UNAUTHORIZED;
        if (cookie != null && !cookie.isEmpty()) {
            try {
                code = openAndRead(server, cookie);
            } catch (IOException failed) {
                code = 0;
            }
        }
        return code;
    }

    /**
     * Opens the request, reads it to the end and closes it.
     *
     * @param server the server URL
     * @param cookie the session cookie
     * @return the HTTP status
     * @throws IOException when the connection fails
     */
    private int openAndRead(final String server, final String cookie)
            throws IOException {
        final HttpURLConnection request = open(server);
        connection = request;
        try {
            return read(request, server, cookie);
        } finally {
            connection = null;
            request.disconnect();
        }
    }

    /**
     * A request for the server's event stream.
     *
     * @param server the server URL
     * @return the unopened request
     * @throws IOException never for an http(s) URL
     */
    private static HttpURLConnection open(final String server)
            throws IOException {
        return (HttpURLConnection) new URL(server + "/api/events")
            .openConnection();
    }

    /**
     * Sends the request and reads the events.
     *
     * @param request the unopened request
     * @param server the server URL
     * @param cookie the session cookie
     * @return the HTTP status
     * @throws IOException when the connection fails
     */
    private int read(final HttpURLConnection request, final String server,
            final String cookie) throws IOException {
        request.setConnectTimeout(CONNECT_MS);
        request.setReadTimeout(READ_MS);
        request.setRequestMethod("GET");
        request.setRequestProperty("Accept", "text/event-stream");
        request.setRequestProperty("X-Kompanion", "1");
        request.setRequestProperty("Origin", server);
        request.setRequestProperty("Cookie", cookie);
        final int code = request.getResponseCode();
        if (code == HTTP_OK) {
            status.show(R.string.live_connected);
            readLines(new BufferedReader(new InputStreamReader(
                request.getInputStream(), StandardCharsets.UTF_8)));
        }
        return code;
    }

    /**
     * Reads lines while the service runs and the stream is open.
     *
     * @param reader the stream
     * @throws IOException when reading fails
     */
    private void readLines(final BufferedReader reader) throws IOException {
        event.setLength(0);
        String line = reader.readLine();
        while (open && line != null) {
            delivered = true;
            onLine(line);
            line = reader.readLine();
        }
    }

    /**
     * One line of the stream: "data:" lines collect, an empty line ends the
     * event; comments (": keep-alive") and "event:" lines are ignored.
     *
     * @param line the line without its line break
     */
    private void onLine(final String line) {
        if (line.startsWith(DATA)) {
            String data = line.substring(DATA.length());
            if (data.startsWith(" ")) {
                data = data.substring(1);
            }
            if (event.length() > 0) {
                event.append('\n');
            }
            event.append(data);
        } else if (line.isEmpty() && event.length() > 0) {
            dispatch(event.toString());
            event.setLength(0);
        }
    }

    /**
     * Shows a "notify" event as a task notification.
     *
     * @param data the event's JSON
     */
    private void dispatch(final String data) {
        try {
            final JSONObject json = new JSONObject(data);
            if ("notify".equals(json.optString("type"))) {
                Notifier.show(context, json.optString("title", "Kompanion"),
                    json.optString("state"), json.optString("url"));
            }
        } catch (JSONException broken) {
            // Not an event we know: skip it.
        }
    }
}
