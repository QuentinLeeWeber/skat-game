use crate::knows_skat::KnowsSkatRules;
use prelude::*;
use rand::seq::SliceRandom;
use tokio::task::JoinHandle;
use tokio::time::{Duration, sleep};

pub struct GameHandle {
    pub player_ids: Vec<u32>,
    handle: JoinHandle<()>,
}

impl Drop for GameHandle {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

impl GameHandle {
    pub fn new(
        player_1: Box<dyn KnowsSkatRules>,
        player_2: Box<dyn KnowsSkatRules>,
        player_3: Box<dyn KnowsSkatRules>,
    ) -> GameHandle {
        GameHandle {
            player_ids: vec![player_1.id(), player_2.id(), player_3.id()],
            handle: tokio::spawn(
                async move { Game::new(player_1, player_2, player_3).start().await },
            ),
        }
    }

    pub fn has_player_by_id(&self, id: u32) -> bool {
        self.player_ids.contains(&id)
    }
}

struct Game {
    pub player_1: Box<dyn KnowsSkatRules>,
    pub player_2: Box<dyn KnowsSkatRules>,
    pub player_3: Box<dyn KnowsSkatRules>,
    cycle_count: i32,
    cards: Vec<Card>,
    game_value: u32,
}

impl Game {
    pub fn new(
        player_1: Box<dyn KnowsSkatRules>,
        player_2: Box<dyn KnowsSkatRules>,
        player_3: Box<dyn KnowsSkatRules>,
    ) -> Game {
        Game {
            player_1,
            player_2,
            player_3,
            cycle_count: 0,
            cards: new_shuffled_deck(),
            game_value: 0,
        }
    }

    fn all_players(&mut self) -> Vec<&mut Box<dyn KnowsSkatRules>> {
        vec![&mut self.player_1, &mut self.player_2, &mut self.player_3]
    }

    fn next_player(&mut self) -> &mut Box<dyn KnowsSkatRules> {
        self.cycle_count += 1;
        self.map_players()
    }

    fn map_players(&mut self) -> &mut Box<dyn KnowsSkatRules> {
        match self.cycle_count.rem_euclid(3) {
            0 => &mut self.player_1,
            1 => &mut self.player_2,
            2 => &mut self.player_3,
            _ => unreachable!(),
        }
    }

    fn player_by_id(&mut self, id: i32) -> &mut Box<dyn KnowsSkatRules> {
        match id.rem_euclid(3) {
            0 => &mut self.player_1,
            1 => &mut self.player_2,
            2 => &mut self.player_3,
            _ => unreachable!(),
        }
    }

    async fn broadcast_message(&mut self, msg: S2CMessage) {
        self.player_1.send_message(msg.clone()).await;
        self.player_2.send_message(msg.clone()).await;
        self.player_3.send_message(msg.clone()).await;
    }

    pub async fn start(&mut self) {
        for _ in 0..10 {
            let mut next_cards: Vec<Card> = self.cards.drain(..3).collect();
            for player in self.all_players().iter_mut() {
                let card = next_cards.pop().unwrap();
                player
                    .send_message(S2CMessage::DrawCard(card.clone()))
                    .await;
            }
            sleep(Duration::from_millis(100)).await
        }

        self.assign_roles().await;
        let solo = self.bid().await;

        if let Some(solo) = solo {
            self.normal_game(solo).await;
        } else {
            self.loosing_hand().await;
        }
    }

    async fn assign_roles(&mut self) {
        self.next_player()
            .send_message(S2CMessage::AssignBidRole(BidRole::Hear))
            .await;

        self.next_player()
            .send_message(S2CMessage::AssignBidRole(BidRole::Say))
            .await;

        self.next_player()
            .send_message(S2CMessage::AssignBidRole(BidRole::SayFurther))
            .await;
    }

    async fn bid(&mut self) -> Option<i32> {
        let mut solo = None;
        for i in [0, 2, 1] {
            loop {
                self.player_by_id(i)
                    .send_message(S2CMessage::YourTurn)
                    .await;

                let val = self.player_by_id(i).expect_message_bid().await;

                if val == 0 {
                    break;
                } else {
                    self.increase_game_value();
                    solo = Some(i);
                    self.broadcast_message(S2CMessage::NewBid(self.game_value as i32))
                        .await;
                }
            }
        }
        solo
    }

    fn increase_game_value(&mut self) {
        let possible_values = [
            0, 18, 20, 22, 23, 24, 27, 30, 33, 35, 36, 40, 44, 45, 46, 48, 50, 55, 59, 60, 72, 96,
            120,
        ];
        let last_index = possible_values.iter().position(|i| *i == self.game_value);
        self.game_value = *possible_values.get(last_index.unwrap() + 1).unwrap_or(&120);
    }

    async fn normal_game(&mut self, solo: i32) {
        for i in 0..3 {
            let p = self.player_by_id(i);
            if i == solo {
                p.send_message(S2CMessage::AssignGameRole(GameRole::NormalSolo))
                    .await;
            } else {
                p.send_message(S2CMessage::AssignGameRole(GameRole::NormalDuo))
                    .await;
            }
        }

        let mut solo_trick = vec![];
        let mut duo_trick = vec![];

        //Skat
        for _ in 0..2 {
            sleep(Duration::from_millis(750)).await;
            let msg = S2CMessage::DrawCard(self.cards.pop().unwrap());
            self.player_by_id(solo).send_message(msg).await;
        }

        for _ in 0..2 {
            self.player_by_id(solo)
                .send_message(S2CMessage::YourTurn)
                .await;
            let card = self.player_by_id(solo).expect_message_play_card().await;
            solo_trick.push(card);
        }

        //Get trump
        self.player_by_id(solo)
            .send_message(S2CMessage::SelectTrump)
            .await;

        let trump = self.player_by_id(solo).expect_message_trump().await;
        self.broadcast_message(S2CMessage::Trump(trump.clone()))
            .await;

        let mut last_winner = 0;

        //PLay 10 rounds
        for _ in 0..10 {
            let mut current_trick = vec![];

            for current_player in turn_order(last_winner) {
                self.player_by_id(current_player as i32)
                    .send_message(S2CMessage::YourTurn)
                    .await;

                let card = self
                    .player_by_id(current_player as i32)
                    .expect_message_play_card()
                    .await;

                self.broadcast_message(S2CMessage::PlayedCard(card.clone()))
                    .await;
                current_trick.push((card, current_player));
            }

            let trick_color = if current_trick
                .iter()
                .any(|c| c.0.suit == trump || c.0.rank == Rank::Jack)
            {
                trump.clone()
            } else {
                current_trick.first().unwrap().0.suit.clone()
            };

            last_winner = current_trick
                .iter()
                .filter(|c| c.0.suit == trick_color || c.0.rank == Rank::Jack)
                .max_by_key(|c| normal_rank_value(&c.0.rank))
                .map(|c| c.1)
                .unwrap();

            if last_winner == solo as usize {
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
            S2CMessage::GameOver(GameOverMessage {
                winner_id: Some(self.player_by_id(solo).id()),
                winner_points: solo_points,
                loser_points: duo_points,
            })
        } else if solo_points < duo_points {
            S2CMessage::GameOver(GameOverMessage {
                winner_id: Some(self.player_by_id(solo + 1).id()),
                winner_points: duo_points,
                loser_points: solo_points,
            })
        } else {
            S2CMessage::GameOver(GameOverMessage {
                winner_id: None,
                winner_points: 60,
                loser_points: 60,
            })
        };
        self.broadcast_message(won_msg).await;
    }

    async fn loosing_hand(&mut self) {
        todo!()
    }
}

fn new_shuffled_deck() -> Vec<Card> {
    use prelude::{Rank::*, Suit::*};

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

fn turn_order(start: usize) -> impl Iterator<Item = usize> {
    (0..3).map(move |i| (i + start) % 3)
}

fn evaluate_cards_value(cards: &Vec<Card>) -> u32 {
    cards.iter().map(|c| c.rank.value()).sum()
}
