// Service Worker: macht Zento als installierte Web-App offline nutzbar.
const VERSION = '20261114010305';
const CACHE = 'wwp-' + VERSION;
const FILES = [
  './',
  './index.html',
  './style.css',
  './fonts/inter.woff2',
  './manifest.webmanifest',
  './pkg/wwp.js',
  './pkg/wwp_bg.wasm',
  './icons/icon.svg',
  './icons/icon-192.png',
  './icons/icon-512.png',
  './icons/icon-maskable-512.png',
];

self.addEventListener('install', (e) => {
  // cache: 'reload' – beim Update nie veraltete Dateien aus dem Browser-Zwischenspeicher übernehmen
  e.waitUntil(caches.open(CACHE).then((c) => c.addAll(FILES.map((f) => new Request(f, { cache: 'reload' })))).then(() => self.skipWaiting()));
});

self.addEventListener('activate', (e) => {
  e.waitUntil(
    caches.keys()
      .then((keys) => Promise.all(keys.filter((k) => k !== CACHE).map((k) => caches.delete(k))))
      .then(() => self.clients.claim())
  );
});

// Netzwerk zuerst (für Updates), bei fehlender Verbindung aus dem Cache.
self.addEventListener('fetch', (e) => {
  if (e.request.method !== 'GET') return;
  // eigene Dateien immer beim Server nachfragen (no-cache = mit ETag prüfen, kostet kaum Datenvolumen),
  // damit eine neue Version sofort ankommt und nicht erst nach Ablauf des Zwischenspeichers
  const eigen = new URL(e.request.url).origin === location.origin;
  e.respondWith(
    (eigen ? fetch(e.request.url, { cache: 'no-cache', credentials: 'same-origin' }) : fetch(e.request))
      .then((res) => {
        if (res.ok && new URL(e.request.url).origin === location.origin) {
          const copy = res.clone();
          caches.open(CACHE).then((c) => c.put(e.request, copy));
        }
        return res;
      })
      .catch(() => caches.match(e.request, { ignoreSearch: true }).then((r) => r || caches.match('./index.html')))
  );
});
