package com.kreativekompas.kompanion;

import android.app.Notification;
import android.app.NotificationManager;
import android.app.PendingIntent;
import android.content.Context;
import android.content.Intent;

/**
 * The live connection's ongoing notification (connected / sign in).
 */
public final class LiveNotification {
    /** Id of the ongoing notification. */
    public static final int NOTIFICATION_ID = 1;
    /** Kompas violet, the notification's accent colour. */
    private static final int VIOLET = 0xFF5C398E;
    /** Request code of the settings intent. */
    private static final int OPEN_SETTINGS = 1;

    /** The service that shows the notification. */
    private final Context context;
    /** The text shown now, a string resource, or 0 before the first. */
    private int shown;

    /**
     * The notification of one live service.
     *
     * @param service the live service
     */
    public LiveNotification(final Context service) {
        context = service;
    }

    /**
     * Builds the notification for a text and remembers it as shown.
     *
     * @param textRes the text's string resource
     * @return the notification
     */
    public Notification build(final int textRes) {
        shown = textRes;
        final Intent settings = new Intent(context, SettingsActivity.class);
        return new Notification.Builder(context, "live")
            .setSmallIcon(R.drawable.ic_stat)
            .setContentTitle(context.getString(textRes))
            .setOngoing(true)
            .setColor(VIOLET)
            .setContentIntent(PendingIntent.getActivity(context,
                OPEN_SETTINGS, settings, PendingIntent.FLAG_UPDATE_CURRENT
                    | PendingIntent.FLAG_IMMUTABLE))
            .build();
    }

    /**
     * Shows another text, when it changed.
     *
     * @param textRes the text's string resource
     */
    public void show(final int textRes) {
        if (textRes != shown) {
            context.getSystemService(NotificationManager.class)
                .notify(NOTIFICATION_ID, build(textRes));
        }
    }
}
