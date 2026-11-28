const CACHE_NAME = 'rust-pwa-v1';
const APP_SHELL = [
    '/',
    '/manifest.json',
    '/icon-192.png',
    '/icon-512.png',
    '/icon-maskable-512.png'
];

self.addEventListener('install', (event) => {
    event.waitUntil(
        caches.open(CACHE_NAME).then((cache) => cache.addAll(APP_SHELL))
    );
    self.skipWaiting();
});

self.addEventListener('activate', (event) => {
    event.waitUntil((async () => {
        const cacheNames = await caches.keys();
        await Promise.all(
            cacheNames
                .filter((name) => name.startsWith('rust-pwa-') && name !== CACHE_NAME)
                .map((name) => caches.delete(name))
        );
        await self.clients.claim();
    })());
});

self.addEventListener('fetch', (event) => {
    const request = event.request;
    if (request.method !== 'GET' || request.mode !== 'navigate') {
        return;
    }

    event.respondWith((async () => {
        const cache = await caches.open(CACHE_NAME);
        try {
            const response = await fetch(request);
            if (response.ok) {
                await cache.put(request, response.clone());
            }
            return response;
        } catch {
            return (await cache.match(request)) || (await cache.match('/'));
        }
    })());
});

self.addEventListener('push', (event) => {
    let data = { title: '알림', body: '새로운 메시지가 도착했습니다.' };

    if (event.data) {
        try {
            data = event.data.json();
        } catch (error) {
            console.error('푸시 데이터 파싱 실패:', error);
        }
    }
    
    const options = {
        body: data.body,
        icon: '/icon-192.png',
    };

    event.waitUntil(
        self.registration.showNotification(data.title, options)
    );
});