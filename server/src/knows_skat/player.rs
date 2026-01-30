use crate::knows_skat::KnowsSkatRules;
use crate::lobby::LobbyCommand;
use async_trait::async_trait;
use macros::message_types;
use prelude::*;
use std::fmt;
use std::result::Result::Ok;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::{Mutex, mpsc};
use tokio::task::JoinHandle;
use tokio::time::{Duration, sleep};

pub struct Player {
    pub id: u32,
    pub name: String,
    network_tx: tokio_mpmc::Sender<S2CMessage>,
    ip_addr: String,
    //these are filtered messages, only for message relavent to the game
    game_messages_rx: tokio_mpmc::Receiver<C2SMessage>,
    game_messages_tx: tokio_mpmc::Sender<C2SMessage>,
    task_handles: Option<Vec<JoinHandle<()>>>,
    lobby_cmd_cnl: mpsc::Sender<LobbyCommand>,
    last_keep_alive: Arc<Mutex<u128>>,
}

impl Clone for Player {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            name: self.name.clone(),
            ip_addr: self.ip_addr.clone(),
            network_tx: self.network_tx.clone(),
            game_messages_tx: self.game_messages_tx.clone(),
            game_messages_rx: self.game_messages_rx.clone(),
            task_handles: None,
            lobby_cmd_cnl: self.lobby_cmd_cnl.clone(),
            last_keep_alive: self.last_keep_alive.clone(),
        }
    }
}

impl fmt::Debug for Player {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Player")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("ip_addr", &self.ip_addr)
            .finish()
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        if let Some(handles) = &mut self.task_handles {
            for handle in handles {
                handle.abort();
            }
        }
    }
}

#[async_trait]
impl KnowsSkatRules for Player {
    #[message_types(Trump(Suit), PlayCard(Card), Bid(i32))]
    async fn expect_message(&mut self) -> C2SMessage {
        self.read_message().await
    }

    async fn send_message(&mut self, msg: S2CMessage) {
        let _ = self.network_tx.send(msg).await;
    }

    fn name(&self) -> String {
        self.name.clone()
    }

    fn id(&self) -> u32 {
        self.id
    }

    /*fn into_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }*/
}

impl Player {
    pub fn new(
        tcp_stream: TcpStream,
        id: u32,
        ip_addr: String,
        lobby_cmd_cnl: mpsc::Sender<LobbyCommand>,
    ) -> Self {
        let (tcp_reader, tcp_writer) = TcpStream::into_split(tcp_stream);
        let (network_tx, network_rx) = tokio_mpmc::channel::<S2CMessage>(32);
        let (game_messages_tx, game_messages_rx) = tokio_mpmc::channel::<C2SMessage>(32);

        let mut player = Player {
            id,
            name: String::from(""),
            ip_addr,
            game_messages_rx,
            game_messages_tx,
            task_handles: None,
            network_tx,
            lobby_cmd_cnl: lobby_cmd_cnl.clone(),
            last_keep_alive: Arc::new(Mutex::new(system_time())),
        };

        player.task_handles = Some(vec![
            Self::spawn_keep_alive_thread(player.clone()),
            Self::spawn_reciever_thread(player.clone(), tcp_reader),
            Self::spawn_sender_thread(player.clone(), network_rx, tcp_writer),
        ]);

        player
    }

    async fn disconnect(&self) {
        println!("player: {} wants to disconnect", self.name);
        self.lobby_cmd_cnl
            .send(LobbyCommand::Disconnect { player_id: self.id })
            .await
            .unwrap_or_else(|_| unreachable!());
    }

    async fn read_message(&mut self) -> C2SMessage {
        match self.game_messages_rx.recv().await.unwrap() {
            Some(msg) => msg,
            None => loop {
                //this only happens when the player is about to get deleted
                sleep(Duration::from_millis(1)).await;
            },
        }
    }

    fn spawn_sender_thread(
        self,
        network_rx: tokio_mpmc::Receiver<S2CMessage>,
        mut tcp_writer: OwnedWriteHalf,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                let msg = network_rx.recv().await.unwrap().unwrap();
                println!("sending message: {:?}, to Player: {}", msg, self.name);
                let mut serialized = serde_json::to_string(&msg).unwrap();
                serialized.push('\n');
                if (tcp_writer.write_all(serialized.as_bytes()).await).is_err() {
                    println!(
                        "player: {}, failed to send a Message: disconnecting",
                        self.name
                    );
                    self.disconnect().await;
                }
            }
        })
    }

    fn spawn_keep_alive_thread(self) -> JoinHandle<()> {
        tokio::spawn({
            async move {
                loop {
                    let time_since = system_time() - *self.last_keep_alive.lock().await;
                    if time_since > 5000 {
                        println!("player with id: {} timeouted", self.id);
                        self.lobby_cmd_cnl
                            .send(LobbyCommand::Disconnect { player_id: self.id })
                            .await
                            .unwrap_or_else(|_| unreachable!());
                    }
                    sleep(Duration::from_millis(500)).await;
                }
            }
        })
    }

    fn spawn_reciever_thread(self, tcp_reader: OwnedReadHalf) -> JoinHandle<()> {
        let mut tcp_reader = BufReader::new(tcp_reader);

        tokio::spawn(async move {
            loop {
                let mut buf = String::new();
                match tcp_reader.read_line(&mut buf).await {
                    Ok(_) => {}
                    Err(e) => {
                        println!("reading from tcp_stream failed! : {}", e);
                    }
                }
                let msg: Option<C2SMessage> = serde_json::from_str(&buf).ok();
                self.handle_server_msg(msg).await;
                sleep(Duration::from_millis(1)).await;
            }
        })
    }

    async fn handle_server_msg(&self, msg: Option<C2SMessage>) {
        match msg {
            Some(C2SMessage::KeepAlive) => {
                *self.last_keep_alive.lock().await = system_time();
            }
            Some(C2SMessage::JoinGame) => {
                self.lobby_cmd_cnl
                    .send(LobbyCommand::JoinGame { player_id: self.id })
                    .await
                    .unwrap_or_else(|_| unreachable!());
            }
            Some(C2SMessage::Login(name)) => {
                self.lobby_cmd_cnl
                    .send(LobbyCommand::Login {
                        player_id: self.id,
                        name,
                    })
                    .await
                    .unwrap_or_else(|_| unreachable!());
            }
            Some(C2SMessage::AddNPC) => {
                self.lobby_cmd_cnl
                    .send(LobbyCommand::AddNPC)
                    .await
                    .unwrap_or_else(|_| unreachable!());
            }
            Some(msg) => {
                self.game_messages_tx
                    .send(msg)
                    .await
                    .unwrap_or_else(|_| unreachable!());
            }
            None => {
                self.lobby_cmd_cnl
                    .send(LobbyCommand::Disconnect { player_id: self.id })
                    .await
                    .unwrap_or_else(|_| unreachable!());
            }
        }
    }
}
