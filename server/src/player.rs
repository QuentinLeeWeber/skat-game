use crate::lobby::LobbyCommand;
use macros::message_types;
use proto::*;
use std::result::Result::Ok;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::tcp::OwnedReadHalf;
use tokio::net::{TcpStream, tcp};
use tokio::sync::{Mutex, mpsc};
use tokio::task::JoinHandle;
use tokio::time::{Duration, sleep};

#[derive(Debug)]
pub struct Player {
    pub id: u32,
    pub name: String,
    tcp_writer: tcp::OwnedWriteHalf,
    ip_addr: String,
    game_messages: mpsc::Receiver<Message>,
    network_handle: JoinHandle<()>,
    keep_alive_handle: JoinHandle<()>,
    lobby_cmd_cnl: mpsc::Sender<LobbyCommand>,
}

impl Drop for Player {
    fn drop(&mut self) {
        self.network_handle.abort();
        self.keep_alive_handle.abort();
    }
}

impl Player {
    pub async fn new(
        tcp_stream: TcpStream,
        id: u32,
        ip_addr: String,
        lobby_cmd_cnl: mpsc::Sender<LobbyCommand>,
    ) -> Self {
        let (tcp_reader, tcp_writer) = TcpStream::into_split(tcp_stream);

        let (game_messages_tx, game_messages) = mpsc::channel::<Message>(100);

        let (network_handle, keep_alive_handle) =
            Self::spawn_network_treads(id, tcp_reader, lobby_cmd_cnl.clone(), game_messages_tx);

        let mut new_player = Player {
            id: id as u32,
            name: String::from(""),
            tcp_writer,
            ip_addr: ip_addr,
            game_messages,
            network_handle,
            keep_alive_handle,
            lobby_cmd_cnl,
        };

        let name = new_player.expect_message_login().await;
        println!(
            "player with IP adress: {}, logged in as: \"{}\"",
            new_player.ip_addr, name
        );
        new_player.name = name;
        new_player
    }

    #[message_types(Login(String), Trump(Suit), PlayCard(Card), Bid(i32))]
    async fn expect_message(&mut self) -> Message {
        self.read_message().await
    }

    async fn disconnect(&mut self) {
        println!("player: {} wants to disconnect", self.name);
        self.lobby_cmd_cnl
            .send(LobbyCommand::Disconnect { player_id: self.id })
            .await
            .unwrap_or_else(|_| unreachable!());
    }

    pub async fn send_message(&mut self, msg: Message) {
        let mut serialized = serde_json::to_string(&msg).unwrap();
        serialized.push('\n');
        if let Err(_) = self.tcp_writer.write_all(&serialized.as_bytes()).await {
            println!(
                "player: {}, failed to send a Message: disconecting",
                self.name
            );
            self.disconnect().await;
        }
    }

    async fn read_message(&mut self) -> Message {
        match self.game_messages.recv().await {
            Some(msg) => msg,
            None => loop {
                //this only happens when the player is about to get deleted
                sleep(Duration::from_millis(1)).await;
            },
        }
    }

    fn spawn_network_treads(
        id: u32,
        tcp_reader: OwnedReadHalf,
        lobby_cmd_cnl: mpsc::Sender<LobbyCommand>,
        game_messages_tx: mpsc::Sender<Message>,
    ) -> (JoinHandle<()>, JoinHandle<()>) {
        let mut tcp_reader = BufReader::new(tcp_reader);
        let last_keep_alive = Arc::new(Mutex::new(system_time()));

        let nework_handle = tokio::spawn({
            let last_keep_alive = Arc::clone(&last_keep_alive);
            let lobby_cmd_cnl = lobby_cmd_cnl.clone();

            async move {
                loop {
                    let mut buf = String::new();
                    match tcp_reader.read_line(&mut buf).await {
                        Ok(_) => {}
                        Err(e) => {
                            println!("reading from tcp_stream failed! : {}", e);
                        }
                    }
                    let msg: Option<Message> = match serde_json::from_str(&buf) {
                        Ok(msg) => Some(msg),
                        Err(_) => None,
                    };
                    match msg {
                        Some(Message::KeepAlive(time_stamp)) => {
                            *last_keep_alive.lock().await = time_stamp;
                        }
                        Some(Message::JoinGame) => {
                            lobby_cmd_cnl
                                .send(LobbyCommand::JoinGame { player_id: id })
                                .await
                                .unwrap_or_else(|_| unreachable!());
                        }
                        Some(msg) => {
                            game_messages_tx
                                .send(msg)
                                .await
                                .unwrap_or_else(|_| unreachable!());
                        }
                        None => {
                            lobby_cmd_cnl
                                .send(LobbyCommand::Disconnect { player_id: id })
                                .await
                                .unwrap_or_else(|_| unreachable!());
                        }
                    }
                    sleep(Duration::from_millis(1)).await;
                }
            }
        });

        let keep_alive_handle = tokio::spawn({
            async move {
                loop {
                    let time_since = system_time() - *last_keep_alive.lock().await;
                    if time_since > 5000 {
                        println!("player with id: {} timeouted", id);
                        lobby_cmd_cnl
                            .send(LobbyCommand::Disconnect { player_id: id })
                            .await
                            .unwrap_or_else(|_| unreachable!());
                    }
                    sleep(Duration::from_millis(500)).await;
                }
            }
        });

        (nework_handle, keep_alive_handle)
    }
}
