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
            id: player.id as i32,
        }
    }
}

impl From<CardSlint> for Card {
    fn from(card_slint: CardSlint) -> Self {
        Self {
            suit: card_slint.suit.into(),
            rank: card_slint.rank.into(),
        }
    }
}

impl From<CardSuitSlint> for Suit {
    fn from(suit: CardSuitSlint) -> Self {
        match suit {
            CardSuitSlint::Clubs => Suit::Clubs,
            CardSuitSlint::Diamond => Suit::Diamonds,
            CardSuitSlint::Spade => Suit::Spades,
            CardSuitSlint::Heart => Suit::Hearts,
        }
    }
}

impl From<CardRankSlint> for Rank {
    fn from(rank: CardRankSlint) -> Self {
        match rank {
            CardRankSlint::Ace => Rank::Ace,
            CardRankSlint::Seven => Rank::Seven,
            CardRankSlint::Eight => Rank::Eight,
            CardRankSlint::Nine => Rank::Nine,
            CardRankSlint::Ten => Rank::Ten,
            CardRankSlint::Jack => Rank::Jack,
            CardRankSlint::Queen => Rank::Queen,
            CardRankSlint::King => Rank::King,
        }
    }
}
