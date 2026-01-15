use crate::game_handle::GameHandle;
use crate::knows_skat::KnowsSkatRules;
use proto::*;
use std::{fmt::Debug, mem, vec};

#[derive(Default, Debug)]
pub struct PendingGame {
    player_1: Option<Box<dyn KnowsSkatRules>>,
    player_2: Option<Box<dyn KnowsSkatRules>>,
    player_3: Option<Box<dyn KnowsSkatRules>>,
    player_count: u32,
}

impl PendingGame {
    pub async fn add_player(&mut self, player: Box<dyn KnowsSkatRules>) -> Option<GameHandle> {
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
            .flatten()
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

        if self.player_count == 3 {
            println!("pending game full: starting new game!");
            self.broadcast_message(S2CMessage::StartGame).await;
            Some(self.to_game())
        } else {
            None
        }
    }

    pub async fn try_remove_player(&mut self, id: u32) {
        let mut removed = false;
        if let Some(player) = &self.player_1
            && player.id() == id {
                self.player_1 = mem::take(&mut self.player_2);
                self.player_2 = mem::take(&mut self.player_3);
                self.player_3 = None;
                removed = true;
            }
        if let Some(player) = &self.player_2
            && player.id() == id {
                self.player_2 = mem::take(&mut self.player_3);
                self.player_3 = None;
                removed = true;
            }
        if let Some(player) = &self.player_3
            && player.id() == id {
                self.player_3 = None;
                removed = true;
            }
        self.broadcast_message(S2CMessage::PlayerLeave(id)).await;
        if removed {
            self.player_count -= 1;
            println!("removed player with id: {} from pending game", id);
            println!("pending game is now:\n{:#?}", self);
        }
    }

    pub fn to_game(&mut self) -> GameHandle {
        self.player_count = 0;
        GameHandle::new(
            mem::take(&mut self.player_1).unwrap(),
            mem::take(&mut self.player_2).unwrap(),
            mem::take(&mut self.player_3).unwrap(),
        )
    }

    async fn broadcast_message(&mut self, msg: S2CMessage) {
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
