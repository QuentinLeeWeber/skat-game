use crate::Message;
use async_trait::async_trait;
use macros::message_types_trait;
use proto::*;
use std::any::Any;
use std::fmt::Debug;

pub mod npc;
pub mod player;

#[async_trait]
pub trait KnowsSkatRules: Debug + Send + Any {
    #[message_types_trait(Trump(Suit), PlayCard(Card), Bid(i32))]
    async fn expect_message(&mut self) -> Message;
    async fn send_message(&mut self, msg: Message);
    fn into_any(self: Box<Self>) -> Box<dyn Any>;
    fn name(&self) -> String;
    fn id(&self) -> u32;
}
