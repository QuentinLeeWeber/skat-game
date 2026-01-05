use crate::knows_skat::KnowsSkatRules;
use proto::*;
use std::{fmt::Debug, mem, vec};
use tokio::task::JoinHandle;

#[derive(Default, Debug)]
pub struct PendingGame {
    player_1: Option<Box<dyn KnowsSkatRules>>,
    player_2: Option<Box<dyn KnowsSkatRules>>,
    player_3: Option<Box<dyn KnowsSkatRules>>,
    pub player_count: u32,
}

impl PendingGame {
    pub async fn add_player(&mut self, player: Box<dyn KnowsSkatRules>) {
        println!("player: {} joined Pending Game", player.name());
        match self.player_count {
            0 => {
                self.player_1 = Some(player);
            }
            1 => {
                self.player_2 = Some(player);
            }
            _ => {
                self.player_3 = Some(player);
            }
        }
        self.player_count += 1;

        let msgs = vec![&self.player_1, &self.player_2, &self.player_3]
            .into_iter()
            .flat_map(|p| p)
            .map(|player| {
                Message::PlayerJoin(PlayerJoinMessage {
                    id: player.id(),
                    name: player.name(),
                })
            })
            .collect::<Vec<_>>();

        for msg in msgs {
            self.broadcast_message(msg).await;
        }
        println!("pending game is now:\n{:#?}", self);
    }

    pub async fn try_remove_player(&mut self, id: u32) {
        let mut removed = false;
        if let Some(player) = &self.player_1 {
            if player.id() == id {
                self.player_1 = mem::take(&mut self.player_2);
                self.player_2 = mem::take(&mut self.player_3);
                self.player_3 = None;
                removed = true;
            }
        }
        if let Some(player) = &self.player_2 {
            if player.id() == id {
                self.player_2 = mem::take(&mut self.player_3);
                self.player_3 = None;
                removed = true;
            }
        }
        if let Some(player) = &self.player_3 {
            if player.id() == id {
                self.player_3 = None;
                removed = true;
            }
        }
        self.broadcast_message(Message::PlayerLeave(id)).await;
        if removed {
            self.player_count -= 1;
            println!("removed player with id: {} from pending game", id);
            println!("pending game is now:\n{:#?}", self);
        }
    }

    pub fn to_game(&mut self, id: u32) -> Game {
        self.player_count = 0;
        Game::new(
            mem::take(&mut self.player_1).unwrap(),
            mem::take(&mut self.player_2).unwrap(),
            mem::take(&mut self.player_3).unwrap(),
            id,
        )
    }

    pub async fn add_npc(&mut self) {}

    async fn broadcast_message(&mut self, msg: Message) {
        if let Some(p) = &mut self.player_1 {
            p.send_message(msg.clone()).await;
        }
        if let Some(p) = &mut self.player_2 {
            p.send_message(msg.clone()).await;
        }
        if let Some(p) = &mut self.player_3 {
            p.send_message(msg.clone()).await;
        }
    }
}

pub struct Game {
    id: u32,
    player_1: Box<dyn KnowsSkatRules>,
    player_2: Box<dyn KnowsSkatRules>,
    player_3: Box<dyn KnowsSkatRules>,
    task_handle: JoinHandle<()>,
}

impl Game {
    pub fn new(
        player_1: Box<dyn KnowsSkatRules>,
        player_2: Box<dyn KnowsSkatRules>,
        player_3: Box<dyn KnowsSkatRules>,
        id: u32,
    ) -> Game {
        let task_handle = tokio::spawn(async move {});
        Game {
            id,
            player_1,
            player_2,
            player_3,
            task_handle,
        }
    }

    pub fn close(self) -> Vec<Box<dyn KnowsSkatRules>> {
        self.task_handle.abort();
        vec![self.player_1, self.player_2, self.player_3]
    }

    pub fn has_player_by_id(&self, id: u32) -> bool {
        return self.player_1.id() == id || self.player_2.id() == id || self.player_3.id() == id;
    }
}
