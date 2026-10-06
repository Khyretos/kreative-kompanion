package com.kreativekompas.kompanion;

import android.app.Notification;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.content.Context;
import android.content.Intent;
import android.os.Bundle;
import android.app.PendingIntent;

public final class Notifier {

    private Notifier() {
        // Private constructor to prevent instantiation
    }

    /**
     * Ensures notification channels exist for tasks and live updates.
     */
    public static void ensureChannels(Context c) {
        NotificationManager manager = (NotificationManager) c.getSystemService(Context.NOTIFICATION_SERVICE);

        // Channel "tasks": IMPORTANCE_HIGH
        if (manager.getNotificationChannel("tasks") == null) {
            NotificationChannel channelTasks = new NotificationChannel(
                    "tasks",
                    c.getString(R.string.channel_tasks),
                    NotificationManager.IMPORTANCE_HIGH
            );
            manager.createNotificationChannel(channelTasks);
        }

        // Channel "live": IMPORTANCE_MIN, setShowBadge(false)
        if (manager.getNotificationChannel("live") == null) {
            NotificationChannel channelLive = new NotificationChannel(
                    "live",
                    c.getString(R.string.channel_live),
                    NotificationManager.IMPORTANCE_MIN
            );
            channelLive.setShowBadge(false);
            manager.createNotificationChannel(channelLive);
        }
    }

    /**
     * Shows a notification based on task state.
     *
     * @param c Context
     * @param title Title of the notification (null becomes "Kompanion")
     * @param state State string ("needs you", "failed", "done", or raw text)
     * @param url URL to open (null becomes "")
     */
    public static void show(Context c, String title, String state, String url) {
        // Normalize inputs
        if (title == null || title.isEmpty()) {
            title = "Kompanion";
        }
        if (url == null || url.isEmpty()) {
            url = "";
        }

        // Map state to resource ID or keep raw string
        int resId;
        if ("needs you".equals(state)) {
            resId = R.string.state_needs_you;
        } else if ("failed".equals(state)) {
            resId = R.string.state_failed;
        } else if ("done".equals(state)) {
            resId = R.string.state_done;
        } else {
            // Fallback: use the state string itself as content text
            resId = 0; // Sentinel value indicating we should use the raw string
        }

        String label;
        if (resId != 0) {
            label = c.getString(resId);
        } else {
            label = state;
        }

        ensureChannels(c);

        Intent intent = new Intent(c, MainActivity.class);
        intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK | Intent.FLAG_ACTIVITY_CLEAR_TOP);

        Bundle extras = new Bundle();
        extras.putString("url", url);
        intent.putExtras(extras);

        PendingIntent pendingIntent;
        if (url.isEmpty()) {
            pendingIntent = PendingIntent.getActivity(c, 0, intent, 
                    PendingIntent.FLAG_UPDATE_CURRENT | PendingIntent.FLAG_IMMUTABLE);
        } else {
            pendingIntent = PendingIntent.getActivity(c, url.hashCode(), intent, 
                    PendingIntent.FLAG_UPDATE_CURRENT | PendingIntent.FLAG_IMMUTABLE);
        }

        Notification.Builder builder = new Notification.Builder(c, "tasks")
                .setSmallIcon(R.drawable.ic_stat)
                .setContentTitle(title)
                .setContentText(label)
                .setColor(0xFFF3941F)
                .setAutoCancel(true)
                .setContentIntent(pendingIntent);

        int notificationId;
        if (url.isEmpty()) {
            notificationId = (int) System.currentTimeMillis();
        } else {
            notificationId = url.hashCode();
        }

        NotificationManager notificationManager = (NotificationManager) c.getSystemService(Context.NOTIFICATION_SERVICE);
        notificationManager.notify(notificationId, builder.build());
    }
}
