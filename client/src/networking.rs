use crate::app_main::*;
use prelude::*;
use slint::{Model, VecModel, Weak};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpStream;
use tokio::time::{sleep, Duration};

const IP_ADDR: &str = include_str!("../server.conf");

pub fn connect_to_server(
    app_model: Arc<Mutex<AppModel>>,
    ui: Weak<MainWindow>,
) -> mpsc::Sender<C2SMessage> {
    let (sock_tx, sock_rx) = mpsc::channel::<C2SMessage>();
    let sock_rx = Arc::new(Mutex::new(sock_rx));

    let msg_sender = sock_tx.clone();
    tokio::spawn(async move {
        loop {
            let ui = ui.clone();
            let msg_channel = Arc::clone(&sock_rx);
            let app_model = Arc::clone(&app_model);
            let msg_sender = msg_sender.clone();

            match TcpStream::connect(IP_ADDR.trim()).await {
                Ok(tcp_stream) => {
                    let (reader, writer) = tokio::net::TcpStream::into_split(tcp_stream);
                    let reader = BufReader::new(reader);

                    if let Some(name) = &app_model.lock().unwrap().name {
                        let _ = msg_sender.send(C2SMessage::Login(name.into()));
                    }

                    let keep_alive_tread = spawn_keep_alive_thread(msg_sender);
                    let sender_thread = spawn_sender_thread(msg_channel, writer);
                    let reciever_thread = spawn_reciever_thread(app_model, ui.clone(), reader);
                    tokio::select! {
                        _ = keep_alive_tread => {}
                        _ = sender_thread => {}
                        _ = reciever_thread => {}
                    }
                    let _ = slint::invoke_from_event_loop(move || {
                        ui.unwrap().invoke_return_to_lobby();
                        ui.unwrap()
                            .invoke_alert("Connection to Server lost! Return to lobby.".into());
                    });
                    println!("connection to server lost");
                }
                Err(e) => println!("could not connect to server: {} retry in 1 sec", e),
            };
            sleep(Duration::from_secs(1)).await;
        }
    });
    sock_tx
}

fn spawn_keep_alive_thread(sender: mpsc::Sender<C2SMessage>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            let sender = sender.clone();
            tokio::task::spawn_blocking(move || {
                let _ = sender.send(C2SMessage::KeepAlive);
            })
            .await
            .unwrap();
            sleep(Duration::from_millis(1000)).await;
        }
    })
}

fn spawn_sender_thread(
    msg_channel: Arc<Mutex<mpsc::Receiver<C2SMessage>>>,
    mut writer: OwnedWriteHalf,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            let msg_channel = Arc::clone(&msg_channel);
            let msg = tokio::task::spawn_blocking(move || msg_channel.lock().unwrap().recv())
                .await
                .unwrap();

            if let Ok(msg) = msg {
                if !matches!(msg, C2SMessage::KeepAlive) {
                    println!("sending Message: {:?}", msg);
                }
                let mut msg = serde_json::to_string(&msg).unwrap();
                msg.push('\n');
                if (writer.write_all(msg.as_bytes()).await).is_err() {
                    break;
                }
            }
            sleep(Duration::from_millis(1)).await;
        }
    })
}

fn spawn_reciever_thread(
    app_model: Arc<Mutex<AppModel>>,
    ui: Weak<MainWindow>,
    mut socket: BufReader<OwnedReadHalf>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            let ui = ui.clone();

            let mut buf = String::new();
            if (socket.read_line(&mut buf).await).is_err() {
                break;
            };
            let msg: S2CMessage = serde_json::from_str(&buf)
                .unwrap_or_else(|e| panic!("unreachable deserialize should always work: {}", e));

            println!("recieved Message: {:?}", msg);

            match msg {
                S2CMessage::ConfirmJoin(id) => {
                    app_model.lock().unwrap().player_id = id;
                }
                S2CMessage::DrawCard(card) => {
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
                S2CMessage::PlayerJoin(new_player) => {
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
                S2CMessage::PlayerLeave(id) => {
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
                S2CMessage::StartGame => {
                    app_model.lock().unwrap().state = AppState::Bid;
                    let _ = slint::invoke_from_event_loop(move || {
                        ui.unwrap().set_app_state(AppState::Bid);
                        ui.unwrap().set_my_turn(false);
                    });
                }
                S2CMessage::AssignBidRole(_) => {}
                S2CMessage::NewBid(bid) => {
                    let _ = slint::invoke_from_event_loop(move || {
                        ui.unwrap().set_game_value(format!("{}", bid).into());
                    });
                }
                S2CMessage::AssignGameRole(role) => {
                    app_model.lock().unwrap().state = AppState::Game;
                    let solo = match role {
                        GameRole::NormalDuo => false,
                        GameRole::NormalSolo => true,
                    };
                    let _ = slint::invoke_from_event_loop(move || {
                        ui.unwrap().set_app_state(AppState::Game);
                        ui.unwrap().set_solo(solo);
                    });
                }
                S2CMessage::YourTurn => {
                    let _ = slint::invoke_from_event_loop(move || {
                        ui.unwrap().set_my_turn(true);
                    });
                }
                S2CMessage::SelectTrump => {
                    let _ = slint::invoke_from_event_loop(move || {
                        ui.unwrap().set_select_trump(true);
                    });
                }
                S2CMessage::Trump(suit) => {
                    app_model.lock().unwrap().trump = Some(suit.clone());
                    let _ = slint::invoke_from_event_loop(move || {
                        ui.unwrap().set_game_trump(suit.into());
                    });
                }
                S2CMessage::PlayedCard(card) => {
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(ui) = ui.upgrade() {
                            let table_cards = ui.get_table_cards();
                            let vec_model = table_cards
                                .as_any()
                                .downcast_ref::<VecModel<CardSlint>>()
                                .unwrap();

                            if vec_model.iter().count() == 3 {
                                vec_model.clear();
                            }
                            vec_model.push(card.into());
                        }
                    });
                }
                S2CMessage::GameOver(msg) => {
                    let GameOverMessage {
                        winner_id,
                        winner_points,
                        loser_points,
                    } = msg;
                    let player_id = app_model.lock().unwrap().player_id;
                    println!("player_id: {}, winner_id: {:?}", player_id, winner_id);
                    let (status, points) = match winner_id {
                        Some(id) if id == player_id => (AppState::GameWin, winner_points),
                        Some(_) => (AppState::GameLoose, loser_points),
                        None => (AppState::GameTie, winner_points),
                    };
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(ui) = ui.upgrade() {
                            ui.set_points(points as i32);
                            ui.set_app_state(status);
                        }
                    });
                }
                S2CMessage::BackToLobby => {
                    let _ = slint::invoke_from_event_loop(move || {
                        ui.unwrap().invoke_return_to_lobby();
                        ui.unwrap().invoke_alert(
                            "The lobby was closed, because a player has left it.".into(),
                        );
                    });
                }
            }
            sleep(Duration::from_millis(1)).await;
        }
    })
}
