// Retires the service worker that served the game from the site root before
// the game moved to play/ (DEC-083). Browsers that installed the old worker
// fetch this file at the same URL when they check for an update; it takes
// over at once, deletes the old app's caches, unregisters itself, and reloads
// the pages it controlled, which then get the landing page from the network.
//
// Only caches made for this worker's own scope are deleted (workbox names its
// precache after the scope). The game's caches at play/ share the origin and
// must survive. Keep this file at this path for as long as old installs may
// exist; nothing registers it any more.
self.addEventListener("install", () => {
  self.skipWaiting();
});

self.addEventListener("activate", (event) => {
  event.waitUntil(
    (async () => {
      const scope = self.registration.scope;
      const names = await caches.keys();
      await Promise.all(names.filter((n) => n.endsWith(scope)).map((n) => caches.delete(n)));
      await self.registration.unregister();
      const windows = await self.clients.matchAll({ type: "window" });
      await Promise.all(windows.map((w) => w.navigate(w.url).catch(() => undefined)));
    })(),
  );
});
