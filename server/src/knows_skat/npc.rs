use crate::knows_skat::KnowsSkatRules;
use async_trait::async_trait;
use macros::message_types;
use proto::*;
use std::{collections::VecDeque, fmt};
use tokio::time::{Duration, sleep};

pub struct NPC {
    id: u32,
    name: String,
    msg_stack: VecDeque<C2SMessage>,
}

impl NPC {
    pub fn new(id: u32) -> Self {
        use proto::{Rank::*, Suit::*};
        let mut msg_stack: VecDeque<C2SMessage> = vec![
            (Spades, Ace),
            (Diamonds, Ace),
            (Clubs, Ace),
            (Hearts, Ace),
            (Hearts, Seven),
            (Spades, Seven),
            (Diamonds, Seven),
            (Clubs, Seven),
            (Clubs, Jack),
            (Clubs, Jack),
        ]
        .into_iter()
        .map(|(suit, rank)| C2SMessage::PlayCard(Card { suit, rank }))
        .collect();

        msg_stack.push_front(C2SMessage::Bid(0));

        Self {
            id,
            name: String::from("NPC"),
            msg_stack,
        }
    }
}

#[async_trait]
impl KnowsSkatRules for NPC {
    #[message_types(Trump(Suit), PlayCard(Card), Bid(i32))]
    async fn expect_message(&mut self) -> C2SMessage {
        let msg = self.msg_stack.pop_front().unwrap_or_default();
        sleep(Duration::from_millis(500)).await;
        println!("npc with id: {}: {:?}", self.id, msg);
        msg
    }

    async fn send_message(&mut self, _msg: S2CMessage) {}

    fn name(&self) -> String {
        self.name.clone()
    }

    fn id(&self) -> u32 {
        self.id
    }

    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
}

impl fmt::Debug for NPC {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Player")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("ip_addr", &"LOCAL (BOT)")
            .finish()
    }
}
