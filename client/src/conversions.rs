use crate::{Card, CardRankSlint, CardSlint, CardSuitSlint, Player, PlayerSlint, Rank, Suit};

impl From<Card> for CardSlint {
    fn from(card: Card) -> Self {
        Self {
            suit: card.suit.into(),
            rank: card.rank.into(),
        }
    }
}

impl From<Suit> for CardSuitSlint {
    fn from(suit: Suit) -> Self {
        match suit {
            Suit::Clubs => CardSuitSlint::Clubs,
            Suit::Diamonds => CardSuitSlint::Diamond,
            Suit::Spades => CardSuitSlint::Spade,
            Suit::Hearts => CardSuitSlint::Heart,
        }
    }
}

impl From<Rank> for CardRankSlint {
    fn from(rank: Rank) -> Self {
        match rank {
            Rank::Ace => CardRankSlint::Ace,
            Rank::Eight => CardRankSlint::Eight,
            Rank::Jack => CardRankSlint::Jack,
            Rank::King => CardRankSlint::King,
            Rank::Nine => CardRankSlint::Nine,
            Rank::Queen => CardRankSlint::Queen,
            Rank::Seven => CardRankSlint::Seven,
            Rank::Ten => CardRankSlint::Ten,
        }
    }
}

impl From<Player> for PlayerSlint {
    fn from(player: Player) -> Self {
        PlayerSlint {
            name: player.name.into(),
        }
    }
}
