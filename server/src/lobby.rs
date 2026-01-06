use crate::knows_skat::player::Player;
use crate::knows_skat::{KnowsSkatRules, npc::NPC};
use crate::{game::Game, pending_game::PendingGame};
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
    player_count: u32,
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
            player_count: 0,
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
                loop {
                    if let Some(cmd) = cmd_cnl_rx.recv().await {
                        match cmd {
                            LobbyCommand::JoinGame { player_id } => {
                                let mut this_lobby = this_lobby.lock().await;
                                let player_pos = this_lobby
                                    .players
                                    .iter()
                                    .position(|p| p.id == player_id)
                                    .clone();

                                if let Some(pos) = player_pos {
                                    let player = this_lobby.players.remove(pos);
                                    if let Some(game) =
                                        this_lobby.pending_game.add_player(Box::new(player)).await
                                    {
                                        this_lobby.games.push(game);
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
                                let mut this_lobby = this_lobby.lock().await;
                                this_lobby.player_count += 1;
                                let new_id = this_lobby.player_count;
                                if let Some(game) = this_lobby
                                    .pending_game
                                    .add_player(Box::new(NPC::new(new_id)))
                                    .await
                                {
                                    this_lobby.games.push(game);
                                }
                            }
                        }
                    }
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
            println!("removed Game with player: {}", id);
            let game = self.games.remove(remove_game);

            let mut remaining_player: Vec<Player> = game
                .close()
                .into_iter()
                .filter_map(|x| x.into_any().downcast::<Player>().ok().map(|b| *b))
                .filter(|p| p.id() != id)
                .collect();

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

    pub async fn add_new_player(this: Arc<Mutex<Lobby>>, stream: TcpStream, addr: String) {
        let cmd_channel = this.lock().await.cmd_channel.clone();
        let id = {
            let mut this = this.lock().await;
            this.player_count += 1;
            this.player_count - 1
        };
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
