use super::*;
use std::env;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::time::{Duration, sleep, timeout};

const TIMEOUT_SECS: f64 = 3.0;

#[tokio::test]
async fn test_default_game_1() {
    let call = default_game_1();

    if let Err(_) = timeout(Duration::from_secs_f64(TIMEOUT_SECS), call).await {
        panic!("timeout");
    }
}

async fn default_game_1() {
    std::thread::spawn(|| {
        unsafe {
            env::set_var("SERVER_PORT", "1234");
        }
        let _ = main();
    });

    sleep(Duration::from_secs_f64(0.1)).await;

    tokio::spawn(async move {
        sleep(Duration::from_secs_f64(TIMEOUT_SECS)).await;
        assert!(false);
    });

    let mut stream_1 = connect("127.0.0.1:1234".to_string()).await;
    stream_1
        .send_message(Message::Login("Markus Rühl".into()))
        .await;
    assert_eq!(Message::ConfirmJoin(0), stream_1.read_message().await);
    assert_eq!(
        Message::PlayerJoin(PlayerJoinMessage {
            id: 0,
            name: "Markus Rühl".into()
        }),
        stream_1.read_message().await
    );

    let mut stream_2 = connect("127.0.0.1:1234".to_string()).await;
    sleep(Duration::from_millis(50)).await;
    let mut stream_3 = connect("127.0.0.1:1234".to_string()).await;
    stream_2.send_message(Message::Login("Elon".into())).await;
    sleep(Duration::from_millis(50)).await;
    stream_3
        .send_message(Message::Login("Mr. Beast".into()))
        .await;
    assert_eq!(Message::ConfirmJoin(1), stream_2.read_message().await);
    assert_eq!(Message::ConfirmJoin(2), stream_3.read_message().await);
    assert_eq!(
        Message::PlayerJoin(PlayerJoinMessage {
            id: 0,
            name: "Markus Rühl".into()
        }),
        stream_2.read_message().await
    );
    assert_eq!(
        Message::PlayerJoin(PlayerJoinMessage {
            id: 1,
            name: "Elon".into()
        }),
        stream_2.read_message().await
    );
    assert_eq!(
        Message::PlayerJoin(PlayerJoinMessage {
            id: 0,
            name: "Markus Rühl".into()
        }),
        stream_3.read_message().await
    );
    assert_eq!(
        Message::PlayerJoin(PlayerJoinMessage {
            id: 1,
            name: "Elon".into()
        }),
        stream_3.read_message().await
    );
    assert_eq!(
        Message::PlayerJoin(PlayerJoinMessage {
            id: 2,
            name: "Mr. Beast".into()
        }),
        stream_3.read_message().await
    );

    // clear lefover PlayerJoin Messages
    for _ in 0..5 {
        let _ = stream_1.read_message().await;
    }
    for _ in 0..3 {
        let _ = stream_2.read_message().await;
    }

    //Getting Cards
    let mut streams = vec![stream_1, stream_2, stream_3];
    for _ in 0..10 {
        for stream in &mut streams {
            assert!(matches!(stream.read_message().await, Message::DrawCard(_)));
        }
    }
}

async fn connect(ip: String) -> BufReader<TcpStream> {
    BufReader::new(tokio::net::TcpStream::connect(ip).await.unwrap())
}

trait BufReaderExt {
    async fn send_message(&mut self, msg: Message);
    async fn read_message(&mut self) -> Message;
}

impl BufReaderExt for BufReader<TcpStream> {
    async fn send_message(&mut self, msg: Message) {
        let serialized = serde_json::to_string(&msg).unwrap();
        self.write_all(&serialized.as_bytes()).await.unwrap();
        self.write_all("\n".as_bytes()).await.unwrap();
    }

    async fn read_message(&mut self) -> Message {
        let mut buf = String::new();
        let _ = self.read_line(&mut buf).await.unwrap();
        let msg: Message = serde_json::from_str(&buf).unwrap();
        msg
    }
}
