package com.kreativekompas.kompanion;

import android.app.Activity;
import android.content.Context;

import org.unifiedpush.android.connector.UnifiedPush;

import kotlin.Unit;

/**
 * Picks the UnifiedPush distributor, or falls back to the own live
 * connection when there is none.
 */
public final class Distributors {
    /** Preferences file of the push state. */
    private static final String PREFS = "push";
    /** Key of the delivery mode ("push" while a distributor is used). */
    private static final String MODE = "mode";

    private Distributors() {
        // Only static helpers.
    }

    /**
     * Uses the current or default distributor, else an installed one.
     *
     * @param activity the main activity
     */
    public static void choose(final Activity activity) {
        UnifiedPush.tryUseCurrentOrDefaultDistributor(activity, found -> {
            if (found) {
                register(activity);
            } else {
                activity.runOnUiThread(() -> pickInstalled(activity));
            }
            return Unit.INSTANCE;
        });
    }

    /**
     * Registers with the chosen distributor and stops the live connection.
     *
     * @param context any context of the app
     */
    private static void register(final Context context) {
        context.getSharedPreferences(PREFS, Context.MODE_PRIVATE).edit()
            .putString(MODE, "push").apply();
        LiveService.stop(context);
        UnifiedPush.register(context, "default",
            context.getString(R.string.app_name), null);
    }

    /**
     * No default distributor answered: use the first installed one that is
     * not this app (usually ntfy), else the app's own live connection (no
     * setup needed).
     *
     * @param context any context of the app
     */
    private static void pickInstalled(final Context context) {
        String chosen = null;
        for (final String distributor : UnifiedPush.getDistributors(context)) {
            if (chosen == null
                && !distributor.equals(context.getPackageName())) {
                chosen = distributor;
            }
        }
        if (chosen == null) {
            context.getSharedPreferences(PREFS, Context.MODE_PRIVATE).edit()
                .remove(MODE).apply();
            LiveService.startIfWanted(context);
        } else {
            UnifiedPush.saveDistributor(context, chosen);
            register(context);
        }
    }
}
