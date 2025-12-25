use anyhow::{Ok, Result};
use postcard::{from_bytes, to_stdvec};
use proto::*;
use rand::seq::SliceRandom;
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

    async fn send_message(&mut self, msg: Message) -> Result<(), anyhow::Error> {
        let serialized = to_stdvec(&msg).unwrap();
        self.tcp_stream.get_mut().write_all(&serialized).await?;
        Ok(())
    }
}

trait EvilGet<T> {
    fn evil_get(&mut self, index: usize) -> &mut T;
}

impl EvilGet<Player> for Vec<Player> {
    fn evil_get(&mut self, index: usize) -> &mut Player {
        self.get_mut(index).unwrap()
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

    players.evil_get(2).send_message(Message::Say).await?;

    Ok(())
}
