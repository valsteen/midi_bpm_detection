const cacheName = 'midi-bpm-detector-static-v1';
const filesToCache = [
  './manifest.json',
  './icon-1024.png',
  './icon-256.png',
  './icon_ios_touch_192.png',
  './maskable_icon_x512.png',
];

self.addEventListener('install', function (e) {
  e.waitUntil(
    caches.open(cacheName).then(function (cache) {
      return cache.addAll(filesToCache);
    })
  );
  self.skipWaiting();
});

self.addEventListener('activate', function (e) {
  e.waitUntil(
    caches.keys().then(function (cacheNames) {
      return Promise.all(
        cacheNames
          .filter(function (activeCacheName) {
            return activeCacheName !== cacheName;
          })
          .map(function (staleCacheName) {
            return caches.delete(staleCacheName);
          })
      );
    }).then(function () {
      return self.clients.claim();
    })
  );
});

self.addEventListener('fetch', function (e) {
  if (e.request.mode === 'navigate') {
    return;
  }

  e.respondWith(
    caches.match(e.request).then(function (response) {
      return response || fetch(e.request);
    })
  );
});
