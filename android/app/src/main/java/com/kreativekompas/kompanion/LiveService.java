package com.kreativekompas.kompanion;

import android.app.Notification;
import android.app.Service;
import android.content.Context;
import android.content.Intent;
import android.content.SharedPreferences;
import android.content.pm.ServiceInfo;
import android.net.ConnectivityManager;
import android.net.Network;
import android.os.Build.VERSION;
import android.os.IBinder;
import android.util.Log;
import android.webkit.CookieManager;

/**
 * The app's own live connection: keeps the server's event stream open when
 * no UnifiedPush distributor delivers notifications.
 */
public final class LiveService extends Service {
    /** Log tag. */
    private static final String TAG = "Kompanion";
    /** Preferences file of the settings screen. */
    private static final String PREFS = "settings";
    /** Key of the own-connection switch. */
    private static final String KEY_LIVE = "live";
    /** Preferences file of the push state. */
    private static final String PREFS_PUSH = "push";
    /** Key of the delivery mode ("push" while a distributor is used). */
    private static final String KEY_MODE = "mode";
    /** Android 14 (API 34): foreground services name their type. */
    private static final int UPSIDE_DOWN_CAKE = 34;
    /** HTTP 401: not signed in. */
    private static final int HTTP_UNAUTHORIZED = 401;
    /** HTTP 403: not allowed. */
    private static final int HTTP_FORBIDDEN = 403;
    /** First retry wait, ms. */
    private static final long FIRST_WAIT_MS = 2_000;
    /** Longest wait while signed out: 15 minutes. */
    private static final long MAX_WAIT_AUTH_MS = 900_000;
    /** Longest wait after other failures: 5 minutes. */
    private static final long MAX_WAIT_OTHER_MS = 300_000;

    /** Guards the backoff and wakes the waiting loop. */
    private final Object lock = new Object();
    /** False once the service is destroyed. */
    private volatile boolean running = true;
    /** The next retry wait, ms. */
    private long delayMs = FIRST_WAIT_MS;
    /** The ongoing notification. */
    private LiveNotification status;
    /** The event stream. */
    private EventStream stream;
    /** The thread that keeps the stream open. */
    private Thread worker;
    /** Retries at once when the network comes back. */
    private ConnectivityManager.NetworkCallback networkCallback;

    /** Created by Android. */
    public LiveService() {
        super();
    }

    @Override
    public void onCreate() {
        super.onCreate();
        CookieManager.getInstance();
        Notifier.ensureChannels(this);
        status = new LiveNotification(this);
        stream = new EventStream(this, status);
        final Notification ongoing = status.build(R.string.live_connected);
        if (VERSION.SDK_INT >= UPSIDE_DOWN_CAKE) {
            startForeground(LiveNotification.NOTIFICATION_ID, ongoing,
                ServiceInfo.FOREGROUND_SERVICE_TYPE_REMOTE_MESSAGING);
        } else {
            startForeground(LiveNotification.NOTIFICATION_ID, ongoing);
        }
        networkCallback = new ConnectivityManager.NetworkCallback() {
            @Override
            public void onAvailable(final Network network) {
                wake();
            }
        };
        getSystemService(ConnectivityManager.class)
            .registerDefaultNetworkCallback(networkCallback);
        worker = new Thread(this::loop);
        worker.start();
    }

    @Override
    public int onStartCommand(final Intent intent, final int flags,
            final int startId) {
        // Started again (sign-in, settings): retry now, not after the backoff.
        wake();
        return START_STICKY;
    }

    @Override
    public IBinder onBind(final Intent intent) {
        return null;
    }

    @Override
    public void onDestroy() {
        running = false;
        if (networkCallback != null) {
            try {
                getSystemService(ConnectivityManager.class)
                    .unregisterNetworkCallback(networkCallback);
            } catch (IllegalArgumentException notRegistered) {
                // Registration failed earlier: nothing to undo.
            }
        }
        stream.close();
        if (worker != null && worker.isAlive()) {
            worker.interrupt();
        }
        super.onDestroy();
    }

    /**
     * Whether the own connection should run: switched on and no distributor.
     *
     * @param context any context of the app
     * @return true when it should run
     */
    public static boolean wanted(final Context context) {
        final SharedPreferences prefs =
            context.getSharedPreferences(PREFS, Context.MODE_PRIVATE);
        final SharedPreferences push =
            context.getSharedPreferences(PREFS_PUSH, Context.MODE_PRIVATE);
        return prefs.getBoolean(KEY_LIVE, true)
            && !"push".equals(push.getString(KEY_MODE, ""));
    }

    /**
     * Starts the service when it is wanted.
     *
     * @param context any context of the app
     */
    public static void startIfWanted(final Context context) {
        try {
            if (wanted(context)) {
                context.startForegroundService(
                    new Intent(context, LiveService.class));
            }
        } catch (IllegalStateException notAllowed) {
            Log.w(TAG, "Could not start foreground service: "
                + notAllowed.getMessage());
        }
    }

    /**
     * Stops the service.
     *
     * @param context any context of the app
     */
    public static void stop(final Context context) {
        context.stopService(new Intent(context, LiveService.class));
    }

    /** Keeps the stream open, retrying with a growing wait. */
    private void loop() {
        boolean again = true;
        while (running && again) {
            final int code = stream.connect();
            again = running && waitBeforeRetry(code);
        }
    }

    /**
     * Waits before the next try: longer after each failure, at most 15
     * minutes while signed out and 5 minutes otherwise.
     *
     * @param code the last HTTP status
     * @return false when the thread was interrupted
     */
    private boolean waitBeforeRetry(final int code) {
        long maxWait = MAX_WAIT_OTHER_MS;
        if (code == HTTP_UNAUTHORIZED || code == HTTP_FORBIDDEN) {
            status.show(R.string.live_signin);
            maxWait = MAX_WAIT_AUTH_MS;
        }
        boolean waited = true;
        synchronized (lock) {
            // A stream that delivered something starts the backoff over.
            if (stream.deliveredAny()) {
                delayMs = FIRST_WAIT_MS;
            } else {
                delayMs = Math.min(delayMs, maxWait);
            }
            final long wait = delayMs;
            delayMs = Math.min(delayMs * 2, maxWait);
            try {
                // wake() cuts this short when the network comes back.
                lock.wait(wait);
            } catch (InterruptedException stopped) {
                waited = false;
            }
        }
        return waited;
    }

    /** Retries now: the network came back or the app started us again. */
    private void wake() {
        synchronized (lock) {
            delayMs = FIRST_WAIT_MS;
            lock.notifyAll();
        }
    }
}
