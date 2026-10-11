package com.kreativekompas.kompanion;

import android.app.Activity;
import android.content.ActivityNotFoundException;
import android.content.Intent;
import android.net.Uri;
import android.webkit.CookieManager;
import android.webkit.WebResourceRequest;
import android.webkit.WebView;
import android.webkit.WebViewClient;

import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;

/**
 * Keeps the server's pages in the app and opens every other link in the
 * phone's browser. Sign-in pages on another host (any server's own login
 * provider) stay in the app because the server redirects to them.
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
     * @param serverUrl the chosen server's URL
     */
    public WebClient(final Activity owner, final String serverUrl) {
        super();
        activity = owner;
        server = serverUrl;
        insideHosts = new ArrayList<>(Arrays.asList(
            owner.getResources().getStringArray(R.array.inside_hosts)));
        insideHosts.add(Uri.parse(serverUrl).getHost());
    }

    @Override
    public boolean shouldOverrideUrlLoading(final WebView view,
            final WebResourceRequest request) {
        final Uri uri = request.getUrl();
        final boolean inside = request.isRedirect()
            || insideHosts.contains(uri.getHost())
            || sameHost(uri, view.getUrl());
        if (!inside) {
            openInBrowser(uri);
        }
        return !inside;
    }

    /**
     * Whether a link stays on the host of the page that shows it.
     *
     * @param uri the link
     * @param current the shown page's URL, or null
     * @return true for a link on the same host
     */
    private static boolean sameHost(final Uri uri, final String current) {
        final String host = uri.getHost();
        boolean same = false;
        if (host != null && current != null) {
            same = host.equals(Uri.parse(current).getHost());
        }
        return same;
    }

    /**
     * Opens a link of another host in the phone's browser.
     *
     * @param uri the link
     */
    private void openInBrowser(final Uri uri) {
        try {
            activity.startActivity(new Intent(Intent.ACTION_VIEW, uri));
        } catch (ActivityNotFoundException noBrowser) {
            // No app can open it: the tap does nothing.
        }
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
