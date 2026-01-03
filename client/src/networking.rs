use crate::{AppState, CardSlint, MainWindow, Player, PlayerSlint};
use proto::*;
use slint::{Model, VecModel, Weak};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpStream;
use tokio::time::{sleep, Duration};

const IP_ADDR: &str = "127.0.0.1:6969";

pub fn connect_to_server(
    app_model: Arc<Mutex<crate::AppModel>>,
    ui: Weak<MainWindow>,
) -> mpsc::Sender<Message> {
    let (sock_tx, sock_rx) = mpsc::channel::<Message>();
    let sock_rx = Arc::new(Mutex::new(sock_rx));

    let msg_sender = sock_tx.clone();
    tokio::spawn(async move {
        loop {
            let ui = ui.clone();
            let msg_channel = Arc::clone(&sock_rx);
            let app_model = Arc::clone(&app_model);
            let msg_sender = msg_sender.clone();

            match TcpStream::connect(IP_ADDR).await {
                Ok(tcp_stream) => {
                    let (reader, writer) = tokio::net::TcpStream::into_split(tcp_stream);
                    let reader = BufReader::new(reader);

                    let keep_alive_tread = spawn_keep_alive_thread(msg_sender);
                    let sender_thread = spawn_sender_thread(msg_channel, writer);
                    let reciever_thread = spawn_reciever_thread(app_model, ui, reader);
                    keep_alive_tread.await.unwrap();
                    sender_thread.await.unwrap();
                    reciever_thread.await.unwrap();
                    println!("connection to server lost");
                }
                Err(_) => println!("could not connect to server! retry in 1 sec"),
            };
            sleep(Duration::from_secs(1)).await;
        }
    });
    sock_tx
}

fn spawn_keep_alive_thread(sender: mpsc::Sender<Message>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            let sender = sender.clone();
            tokio::task::spawn_blocking(move || {
                let _ = sender.send(Message::KeepAlive(system_time()));
            })
            .await
            .unwrap();
            sleep(Duration::from_millis(1000)).await;
        }
    })
}

fn spawn_sender_thread(
    msg_channel: Arc<Mutex<mpsc::Receiver<Message>>>,
    mut writer: OwnedWriteHalf,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            let msg_channel = Arc::clone(&msg_channel);
            let msg = tokio::task::spawn_blocking(move || msg_channel.lock().unwrap().recv())
                .await
                .unwrap();

            if let Ok(msg) = msg {
                println!("sending Message: {:?}", msg);
                let mut msg = serde_json::to_string(&msg).unwrap();
                msg.push('\n');
                match writer.write_all(&msg.as_bytes()).await {
                    Err(_) => break,
                    _ => {}
                }
            }
            sleep(Duration::from_millis(1)).await;
        }
    })
}

fn spawn_reciever_thread(
    app_model: Arc<Mutex<crate::AppModel>>,
    ui: Weak<MainWindow>,
    mut socket: BufReader<OwnedReadHalf>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            let ui = ui.clone();

            let mut buf = String::new();
            match socket.read_line(&mut buf).await {
                Err(_) => break,
                _ => {}
            };
            let msg: Message = serde_json::from_str(&buf)
                .unwrap_or_else(|e| panic!("unreachable deserialize should always work: {}", e));

            println!("recieved Message: {:?}", msg);

            match msg {
                Message::ConfirmJoin(id) => {
                    app_model.lock().unwrap().player_id = id;
                }
                Message::DrawCard(card) => {
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(ui) = ui.upgrade() {
                            let hand_model = ui.get_hand();
                            let vec_model = hand_model
                                .as_any()
                                .downcast_ref::<VecModel<CardSlint>>()
                                .unwrap();

                            vec_model.push(card.into());
                        }
                    });
                }
                Message::PlayerJoin(new_player) => {
                    let mut app_model = app_model.lock().unwrap();

                    let is_me = app_model.player_id == new_player.id;
                    let is_other = app_model.other_player.iter().any(|p| p.id == new_player.id);

                    if is_me && app_model.state == AppState::Lobby {
                        app_model.state = AppState::PendingGame;
                        let ui = ui.clone();
                        let _ = slint::invoke_from_event_loop(move || {
                            ui.unwrap().set_app_state(AppState::PendingGame);
                        });
                    }

                    if !is_me && !is_other {
                        let player = Player {
                            name: new_player.name,
                            id: new_player.id,
                        };

                        app_model.other_player.push(player.clone());
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(ui) = ui.upgrade() {
                                let players = ui.get_players();
                                let vec_model = players
                                    .as_any()
                                    .downcast_ref::<VecModel<PlayerSlint>>()
                                    .unwrap();

                                vec_model.push(player.into());
                            }
                        });
                    }
                }
                Message::PlayerLeave(id) => {
                    app_model
                        .lock()
                        .unwrap()
                        .other_player
                        .retain(|p| p.id != id);

                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(ui) = ui.upgrade() {
                            let players = ui.get_players();
                            let vec_model = players
                                .as_any()
                                .downcast_ref::<VecModel<PlayerSlint>>()
                                .unwrap();

                            if let Some(index) = vec_model.iter().position(|p| p.id as u32 == id) {
                                vec_model.remove(index);
                            }
                        }
                    });
                }
                _ => {}
            }
            sleep(Duration::from_millis(1)).await;
        }
    })
}
