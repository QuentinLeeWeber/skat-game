use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum S2CMessage {
    ConfirmJoin(u32),
    PlayerJoin(PlayerJoinMessage),
    DrawCard(Card),
    AssignBidRole(BidRole),
    NewBid(i32),
    AssignGameRole(GameRole),
    YourTurn,
    Trump(Suit),
    GameOver(GameOverMessage),
    BackToLobby,
    PlayerLeave(u32),
    StartGame,
    SelectTrump,
    PlayedCard(Card),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum C2SMessage {
    #[default]
    None,
    Login(String),
    Bid(i32),
    PlayCard(Card),
    KeepAlive(u128),
    AddNPC,
    JoinGame,
    Trump(Suit),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameRole {
    NormalSolo,
    NormalDuo,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BidRole {
    Hear,
    Say,
    SayFurther,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameOverMessage {
    pub winner_id: Option<u32>,
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

pub fn possible_moves(hand: &Vec<Card>, table: &Vec<Card>, trump: &Option<Suit>) -> Vec<Card> {
    if table.is_empty() || table.len() == 3 {
        return hand.clone();
    }

    let leading_suit = &table[0].suit;
    let leading_rank = &table[0].rank;

    let leading_suit_is_trump = match trump {
        Some(t) => t == leading_suit,
        None => leading_rank == &Rank::Jack,
    };

    if leading_suit_is_trump {
        return hand
            .clone()
            .into_iter()
            .filter(|card| match trump {
                Some(t) => &card.suit == t || &card.rank == &Rank::Jack,
                None => &card.rank == &Rank::Jack,
            })
            .collect();
    }

    let mut has_leading_suit = false;
    for card in hand {
        if &card.suit == leading_suit {
            has_leading_suit = true;
            break;
        }
    }

    if has_leading_suit {
        hand.into_iter()
            .filter(|card| &card.suit == leading_suit)
            .cloned()
            .collect()
    } else {
        hand.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(suit: Suit, rank: Rank) -> Card {
        Card { suit, rank }
    }

    #[test]
    fn test_empty_table_returns_all_hand() {
        let hand = vec![
            card(Suit::Hearts, Rank::Ace),
            card(Suit::Clubs, Rank::Seven),
        ];
        let table = vec![];
        let trump = None;

        let moves = possible_moves(&hand, &table, &trump);
        assert_eq!(moves, hand);
    }

    #[test]
    fn test_follow_leading_suit() {
        let hand = vec![
            card(Suit::Hearts, Rank::Ace),
            card(Suit::Clubs, Rank::Seven),
        ];
        let table = vec![card(Suit::Hearts, Rank::King)];
        let trump = None;

        let moves = possible_moves(&hand, &table, &trump);
        assert_eq!(moves, vec![card(Suit::Hearts, Rank::Ace)]);
    }

    #[test]
    fn test_no_leading_suit_all_cards_playable() {
        let hand = vec![
            card(Suit::Clubs, Rank::Seven),
            card(Suit::Diamonds, Rank::Nine),
        ];
        let table = vec![card(Suit::Hearts, Rank::King)];
        let trump = None;

        let moves = possible_moves(&hand, &table, &trump);
        assert_eq!(moves, hand);
    }

    #[test]
    fn test_with_trump() {
        let hand = vec![
            card(Suit::Hearts, Rank::Ace),
            card(Suit::Spades, Rank::Jack),
        ];
        let table = vec![card(Suit::Hearts, Rank::Ten)];
        let trump = Some(Suit::Spades);

        let moves = possible_moves(&hand, &table, &trump);

        assert_eq!(moves, vec![card(Suit::Hearts, Rank::Ace)]);
    }

    #[test]
    fn test_with_trump_leading() {
        let hand = vec![
            card(Suit::Hearts, Rank::Ace),
            card(Suit::Spades, Rank::Jack),
            card(Suit::Spades, Rank::Nine),
        ];
        let table = vec![card(Suit::Spades, Rank::Ten)];
        let trump = Some(Suit::Spades);

        let moves = possible_moves(&hand, &table, &trump);

        assert_eq!(
            moves,
            vec![
                card(Suit::Spades, Rank::Jack),
                card(Suit::Spades, Rank::Nine)
            ]
        );
    }

    #[test]
    fn test_with_trump_leading_2() {
        let hand = vec![
            card(Suit::Hearts, Rank::Ace),
            card(Suit::Diamonds, Rank::Jack),
            card(Suit::Spades, Rank::Nine),
        ];
        let table = vec![card(Suit::Spades, Rank::Ten)];
        let trump = Some(Suit::Spades);

        let moves = possible_moves(&hand, &table, &trump);

        assert_eq!(
            moves,
            vec![
                card(Suit::Diamonds, Rank::Jack),
                card(Suit::Spades, Rank::Nine)
            ]
        );
    }

    #[test]
    fn test_with_trump_no_leading_suit() {
        let hand = vec![
            card(Suit::Hearts, Rank::Ace),
            card(Suit::Spades, Rank::Jack),
            card(Suit::Spades, Rank::Nine),
        ];
        let table = vec![card(Suit::Clubs, Rank::Ten)];
        let trump = Some(Suit::Spades);

        let moves = possible_moves(&hand, &table, &trump);

        assert_eq!(
            moves,
            vec![
                card(Suit::Hearts, Rank::Ace),
                card(Suit::Spades, Rank::Jack),
                card(Suit::Spades, Rank::Nine),
            ]
        );
    }

    #[test]
    fn test_full_table() {
        let hand = vec![
            card(Suit::Hearts, Rank::Ace),
            card(Suit::Spades, Rank::Jack),
            card(Suit::Spades, Rank::Nine),
        ];
        let table = vec![
            card(Suit::Clubs, Rank::Ten),
            card(Suit::Clubs, Rank::Ten),
            card(Suit::Clubs, Rank::Ten),
        ];
        let trump = None;

        let moves = possible_moves(&hand, &table, &trump);

        assert_eq!(
            moves,
            vec![
                card(Suit::Hearts, Rank::Ace),
                card(Suit::Spades, Rank::Jack),
                card(Suit::Spades, Rank::Nine),
            ]
        );
    }
}
