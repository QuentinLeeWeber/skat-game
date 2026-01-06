use crate::knows_skat::KnowsSkatRules;
use std::vec;
use tokio::task::JoinHandle;

pub struct Game {
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
    ) -> Game {
        let task_handle = tokio::spawn(async move {});
        Game {
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
