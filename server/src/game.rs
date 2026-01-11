use crate::knows_skat::KnowsSkatRules;
use proto::*;
use rand::seq::SliceRandom;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{Duration, sleep};

pub struct Game {
    pub player_1: Arc<Mutex<Option<Box<dyn KnowsSkatRules>>>>,
    pub player_2: Arc<Mutex<Option<Box<dyn KnowsSkatRules>>>>,
    pub player_3: Arc<Mutex<Option<Box<dyn KnowsSkatRules>>>>,
    cycle_count: i32,
    playing_player: Option<i32>,
    cards: Vec<Card>,
}

impl Game {
    pub fn new(
        player_1: Arc<Mutex<Option<Box<dyn KnowsSkatRules>>>>,
        player_2: Arc<Mutex<Option<Box<dyn KnowsSkatRules>>>>,
        player_3: Arc<Mutex<Option<Box<dyn KnowsSkatRules>>>>,
    ) -> Game {
        Game {
            player_1,
            player_2,
            player_3,
            cycle_count: 0,
            playing_player: None,
            cards: new_shuffled_deck(),
        }
    }

    fn all_players(&mut self) -> Vec<Arc<Mutex<Option<Box<dyn KnowsSkatRules>>>>> {
        vec![
            Arc::clone(&self.player_1),
            Arc::clone(&self.player_2),
            Arc::clone(&self.player_3),
        ]
    }

    fn next_player(&mut self) -> Arc<Mutex<Option<Box<dyn KnowsSkatRules>>>> {
        self.cycle_count += 1;
        self.map_players()
    }

    fn prev_player(&mut self) -> Arc<Mutex<Option<Box<dyn KnowsSkatRules>>>> {
        self.cycle_count -= 1;
        self.map_players()
    }

    fn map_players(&mut self) -> Arc<Mutex<Option<Box<dyn KnowsSkatRules>>>> {
        match self.cycle_count.rem_euclid(3) {
            0 => Arc::clone(&self.player_1),
            1 => Arc::clone(&self.player_2),
            2 => Arc::clone(&self.player_3),
            _ => unreachable!(),
        }
    }

    async fn broadcast_message(&mut self, msg: Message) {
        self.player_1
            .lock()
            .await
            .as_mut()
            .unwrap()
            .send_message(msg.clone())
            .await;

        self.player_2
            .lock()
            .await
            .as_mut()
            .unwrap()
            .send_message(msg.clone())
            .await;

        self.player_3
            .lock()
            .await
            .as_mut()
            .unwrap()
            .send_message(msg.clone())
            .await;
    }

    pub async fn start(&mut self) {
        for _ in 0..10 {
            let card = self.cards.pop().unwrap();
            for player in self.all_players().iter() {
                player
                    .lock()
                    .await
                    .as_mut()
                    .unwrap()
                    .send_message(Message::DrawCard(card.clone()))
                    .await;
            }
            sleep(Duration::from_millis(100)).await
        }

        self.assign_roles().await;
        self.bid().await;

        if self.playing_player.is_some() {
            self.normal_game().await;
        } else {
            self.loosing_hand().await;
        }
    }

    async fn assign_roles(&mut self) {
        self.next_player()
            .lock()
            .await
            .as_mut()
            .unwrap()
            .send_message(Message::AssignBitRole(BitRole::Hear))
            .await;

        self.next_player()
            .lock()
            .await
            .as_mut()
            .unwrap()
            .send_message(Message::AssignBitRole(BitRole::Say))
            .await;

        self.next_player()
            .lock()
            .await
            .as_mut()
            .unwrap()
            .send_message(Message::AssignBitRole(BitRole::SayFurther))
            .await;
    }

    async fn bid(&mut self) {
        let mut bid;
        for i in [0, 2, 1] {
            loop {
                let val = self
                    .prev_player()
                    .lock()
                    .await
                    .as_mut()
                    .unwrap()
                    .expect_message_bid()
                    .await;
                if val == 0 {
                    break;
                } else {
                    bid = val;
                    self.playing_player = Some(i);
                    self.broadcast_message(Message::NewBid(bid)).await;
                }
            }
        }
    }

    async fn normal_game(&mut self) {}

    async fn broadcast_played_game(&mut self) {
        todo!()
        /*for i in 0..3 {
            let p = self.(i);
            if i == self.solo {
                p.send_message(Message::PlayNormalSolo).await;
            } else {
                p.send_message(Message::PlayNormalDuo).await;
            }
        }*/
    }

    async fn loosing_hand(&mut self) {
        todo!()
    }
}

fn new_shuffled_deck() -> Vec<Card> {
    use proto::{Rank::*, Suit::*};

    let mut deck = vec![];
    for suit in [Hearts, Diamonds, Clubs, Spades] {
        for rank in [Seven, Eight, Nine, Ten, Jack, Queen, King, Ace] {
            deck.push(Card {
                suit: suit.clone(),
                rank: rank.clone(),
            });
        }
    }
    deck.shuffle(&mut rand::rng());
    deck
}

/*
async fn normal_game(
    mut players: Vec<Player>,
    solo: usize,
    mut skat: Vec<Card>,
) -> Result<(), Error> {
    //Broadcast Played Game
    for i in 0..3 {
        let p = players.evil_get(i);
        if i == solo {
            p.send_message(Message::PlayNormalSolo).await;
        } else {
            p.send_message(Message::PlayNormalDuo).await;
        }
    }

    let mut solo_trick = vec![];
    let mut duo_trick = vec![];

    //Skat
    for _ in 0..2 {
        let msg = Message::DrawCard(skat.pop().unwrap());
        players.evil_get(solo).send_message(msg).await
    }

    for _ in 0..2 {
        let card = players.evil_get(solo).expect_message_play_card().await;
        solo_trick.push(card);
    }

    //Get trump
    let trump = players.evil_get(solo).expect_message_trump().await;
    players
        .broadcast_message(Message::Trump(trump.clone()))
        .await;

    let mut last_winner = 0;

    //PLay 10 rounds
    for _ in 0..10 {
        let mut current_trick = vec![];

        for current_player in turn_order(last_winner) {
            players
                .evil_get(current_player)
                .send_message(Message::YourTurn)
                .await;

            let card = players
                .evil_get(current_player)
                .expect_message_play_card()
                .await;
            current_trick.push((card, current_player));
        }

        let trick_color = if current_trick
            .iter()
            .any(|c| &c.0.suit == &trump || c.0.rank == Rank::Jack)
        {
            trump.clone()
        } else {
            current_trick.get(0).unwrap().0.suit.clone()
        };

        last_winner = current_trick
            .iter()
            .filter(|c| c.0.suit == trick_color || c.0.rank == Rank::Jack)
            .max_by_key(|c| normal_rank_value(&c.0.rank))
            .map(|c| c.1)
            .unwrap();

        if last_winner == solo {
            &mut solo_trick
        } else {
            &mut duo_trick
        }
        .append(&mut current_trick.into_iter().map(|c| c.0).collect());
    }

    //Evaluate Winner
    let solo_points = evaluate_cards_value(&solo_trick);
    let duo_points = evaluate_cards_value(&duo_trick);
    let won_msg = if solo_points > duo_points {
        Message::GameWon(GameWonMessage {
            id: Some(solo as u32),
            winner_points: solo_points,
            loser_points: duo_points,
        })
    } else if solo_points < duo_points {
        Message::GameWon(GameWonMessage {
            id: Some(solo as u32 + 1),
            winner_points: duo_points,
            loser_points: solo_points,
        })
    } else {
        Message::GameWon(GameWonMessage {
            id: None,
            winner_points: 60,
            loser_points: 60,
        })
    };
    players.broadcast_message(won_msg).await;

    Ok(())
}

fn evaluate_cards_value(cards: &Vec<Card>) -> u32 {
    cards.iter().map(|c| c.rank.value()).sum()
}

fn evaluate_round_winner(first: Card, second: Card, third: Card) {
    todo!()
}

async fn loosing_hand(players: Vec<Player>) -> Result<(), Error> {
    todo!()
}
*/
