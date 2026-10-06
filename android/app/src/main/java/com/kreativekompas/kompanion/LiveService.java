package com.kreativekompas.kompanion;

import android.app.Notification;
import android.app.NotificationManager;
import android.app.PendingIntent;
import android.app.Service;
import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;
import android.content.SharedPreferences;
import android.content.pm.ServiceInfo;
import android.net.ConnectivityManager;
import android.net.Network;
import android.os.Build;
import android.os.IBinder;
import android.util.Log;
import android.webkit.CookieManager;
import org.json.JSONObject;
import java.io.BufferedReader;
import java.io.IOException;
import java.io.InputStreamReader;
import java.net.HttpURLConnection;
import java.net.URL;
import java.nio.charset.StandardCharsets;

public class LiveService extends Service {
    private static final String TAG = "Kompanion";
    private static final String PREFS = "settings";
    private static final String KEY_LIVE = "live";
    private static final String PREFS_PUSH = "push";
    private static final String KEY_MODE = "mode";
    private static final int NOTIFICATION_ID = 1;

    private volatile boolean running = true;
    private volatile HttpURLConnection connection = null;
    private volatile boolean gotLine = false;
    private volatile long delayMs = 2000;
    private static final long MAX_WAIT_AUTH_MS = 900000; // 15 minutes while signed out
    private static final long MAX_WAIT_OTHER_MS = 300000; // 5 minutes
    private int shownStatus = 0;

    private Thread workerThread;
    private ConnectivityManager.NetworkCallback networkCallback;
    private final Object lock = new Object();

    @Override
    public void onCreate() {
        super.onCreate();
        CookieManager.getInstance();
        Notifier.ensureChannels(this);
        
        Notification n = ongoing(R.string.live_connected);
        shownStatus = R.string.live_connected;
        if (Build.VERSION.SDK_INT >= 34) {
            startForeground(NOTIFICATION_ID, n, ServiceInfo.FOREGROUND_SERVICE_TYPE_REMOTE_MESSAGING);
        } else {
            startForeground(NOTIFICATION_ID, n);
        }

        ConnectivityManager cm = (ConnectivityManager) getSystemService(CONNECTIVITY_SERVICE);
        networkCallback = new ConnectivityManager.NetworkCallback() {
            @Override
            public void onAvailable(Network network) {
                wake();
            }
        };
        cm.registerDefaultNetworkCallback(networkCallback);

        workerThread = new Thread(this::loop);
        workerThread.start();
    }

    @Override
    public int onStartCommand(Intent intent, int flags, int startId) {
        wake(); // started again (sign-in, settings): retry now instead of after the backoff
        return START_STICKY;
    }

    @Override
    public IBinder onBind(Intent intent) {
        return null;
    }

    @Override
    public void onDestroy() {
        running = false;
        if (networkCallback != null) {
            try {
                ((ConnectivityManager) getSystemService(CONNECTIVITY_SERVICE)).unregisterNetworkCallback(networkCallback);
            } catch (IllegalArgumentException e) {
                // Ignore
            }
        }
        if (connection != null) {
            connection.disconnect();
        }
        if (workerThread != null && workerThread.isAlive()) {
            workerThread.interrupt();
        }
        super.onDestroy();
    }

    public static boolean wanted(Context c) {
        SharedPreferences prefs = c.getSharedPreferences(PREFS, Context.MODE_PRIVATE);
        boolean liveEnabled = prefs.getBoolean(KEY_LIVE, true);
        SharedPreferences pushPrefs = c.getSharedPreferences(PREFS_PUSH, Context.MODE_PRIVATE);
        String mode = pushPrefs.getString(KEY_MODE, "");
        return liveEnabled && !"push".equals(mode);
    }

    public static void startIfWanted(Context c) {
        try {
            if (wanted(c)) {
                c.startForegroundService(new Intent(c, LiveService.class));
            }
        } catch (IllegalStateException e) {
            Log.w(TAG, "Could not start foreground service: " + e.getMessage());
        }
    }

    public static void stop(Context c) {
        c.stopService(new Intent(c, LiveService.class));
    }

    public static class Boot extends BroadcastReceiver {
        @Override
        public void onReceive(Context context, Intent intent) {
            if (Intent.ACTION_BOOT_COMPLETED.equals(intent.getAction())) {
                startIfWanted(context);
            }
        }
    }

    private void loop() {
        while (running) {
            gotLine = false;
            int code;
            try {
                code = connect();
            } catch (IOException e) {
                code = 0;
            }
            if (!running) return;
            long maxWait = MAX_WAIT_OTHER_MS;
            if (code == 401 || code == 403) {
                setStatus(R.string.live_signin);
                maxWait = MAX_WAIT_AUTH_MS;
            }
            long wait;
            synchronized (lock) {
                // A stream that delivered something starts the backoff over.
                delayMs = gotLine ? 2000 : Math.min(delayMs, maxWait);
                wait = delayMs;
                delayMs = Math.min(delayMs * 2, maxWait);
                try {
                    lock.wait(wait); // wake() cuts this short when the network comes back
                } catch (InterruptedException e) {
                    return;
                }
            }
        }
    }

    private int connect() throws IOException {
        String server = getString(R.string.server_url);
        CookieManager cookieManager = CookieManager.getInstance();
        String cookie = cookieManager.getCookie(server);
        
        if (cookie == null || cookie.isEmpty()) {
            return 401;
        }

        HttpURLConnection conn = null;
        try {
            URL url = new URL(server + "/api/events");
            conn = (HttpURLConnection) url.openConnection();
            connection = conn;
            conn.setConnectTimeout(15000);
            conn.setReadTimeout(60000);
            conn.setRequestMethod("GET");
            conn.setRequestProperty("Accept", "text/event-stream");
            conn.setRequestProperty("X-Kompanion", "1");
            conn.setRequestProperty("Origin", server);
            conn.setRequestProperty("Cookie", cookie);

            int responseCode = conn.getResponseCode();
            if (responseCode != 200) {
                return responseCode;
            }

            setStatus(R.string.live_connected);
            StringBuilder buffer = new StringBuilder();
            BufferedReader reader = new BufferedReader(new InputStreamReader(conn.getInputStream(), StandardCharsets.UTF_8));
            String line;
            while (running && (line = reader.readLine()) != null) {
                gotLine = true;
                if (line.startsWith("data:")) {
                    String data = line.substring(5);
                    if (data.startsWith(" ")) data = data.substring(1);
                    if (buffer.length() > 0) buffer.append('\n');
                    buffer.append(data);
                } else if (line.isEmpty()) {
                    if (buffer.length() > 0) {
                        dispatch(buffer.toString());
                        buffer.setLength(0);
                    }
                }
                // comments (": keep-alive") and "event:" lines are ignored
            }
            return 200;
        } finally {
            connection = null;
            if (conn != null) {
                conn.disconnect();
            }
        }
    }

    private void dispatch(String data) {
        try {
            JSONObject json = new JSONObject(data);
            String type = json.optString("type");
            if ("notify".equals(type)) {
                String title = json.optString("title", "Kompanion");
                String state = json.optString("state");
                String url = json.optString("url");
                Notifier.show(this, title, state, url);
            }
        } catch (Exception e) {
            // Ignore JSONException and other parsing errors
        }
    }

    private void wake() {
        synchronized (lock) {
            delayMs = 2000;
            lock.notifyAll();
        }
    }

    private void setStatus(int textRes) {
        if (textRes == shownStatus) return;
        shownStatus = textRes;
        Notification notification = ongoing(textRes);
        NotificationManager nm = (NotificationManager) getSystemService(NOTIFICATION_SERVICE);
        nm.notify(NOTIFICATION_ID, notification);
    }

    private Notification ongoing(int textRes) {
        Notification.Builder builder = new Notification.Builder(this, "live")
                .setSmallIcon(R.drawable.ic_stat)
                .setContentTitle(getString(textRes))
                .setOngoing(true)
                .setColor(0xFFF3941F)
                .setContentIntent(PendingIntent.getActivity(this, 1, new Intent(this, SettingsActivity.class), PendingIntent.FLAG_UPDATE_CURRENT | PendingIntent.FLAG_IMMUTABLE));
        return builder.build();
    }
}
