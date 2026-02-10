use crate::game::GameHandle;
use crate::knows_skat::KnowsSkatRules;
use prelude::*;
use std::{
    fmt::Debug,
    mem,
    ops::{Deref, DerefMut},
};

#[derive(Default, Debug)]
pub struct PendingGame(Vec<Box<dyn KnowsSkatRules>>);
impl Deref for PendingGame {
    type Target = Vec<Box<dyn KnowsSkatRules>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for PendingGame {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl PendingGame {
    pub async fn add_player(&mut self, player: Box<dyn KnowsSkatRules>) -> Option<GameHandle> {
        println!("player: {} joined Pending Game", player.name());
        self.0.push(player);

        let msgs = self
            .iter()
            .map(|player| {
                S2CMessage::PlayerJoin(PlayerJoinMessage {
                    id: player.id(),
                    name: player.name(),
                })
            })
            .collect::<Vec<_>>();

        for msg in msgs {
            self.broadcast_message(msg).await;
        }
        println!("pending game is now:\n{:#?}", self);

        if self.len() == 3 {
            println!("pending game full: starting new game!");
            self.broadcast_message(S2CMessage::StartGame).await;
            Some(self.to_game())
        } else {
            None
        }
    }

    pub async fn try_remove_player(&mut self, id: u32) {
        let player_count_before = self.len();
        self.retain(|p| p.id() != id);
        if player_count_before != self.len() {
            self.broadcast_message(S2CMessage::PlayerLeave(id)).await;
            println!("removed player with id: {} from pending game", id);
            println!("pending game is now:\n{:#?}", self);
        }
    }

    pub fn to_game(&mut self) -> GameHandle {
        let mut players = mem::take(&mut self.0);
        GameHandle::new(
            players.pop().unwrap(),
            players.pop().unwrap(),
            players.pop().unwrap(),
        )
    }

    async fn broadcast_message(&mut self, msg: S2CMessage) {
        for player in &mut self.0 {
            player.send_message(msg.clone()).await;
        }
    }
}
