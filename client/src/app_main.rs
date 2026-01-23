use crate::networking;
use prelude::*;
use slint::{Model, ModelRc, VecModel};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

slint::include_modules!();

#[derive(Clone)]
pub struct Player {
    pub id: u32,
    pub name: String,
}

pub struct AppModel {
    pub player_id: u32,
    pub state: AppState,
    pub other_player: Vec<Player>,
    pub trump: Option<Suit>,
    pub name: Option<String>,
}

impl AppModel {
    fn new() -> Self {
        Self {
            player_id: 0,
            state: AppState::Login,
            other_player: Vec::new(),
            trump: None,
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
pub async fn main() -> Result<(), slint::PlatformError> {
    let ui = MainWindow::new()?;
    let ui_weak = ui.as_weak();

    let app_model = Arc::new(Mutex::new(AppModel::new()));
    let hand_model = Rc::new(VecModel::from(Vec::<CardSlint>::new()));
    ui.set_hand(ModelRc::from(Rc::clone(&hand_model)));
    let players_model = Rc::new(VecModel::from(Vec::<PlayerSlint>::new()));
    ui.set_players(ModelRc::from(Rc::clone(&players_model)));
    let table_cards = Rc::new(VecModel::from(Vec::<CardSlint>::new()));
    ui.set_table_cards(ModelRc::from(Rc::clone(&table_cards)));

    let sock_tx = networking::connect_to_server(Arc::clone(&app_model), ui_weak.clone());

    ui.on_play_card({
        let hand_model = Rc::clone(&hand_model);
        let table_cards = Rc::clone(&table_cards);
        let app_model = Arc::clone(&app_model);
        let sock_tx = sock_tx.clone();
        let ui_weak = ui_weak.clone();

        move |card| {
            let played_card: Card = { card.clone().into() };
            let app_lock = app_model.lock().unwrap();
            let hand_model_into: Vec<Card> = hand_model.iter().map(|c| c.into()).collect();
            let table_cards_into: Vec<Card> = table_cards.iter().map(|c| c.into()).collect();

            if !possible_moves(&hand_model_into, &table_cards_into, &app_lock.trump)
                .iter()
                .any(|c| *c == played_card)
            {
                return;
            }

            if app_lock.state == AppState::Game {
                let index = hand_model.iter().position(|c| c == card);
                if let Some(i) = index {
                    hand_model.remove(i);
                }
                if let Some(ui) = ui_weak.upgrade() {
                    ui.set_my_turn(false);
                }
                let _ = sock_tx.send(C2SMessage::PlayCard(card.into()));
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
        let sock_tx = sock_tx.clone();
        let ui_weak = ui_weak.clone();

        move |name| {
            if name.is_empty() {
                return;
            }
            let mut app_model = app_model.lock().unwrap();
            app_model.submit_name(name.to_string());
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_name(name.clone());
                ui.set_app_state(AppState::Lobby);

                let _ = sock_tx.send(C2SMessage::Login(name.into()));
            }
        }
    });

    ui.on_join_game({
        let sock_tx = sock_tx.clone();
        move || {
            let _ = sock_tx.send(C2SMessage::JoinGame);
        }
    });

    ui.on_add_npc({
        let sock_tx = sock_tx.clone();
        move || {
            let _ = sock_tx.send(C2SMessage::AddNPC);
        }
    });

    ui.on_bid_further({
        let sock_tx = sock_tx.clone();
        move || {
            let _ = sock_tx.send(C2SMessage::Bid(1));
        }
    });

    ui.on_pass({
        let sock_tx = sock_tx.clone();
        move || {
            let _ = sock_tx.send(C2SMessage::Bid(0));
        }
    });

    ui.on_selected_trump({
        let sock_tx = sock_tx.clone();
        move |suit| {
            let _ = sock_tx.send(C2SMessage::Trump(suit.into()));
        }
    });

    ui.on_return_to_lobby({
        let ui_weak = ui_weak.clone();
        let app_model = Arc::clone(&app_model);
        let hand_model = Rc::clone(&hand_model);
        let players_model = Rc::clone(&players_model);
        let table_cards = Rc::clone(&table_cards);
        move || {
            let mut app_model = app_model.lock().unwrap();
            app_model.state = AppState::Lobby;
            app_model.other_player.clear();
            app_model.trump = None;
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_app_state(AppState::Lobby);
                ui.set_game_trump(CardSuitSlint::None);
                ui.set_game_value("0".into());
                hand_model.clear();
                players_model.clear();
                table_cards.clear();
            }
        }
    });

    ui.run()
}
