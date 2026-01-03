use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Message {
    Login(String),
    ConfirmJoin(u32),
    PlayerJoin(PlayerJoinMessage),
    DrawCard(Card),
    Hear,
    Say,
    SayFurther,
    Bid(i32),
    NewBid(i32),
    PlayCard(Card),
    PlayNormalSolo,
    PlayNormalDuo,
    YourTurn,
    Trump(Suit),
    GameWon(GameWonMessage),
    KeepAlive(u128),
    BackToLobby,
    JoinGame,
    PlayerLeave(u32),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameWonMessage {
    pub id: Option<u32>,
    pub winner_points: u32,
    pub loser_points: u32,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerJoinMessage {
    pub id: u32,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Card {
    pub suit: Suit,
    pub rank: Rank,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Suit {
    Hearts,
    Diamonds,
    Clubs,
    Spades,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Rank {
    Seven,
    Eight,
    Nine,
    Ten,
    Jack,
    Queen,
    King,
    Ace,
}

impl Rank {
    pub fn value(&self) -> u32 {
        match self {
            Rank::Seven => 0,
            Rank::Eight => 0,
            Rank::Nine => 0,
            Rank::Jack => 2,
            Rank::Queen => 3,
            Rank::King => 4,
            Rank::Ten => 10,
            Rank::Ace => 11,
        }
    }
}

pub fn normal_rank_value(rank: &Rank) -> u32 {
    match rank {
        Rank::Seven => 0,
        Rank::Eight => 1,
        Rank::Nine => 2,
        Rank::Queen => 3,
        Rank::King => 4,
        Rank::Ten => 5,
        Rank::Ace => 6,
        Rank::Jack => 6969,
    }
}

pub fn system_time() -> u128 {
    let system_time = std::time::SystemTime::now();
    system_time
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis()
}
