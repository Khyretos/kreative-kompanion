package com.kreativekompas.kompanion;

import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;

/**
 * Starts the live connection after the phone booted.
 */
public final class BootReceiver extends BroadcastReceiver {
    /** Created by Android. */
    public BootReceiver() {
        super();
    }

    @Override
    public void onReceive(final Context context, final Intent intent) {
        if (Intent.ACTION_BOOT_COMPLETED.equals(intent.getAction())) {
            LiveService.startIfWanted(context);
        }
    }
}
