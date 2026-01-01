use crate::{CardSlint, MainWindow, Player};
use postcard::{from_bytes, to_stdvec};
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
    msg_channel: mpsc::Receiver<Message>,
    ui: Weak<MainWindow>,
) {
    let msg_channel = Arc::new(tokio::sync::Mutex::new(msg_channel));

    tokio::spawn(async move {
        loop {
            let ui = ui.clone();
            let msg_channel = Arc::clone(&msg_channel);
            let app_model = Arc::clone(&app_model);

            match TcpStream::connect(IP_ADDR).await {
                Ok(tcp_stream) => {
                    let (reader, writer) = tokio::net::TcpStream::into_split(tcp_stream);
                    let reader = BufReader::new(reader);

                    let sender_thread = spawn_sender_thread(msg_channel, writer);
                    let reciever_thread = spawn_reciever_thread(app_model, ui, reader);
                    sender_thread.await.unwrap();
                    reciever_thread.await.unwrap();
                    println!("connection to server lost");
                }
                Err(_) => println!("could not connect to server! retry in 1 sec"),
            };
            sleep(Duration::from_secs(1)).await;
        }
    });
}

pub fn spawn_sender_thread(
    msg_channel: Arc<tokio::sync::Mutex<mpsc::Receiver<Message>>>,
    mut writer: OwnedWriteHalf,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            if let Ok(msg) = msg_channel.lock().await.recv() {
                println!("sending Message: {:?}", msg);
                let mut msg = to_stdvec(&msg).unwrap();
                msg.push(b'\n');
                match writer.write_all(&msg).await {
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
            println!("message:  {}", buf);
            let msg: Message = from_bytes(&buf.as_bytes())
                .unwrap_or_else(|e| panic!("unreachable deserialize should always work: {}", e));

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
                    if !is_me && !is_other {
                        app_model.other_player.push(Player {
                            name: new_player.name,
                            id: new_player.id,
                        });
                    }
                }
                _ => {}
            }
            sleep(Duration::from_millis(1)).await;
        }
    })
}
