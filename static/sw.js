self.addEventListener('push', (event) => {
    const data = event.data ? event.data.json() : { title: '알림', body: '새로운 메시지가 도착했습니다.' };
    
    const options = {
        body: data.body,
        icon: '/icon.png', // 필요시 아이콘 추가
    };

    event.waitUntil(
        self.registration.showNotification(data.title, options)
    );
});