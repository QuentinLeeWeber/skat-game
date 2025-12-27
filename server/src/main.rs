use anyhow::{Error, Result};
use macros::message_types;
use postcard::{from_bytes, to_stdvec};
use proto::*;
use rand::seq::SliceRandom;
use std::result::Result::Ok;
use std::sync::Arc;
use std::{mem, vec};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tokio::time::{Duration, sleep};

#[derive(Debug)]
struct Player {
    id: u32,
    name: String,
    tcp_stream: BufReader<TcpStream>,
}

impl Player {
    async fn new(tcp_stream: TcpStream, id: u32) -> Self {
        let mut tcp_stream = BufReader::new(tcp_stream);

        let mut name = String::new();
        let _ = tcp_stream.read_line(&mut name).await.unwrap();
        Player {
            id: id as u32,
            name,
            tcp_stream,
        }
    }

    async fn send_message(&mut self, msg: Message) -> Result<(), Error> {
        let serialized = to_stdvec(&msg).unwrap();
        self.tcp_stream.get_mut().write_all(&serialized).await?;
        Ok(())
    }

    async fn read_message(&mut self) -> Result<Message, Error> {
        let mut buf = String::new();
        let _ = self.tcp_stream.read_line(&mut buf).await?;
        let msg: Message = from_bytes(&buf.as_bytes()).unwrap();
        Ok(msg)
    }

    #[message_types(Trump(Suit), PlayCard(Card), Bid(i32))]
    async fn expect_message(&mut self) -> Message {
        let message = loop {
            let message = self.read_message().await;
            match message {
                Ok(m) => break m,
                Err(e) => {
                    eprint!("{}", e);
                }
            }
        };
        message
    }
}

trait VecExt<T> {
    fn evil_get(&mut self, index: usize) -> &mut T;
    async fn broadcast_message(&mut self, msg: Message) -> Result<(), Error>;
}

impl VecExt<Player> for Vec<Player> {
    fn evil_get(&mut self, id: usize) -> &mut Player {
        self.iter_mut().find(|p| p.id == id as u32).unwrap()
    }

    async fn broadcast_message(&mut self, msg: Message) -> Result<(), Error> {
        for player in &mut self.iter_mut() {
            player.send_message(msg.clone()).await?;
        }
        Ok(())
    }
}

type SharedPlayers = Arc<Mutex<Vec<Player>>>;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    println!("starting server...");

    let listener = TcpListener::bind("127.0.0.1:6969").await?;
    loop {
        let players = Arc::new(Mutex::new(vec![]));
        let mut threads = vec![];

        for i in 0..2 {
            let mut players = Arc::clone(&players);
            let (stream, addr) = listener.accept().await?;

            println!("client with ip: {}, joined!", addr);

            threads.push(tokio::spawn(async move {
                let mut new_player = Player::new(stream, i).await;
                let msg = Message::ConfirmJoin(i);
                new_player.send_message(msg).await.unwrap();

                let mut players_lock = players.lock().await;
                players_lock.push(new_player);

                let msgs = players_lock
                    .iter_mut()
                    .map(|player| {
                        Message::PlayerJoin(PlayerJoinMessage {
                            id: player.id,
                            name: player.name.clone(),
                        })
                    })
                    .collect::<Vec<_>>();

                mem::drop(players_lock);

                for msg in msgs {
                    broadcast_message_shared(&mut players, msg).await.unwrap();
                }
            }));
        }

        for t in threads {
            let _ = t.await?;
        }

        let mutex = Arc::try_unwrap(players).expect("unreachable");
        let players = mutex.into_inner();
        tokio::spawn(async move { play_game(players).await });
    }
}

async fn broadcast_message_shared(
    players: &mut SharedPlayers,
    msg: Message,
) -> Result<(), anyhow::Error> {
    let serialized = to_stdvec(&msg).unwrap();
    let mut players = players.lock().await;
    for player in &mut players.iter_mut() {
        player.tcp_stream.write_all(&serialized).await?;
    }
    Ok(())
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

async fn play_game(mut players: Vec<Player>) -> Result<(), anyhow::Error> {
    let mut cards = new_shuffled_deck();

    for _ in 0..10 {
        let card = cards.pop().unwrap();
        for player in &mut players {
            let msg = Message::DrawCard(card.clone());
            let serialized = to_stdvec(&msg).unwrap();
            player.tcp_stream.write_all(&serialized).await?;
        }
        sleep(Duration::from_millis(300)).await
    }

    players.evil_get(0).send_message(Message::Hear).await?;
    players.evil_get(1).send_message(Message::Say).await?;
    players
        .evil_get(2)
        .send_message(Message::SayFurther)
        .await?;

    match bid(&mut players).await? {
        Some(i) => {
            normal_game(players, i, cards).await?;
        }
        None => {
            loosing_hand(players).await?;
        }
    }

    Ok(())
}

fn turn_order(start: usize) -> impl Iterator<Item = usize> {
    (0..3).map(move |i| (i + start) % 3)
}

async fn normal_game(
    mut players: Vec<Player>,
    solo: usize,
    mut skat: Vec<Card>,
) -> Result<(), Error> {
    //Broadcast Played Game
    for i in 0..3 {
        let p = players.evil_get(i);
        if i == solo {
            p.send_message(Message::PlayNormalSolo).await?;
        } else {
            p.send_message(Message::PlayNormalDuo).await?;
        }
    }

    let mut solo_trick = vec![];
    let mut duo_trick = vec![];

    //Skat
    for _ in 0..2 {
        let msg = Message::DrawCard(skat.pop().unwrap());
        players.evil_get(solo).send_message(msg).await?
    }

    for _ in 0..2 {
        let card = players.evil_get(solo).expect_message_play_card().await;
        solo_trick.push(card);
    }

    //Get trump
    let trump = players.evil_get(solo).expect_message_trump().await;
    players
        .broadcast_message(Message::Trump(trump.clone()))
        .await?;

    let mut last_winner = 0;

    //PLay 10 rounds
    for _ in 0..10 {
        let mut current_trick = vec![];

        for current_player in turn_order(last_winner) {
            players
                .evil_get(current_player)
                .send_message(Message::YourTurn)
                .await?;

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
    players.broadcast_message(won_msg).await?;

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

async fn bid(players: &mut Vec<Player>) -> Result<Option<usize>, Error> {
    let mut bid;
    let mut highest_bider = None;
    for i in [1, 2, 0] {
        loop {
            let val = players.evil_get(i).expect_message_bid().await;
            if val == 0 {
                break;
            } else {
                bid = val;
                highest_bider = Some(i);
                players.broadcast_message(Message::NewBid(bid)).await?;
            }
        }
    }

    Ok(highest_bider)
}
