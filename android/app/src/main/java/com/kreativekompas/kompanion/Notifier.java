package com.kreativekompas.kompanion;

import android.app.Notification;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.app.PendingIntent;
import android.content.Context;
import android.content.Intent;

/**
 * Notification channels and task notifications.
 */
public final class Notifier {
    /** Channel of task notifications. */
    private static final String TASKS = "tasks";
    /** Channel of the live connection's silent notification. */
    private static final String LIVE = "live";
    /** Kompas violet, the notification's accent colour. */
    private static final int VIOLET = 0xFF5C398E;
    /** Flags of every pending intent we make. */
    private static final int INTENT_FLAGS =
        PendingIntent.FLAG_UPDATE_CURRENT | PendingIntent.FLAG_IMMUTABLE;

    private Notifier() {
        // Only static helpers.
    }

    /**
     * Ensures notification channels exist for tasks and live updates.
     *
     * @param context any context of the app
     */
    public static void ensureChannels(final Context context) {
        final NotificationManager manager =
            context.getSystemService(NotificationManager.class);
        if (manager.getNotificationChannel(TASKS) == null) {
            manager.createNotificationChannel(new NotificationChannel(TASKS,
                context.getString(R.string.channel_tasks),
                NotificationManager.IMPORTANCE_HIGH));
        }
        if (manager.getNotificationChannel(LIVE) == null) {
            final NotificationChannel live = new NotificationChannel(LIVE,
                context.getString(R.string.channel_live),
                NotificationManager.IMPORTANCE_MIN);
            live.setShowBadge(false);
            manager.createNotificationChannel(live);
        }
    }

    /**
     * Shows a notification based on task state.
     *
     * @param context any context of the app
     * @param title title of the notification (null becomes "Kompanion")
     * @param state "needs you", "failed", "done", or raw text
     * @param url URL to open (null becomes "")
     */
    public static void show(final Context context, final String title,
            final String state, final String url) {
        final String link = orEmpty(url);
        ensureChannels(context);
        final Intent intent = new Intent(context, MainActivity.class);
        intent.addFlags(
            Intent.FLAG_ACTIVITY_NEW_TASK | Intent.FLAG_ACTIVITY_CLEAR_TOP);
        intent.putExtra("url", link);
        final Notification.Builder builder =
            new Notification.Builder(context, TASKS)
                .setSmallIcon(R.drawable.ic_stat)
                .setContentTitle(orApp(title))
                .setContentText(label(context, state))
                .setColor(VIOLET)
                .setAutoCancel(true)
                .setContentIntent(PendingIntent.getActivity(context,
                    link.hashCode(), intent, INTENT_FLAGS));
        context.getSystemService(NotificationManager.class)
            .notify(notificationId(link), builder.build());
    }

    /**
     * The text for a task state.
     *
     * @param context any context of the app
     * @param state the state sent by the server
     * @return the translated label, or the state itself
     */
    private static String label(final Context context, final String state) {
        final String label;
        if ("needs you".equals(state)) {
            label = context.getString(R.string.state_needs_you);
        } else if ("failed".equals(state)) {
            label = context.getString(R.string.state_failed);
        } else if ("done".equals(state)) {
            label = context.getString(R.string.state_done);
        } else {
            label = state;
        }
        return label;
    }

    /**
     * One notification per link; links-less ones never replace each other.
     *
     * @param link the URL the notification opens, or ""
     * @return the notification id
     */
    private static int notificationId(final String link) {
        final int notificationId;
        if (link.isEmpty()) {
            notificationId = (int) System.currentTimeMillis();
        } else {
            notificationId = link.hashCode();
        }
        return notificationId;
    }

    /**
     * Null-safe text.
     *
     * @param text any text or null
     * @return the text, or "" for null
     */
    private static String orEmpty(final String text) {
        final String safe;
        if (text == null) {
            safe = "";
        } else {
            safe = text;
        }
        return safe;
    }

    /**
     * The title, or the app's name when there is none.
     *
     * @param title any title or null
     * @return a title that is never empty
     */
    private static String orApp(final String title) {
        final String safe;
        if (title == null || title.isEmpty()) {
            safe = "Kompanion";
        } else {
            safe = title;
        }
        return safe;
    }
}
