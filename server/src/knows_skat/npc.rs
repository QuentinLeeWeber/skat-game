use crate::knows_skat::KnowsSkatRules;
use async_trait::async_trait;
use macros::message_types;
use proto::*;
use std::{collections::VecDeque, fmt};

pub struct NPC {
    id: u32,
    name: String,
    msg_stack: VecDeque<Message>,
}

impl NPC {
    pub fn new(id: u32) -> Self {
        use proto::{Rank::*, Suit::*};
        let msg_stack = vec![(Spades, Ace), (Diamonds, Ace), (Clubs, Ace), (Hearts, Ace)]
            .into_iter()
            .map(|(suit, rank)| Message::PlayCard(Card { suit, rank }))
            .collect();

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
    async fn expect_message(&mut self) -> Message {
        self.msg_stack.pop_front().unwrap_or_default()
    }

    async fn send_message(&mut self, _msg: Message) {}

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
