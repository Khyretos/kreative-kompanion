package com.kreativekompas.kompanion;

import android.app.Activity;
import android.content.ActivityNotFoundException;
import android.content.Intent;
import android.net.Uri;
import android.webkit.CookieManager;
import android.webkit.WebResourceRequest;
import android.webkit.WebView;
import android.webkit.WebViewClient;

import java.util.Arrays;
import java.util.List;

/**
 * Keeps the server's pages in the app and opens every other link in the
 * phone's browser.
 */
public final class WebClient extends WebViewClient {
    /** The activity that shows the web view. */
    private final Activity activity;
    /** The server URL; finished pages under it mean a session exists. */
    private final String server;
    /** Hosts whose pages stay inside the app. */
    private final List<String> insideHosts;

    /**
     * A client for the main activity's web view.
     *
     * @param owner the activity that shows the web view
     * @param serverUrl the server URL from the app's strings
     */
    public WebClient(final Activity owner, final String serverUrl) {
        super();
        activity = owner;
        server = serverUrl;
        insideHosts = Arrays.asList(
            owner.getResources().getStringArray(R.array.inside_hosts));
    }

    @Override
    public boolean shouldOverrideUrlLoading(final WebView view,
            final WebResourceRequest request) {
        return openOutside(request.getUrl());
    }

    /**
     * Opens a link of another host in the phone's browser.
     *
     * @param uri the link
     * @return true when the web view must not load it
     */
    private boolean openOutside(final Uri uri) {
        final boolean outside = !insideHosts.contains(uri.getHost());
        if (outside) {
            try {
                activity.startActivity(new Intent(Intent.ACTION_VIEW, uri));
            } catch (ActivityNotFoundException noBrowser) {
                // No app can open it: the tap does nothing.
            }
        }
        return outside;
    }

    @Override
    public void onPageFinished(final WebView view, final String url) {
        if (url.startsWith(server)) {
            CookieManager.getInstance().flush();
            PushRegistration.sendIfNeeded(activity);
            // Reconnects at once after a sign-in.
            LiveService.startIfWanted(activity);
        }
    }
}
