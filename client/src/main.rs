use proto::*;
use slint::{Model, ModelRc, VecModel, Weak};
use std::rc::Rc;
use std::sync::{mpsc, Arc, Mutex};

slint::include_modules!();

mod networking;

struct Player {
    id: u32,
    name: String,
}

struct AppModel {
    pub player_id: u32,
    pub state: AppState,
    pub other_player: Vec<Player>,
    name: Option<String>,
}

impl AppModel {
    fn new() -> Self {
        Self {
            player_id: 0,
            state: AppState::Login,
            other_player: Vec::new(),
            name: None,
        }
    }

    fn submit_name(&mut self, name: String) {
        if !name.trim().is_empty() {
            self.state = AppState::Lobby;
            self.name = Some(name);
            self.state = AppState::Lobby;
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), slint::PlatformError> {
    let ui = MainWindow::new()?;
    let ui_weak = ui.as_weak();

    let app_model = Arc::new(Mutex::new(AppModel::new()));
    let hand_model = Rc::new(VecModel::from(Vec::<CardSlint>::new()));
    ui.set_hand(ModelRc::from(Rc::clone(&hand_model)));

    let sock_tx = networking::connect_to_server(Arc::clone(&app_model), ui_weak.clone());

    ui.on_play_card({
        let hand_model = Rc::clone(&hand_model);

        move |card| {
            println!("Playing card: {:?}", card);
            let index = hand_model.iter().position(|c| c == card);
            if let Some(i) = index {
                hand_model.remove(i);
            }
        }
    });

    ui.on_set_position({
        let hand_model = Rc::clone(&hand_model);

        move |from, to| {
            println!("Moving card from {} to {}", from, to);
            let card = hand_model.remove(from as usize);
            hand_model.insert(to as usize, card);
        }
    });

    ui.on_submit_name({
        let app_model = Arc::clone(&app_model);

        move |name| {
            if name == "" {
                return;
            }
            let mut app_model = app_model.lock().unwrap();
            app_model.submit_name(name.to_string());
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_name(name.clone().into());
                ui.set_app_state(AppState::Lobby);

                let _ = sock_tx.send(Message::Login(name.into()));
            }
        }
    });

    let weak_app = ui.as_weak();
    tokio::spawn(async move {
        main_loop(weak_app).await.unwrap();
    });

    ui.run()
}

async fn main_loop(ui: Weak<MainWindow>) -> Result<(), slint::PlatformError> {
    let cards_to_add = vec![
        CardSlint {
            suit: CardSuitSlint::Heart,
            rank: CardRankSlint::Seven,
        },
        CardSlint {
            suit: CardSuitSlint::Diamond,
            rank: CardRankSlint::Eight,
        },
        CardSlint {
            suit: CardSuitSlint::Clubs,
            rank: CardRankSlint::Nine,
        },
        CardSlint {
            suit: CardSuitSlint::Spade,
            rank: CardRankSlint::Ace,
        },
    ];

    for card in cards_to_add {
        let ui = ui.clone();
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(ui) = ui.upgrade() {
                let hand_model = ui.get_hand();
                let vec_model = hand_model
                    .as_any()
                    .downcast_ref::<VecModel<CardSlint>>()
                    .unwrap();

                vec_model.push(card);
            }
        });
        std::thread::sleep(std::time::Duration::from_millis(500));
    }

    Ok(())
}

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
