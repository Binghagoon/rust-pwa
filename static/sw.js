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
        icon: '/icon.png', // 필요시 아이콘 추가
    };

    event.waitUntil(
        self.registration.showNotification(data.title, options)
    );
});