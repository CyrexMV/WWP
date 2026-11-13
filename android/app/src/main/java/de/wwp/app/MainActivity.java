package de.wwp.app;

import android.app.Activity;
import android.content.ActivityNotFoundException;
import android.content.ClipData;
import android.content.ContentResolver;
import android.content.ContentValues;
import android.content.Intent;
import android.net.Uri;
import android.os.Build;
import android.os.Bundle;
import android.os.Environment;
import android.provider.CalendarContract;
import android.provider.MediaStore;
import android.util.Base64;
import android.view.View;
import android.view.WindowInsets;
import android.webkit.JavascriptInterface;
import android.webkit.ValueCallback;
import android.webkit.WebChromeClient;
import android.webkit.WebResourceRequest;
import android.webkit.WebResourceResponse;
import android.webkit.WebSettings;
import android.webkit.WebView;
import android.webkit.WebViewClient;
import android.widget.FrameLayout;
import android.widget.Toast;

import java.io.File;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

/**
 * Android-Hülle für Zento.
 *
 * Die komplette App (Rust/WebAssembly + Oberfläche) liegt in der APK unter assets/www und wird
 * über eine virtuelle HTTPS-Adresse geladen – es wird kein Internet benötigt. Die Daten liegen
 * in der IndexedDB der WebView (nur auf dem Gerät).
 *
 * Brücke `WWPAndroid` für die Rust-App: Dateien speichern (Download/Zento), öffnen (PDF-Betrachter)
 * und teilen bzw. per E-Mail versenden (mit vorausgefülltem Empfänger, Betreff und Text).
 */
public class MainActivity extends Activity {
    private static final String HOST = "appassets.androidplatform.net";
    private static final String START_URL = "https://" + HOST + "/index.html";
    private static final int REQUEST_FILE = 4711;

    private WebView web;
    private FrameLayout root;
    private ValueCallback<Uri[]> fileCallback;
    private Uri captureUri;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);

        root = new FrameLayout(this);
        root.setBackgroundColor(getColor(R.color.bg));
        web = new WebView(this);
        web.setBackgroundColor(getColor(R.color.bg));
        web.setOverScrollMode(View.OVER_SCROLL_NEVER);
        root.addView(web, new FrameLayout.LayoutParams(FrameLayout.LayoutParams.MATCH_PARENT, FrameLayout.LayoutParams.MATCH_PARENT));
        setContentView(root);

        // Ab Android 15 zeichnet die App unter Status-/Navigationsleiste und Tastatur:
        // diese Bereiche als Innenabstand freihalten.
        root.setOnApplyWindowInsetsListener((v, insets) -> {
            int l, t, r, b;
            if (Build.VERSION.SDK_INT >= 30) {
                android.graphics.Insets i = insets.getInsets(WindowInsets.Type.systemBars()
                        | WindowInsets.Type.ime() | WindowInsets.Type.displayCutout());
                l = i.left; t = i.top; r = i.right; b = i.bottom;
            } else {
                l = insets.getSystemWindowInsetLeft();
                t = insets.getSystemWindowInsetTop();
                r = insets.getSystemWindowInsetRight();
                b = insets.getSystemWindowInsetBottom();
            }
            v.setPadding(l, t, r, b);
            return insets;
        });

        WebSettings s = web.getSettings();
        s.setJavaScriptEnabled(true);
        s.setDomStorageEnabled(true);
        s.setDatabaseEnabled(true);
        s.setAllowFileAccess(false);
        s.setAllowContentAccess(true);
        s.setSupportZoom(false);
        s.setBuiltInZoomControls(false);
        s.setDisplayZoomControls(false);
        s.setTextZoom(100);
        s.setCacheMode(WebSettings.LOAD_NO_CACHE);

        web.addJavascriptInterface(new Bridge(), "WWPAndroid");
        web.setWebViewClient(new AssetClient());
        web.setWebChromeClient(new ChromeClient());
        web.loadUrl(START_URL);
    }

    // ------------------------------------------------------------------
    // Lebenszyklus
    // ------------------------------------------------------------------

    @Override
    protected void onPause() {
        super.onPause();
        if (web != null) {
            // beim Verlassen sofort speichern
            web.evaluateJavascript("document.dispatchEvent(new Event('visibilitychange'))", null);
            web.onPause();
        }
    }

    @Override
    protected void onResume() {
        super.onResume();
        if (web != null) {
            web.onResume();
        }
    }

    @Override
    protected void onDestroy() {
        if (web != null) {
            web.destroy();
            web = null;
        }
        super.onDestroy();
    }

    /** Zurück-Taste: Dialog schließen → vorige Ansicht → App verlassen (alles ist automatisch gespeichert). */
    @Override
    @SuppressWarnings("deprecation")
    public void onBackPressed() {
        if (web == null) {
            finish();
            return;
        }
        web.evaluateJavascript("(window.wwpBack&&window.wwpBack())?1:0", result -> {
            if (!"1".equals(result)) {
                finish();
            }
        });
    }

    // ------------------------------------------------------------------
    // Dateiauswahl (Logo, Belege inkl. Kamera, Backup)
    // ------------------------------------------------------------------

    @Override
    @SuppressWarnings("deprecation")
    protected void onActivityResult(int requestCode, int resultCode, Intent data) {
        if (requestCode == REQUEST_FILE) {
            Uri[] result = null;
            if (resultCode == RESULT_OK) {
                if (data != null && data.getData() != null) {
                    result = new Uri[]{data.getData()};
                } else if (captureUri != null) {
                    File f = ShareProvider.fileFor(this, captureUri.getLastPathSegment());
                    if (f != null && f.length() > 0) {
                        result = new Uri[]{captureUri};
                    }
                }
            }
            captureUri = null;
            if (fileCallback != null) {
                fileCallback.onReceiveValue(result);
                fileCallback = null;
            }
            return;
        }
        super.onActivityResult(requestCode, resultCode, data);
    }

    private class ChromeClient extends WebChromeClient {
        @Override
        @SuppressWarnings("deprecation")
        public boolean onShowFileChooser(WebView view, ValueCallback<Uri[]> callback, FileChooserParams params) {
            if (fileCallback != null) {
                fileCallback.onReceiveValue(null);
            }
            fileCallback = callback;
            List<String> mimes = new ArrayList<>();
            boolean onlyMimes = true;
            boolean images = false;
            String[] types = params.getAcceptTypes();
            if (types != null) {
                for (String raw : types) {
                    if (raw == null) {
                        continue;
                    }
                    for (String part : raw.split(",")) {
                        String t = part.trim().toLowerCase();
                        if (t.isEmpty()) {
                            continue;
                        }
                        if (!t.contains("/")) {
                            onlyMimes = false;
                            continue;
                        }
                        if (t.startsWith("image")) {
                            images = true;
                        }
                        mimes.add(t);
                    }
                }
            }
            Intent pick = new Intent(Intent.ACTION_OPEN_DOCUMENT);
            pick.addCategory(Intent.CATEGORY_OPENABLE);
            if (onlyMimes && mimes.size() == 1) {
                pick.setType(mimes.get(0));
            } else {
                pick.setType("*/*");
                if (onlyMimes && !mimes.isEmpty()) {
                    pick.putExtra(Intent.EXTRA_MIME_TYPES, mimes.toArray(new String[0]));
                }
            }
            Intent launch = pick;
            // Bei Bildern zusätzlich die Kamera anbieten (Beleg fotografieren)
            if (images) {
                try {
                    String name = "capture_" + System.currentTimeMillis() + ".jpg";
                    ShareProvider.cleanup(MainActivity.this);
                    captureUri = new Uri.Builder().scheme("content").authority(ShareProvider.AUTHORITY).appendPath(name).build();
                    Intent cam = new Intent(MediaStore.ACTION_IMAGE_CAPTURE);
                    cam.putExtra(MediaStore.EXTRA_OUTPUT, captureUri);
                    cam.setClipData(ClipData.newRawUri(name, captureUri));
                    cam.addFlags(Intent.FLAG_GRANT_WRITE_URI_PERMISSION | Intent.FLAG_GRANT_READ_URI_PERMISSION);
                    if (cam.resolveActivity(getPackageManager()) != null) {
                        launch = Intent.createChooser(pick, "Datei wählen oder Foto aufnehmen");
                        launch.putExtra(Intent.EXTRA_INITIAL_INTENTS, new Intent[]{cam});
                    } else {
                        captureUri = null;
                    }
                } catch (Exception e) {
                    captureUri = null;
                }
            }
            try {
                startActivityForResult(launch, REQUEST_FILE);
            } catch (ActivityNotFoundException e) {
                fileCallback = null;
                captureUri = null;
                callback.onReceiveValue(null);
                return false;
            }
            return true;
        }
    }

    // ------------------------------------------------------------------
    // App-Dateien aus der APK ausliefern
    // ------------------------------------------------------------------

    private class AssetClient extends WebViewClient {
        @Override
        public WebResourceResponse shouldInterceptRequest(WebView view, WebResourceRequest request) {
            Uri uri = request.getUrl();
            if (!HOST.equals(uri.getHost())) {
                return null;
            }
            String path = uri.getPath();
            if (path == null || path.isEmpty() || "/".equals(path)) {
                path = "/index.html";
            }
            if (path.contains("..")) {
                return notFound();
            }
            try {
                InputStream in = getAssets().open("www" + path);
                String mime = mimeType(path);
                boolean text = mime.startsWith("text/") || mime.endsWith("json") || mime.endsWith("javascript");
                WebResourceResponse res = new WebResourceResponse(mime, text ? "utf-8" : null, in);
                Map<String, String> headers = new HashMap<>();
                headers.put("Cache-Control", "no-cache");
                res.setResponseHeaders(headers);
                return res;
            } catch (IOException e) {
                return notFound();
            }
        }

        @Override
        public boolean shouldOverrideUrlLoading(WebView view, WebResourceRequest request) {
            Uri uri = request.getUrl();
            if (HOST.equals(uri.getHost())) {
                return false;
            }
            String scheme = uri.getScheme() == null ? "" : uri.getScheme();
            try {
                if ("mailto".equals(scheme)) {
                    startActivity(new Intent(Intent.ACTION_SENDTO, uri));
                } else if ("tel".equals(scheme)) {
                    startActivity(new Intent(Intent.ACTION_DIAL, uri));
                } else {
                    startActivity(new Intent(Intent.ACTION_VIEW, uri));
                }
            } catch (ActivityNotFoundException ignored) {
                Toast.makeText(MainActivity.this, "Keine passende App gefunden", Toast.LENGTH_SHORT).show();
            }
            return true;
        }

        private WebResourceResponse notFound() {
            return new WebResourceResponse("text/plain", "utf-8", 404, "Not Found", null, null);
        }
    }

    private static String mimeType(String path) {
        String p = path.toLowerCase();
        if (p.endsWith(".html")) return "text/html";
        if (p.endsWith(".js") || p.endsWith(".mjs")) return "text/javascript";
        if (p.endsWith(".wasm")) return "application/wasm";
        if (p.endsWith(".css")) return "text/css";
        if (p.endsWith(".json")) return "application/json";
        if (p.endsWith(".webmanifest")) return "application/manifest+json";
        if (p.endsWith(".svg")) return "image/svg+xml";
        if (p.endsWith(".png")) return "image/png";
        if (p.endsWith(".jpg") || p.endsWith(".jpeg")) return "image/jpeg";
        return "application/octet-stream";
    }

    // ------------------------------------------------------------------
    // Brücke für die Rust-App
    // ------------------------------------------------------------------

    private static String safeName(String name) {
        String n = name == null || name.isEmpty() ? "Datei" : name;
        return n.replaceAll("[^A-Za-z0-9ÄÖÜäöüß ._-]", "_");
    }

    /** Datei in den App-Cache legen und als content://-Adresse liefern. */
    private Uri cacheFile(String name, String base64) throws IOException {
        byte[] data = Base64.decode(base64, Base64.DEFAULT);
        ShareProvider.cleanup(this);
        String safe = safeName(name);
        File f = new File(ShareProvider.dir(this), safe);
        try (FileOutputStream os = new FileOutputStream(f)) {
            os.write(data);
        }
        return new Uri.Builder().scheme("content").authority(ShareProvider.AUTHORITY).appendPath(safe).build();
    }

    public class Bridge {
        /** Speichert eine Datei: Bilder in „Bilder/Zento“, alles andere in „Download/Zento“. */
        @JavascriptInterface
        public boolean saveFile(String name, String mime, String base64) {
            try {
                byte[] data = Base64.decode(base64, Base64.DEFAULT);
                boolean image = mime != null && mime.startsWith("image/");
                ContentValues v = new ContentValues();
                v.put(MediaStore.MediaColumns.DISPLAY_NAME, safeName(name));
                v.put(MediaStore.MediaColumns.MIME_TYPE, mime);
                v.put(MediaStore.MediaColumns.RELATIVE_PATH,
                        (image ? Environment.DIRECTORY_PICTURES : Environment.DIRECTORY_DOWNLOADS) + "/Zento");
                v.put(MediaStore.MediaColumns.IS_PENDING, 1);
                Uri collection = image
                        ? MediaStore.Images.Media.getContentUri(MediaStore.VOLUME_EXTERNAL_PRIMARY)
                        : MediaStore.Downloads.getContentUri(MediaStore.VOLUME_EXTERNAL_PRIMARY);
                ContentResolver cr = getContentResolver();
                Uri uri = cr.insert(collection, v);
                if (uri == null) {
                    return false;
                }
                try (OutputStream os = cr.openOutputStream(uri)) {
                    if (os == null) {
                        return false;
                    }
                    os.write(data);
                }
                v.clear();
                v.put(MediaStore.MediaColumns.IS_PENDING, 0);
                cr.update(uri, v, null, null);
                final String where = image ? "Bilder/Zento" : "Download/Zento";
                runOnUiThread(() -> Toast.makeText(MainActivity.this, "Gespeichert in " + where, Toast.LENGTH_SHORT).show());
                return true;
            } catch (Exception e) {
                return false;
            }
        }

        /** Datei mit einer passenden App ansehen (z. B. PDF-Betrachter). */
        @JavascriptInterface
        public boolean openFile(String name, String mime, String base64) {
            try {
                Uri uri = cacheFile(name, base64);
                Intent view = new Intent(Intent.ACTION_VIEW);
                view.setDataAndType(uri, mime == null || mime.isEmpty() ? "application/octet-stream" : mime);
                view.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION | Intent.FLAG_ACTIVITY_NEW_TASK);
                if (view.resolveActivity(getPackageManager()) == null) {
                    return false;
                }
                runOnUiThread(() -> {
                    try {
                        startActivity(view);
                    } catch (ActivityNotFoundException e) {
                        Toast.makeText(MainActivity.this, "Keine App zum Öffnen gefunden", Toast.LENGTH_SHORT).show();
                    }
                });
                return true;
            } catch (Exception e) {
                return false;
            }
        }

        /**
         * Datei teilen bzw. per E-Mail senden. Empfänger, Betreff und Text werden vorausgefüllt
         * (Gmail, Outlook, Samsung E-Mail …); andere Apps (WhatsApp, Drive) erhalten die Datei.
         */
        @JavascriptInterface
        public boolean shareFile(String name, String mime, String base64, String to, String subject, String text) {
            try {
                Uri uri = cacheFile(name, base64);
                Intent send = new Intent(Intent.ACTION_SEND);
                send.setType(mime == null || mime.isEmpty() ? "application/octet-stream" : mime);
                send.putExtra(Intent.EXTRA_STREAM, uri);
                if (to != null && !to.trim().isEmpty()) {
                    send.putExtra(Intent.EXTRA_EMAIL, new String[]{to.trim()});
                }
                if (subject != null && !subject.isEmpty()) {
                    send.putExtra(Intent.EXTRA_SUBJECT, subject);
                }
                if (text != null && !text.isEmpty()) {
                    send.putExtra(Intent.EXTRA_TEXT, text);
                }
                send.setClipData(ClipData.newRawUri(safeName(name), uri));
                send.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION);
                Intent chooser = Intent.createChooser(send, "Senden mit …");
                chooser.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION);
                runOnUiThread(() -> startActivity(chooser));
                return true;
            } catch (Exception e) {
                return false;
            }
        }

        /** Termin in die Kalender-App übernehmen (Samsung Kalender, Google Kalender …). */
        @JavascriptInterface
        public boolean calendarInsert(String title, double begin, double end, boolean allDay, String location, String description) {
            Intent i = new Intent(Intent.ACTION_INSERT)
                    .setData(CalendarContract.Events.CONTENT_URI)
                    .putExtra(CalendarContract.EXTRA_EVENT_BEGIN_TIME, (long) begin)
                    .putExtra(CalendarContract.EXTRA_EVENT_END_TIME, (long) end)
                    .putExtra(CalendarContract.EXTRA_EVENT_ALL_DAY, allDay)
                    .putExtra(CalendarContract.Events.TITLE, title)
                    .putExtra(CalendarContract.Events.EVENT_LOCATION, location)
                    .putExtra(CalendarContract.Events.DESCRIPTION, description);
            try {
                runOnUiThread(() -> {
                    try {
                        startActivity(i);
                    } catch (ActivityNotFoundException e) {
                        Toast.makeText(MainActivity.this, "Keine Kalender-App gefunden", Toast.LENGTH_SHORT).show();
                    }
                });
                return true;
            } catch (Exception e) {
                return false;
            }
        }

        /** Webadresse (z. B. Google Maps) in der passenden App öffnen. */
        @JavascriptInterface
        public boolean openUrl(String url) {
            try {
                Intent i = new Intent(Intent.ACTION_VIEW, Uri.parse(url));
                runOnUiThread(() -> {
                    try {
                        startActivity(i);
                    } catch (ActivityNotFoundException e) {
                        Toast.makeText(MainActivity.this, "Keine passende App gefunden", Toast.LENGTH_SHORT).show();
                    }
                });
                return true;
            } catch (Exception e) {
                return false;
            }
        }

        @JavascriptInterface
        public String version() {
            try {
                return getPackageManager().getPackageInfo(getPackageName(), 0).versionName;
            } catch (Exception e) {
                return "";
            }
        }
    }
}
