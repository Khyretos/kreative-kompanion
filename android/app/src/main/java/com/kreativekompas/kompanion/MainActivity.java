package com.kreativekompas.kompanion;

import android.Manifest;
import android.app.Activity;
import android.content.ActivityNotFoundException;
import android.content.Intent;
import android.content.pm.PackageManager;
import android.os.Build;
import android.os.Bundle;
import android.webkit.CookieManager;
import android.webkit.WebChromeClient;
import android.webkit.WebResourceRequest;
import android.webkit.WebSettings;
import android.webkit.WebView;
import android.webkit.WebViewClient;
import android.window.OnBackInvokedDispatcher;
import java.util.Arrays;
import org.unifiedpush.android.connector.UnifiedPush;
import kotlin.Unit;

public class MainActivity extends Activity {
    private WebView web;
    private String server;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        setContentView(R.layout.activity_main);
        // BUG-05: with targetSdk 36 Android 16 no longer calls onBackPressed (predictive back),
        // so back is registered here as well.
        if (Build.VERSION.SDK_INT >= 33) {
            getOnBackInvokedDispatcher().registerOnBackInvokedCallback(
                    OnBackInvokedDispatcher.PRIORITY_DEFAULT, this::back);
        }
        server = getString(R.string.server_url);
        web = findViewById(R.id.web);

        WebSettings s = web.getSettings();
        s.setJavaScriptEnabled(true);
        s.setDomStorageEnabled(true);
        s.setMediaPlaybackRequiresUserGesture(false);

        CookieManager.getInstance().setAcceptCookie(true);
        CookieManager.getInstance().setAcceptThirdPartyCookies(web, false);

        web.setWebViewClient(new WebViewClient() {
            @Override
            public boolean shouldOverrideUrlLoading(WebView v, WebResourceRequest r) {
                String host = r.getUrl().getHost();
                if (Arrays.asList(getResources().getStringArray(R.array.inside_hosts)).contains(host)) {
                    return false;
                } else {
                    try {
                        startActivity(new Intent(Intent.ACTION_VIEW, r.getUrl()));
                    } catch (ActivityNotFoundException e) {
                        // ignore
                    }
                    return true;
                }
            }

            @Override
            public void onPageFinished(WebView v, String url) {
                if (url.startsWith(server)) {
                    CookieManager.getInstance().flush();
                    Push.sendIfNeeded(MainActivity.this);
                    LiveService.startIfWanted(MainActivity.this); // reconnects at once after a sign-in
                }
            }
        });

        web.setWebChromeClient(new WebChromeClient());

        if (savedInstanceState != null) {
            web.restoreState(savedInstanceState);
        } else {
            web.loadUrl(startUrl(getIntent()));
        }

        if (Build.VERSION.SDK_INT >= 33 && checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED) {
            requestPermissions(new String[] { Manifest.permission.POST_NOTIFICATIONS }, 1);
        }
        SettingsActivity.showOnceForMaker(this);

        UnifiedPush.tryUseCurrentOrDefaultDistributor(this, ok -> {
            if (ok) {
                registerPush();
            } else {
                runOnUiThread(this::pickInstalledDistributor);
            }
            return kotlin.Unit.INSTANCE;
        });
    }

    private void registerPush() {
        getSharedPreferences("push", MODE_PRIVATE).edit().putString("mode", "push").apply();
        LiveService.stop(this);
        UnifiedPush.register(this, "default", getString(R.string.app_name), null);
    }

    /** No default distributor answered: use the first installed one that is not this app (usually ntfy),
     *  else the app's own live connection (no setup needed). */
    private void pickInstalledDistributor() {
        for (String d : UnifiedPush.getDistributors(this)) {
            if (!d.equals(getPackageName())) {
                UnifiedPush.saveDistributor(this, d);
                registerPush();
                return;
            }
        }
        getSharedPreferences("push", MODE_PRIVATE).edit().remove("mode").apply();
        LiveService.startIfWanted(this);
    }

    private String startUrl(Intent i) {
        String u = i == null ? null : i.getStringExtra("url");
        return (u != null && u.startsWith(server + "/")) ? u : server + "/";
    }

    @Override
    protected void onNewIntent(Intent i) {
        super.onNewIntent(i);
        setIntent(i);
        String u = i.getStringExtra("url");
        if (u != null && u.startsWith(server + "/")) {
            web.loadUrl(u);
        }
    }

    @Override
    protected void onSaveInstanceState(Bundle b) {
        super.onSaveInstanceState(b);
        web.saveState(b);
    }

    @SuppressWarnings("deprecation")
    @Override
    public void onBackPressed() {
        back();
    }

    /** BUG-05: back asks the page first (it closes a menu or panel, or goes from Studio, Assets or
     *  Capabilities to the chat); only on the chat does the app go to the background. */
    private void back() {
        web.evaluateJavascript("window.kompanionBack ? window.kompanionBack() : false", r -> {
            if ("true".equals(r)) return;
            if (web.canGoBack()) web.goBack();
            else moveTaskToBack(true);
        });
    }

    @Override
    protected void onPause() {
        super.onPause();
        CookieManager.getInstance().flush();
    }
}
