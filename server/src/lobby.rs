use crate::game::{Game, PendingGame};
use crate::player::Player;
use proto::*;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::{Mutex, mpsc};
use tokio::task::JoinHandle;
use tokio::time::{Duration, sleep};

pub enum LobbyCommand {
    JoinGame { player_id: u32 },
    Disconnect { player_id: u32 },
    Login { player_id: u32, name: String },
    AddNPC,
}

pub struct Lobby {
    players: Vec<Player>,
    games: Vec<Game>,
    pending_game: PendingGame,
    task_handle: JoinHandle<()>,
    cmd_channel: mpsc::Sender<LobbyCommand>,
}

impl Drop for Lobby {
    fn drop(&mut self) {
        self.task_handle.abort();
    }
}

impl Lobby {
    pub async fn new() -> Arc<Mutex<Self>> {
        let (cmd_cnl_tx, cmd_cnl_rx) = mpsc::channel::<LobbyCommand>(10);

        let this_lobby = Arc::new(Mutex::new(Self {
            players: Vec::new(),
            games: Vec::new(),
            pending_game: PendingGame::default(),
            task_handle: tokio::spawn(async {}),
            cmd_channel: cmd_cnl_tx,
        }));

        let task_handle = Self::spawn_task(this_lobby.clone(), cmd_cnl_rx);
        this_lobby.lock().await.task_handle = task_handle;
        this_lobby
    }

    fn spawn_task(
        this_lobby: Arc<Mutex<Lobby>>,
        mut cmd_cnl_rx: mpsc::Receiver<LobbyCommand>,
    ) -> JoinHandle<()> {
        tokio::spawn({
            async move {
                let mut game_count = 0;

                loop {
                    if let Some(cmd) = cmd_cnl_rx.recv().await {
                        match cmd {
                            LobbyCommand::JoinGame { player_id } => {
                                let player_pos = this_lobby
                                    .lock()
                                    .await
                                    .players
                                    .iter()
                                    .position(|p| p.id == player_id)
                                    .clone();

                                if let Some(pos) = player_pos {
                                    let player = this_lobby.lock().await.players.remove(pos);
                                    let pending_game = &mut this_lobby.lock().await.pending_game;
                                    pending_game.add_player(player).await;
                                    if pending_game.player_count == 3 {
                                        this_lobby
                                            .lock()
                                            .await
                                            .games
                                            .push(pending_game.to_game(game_count));
                                    }
                                }
                            }
                            LobbyCommand::Disconnect { player_id } => {
                                this_lobby.lock().await.remove_player(player_id).await;
                            }
                            LobbyCommand::Login { player_id, name } => {
                                println!(
                                    "player with id: {}, logged in as: \"{}\"",
                                    player_id, name
                                );
                                let mut lobby = this_lobby.lock().await;
                                let player = lobby.players.iter_mut().find(|p| p.id == player_id);

                                if let Some(player) = player {
                                    player.name = name;
                                }
                            }
                            LobbyCommand::AddNPC => {
                                this_lobby.lock().await.pending_game.add_npc();
                            }
                        }
                    }
                    game_count += 1;
                    sleep(Duration::from_millis(1)).await;
                }
            }
        })
    }

    pub async fn remove_player(&mut self, id: u32) {
        //removing from pending game
        self.pending_game.try_remove_player(id).await;

        //removing from ongoing game (broadcasting closing off Game)
        let remove_game = self.games.iter().position(|g| g.has_player_by_id(id));

        if let Some(remove_game) = remove_game {
            println!("removed Lobby with player");
            let game = self.games.remove(remove_game);

            let mut remaining_player: Vec<Player> =
                game.close().into_iter().filter(|p| p.id != id).collect();

            remaining_player
                .broadcast_message(Message::BackToLobby)
                .await;

            self.players.extend(remaining_player);
        }

        //removing from players list
        let p_count = self.players.len();

        self.players.retain(|p| p.id != id);

        let p_count_after = self.players.len();
        if p_count != p_count_after {
            println!("Player with id: {} left the game!", id);
        }
    }

    pub async fn add_new_player(this: Arc<Mutex<Lobby>>, stream: TcpStream, addr: String, id: u32) {
        let cmd_channel = this.lock().await.cmd_channel.clone();
        let mut new_player = Player::new(stream, id, addr.to_string(), cmd_channel);
        let msg = Message::ConfirmJoin(id);
        new_player.send_message(msg).await;

        this.lock().await.players.push(new_player);
    }
}

trait VecExt<T> {
    async fn broadcast_message(&mut self, msg: Message);
}

impl VecExt<Player> for Vec<Player> {
    async fn broadcast_message(&mut self, msg: Message) {
        for player in &mut self.iter_mut() {
            player.send_message(msg.clone()).await;
        }
    }
}
