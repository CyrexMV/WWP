package de.wwp.app;

import android.content.ContentProvider;
import android.content.ContentValues;
import android.content.Context;
import android.database.Cursor;
import android.database.MatrixCursor;
import android.net.Uri;
import android.os.ParcelFileDescriptor;
import android.provider.OpenableColumns;

import java.io.File;
import java.io.FileNotFoundException;
import java.io.IOException;

/**
 * Stellt Dateien aus dem Teilen-Ordner (cache/share) für andere Apps bereit – nur mit der über
 * das Intent erteilten Freigabe. Kamera-Fotos (capture_*.jpg) dürfen zusätzlich geschrieben werden.
 */
public class ShareProvider extends ContentProvider {
    public static final String AUTHORITY = "de.wwp.app.share";

    static File dir(Context c) {
        File d = new File(c.getCacheDir(), "share");
        //noinspection ResultOfMethodCallIgnored
        d.mkdirs();
        return d;
    }

    /** Datei im Teilen-Ordner (null bei ungültigem Namen). */
    static File fileFor(Context c, String name) {
        if (name == null || c == null) {
            return null;
        }
        File d = dir(c);
        File f = new File(d, name);
        try {
            if (!f.getCanonicalFile().getParentFile().equals(d.getCanonicalFile())) {
                return null;
            }
        } catch (IOException e) {
            return null;
        }
        return f;
    }

    /** Dateien, die älter als eine Stunde sind, entfernen. */
    static void cleanup(Context c) {
        File[] old = dir(c).listFiles();
        if (old == null) {
            return;
        }
        long limit = System.currentTimeMillis() - 60 * 60 * 1000L;
        for (File f : old) {
            if (f.lastModified() < limit) {
                //noinspection ResultOfMethodCallIgnored
                f.delete();
            }
        }
    }

    private File file(Uri uri, boolean write) throws FileNotFoundException {
        String name = uri.getLastPathSegment();
        File f = fileFor(getContext(), name);
        if (f == null) {
            throw new FileNotFoundException();
        }
        if (write) {
            if (!name.startsWith("capture_")) {
                throw new FileNotFoundException();
            }
        } else if (!f.exists()) {
            throw new FileNotFoundException();
        }
        return f;
    }

    @Override
    public boolean onCreate() {
        return true;
    }

    @Override
    public Cursor query(Uri uri, String[] projection, String selection, String[] args, String sort) {
        try {
            File f = file(uri, false);
            MatrixCursor c = new MatrixCursor(new String[]{OpenableColumns.DISPLAY_NAME, OpenableColumns.SIZE});
            c.addRow(new Object[]{f.getName(), f.length()});
            return c;
        } catch (FileNotFoundException e) {
            return null;
        }
    }

    @Override
    public String getType(Uri uri) {
        String n = uri.getLastPathSegment() == null ? "" : uri.getLastPathSegment().toLowerCase();
        if (n.endsWith(".pdf")) return "application/pdf";
        if (n.endsWith(".png")) return "image/png";
        if (n.endsWith(".jpg") || n.endsWith(".jpeg")) return "image/jpeg";
        if (n.endsWith(".csv")) return "text/csv";
        if (n.endsWith(".xml")) return "application/xml";
        if (n.endsWith(".ics")) return "text/calendar";
        if (n.endsWith(".json") || n.endsWith(".wwpbackup")) return "application/json";
        return "application/octet-stream";
    }

    @Override
    public ParcelFileDescriptor openFile(Uri uri, String mode) throws FileNotFoundException {
        boolean write = mode != null && mode.contains("w");
        File f = file(uri, write);
        if (write) {
            return ParcelFileDescriptor.open(f, ParcelFileDescriptor.MODE_READ_WRITE
                    | ParcelFileDescriptor.MODE_CREATE | ParcelFileDescriptor.MODE_TRUNCATE);
        }
        return ParcelFileDescriptor.open(f, ParcelFileDescriptor.MODE_READ_ONLY);
    }

    @Override
    public Uri insert(Uri uri, ContentValues values) {
        throw new UnsupportedOperationException();
    }

    @Override
    public int delete(Uri uri, String selection, String[] args) {
        return 0;
    }

    @Override
    public int update(Uri uri, ContentValues values, String selection, String[] args) {
        return 0;
    }
}
