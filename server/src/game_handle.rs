use crate::game::Game;
use crate::knows_skat::KnowsSkatRules;
use std::sync::Arc;
use std::vec;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

pub struct GameHandle {
    player_ids: Vec<u32>,
    player_1: Arc<Mutex<Option<Box<dyn KnowsSkatRules>>>>,
    player_2: Arc<Mutex<Option<Box<dyn KnowsSkatRules>>>>,
    player_3: Arc<Mutex<Option<Box<dyn KnowsSkatRules>>>>,
    join_handle: JoinHandle<()>,
}

impl GameHandle {
    pub fn new(
        player_1: Box<dyn KnowsSkatRules>,
        player_2: Box<dyn KnowsSkatRules>,
        player_3: Box<dyn KnowsSkatRules>,
    ) -> Self {
        let player_ids = vec![player_1.id(), player_2.id(), player_3.id()];
        let player_1 = Arc::new(Mutex::new(Some(player_1)));
        let player_2 = Arc::new(Mutex::new(Some(player_2)));
        let player_3 = Arc::new(Mutex::new(Some(player_3)));
        let mut game = Game::new(
            Arc::clone(&player_1),
            Arc::clone(&player_2),
            Arc::clone(&player_3),
        );
        let join_handle = tokio::spawn(async move { game.start().await });

        Self {
            player_ids,
            player_1,
            player_2,
            player_3,
            join_handle,
        }
    }

    pub fn has_player_by_id(&self, id: u32) -> bool {
        self.player_ids.contains(&id)
    }

    pub async fn abort(self) -> Vec<Box<dyn KnowsSkatRules>> {
        self.join_handle.abort();

        vec![
            self.player_1.lock().await.take().unwrap(),
            self.player_2.lock().await.take().unwrap(),
            self.player_3.lock().await.take().unwrap(),
        ]
    }
}
