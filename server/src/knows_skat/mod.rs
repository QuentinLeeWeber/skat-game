use crate::Message;

pub mod knows_skat_rules;
pub mod npc;
pub mod player;

pub trait KnowsSkatRules {
    async fn expect_message(&mut self) -> Message;
    async fn send_message(&mut self, msg: Message);
}
