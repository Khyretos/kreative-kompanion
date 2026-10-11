package com.kreativekompas.kompanion;

import android.Manifest;
import android.app.Activity;
import android.content.Intent;
import android.content.pm.PackageManager;
import android.os.Build.VERSION;
import android.os.Bundle;
import android.webkit.CookieManager;
import android.webkit.WebChromeClient;
import android.webkit.WebSettings;
import android.webkit.WebView;
import android.window.OnBackInvokedDispatcher;

/**
 * The app: the Kompanion web app in a web view, plus notifications.
 */
public final class MainActivity extends Activity {
    /** Android 13 (API 33): notification permission and back callbacks. */
    private static final int TIRAMISU = 33;
    /** Request code of the notification permission. */
    private static final int ASK_NOTIFICATIONS = 1;
    /** Asks the page to handle back; it answers true when it did. */
    private static final String PAGE_BACK =
        "window.kompanionBack ? window.kompanionBack() : false";

    /** The web view with the web app. */
    private WebView web;
    /** The chosen server's URL (client mode, Server). */
    private String server;

    /** Created by Android. */
    public MainActivity() {
        super();
    }

    @Override
    protected void onCreate(final Bundle state) {
        super.onCreate(state);
        // Client mode (REL-01): the first start asks for the server.
        if (Server.chosen(this)) {
            setContentView(R.layout.activity_main);
            // BUG-05: with targetSdk 36 Android 16 no longer calls
            // onBackPressed (predictive back), so back is registered here.
            if (VERSION.SDK_INT >= TIRAMISU) {
                getOnBackInvokedDispatcher().registerOnBackInvokedCallback(
                    OnBackInvokedDispatcher.PRIORITY_DEFAULT, this::back);
            }
            server = Server.url(this);
            web = findViewById(R.id.web);
            setUpWebView();
            if (state == null) {
                web.loadUrl(startUrl(getIntent()));
            } else {
                web.restoreState(state);
            }
            askForNotifications();
            SettingsActivity.showOnceForMaker(this);
            Distributors.choose(this);
        } else {
            startActivity(new Intent(this, ServerActivity.class));
            finish();
        }
    }

    /** Settings, cookies and clients of the web view. */
    private void setUpWebView() {
        final WebSettings settings = web.getSettings();
        settings.setJavaScriptEnabled(true);
        settings.setDomStorageEnabled(true);
        settings.setMediaPlaybackRequiresUserGesture(false);
        final CookieManager cookies = CookieManager.getInstance();
        cookies.setAcceptCookie(true);
        cookies.setAcceptThirdPartyCookies(web, false);
        web.setWebViewClient(new WebClient(this, server));
        web.setWebChromeClient(new WebChromeClient());
    }

    /** Android 13 and later: ask once for the notification permission. */
    private void askForNotifications() {
        if (VERSION.SDK_INT >= TIRAMISU
            && checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS)
                != PackageManager.PERMISSION_GRANTED) {
            requestPermissions(
                new String[] {Manifest.permission.POST_NOTIFICATIONS},
                ASK_NOTIFICATIONS);
        }
    }

    /**
     * The page a notification asked for, else the server's start page.
     *
     * @param intent the intent that started the activity, or null
     * @return a URL on the server
     */
    private String startUrl(final Intent intent) {
        final String asked = askedUrl(intent);
        final String url;
        if (asked == null) {
            url = server + "/";
        } else {
            url = asked;
        }
        return url;
    }

    /**
     * The URL in a notification's intent, when it is on the server.
     *
     * @param intent an intent, or null
     * @return the URL, or null
     */
    private String askedUrl(final Intent intent) {
        String url = null;
        if (intent != null) {
            url = intent.getStringExtra("url");
        }
        if (url != null && !url.startsWith(server + "/")) {
            url = null;
        }
        return url;
    }

    @Override
    protected void onNewIntent(final Intent intent) {
        super.onNewIntent(intent);
        setIntent(intent);
        final String url = askedUrl(intent);
        if (url != null) {
            web.loadUrl(url);
        }
    }

    @Override
    protected void onSaveInstanceState(final Bundle state) {
        super.onSaveInstanceState(state);
        web.saveState(state);
    }

    /** Android 12 and older call this; newer ones use the callback above. */
    @SuppressWarnings("deprecation")
    @Override
    public void onBackPressed() {
        back();
    }

    /**
     * BUG-05: back asks the page first (it closes a menu or panel, or goes
     * from Studio, Assets or Capabilities to the chat); only on the chat does
     * the app go to the background.
     */
    private void back() {
        web.evaluateJavascript(PAGE_BACK, handled -> {
            if (!"true".equals(handled)) {
                if (web.canGoBack()) {
                    web.goBack();
                } else {
                    moveTaskToBack(true);
                }
            }
        });
    }

    @Override
    protected void onPause() {
        super.onPause();
        CookieManager.getInstance().flush();
    }
}
